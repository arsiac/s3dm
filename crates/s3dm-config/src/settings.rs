//! 用户偏好设置持久化
//!
//! 管理应用级偏好（主题、语言、下载目录），以明文 JSON 保存到
//! `settings.json`（与 `connections.json` 同目录）。设置不涉及敏感
//! 凭据，仅影响 UI 呈现与默认下载路径。

use crate::ConfigError;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// 应用偏好设置
///
/// 与 `ConnectionConfig` 不同，此处不涉及任何敏感凭据，
/// 仅保存用户对主题、语言与下载目录的偏好。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    /// 主题显示名称（对应 GUI 的 AVAILABLE_THEMES）
    pub theme: String,
    /// 语言代码（对应 GUI 的 LANGUAGES，如 en / zh-CN / zh-TW）
    pub language: String,
    /// 默认下载目录
    pub download_dir: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: "Dark".to_string(),
            language: "en".to_string(),
            download_dir: dirs::download_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default(),
        }
    }
}

impl AppSettings {
    pub fn load() -> Self {
        let path = Self::default_path();
        if path.exists() {
            match fs::read_to_string(&path) {
                Ok(content) => match serde_json::from_str::<AppSettings>(&content) {
                    Ok(settings) => {
                        log::info!("Loaded settings from: {}", path.display());
                        return settings;
                    }
                    Err(e) => {
                        log::error!("Failed to parse settings: {}, using defaults", e);
                    }
                },
                Err(e) => {
                    log::error!("Failed to read settings: {}, using defaults", e);
                }
            }
        } else {
            log::info!("Settings file not found, using defaults");
        }
        Self::default()
    }

    pub fn save(&self) -> Result<(), ConfigError> {
        let path = Self::default_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&path, content)?;
        log::info!("Settings saved to: {}", path.display());
        Ok(())
    }

    fn default_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("s3dm");
        path.push("settings.json");
        path
    }
}
