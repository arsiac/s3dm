//! S3 客户端核心实现
//!
//! 封装 aws-sdk-s3 的 `Client`，提供连接管理、桶/对象列举、删除、
//! 上传、下载、元数据查询等操作，并在底层统一处理重试与错误映射。

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use aws_sdk_s3::config::retry::RetryConfig;
use aws_sdk_s3::config::timeout::TimeoutConfig;
use aws_sdk_s3::config::{BehaviorVersion, Credentials, Region};
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::types::{Delete, ObjectIdentifier};
use tokio::io::AsyncWriteExt;

use crate::http::build_shared_http_client;
use crate::types::{CoreError, ObjectListResult, S3Bucket, S3Object, to_chrono};

/// 将 aws-sdk-s3 的 `SdkError` 分类为 `CoreError`，并据其结构化类型
/// 判定是否可重试（分发失败 / 超时 / 响应中断 / 5xx / 429 限流）。
///
/// 相比匹配错误文案（`e.to_string().contains(...)`），直接依据 `SdkError`
/// 的变体与 HTTP 状态码判断，更稳定，不受 SDK 文案变化影响。
fn map_sdk_error<E>(
    err: &aws_smithy_runtime_api::client::result::SdkError<
        E,
        aws_smithy_runtime_api::client::orchestrator::HttpResponse,
    >,
) -> CoreError {
    use aws_smithy_runtime_api::client::result::SdkError;

    let msg = err.to_string();
    let retryable = match err {
        // 连接分发失败（DNS/连接/TLS 等）与超时：网络抖动，值得重试
        SdkError::DispatchFailure(_) | SdkError::TimeoutError(_) => true,
        // 响应读取过程中断：可重试
        SdkError::ResponseError(_) => true,
        // 服务端返回错误：仅 5xx / 429 限流可重试
        SdkError::ServiceError(ctx) => {
            let status = ctx.raw().status().as_u16();
            status >= 500 || status == 429
        }
        _ => false,
    };
    if retryable {
        CoreError::S3Retryable(msg)
    } else {
        CoreError::S3(msg)
    }
}

/// 确保目标文件的父目录存在（不存在则递归创建）。
///
/// 下载写文件前调用，避免因子目录缺失导致 `File::create` 失败
/// （尤其在按对象 Key 层级重建目录结构时）。
async fn ensure_parent_dir(dest: &Path) -> Result<(), CoreError> {
    if let Some(parent) = dest.parent()
        && !parent.as_os_str().is_empty()
    {
        tokio::fs::create_dir_all(parent).await.map_err(|e| {
            CoreError::Io(format!("failed to create directory {:?}: {}", parent, e))
        })?;
    }
    Ok(())
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
                    // 依据结构化错误类型判断是否可重试（见 map_sdk_error），
                    // 不再匹配错误文案。可重试错误会重建 client 后再试。
                    if e.is_retryable() && attempt < max_attempts {
                        log::warn!(
                            "{} failed on attempt {}/{}: {} (retryable), retrying...",
                            op_name,
                            attempt,
                            max_attempts,
                            e
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
        skip_tls_verify: bool,
    ) -> Self {
        log::info!(
            "Creating S3 client endpoint={} region={} force_path_style={} skip_tls_verify={}",
            endpoint,
            region,
            force_path_style,
            skip_tls_verify
        );
        let creds = Credentials::new(access_key_id, secret_access_key, None, None, "s3dm");

        let config = aws_sdk_s3::config::Config::builder()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new(region.to_string()))
            .endpoint_url(endpoint)
            .credentials_provider(creds)
            .force_path_style(force_path_style)
            .http_client(build_shared_http_client(skip_tls_verify))
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
        skip_tls_verify: bool,
    ) -> Result<(), CoreError> {
        log::info!("Testing S3 connection endpoint={}", endpoint);
        let manager = S3Manager::new(
            endpoint,
            region,
            access_key_id,
            secret_access_key,
            force_path_style,
            skip_tls_verify,
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
                map_sdk_error(&e)
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
                map_sdk_error(&e)
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
                    map_sdk_error(&e)
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
                        map_sdk_error(&e)
                    })?;

                ensure_parent_dir(&dest).await?;
                let mut file = tokio::fs::File::create(&dest).await.map_err(|e| {
                    CoreError::Io(format!("failed to create file {:?}: {}", dest, e))
                })?;

                let mut reader = resp.body.into_async_read();
                let written = tokio::io::copy(&mut reader, &mut file).await.map_err(|e| {
                    CoreError::Io(format!("failed to write file {:?}: {}", dest, e))
                })?;
                file.flush().await.map_err(|e| {
                    CoreError::Io(format!("failed to flush file {:?}: {}", dest, e))
                })?;

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
    /// `Content-Length`（断点续传时为对象完整大小），若服务端未返回则为
    /// `None`（不确定态）。
    ///
    /// 支持断点续传：内部带重试，若某次尝试中途失败，下次尝试会依据已落盘
    /// 的字节数发起 `Range: bytes=N-` 请求，从断点处继续追加写入，避免从头
    /// 重下。若服务端不支持 Range（返回 200 而非 206），则回退为从头重写。
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
        use std::sync::atomic::{AtomicU64, Ordering};

        log::info!(
            "Downloading object (with progress) bucket={} key={} -> {:?}",
            bucket,
            key,
            dest
        );
        let on_progress = &on_progress;
        // 跨重试尝试共享的“已落盘字节数”，用于断点续传
        let resume_from = Arc::new(AtomicU64::new(0));
        self.run_with_retry("get_object", move |client| {
            let dest = dest.to_path_buf();
            let key = key.to_string();
            let resume_from = resume_from.clone();
            async move {
                let log_key = key.clone();
                let start = resume_from.load(Ordering::SeqCst);

                let mut req = client.get_object().bucket(bucket).key(key);
                if start > 0 {
                    log::info!(
                        "Resuming download bucket={} key={} from offset={}",
                        bucket,
                        log_key,
                        start
                    );
                    req = req.range(format!("bytes={}-", start));
                }
                let resp = req.send().await.map_err(|e| {
                    log::error!(
                        "Failed to download object bucket={} key={}: {}",
                        bucket,
                        log_key,
                        e
                    );
                    map_sdk_error(&e)
                })?;

                // 判断服务端是否接受了 Range（206 Partial Content 会带 Content-Range）。
                let is_partial = resp.content_range().is_some();
                let content_length = resp.content_length().map(|v| v as u64);

                // 确定起始偏移与文件打开模式：
                // - 服务端接受续传：追加到已有文件，total = start + 剩余长度
                // - 未接受（或首次请求）：从头覆盖写
                let (mut written, total, file) = if start > 0 && is_partial {
                    let file = tokio::fs::OpenOptions::new()
                        .append(true)
                        .open(&dest)
                        .await
                        .map_err(|e| {
                            CoreError::Io(format!(
                                "failed to open file for append {:?}: {}",
                                dest, e
                            ))
                        })?;
                    let total = content_length.map(|len| start + len);
                    (start, total, file)
                } else {
                    if start > 0 {
                        log::warn!(
                            "Server did not honor Range for bucket={} key={}, restarting from 0",
                            bucket,
                            log_key
                        );
                        resume_from.store(0, Ordering::SeqCst);
                    }
                    ensure_parent_dir(&dest).await?;
                    let file = tokio::fs::File::create(&dest).await.map_err(|e| {
                        CoreError::Io(format!("failed to create file {:?}: {}", dest, e))
                    })?;
                    (0u64, content_length, file)
                };
                let mut file = file;

                let mut reader = resp.body.into_async_read();
                let mut buf = vec![0u8; 64 * 1024];
                // 初始进度，确保 UI 立即进入下载态（续传时从已下载量起步）
                on_progress(written, total);
                loop {
                    let n = tokio::io::AsyncReadExt::read(&mut reader, &mut buf)
                        .await
                        .map_err(|e| CoreError::Io(format!("failed to read response: {}", e)))?;
                    if n == 0 {
                        break;
                    }
                    file.write_all(&buf[..n]).await.map_err(|e| {
                        CoreError::Io(format!("failed to write file {:?}: {}", dest, e))
                    })?;
                    written += n as u64;
                    // 记录已落盘量，供下次重试续传
                    resume_from.store(written, Ordering::SeqCst);
                    on_progress(written, total);
                }
                file.flush().await.map_err(|e| {
                    CoreError::Io(format!("failed to flush file {:?}: {}", dest, e))
                })?;

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
                        map_sdk_error(&e)
                    })?;
                let data = resp
                    .body
                    .collect()
                    .await
                    .map_err(|e| CoreError::Io(format!("failed to read response: {}", e)))?
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
                        map_sdk_error(&e)
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
                    .map_err(|e| CoreError::Io(format!("failed to open file {:?}: {}", src, e)))?;

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
                        map_sdk_error(&e)
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
                    map_sdk_error(&e)
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

    #[test]
    fn s3_manager_is_cloneable() {
        // new() 不应 panic；克隆应共享底层客户端
        let m = S3Manager::new(
            "https://s3.example.com",
            "us-east-1",
            "ak",
            "sk",
            true,
            false,
        );
        let _cloned = m.clone();
    }

    #[tokio::test]
    async fn ensure_parent_dir_creates_nested_dirs() {
        let base = std::env::temp_dir().join(format!("s3dm_epd_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let dest = base.join("a").join("b").join("c.txt");
        ensure_parent_dir(&dest).await.expect("should create dirs");
        assert!(dest.parent().unwrap().is_dir());
        // 幂等：再次调用不报错
        ensure_parent_dir(&dest).await.expect("idempotent");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[tokio::test]
    async fn ensure_parent_dir_noop_without_parent() {
        // 无父目录（纯文件名）不应报错
        ensure_parent_dir(Path::new("file.txt"))
            .await
            .expect("no parent is ok");
    }
}
