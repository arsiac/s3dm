use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    pub id: String,
    pub name: String,
    pub endpoint: String,
    pub region: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub force_path_style: bool,
}

impl ConnectionConfig {
    pub fn new(
        name: String,
        endpoint: String,
        region: String,
        access_key_id: String,
        secret_access_key: String,
        force_path_style: bool,
    ) -> Self {
        let id = Uuid::new_v4().to_string();
        log::info!(
            "Creating connection config id={} name={} endpoint={} force_path_style={}",
            id,
            name,
            endpoint,
            force_path_style
        );
        Self {
            id,
            name,
            endpoint,
            region,
            access_key_id,
            secret_access_key,
            force_path_style,
        }
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.name.trim().is_empty() {
            log::warn!("Validation failed: name is empty id={}", self.id);
            return Err(ConfigError::Validation("名称不能为空".into()));
        }
        if self.endpoint.trim().is_empty() {
            log::warn!("Validation failed: endpoint is empty id={}", self.id);
            return Err(ConfigError::Validation("Endpoint 不能为空".into()));
        }
        if self.access_key_id.trim().is_empty() {
            log::warn!("Validation failed: access key id is empty id={}", self.id);
            return Err(ConfigError::Validation("Access Key ID 不能为空".into()));
        }
        if self.secret_access_key.trim().is_empty() {
            log::warn!(
                "Validation failed: secret access key is empty id={}",
                self.id
            );
            return Err(ConfigError::Validation("Secret Access Key 不能为空".into()));
        }
        log::debug!("Validation passed id={}", self.id);
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON 序列化错误: {0}")]
    Json(#[from] serde_json::Error),
    #[error("验证错误: {0}")]
    Validation(String),
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
