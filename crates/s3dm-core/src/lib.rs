use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use aws_sdk_s3::config::retry::RetryConfig;
use aws_sdk_s3::config::timeout::TimeoutConfig;
use aws_sdk_s3::config::{BehaviorVersion, Credentials, Region};
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::types::{Delete, ObjectIdentifier};
use chrono::{DateTime, TimeZone, Utc};
use thiserror::Error;
use tokio::io::AsyncWriteExt;

fn to_chrono(d: &aws_smithy_types::DateTime) -> Option<DateTime<Utc>> {
    let secs_f64 = d.as_secs_f64();
    let secs = secs_f64 as i64;
    let nsecs = ((secs_f64 - secs as f64) * 1_000_000_000.0) as u32;
    Utc.timestamp_opt(secs, nsecs).single()
}

#[derive(Debug, Clone)]
pub struct S3Object {
    pub key: String,
    pub size: i64,
    pub last_modified: Option<DateTime<Utc>>,
    pub is_folder: bool,
    pub etag: Option<String>,
}

#[derive(Debug, Clone)]
pub struct S3Bucket {
    pub name: String,
    pub creation_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct ObjectListResult {
    pub objects: Vec<S3Object>,
    pub common_prefixes: Vec<String>,
    pub is_truncated: bool,
    pub continuation_token: Option<String>,
}

#[derive(Debug, Clone, Error)]
pub enum CoreError {
    #[error("S3 错误: {0}")]
    S3(String),
    #[error("连接失败: {0}")]
    Connection(String),
    #[error("未找到: {0}")]
    NotFound(String),
    #[error("IO 错误: {0}")]
    Io(String),
}

#[derive(Debug)]
pub struct S3Manager {
    inner: Arc<S3Inner>,
}

#[derive(Debug)]
struct S3Inner {
    client: Mutex<aws_sdk_s3::Client>,
    config: aws_sdk_s3::config::Config,
}

// Manual Clone: config is Clone, client is Clone (Arc clone)
impl Clone for S3Manager {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl S3Manager {
    async fn run_with_retry<F, Fut, T>(
        &self,
        op_name: &'static str,
        mut f: F,
    ) -> Result<T, CoreError>
    where
        F: FnMut(aws_sdk_s3::Client) -> Fut,
        Fut: Future<Output = Result<T, CoreError>>,
    {
        let max_attempts = 3;
        let mut last_err = None;
        for attempt in 1..=max_attempts {
            let client = self.inner.client.lock().expect("lock poisoned").clone();
            match f(client).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    let is_dispatch = e.to_string().contains("dispatch failure");
                    if is_dispatch && attempt < max_attempts {
                        log::warn!(
                            "{} failed on attempt {}/{}: dispatch failure, retrying...",
                            op_name,
                            attempt,
                            max_attempts
                        );
                        last_err = Some(e);
                        let new_client = aws_sdk_s3::Client::from_conf(self.inner.config.clone());
                        *self.inner.client.lock().expect("lock poisoned") = new_client;
                        tokio::time::sleep(Duration::from_millis(500 * attempt)).await;
                    } else {
                        return Err(e);
                    }
                }
            }
        }
        Err(last_err.unwrap())
    }

    pub fn new(
        endpoint: &str,
        region: &str,
        access_key_id: &str,
        secret_access_key: &str,
        force_path_style: bool,
    ) -> Self {
        log::info!(
            "Creating S3 client endpoint={} region={} force_path_style={}",
            endpoint,
            region,
            force_path_style
        );
        let creds = Credentials::new(access_key_id, secret_access_key, None, None, "s3dm");

        let config = aws_sdk_s3::config::Config::builder()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new(region.to_string()))
            .endpoint_url(endpoint)
            .credentials_provider(creds)
            .force_path_style(force_path_style)
            .retry_config(RetryConfig::standard().with_max_attempts(3))
            .timeout_config(
                TimeoutConfig::builder()
                    .connect_timeout(Duration::from_secs(10))
                    .operation_attempt_timeout(Duration::from_secs(10))
                    .operation_timeout(Duration::from_secs(10))
                    .read_timeout(Duration::from_secs(10))
                    .build(),
            )
            .build();
        let client = aws_sdk_s3::Client::from_conf(config.clone());

        log::info!("S3 client created successfully endpoint={}", endpoint);
        Self {
            inner: Arc::new(S3Inner {
                client: Mutex::new(client),
                config,
            }),
        }
    }

    /// 测试 S3 连接是否可用
    ///
    /// 创建临时客户端并调用 `list_buckets`，用于在不建立持久连接的情况下
    /// 验证端点、凭据与网络连通性。成功返回 `Ok(())`，失败返回 `Err`。
    pub async fn test_connection(
        endpoint: &str,
        region: &str,
        access_key_id: &str,
        secret_access_key: &str,
        force_path_style: bool,
    ) -> Result<(), CoreError> {
        log::info!("Testing S3 connection endpoint={}", endpoint);
        let manager = S3Manager::new(
            endpoint,
            region,
            access_key_id,
            secret_access_key,
            force_path_style,
        );
        match manager.list_buckets().await {
            Ok(_) => {
                log::info!("Connection test succeeded");
                Ok(())
            }
            Err(e) => {
                log::error!("Connection test failed: {}", e);
                Err(e)
            }
        }
    }

    pub async fn list_buckets(&self) -> Result<Vec<S3Bucket>, CoreError> {
        log::info!("Listing all buckets");
        self.run_with_retry("list_buckets", move |client| async move {
            let resp = client.list_buckets().send().await.map_err(|e| {
                log::error!("Failed to list buckets: {}", e);
                CoreError::S3(e.to_string())
            })?;

            let buckets: Vec<S3Bucket> = resp
                .buckets()
                .iter()
                .map(|b| {
                    let name = b.name().unwrap_or("").to_string();
                    log::debug!("Found bucket: {}", name);
                    S3Bucket {
                        name,
                        creation_date: b.creation_date().and_then(to_chrono),
                    }
                })
                .collect();

            log::info!("Successfully listed {} buckets", buckets.len());
            Ok(buckets)
        })
        .await
    }

    pub async fn list_objects(
        &self,
        bucket: &str,
        prefix: &str,
        delimiter: &str,
        max_keys: i32,
        continuation_token: Option<&str>,
    ) -> Result<ObjectListResult, CoreError> {
        log::info!(
            "Listing objects bucket={} prefix={:?} delimiter={} max_keys={}",
            bucket,
            prefix,
            delimiter,
            max_keys
        );
        self.run_with_retry("list_objects", move |client| async move {
            let mut req = client
                .list_objects_v2()
                .bucket(bucket)
                .prefix(prefix)
                .delimiter(delimiter)
                .max_keys(max_keys);

            if let Some(token) = continuation_token {
                log::debug!("Using pagination token: {}", token);
                req = req.continuation_token(token);
            }

            let resp = req.send().await.map_err(|e| {
                log::error!(
                    "Failed to list objects bucket={} prefix={:?}: {}",
                    bucket,
                    prefix,
                    e
                );
                CoreError::S3(e.to_string())
            })?;

            let objects: Vec<S3Object> = resp
                .contents()
                .iter()
                .map(|o| {
                    let key = o.key().unwrap_or("").to_string();
                    log::debug!("Found object: {}", key);
                    S3Object {
                        key,
                        size: o.size().unwrap_or(0),
                        last_modified: o.last_modified().and_then(to_chrono),
                        is_folder: false,
                        etag: o.e_tag().map(|s| s.to_string()),
                    }
                })
                .collect();

            let common_prefixes: Vec<String> = resp
                .common_prefixes()
                .iter()
                .filter_map(|p| {
                    let prefix = p.prefix().map(|s| s.to_string());
                    if let Some(ref p) = prefix {
                        log::debug!("Found common prefix (folder): {}", p);
                    }
                    prefix
                })
                .collect();

            let is_truncated = resp.is_truncated().unwrap_or(false);
            let continuation_token = resp.next_continuation_token().map(|s| s.to_string());

            log::info!(
                "Successfully listed objects: {} files, {} folders, truncated: {}",
                objects.len(),
                common_prefixes.len(),
                is_truncated
            );

            Ok(ObjectListResult {
                objects,
                common_prefixes,
                is_truncated,
                continuation_token,
            })
        })
        .await
    }

    pub async fn delete_object(&self, bucket: &str, key: &str) -> Result<(), CoreError> {
        log::info!("Deleting object bucket={} key={}", bucket, key);
        self.run_with_retry("delete_object", move |client| async move {
            client
                .delete_object()
                .bucket(bucket)
                .key(key)
                .send()
                .await
                .map_err(|e| {
                    log::error!(
                        "Failed to delete object bucket={} key={}: {}",
                        bucket,
                        key,
                        e
                    );
                    CoreError::S3(e.to_string())
                })?;
            log::debug!("Object deleted successfully bucket={} key={}", bucket, key);
            Ok(())
        })
        .await
    }

    pub async fn delete_prefix(&self, bucket: &str, prefix: &str) -> Result<(), CoreError> {
        log::info!(
            "Deleting objects under prefix bucket={} prefix={}",
            bucket,
            prefix
        );
        self.run_with_retry("delete_prefix", move |client| async move {
            let mut keys: Vec<String> = Vec::new();
            let mut token: Option<String> = None;

            loop {
                let mut req = client
                    .list_objects_v2()
                    .bucket(bucket)
                    .prefix(prefix)
                    .max_keys(1000);
                if let Some(ref t) = token {
                    req = req.continuation_token(t);
                }
                let resp = req
                    .send()
                    .await
                    .map_err(|e| CoreError::S3(format!("list objects failed: {}", e)))?;

                for obj in resp.contents() {
                    if let Some(key) = obj.key() {
                        keys.push(key.to_string());
                    }
                }
                if !resp.is_truncated().unwrap_or(false) {
                    break;
                }
                token = resp.next_continuation_token().map(|s| s.to_string());
            }

            if keys.is_empty() {
                log::info!("No objects found under prefix={}", prefix);
                return Ok(());
            }

            log::info!("Deleting {} objects under prefix={}", keys.len(), prefix);
            for chunk in keys.chunks(1000) {
                let objects: Vec<ObjectIdentifier> = chunk
                    .iter()
                    .map(|k| ObjectIdentifier::builder().key(k).build().unwrap())
                    .collect();
                let delete = Delete::builder()
                    .set_objects(Some(objects))
                    .build()
                    .map_err(|e| CoreError::S3(format!("build delete request failed: {}", e)))?;

                client
                    .delete_objects()
                    .bucket(bucket)
                    .delete(delete)
                    .send()
                    .await
                    .map_err(|e| CoreError::S3(format!("batch delete failed: {}", e)))?;
            }

            log::info!("Successfully deleted all objects under prefix={}", prefix);
            Ok(())
        })
        .await
    }

    /// 下载对象并流式写入本地文件，避免将整个对象载入内存。
    pub async fn get_object_to_file(
        &self,
        bucket: &str,
        key: &str,
        dest: &Path,
    ) -> Result<u64, CoreError> {
        log::info!(
            "Downloading object bucket={} key={} -> {:?}",
            bucket,
            key,
            dest
        );
        self.run_with_retry("get_object", move |client| {
            let dest = dest.to_path_buf();
            let key = key.to_string();
            async move {
                let log_key = key.clone();
                let resp = client
                    .get_object()
                    .bucket(bucket)
                    .key(key)
                    .send()
                    .await
                    .map_err(|e| {
                        log::error!(
                            "Failed to download object bucket={} key={}: {}",
                            bucket,
                            log_key,
                            e
                        );
                        CoreError::S3(e.to_string())
                    })?;

                let mut file = tokio::fs::File::create(&dest)
                    .await
                    .map_err(|e| CoreError::Io(format!("创建文件失败 {:?}: {}", dest, e)))?;

                let mut reader = resp.body.into_async_read();
                let written = tokio::io::copy(&mut reader, &mut file)
                    .await
                    .map_err(|e| CoreError::Io(format!("写入文件失败 {:?}: {}", dest, e)))?;
                file.flush()
                    .await
                    .map_err(|e| CoreError::Io(format!("刷新文件失败 {:?}: {}", dest, e)))?;

                log::info!(
                    "Object downloaded successfully bucket={} key={} size={}",
                    bucket,
                    log_key,
                    written
                );
                Ok(written)
            }
        })
        .await
    }

    /// 下载对象并流式写入本地文件，同时通过回调上报下载进度。
    ///
    /// `on_progress` 参数为 `(已下载字节数, 总大小)`，总大小取自响应的
    /// `Content-Length`，若服务端未返回则为 `None`（不确定态）。
    ///
    /// 注意：内部带重试，若发生重试将从 0 重新写入并重新上报进度。
    pub async fn get_object_to_file_with_progress<F>(
        &self,
        bucket: &str,
        key: &str,
        dest: &Path,
        on_progress: F,
    ) -> Result<u64, CoreError>
    where
        F: Fn(u64, Option<u64>) + Send + Sync,
    {
        log::info!(
            "Downloading object (with progress) bucket={} key={} -> {:?}",
            bucket,
            key,
            dest
        );
        let on_progress = &on_progress;
        self.run_with_retry("get_object", move |client| {
            let dest = dest.to_path_buf();
            let key = key.to_string();
            async move {
                let log_key = key.clone();
                let resp = client
                    .get_object()
                    .bucket(bucket)
                    .key(key)
                    .send()
                    .await
                    .map_err(|e| {
                        log::error!(
                            "Failed to download object bucket={} key={}: {}",
                            bucket,
                            log_key,
                            e
                        );
                        CoreError::S3(e.to_string())
                    })?;

                let total = resp.content_length().map(|v| v as u64);

                let mut file = tokio::fs::File::create(&dest)
                    .await
                    .map_err(|e| CoreError::Io(format!("创建文件失败 {:?}: {}", dest, e)))?;

                let mut reader = resp.body.into_async_read();
                let mut buf = vec![0u8; 64 * 1024];
                let mut written: u64 = 0;
                // 初始进度（0），确保 UI 立即进入下载态
                on_progress(0, total);
                loop {
                    let n = tokio::io::AsyncReadExt::read(&mut reader, &mut buf)
                        .await
                        .map_err(|e| CoreError::Io(format!("读取响应失败: {}", e)))?;
                    if n == 0 {
                        break;
                    }
                    file.write_all(&buf[..n])
                        .await
                        .map_err(|e| CoreError::Io(format!("写入文件失败 {:?}: {}", dest, e)))?;
                    written += n as u64;
                    on_progress(written, total);
                }
                file.flush()
                    .await
                    .map_err(|e| CoreError::Io(format!("刷新文件失败 {:?}: {}", dest, e)))?;

                log::info!(
                    "Object downloaded successfully bucket={} key={} size={}",
                    bucket,
                    log_key,
                    written
                );
                Ok(written)
            }
        })
        .await
    }

    /// 下载对象全部内容到内存（仅用于小文件预览）。
    ///
    /// 大对象请使用 `get_object_to_file_with_progress` 流式写入磁盘，
    /// 避免将整个对象载入内存造成压力。
    pub async fn get_object_bytes(&self, bucket: &str, key: &str) -> Result<Vec<u8>, CoreError> {
        log::info!("Fetching object bytes bucket={} key={}", bucket, key);
        self.run_with_retry("get_object", move |client| {
            let key = key.to_string();
            async move {
                let log_key = key.clone();
                let resp = client
                    .get_object()
                    .bucket(bucket)
                    .key(key)
                    .send()
                    .await
                    .map_err(|e| {
                        log::error!(
                            "Failed to fetch object bytes bucket={} key={}: {}",
                            bucket,
                            log_key,
                            e
                        );
                        CoreError::S3(e.to_string())
                    })?;
                let data = resp
                    .body
                    .collect()
                    .await
                    .map_err(|e| CoreError::Io(format!("读取响应失败: {}", e)))?
                    .into_bytes()
                    .to_vec();
                log::info!(
                    "Object bytes fetched bucket={} key={} size={}",
                    bucket,
                    log_key,
                    data.len()
                );
                Ok(data)
            }
        })
        .await
    }

    /// 创建一个空对象以模拟“文件夹”（S3 无真实目录）。
    pub async fn create_folder(&self, bucket: &str, key: &str) -> Result<(), CoreError> {
        log::info!("Creating folder marker bucket={} key={}", bucket, key);
        self.run_with_retry("put_object", move |client| {
            let key = key.to_string();
            async move {
                let log_key = key.clone();
                client
                    .put_object()
                    .bucket(bucket)
                    .key(key)
                    .body(ByteStream::from(Vec::new()))
                    .send()
                    .await
                    .map_err(|e| {
                        log::error!(
                            "Failed to create folder marker bucket={} key={}: {}",
                            bucket,
                            log_key,
                            e
                        );
                        CoreError::S3(e.to_string())
                    })?;
                Ok(())
            }
        })
        .await
    }
    pub async fn put_object_from_file(
        &self,
        bucket: &str,
        key: &str,
        src: &Path,
    ) -> Result<(), CoreError> {
        let size = tokio::fs::metadata(src).await.map(|m| m.len()).unwrap_or(0);
        log::info!(
            "Uploading object bucket={} key={} size={} from {:?}",
            bucket,
            key,
            size,
            src
        );
        self.run_with_retry("put_object", move |client| {
            let src = src.to_path_buf();
            async move {
                let stream = ByteStream::from_path(&src)
                    .await
                    .map_err(|e| CoreError::Io(format!("打开文件失败 {:?}: {}", src, e)))?;

                client
                    .put_object()
                    .bucket(bucket)
                    .key(key)
                    .body(stream)
                    .send()
                    .await
                    .map_err(|e| {
                        log::error!(
                            "Failed to upload object bucket={} key={}: {}",
                            bucket,
                            key,
                            e
                        );
                        CoreError::S3(e.to_string())
                    })?;
                log::info!("Object uploaded successfully bucket={} key={}", bucket, key);
                Ok(())
            }
        })
        .await
    }

    pub async fn head_object(&self, bucket: &str, key: &str) -> Result<S3Object, CoreError> {
        log::debug!("Querying object metadata bucket={} key={}", bucket, key);
        self.run_with_retry("head_object", move |client| async move {
            let resp = client
                .head_object()
                .bucket(bucket)
                .key(key)
                .send()
                .await
                .map_err(|e| {
                    log::error!(
                        "Failed to query object metadata bucket={} key={}: {}",
                        bucket,
                        key,
                        e
                    );
                    CoreError::S3(e.to_string())
                })?;

            let size = resp.content_length().unwrap_or(0) as i64;
            log::debug!(
                "Object metadata: bucket={} key={} size={}",
                bucket,
                key,
                size
            );

            Ok(S3Object {
                key: key.to_string(),
                size,
                last_modified: resp.last_modified().and_then(to_chrono),
                is_folder: false,
                etag: resp.e_tag().map(|s| s.to_string()),
            })
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aws_smithy_types::DateTime as SmithyDateTime;

    #[test]
    fn to_chrono_converts_valid_timestamp() {
        // 1704158645 -> 2024-01-02T01:24:05Z
        let secs = 1_704_158_645i64;
        let d = SmithyDateTime::from_secs_and_nanos(secs, 0);
        let dt = to_chrono(&d).expect("should convert");
        assert_eq!(dt.timestamp(), secs);
        assert_eq!(dt.to_rfc3339(), "2024-01-02T01:24:05+00:00");
    }

    #[test]
    fn to_chrono_preserves_nanos() {
        let d = SmithyDateTime::from_secs_and_nanos(1_000, 500);
        let dt = to_chrono(&d).expect("should convert");
        assert_eq!(dt.timestamp_subsec_nanos(), 500);
    }

    #[test]
    fn core_error_messages_are_localized_strings() {
        let e = CoreError::NotFound("bucket-x".to_string());
        assert!(e.to_string().contains("bucket-x"));
        let e = CoreError::Connection("timeout".to_string());
        assert!(e.to_string().contains("timeout"));
        let e = CoreError::S3("aws error".to_string());
        assert!(e.to_string().contains("aws error"));
    }

    #[test]
    fn object_list_result_defaults() {
        let r = ObjectListResult {
            objects: vec![],
            common_prefixes: vec!["a/".to_string()],
            is_truncated: false,
            continuation_token: None,
        };
        assert!(!r.is_truncated);
        assert_eq!(r.common_prefixes.len(), 1);
    }

    #[test]
    fn s3_manager_is_cloneable() {
        // new() 不应 panic；克隆应共享底层客户端
        let m = S3Manager::new("https://s3.example.com", "us-east-1", "ak", "sk", true);
        let _cloned = m.clone();
    }
}
