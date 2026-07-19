use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use thiserror::Error;
use uuid::Uuid;

mod settings;
pub use settings::AppSettings;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    pub id: String,
    pub name: String,
    pub endpoint: String,
    pub region: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub force_path_style: bool,
    /// 是否跳过 TLS 证书校验（用于自签名证书 / 内网 HTTPS，不安全）
    #[serde(default)]
    pub skip_tls_verify: bool,
}

impl ConnectionConfig {
    pub fn new(
        name: String,
        endpoint: String,
        region: String,
        access_key_id: String,
        secret_access_key: String,
        force_path_style: bool,
        skip_tls_verify: bool,
    ) -> Self {
        let id = Uuid::new_v4().to_string();
        log::info!(
            "Creating connection config id={} name={} endpoint={} force_path_style={} skip_tls_verify={}",
            id,
            name,
            endpoint,
            force_path_style,
            skip_tls_verify
        );
        Self {
            id,
            name,
            endpoint,
            region,
            access_key_id,
            secret_access_key,
            force_path_style,
            skip_tls_verify,
        }
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.name.trim().is_empty() {
            log::warn!("Validation failed: name is empty id={}", self.id);
            return Err(ConfigError::Validation("name".into()));
        }
        if self.endpoint.trim().is_empty() {
            log::warn!("Validation failed: endpoint is empty id={}", self.id);
            return Err(ConfigError::Validation("endpoint".into()));
        }
        if !self.endpoint.starts_with("http://") && !self.endpoint.starts_with("https://") {
            log::warn!(
                "Validation failed: endpoint missing http/https scheme id={}",
                self.id
            );
            return Err(ConfigError::InvalidEndpoint);
        }
        if self.access_key_id.trim().is_empty() {
            log::warn!("Validation failed: access key id is empty id={}", self.id);
            return Err(ConfigError::Validation("access_key_id".into()));
        }
        if self.secret_access_key.trim().is_empty() {
            log::warn!(
                "Validation failed: secret access key is empty id={}",
                self.id
            );
            return Err(ConfigError::Validation("secret_access_key".into()));
        }
        log::debug!("Validation passed id={}", self.id);
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Validation error: {0}")]
    Validation(String),
    #[error("Invalid endpoint: must start with http:// or https://")]
    InvalidEndpoint,
}

pub struct ConfigStore {
    file_path: PathBuf,
    connections: Vec<ConnectionConfig>,
}

impl ConfigStore {
    pub fn new() -> Self {
        let file_path = Self::default_path();
        log::info!("Loading config from: {}", file_path.display());
        let connections = if file_path.exists() {
            match Self::load_from_path(&file_path) {
                Ok(conns) => {
                    log::info!("Loaded {} connection configs", conns.len());
                    conns
                }
                Err(e) => {
                    log::error!("Failed to load config: {}, using empty config", e);
                    Vec::new()
                }
            }
        } else {
            log::info!("Config file not found, using empty config");
            Vec::new()
        };
        Self {
            file_path,
            connections,
        }
    }

    fn default_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("s3dm");
        path.push("connections.json");
        path
    }

    fn load_from_path(path: &PathBuf) -> Result<Vec<ConnectionConfig>, ConfigError> {
        let content = fs::read_to_string(path)?;
        let connections = serde_json::from_str(&content)?;
        Ok(connections)
    }

    /// 将连接配置持久化到磁盘。
    ///
    /// 安全说明：AK/SK 以**明文** JSON 写入本地文件，当前仅通过文件权限
    /// （Unix 下 `0600`，仅当前用户可读写）限制访问，**未加密**。请勿在
    /// 共享/多用户设备上存储敏感凭据；长期方案可考虑接入系统密钥库
    /// （keyring）或加密存储。UI 侧已在连接表单向用户提示该风险。
    fn save(&self) -> Result<(), ConfigError> {
        log::debug!("Saving config to: {}", self.file_path.display());
        if let Some(parent) = self.file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(&self.connections)?;
        fs::write(&self.file_path, content)?;
        // 限制配置（含明文凭据）仅当前用户可读写，避免凭据泄露
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&self.file_path, fs::Permissions::from_mode(0o600));
        }
        log::info!("Config saved, {} connections total", self.connections.len());
        Ok(())
    }

    pub fn list(&self) -> &[ConnectionConfig] {
        &self.connections
    }

    pub fn get(&self, id: &str) -> Option<&ConnectionConfig> {
        self.connections.iter().find(|c| c.id == id)
    }

    pub fn add(&mut self, config: ConnectionConfig) -> Result<(), ConfigError> {
        log::info!("Adding connection: id={} name={}", config.id, config.name);
        config.validate()?;
        self.connections.push(config);
        self.save()
    }

    pub fn update(&mut self, config: ConnectionConfig) -> Result<(), ConfigError> {
        log::info!("Updating connection: id={} name={}", config.id, config.name);
        config.validate()?;
        if let Some(existing) = self.connections.iter_mut().find(|c| c.id == config.id) {
            *existing = config;
            self.save()
        } else {
            log::error!("Update failed: connection not found id={}", config.id);
            Err(ConfigError::Validation("连接未找到".into()))
        }
    }

    pub fn delete(&mut self, id: &str) -> Result<(), ConfigError> {
        let name = self
            .connections
            .iter()
            .find(|c| c.id == id)
            .map(|c| c.name.clone());
        log::info!("Deleting connection: id={} name={:?}", id, name);
        self.connections.retain(|c| c.id != id);
        self.save()
    }
}

impl Default for ConfigStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_config() -> ConnectionConfig {
        ConnectionConfig::new(
            "test".into(),
            "https://s3.example.com".into(),
            "us-east-1".into(),
            "AKID".into(),
            "SECRET".into(),
            true,
            false,
        )
    }

    #[test]
    fn validate_accepts_complete_config() {
        assert!(sample_config().validate().is_ok());
    }

    #[test]
    fn validate_rejects_empty_name() {
        let mut c = sample_config();
        c.name = "   ".into();
        assert!(c.validate().is_err());
    }

    #[test]
    fn validate_rejects_empty_endpoint() {
        let mut c = sample_config();
        c.endpoint.clear();
        assert!(c.validate().is_err());
    }

    #[test]
    fn validate_rejects_endpoint_without_scheme() {
        let mut c = sample_config();
        c.endpoint = "s3.example.com".into();
        assert!(matches!(c.validate(), Err(ConfigError::InvalidEndpoint)));
        c.endpoint = "ftp://s3.example.com".into();
        assert!(matches!(c.validate(), Err(ConfigError::InvalidEndpoint)));
    }

    #[test]
    fn validate_accepts_http_and_https_endpoint() {
        let mut c = sample_config();
        c.endpoint = "https://s3.example.com".into();
        assert!(c.validate().is_ok());
        c.endpoint = "http://192.168.1.10:9000".into();
        assert!(c.validate().is_ok());
    }

    #[test]
    fn validate_rejects_empty_keys() {
        let mut c = sample_config();
        c.access_key_id.clear();
        assert!(c.validate().is_err());
        let mut c2 = sample_config();
        c2.secret_access_key.clear();
        assert!(c2.validate().is_err());
    }

    #[test]
    fn deserialize_defaults_skip_tls_verify_for_legacy_config() {
        // 旧版配置不含 skip_tls_verify 字段，应默认 false（保证向后兼容）
        let json = r#"[{
            "id": "abc",
            "name": "legacy",
            "endpoint": "https://s3.example.com",
            "region": "us-east-1",
            "access_key_id": "AKID",
            "secret_access_key": "SECRET",
            "force_path_style": true
        }]"#;
        let configs: Vec<ConnectionConfig> = serde_json::from_str(json).unwrap();
        assert!(!configs[0].skip_tls_verify);
    }

    #[test]
    fn store_add_update_delete_roundtrip() {
        let mut store = ConfigStore {
            file_path: std::env::temp_dir()
                .join(format!("s3dm-test-{}.json", uuid::Uuid::new_v4())),
            connections: vec![],
        };
        let cfg = sample_config();
        let id = cfg.id.clone();
        store.add(cfg).unwrap();
        assert_eq!(store.list().len(), 1);
        assert!(store.get(&id).is_some());

        let mut updated = sample_config();
        updated.id = id.clone();
        updated.name = "renamed".into();
        store.update(updated).unwrap();
        assert_eq!(store.get(&id).unwrap().name, "renamed");

        store.delete(&id).unwrap();
        assert!(store.get(&id).is_none());
        let _ = std::fs::remove_file(&store.file_path);
    }

    #[test]
    fn settings_roundtrip() {
        let path =
            std::env::temp_dir().join(format!("s3dm-settings-{}.json", uuid::Uuid::new_v4()));
        let _ = std::fs::remove_file(&path);

        let settings = AppSettings {
            theme: "Light".into(),
            language: "zh-CN".into(),
            download_dir: "/tmp".into(),
        };
        // 直接写临时文件验证序列化/反序列化
        let content = serde_json::to_string_pretty(&settings).unwrap();
        std::fs::write(&path, content).unwrap();
        let loaded: AppSettings =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(loaded.theme, "Light");
        assert_eq!(loaded.language, "zh-CN");
        assert_eq!(loaded.download_dir, "/tmp");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn settings_defaults_language_en() {
        assert_eq!(AppSettings::default().language, "en");
        assert_eq!(AppSettings::default().theme, "Dark");
    }
}
