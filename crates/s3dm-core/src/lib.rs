use std::sync::Arc;

use aws_sdk_s3::config::{BehaviorVersion, Credentials, Region};
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::types::{Delete, ObjectIdentifier};
use chrono::{DateTime, TimeZone, Utc};
use thiserror::Error;

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
}

#[derive(Debug, Clone)]
pub struct S3Manager {
    client: aws_sdk_s3::Client,
    runtime: Arc<tokio::runtime::Runtime>,
}

impl S3Manager {
    fn run<F, T>(&self, f: F) -> T
    where
        F: std::future::Future<Output = T>,
    {
        self.runtime.block_on(f)
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
        let runtime = Arc::new(tokio::runtime::Runtime::new().unwrap());
        let creds = Credentials::new(access_key_id, secret_access_key, None, None, "s3dm");

        let client = runtime.block_on(async {
            let config = aws_sdk_s3::config::Config::builder()
                .behavior_version(BehaviorVersion::latest())
                .region(Region::new(region.to_string()))
                .endpoint_url(endpoint)
                .credentials_provider(creds)
                .force_path_style(force_path_style)
                .build();
            aws_sdk_s3::Client::from_conf(config)
        });

        log::info!("S3 client created successfully endpoint={}", endpoint);
        Self { client, runtime }
    }

    pub fn list_buckets(&self) -> Result<Vec<S3Bucket>, CoreError> {
        log::info!("Listing all buckets");
        self.run(async {
            let resp = self.client.list_buckets().send().await.map_err(|e| {
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
    }

    pub fn list_objects(
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
        self.run(async {
            let mut req = self
                .client
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
    }

    pub fn delete_object(&self, bucket: &str, key: &str) -> Result<(), CoreError> {
        log::info!("Deleting object bucket={} key={}", bucket, key);
        self.run(async {
            self.client
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
    }

    pub fn delete_prefix(&self, bucket: &str, prefix: &str) -> Result<(), CoreError> {
        log::info!(
            "Deleting objects under prefix bucket={} prefix={}",
            bucket,
            prefix
        );
        self.run(async {
            let mut keys: Vec<String> = Vec::new();
            let mut token: Option<String> = None;

            loop {
                let mut req = self
                    .client
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

                self.client
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
    }

    pub fn get_object(&self, bucket: &str, key: &str) -> Result<Vec<u8>, CoreError> {
        log::info!("Downloading object bucket={} key={}", bucket, key);
        self.run(async {
            let resp = self
                .client
                .get_object()
                .bucket(bucket)
                .key(key)
                .send()
                .await
                .map_err(|e| {
                    log::error!(
                        "Failed to download object bucket={} key={}: {}",
                        bucket,
                        key,
                        e
                    );
                    CoreError::S3(e.to_string())
                })?;

            let data = resp
                .body
                .collect()
                .await
                .map_err(|e| {
                    log::error!(
                        "Failed to read object stream bucket={} key={}: {}",
                        bucket,
                        key,
                        e
                    );
                    CoreError::S3(e.to_string())
                })?
                .into_bytes()
                .to_vec();

            log::info!(
                "Object downloaded successfully bucket={} key={} size={}",
                bucket,
                key,
                data.len()
            );
            Ok(data)
        })
    }

    pub fn put_object(&self, bucket: &str, key: &str, data: Vec<u8>) -> Result<(), CoreError> {
        log::info!(
            "Uploading object bucket={} key={} size={}",
            bucket,
            key,
            data.len()
        );
        self.run(async {
            self.client
                .put_object()
                .bucket(bucket)
                .key(key)
                .body(ByteStream::from(data))
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
        })
    }

    pub fn head_object(&self, bucket: &str, key: &str) -> Result<S3Object, CoreError> {
        log::debug!("Querying object metadata bucket={} key={}", bucket, key);
        self.run(async {
            let resp = self
                .client
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
    }
}
