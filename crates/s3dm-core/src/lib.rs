//! S3DM 核心库
//!
//! 封装 aws-sdk-s3，提供 S3 兼容存储的连接管理、桶/对象列举、上传下载、
//! 删除与元数据查询等功能。模块划分：
//! - [`types`]：公共数据结构与错误类型；
//! - [`http`]：HTTP 连接器与 TLS 校验控制；
//! - [`client`]：`S3Manager` 核心实现。

mod client;
mod http;
mod types;

pub use client::S3Manager;
pub use types::{CoreError, ObjectListResult, S3Bucket, S3Object};
