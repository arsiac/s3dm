//! 数据模型与类型定义
//!
//! 定义 S3 操作中使用的公共数据结构（`S3Object` / `S3Bucket` /
//! `ObjectListResult`）、核心错误类型 `CoreError`，以及 smithy 时间类型
//! 与 chrono 之间的转换辅助 `to_chrono`。

use chrono::{DateTime, TimeZone, Utc};
use thiserror::Error;

/// 将 smithy 的 `DateTime` 转换为 chrono 的 `DateTime<Utc>`。
pub(crate) fn to_chrono(d: &aws_smithy_types::DateTime) -> Option<DateTime<Utc>> {
    let secs_f64 = d.as_secs_f64();
    let secs = secs_f64 as i64;
    let nsecs = ((secs_f64 - secs as f64) * 1_000_000_000.0) as u32;
    Utc.timestamp_opt(secs, nsecs).single()
}

/// 单个 S3 对象（文件）的元信息。
#[derive(Debug, Clone)]
pub struct S3Object {
    /// 对象键（路径）
    pub key: String,
    /// 对象大小（字节）
    pub size: i64,
    /// 最后修改时间
    pub last_modified: Option<DateTime<Utc>>,
    /// 是否为“文件夹”标记（S3 无真实目录，此处用于 UI 区分）
    pub is_folder: bool,
    /// ETag
    pub etag: Option<String>,
}

/// 单个 S3 存储桶的元信息。
#[derive(Debug, Clone)]
pub struct S3Bucket {
    /// 桶名称
    pub name: String,
    /// 创建时间
    pub creation_date: Option<DateTime<Utc>>,
}

/// `list_objects` 的分页结果。
#[derive(Debug, Clone)]
pub struct ObjectListResult {
    /// 本次返回的对象列表
    pub objects: Vec<S3Object>,
    /// 公共前缀（用于模拟文件夹）
    pub common_prefixes: Vec<String>,
    /// 是否还有后续分页
    pub is_truncated: bool,
    /// 下一页的续传令牌
    pub continuation_token: Option<String>,
}

/// 核心层错误类型。
#[derive(Debug, Clone, Error)]
pub enum CoreError {
    #[error("S3 error: {0}")]
    S3(String),
    /// S3 传输层可重试错误（连接/分发失败、超时等），供内部重试机制识别。
    /// 对上层展示与 `S3` 等同。
    #[error("S3 error: {0}")]
    S3Retryable(String),
    #[error("Connection error: {0}")]
    Connection(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("IO error: {0}")]
    Io(String),
}

impl CoreError {
    /// 是否为可重试的传输层错误。
    pub fn is_retryable(&self) -> bool {
        matches!(self, CoreError::S3Retryable(_))
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
    fn is_retryable_only_for_retryable_variant() {
        assert!(CoreError::S3Retryable("dispatch failure".into()).is_retryable());
        assert!(!CoreError::S3("bad request".into()).is_retryable());
        assert!(!CoreError::Connection("x".into()).is_retryable());
        assert!(!CoreError::NotFound("x".into()).is_retryable());
        assert!(!CoreError::Io("x".into()).is_retryable());
    }

    #[test]
    fn retryable_variant_displays_like_s3() {
        // 对上层展示与 S3 等同（同一 error 文案前缀）
        assert_eq!(
            CoreError::S3Retryable("boom".into()).to_string(),
            CoreError::S3("boom".into()).to_string()
        );
    }
}
