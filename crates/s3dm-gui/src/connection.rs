//! S3 连接表单模型
//!
//! 定义 `ConnectionForm` 结构体，用于添加/编辑 S3 连接的表单数据管理，
//! 提供与 `s3dm_config::ConnectionConfig` 之间的双向转换。

use s3dm_config::ConnectionConfig;

/// 连接表单数据模型
///
/// 用于在 UI 表单中暂存用户输入的连接信息，
/// `id` 为 `None` 时表示新建连接，`Some` 时表示编辑已有连接。
#[derive(Debug, Clone)]
pub struct ConnectionForm {
    /// 连接唯一标识（编辑时存在，新建时为 None）
    pub id: Option<String>,
    /// 连接名称
    pub name: String,
    /// S3 兼容服务端点地址
    pub endpoint: String,
    /// 区域名称
    pub region: String,
    /// 访问密钥 ID
    pub access_key_id: String,
    /// 秘密访问密钥
    pub secret_access_key: String,
    /// 是否使用路径风格（path-style）寻址
    pub force_path_style: bool,
    /// 是否跳过 TLS 证书校验（用于自签名证书 / 内网 HTTPS，不安全）
    pub skip_tls_verify: bool,
}

impl ConnectionForm {
    /// 将表单数据转换为持久化配置
    pub fn to_config(&self) -> ConnectionConfig {
        match &self.id {
            Some(id) => ConnectionConfig {
                id: id.clone(),
                name: self.name.clone(),
                endpoint: self.endpoint.clone(),
                region: self.region.clone(),
                access_key_id: self.access_key_id.clone(),
                secret_access_key: self.secret_access_key.clone(),
                force_path_style: self.force_path_style,
                skip_tls_verify: self.skip_tls_verify,
            },
            None => ConnectionConfig::new(
                self.name.clone(),
                self.endpoint.clone(),
                self.region.clone(),
                self.access_key_id.clone(),
                self.secret_access_key.clone(),
                self.force_path_style,
                self.skip_tls_verify,
            ),
        }
    }

    /// 从已有配置还原表单数据（用于编辑场景）
    pub fn from_config(config: &ConnectionConfig) -> Self {
        Self {
            id: Some(config.id.clone()),
            name: config.name.clone(),
            endpoint: config.endpoint.clone(),
            region: config.region.clone(),
            access_key_id: config.access_key_id.clone(),
            secret_access_key: config.secret_access_key.clone(),
            force_path_style: config.force_path_style,
            skip_tls_verify: config.skip_tls_verify,
        }
    }
}
