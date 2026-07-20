//! 应用更新检查（GitHub Releases）
//!
//! 通过 GitHub Releases API 查询 `arsiac/s3dm` 仓库的最新发布版本，
//! 与当前版本号（来自 `CARGO_PKG_VERSION`）做语义化比较，仅在
//! 存在更高版本时返回 [`ReleaseInfo`]，供 GUI 层做顶部提示与跳转。

use semver::Version;
use serde::Deserialize;

/// 更新检查的目标仓库（GitHub `owner/repo`）。
const REPO: &str = "arsiac/s3dm";
/// GitHub Releases API 端点（最新发布）。
const LATEST_RELEASE_URL: &str = "https://api.github.com/repos/arsiac/s3dm/releases/latest";

/// 解析 GitHub 发布 JSON 时使用的字段子集。
#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    name: Option<String>,
    html_url: String,
    body: Option<String>,
    published_at: Option<String>,
}

/// 一次成功检查后得到的可用更新信息。
#[derive(Debug, Clone)]
pub struct ReleaseInfo {
    /// 最新版本标签（如 `v0.4.0`），去掉前导 `v` 后为语义化版本。
    pub version: String,
    /// 发布说明标题（与 `version` 不同时作为展示名）。
    pub name: String,
    /// 发布页 URL，用于在浏览器中打开下载页。
    pub html_url: String,
    /// 发布说明正文（Markdown 原文）。
    pub body: String,
    /// 发布时间（ISO 8601 字符串，可能为 None）。
    pub published_at: Option<String>,
}

/// 更新检查错误，涵盖网络、HTTP 状态与解析失败。
#[derive(Debug, thiserror::Error)]
pub enum UpdateCheckError {
    /// 网络层错误（请求未发出或连接失败）。
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
    /// GitHub API 返回了非成功状态码。
    #[error("github api returned status {0}")]
    Status(u16),
    /// 响应 JSON 解析失败。
    #[error("failed to parse release info: {0}")]
    Parse(#[from] serde_json::Error),
    /// 本地或远程版本号无法解析为合法的语义化版本。
    #[error("invalid version string: {0}")]
    InvalidVersion(String),
}

/// 将版本字符串规范化为可比较的 [`Version`]。
///
/// 容忍前导 `v`/`V`（如 `v0.4.0`），其余非法输入返回错误。
fn parse_version(raw: &str) -> Result<Version, UpdateCheckError> {
    let trimmed = raw.trim();
    let cleaned = trimmed.strip_prefix(['v', 'V']).unwrap_or(trimmed);
    Version::parse(cleaned).map_err(|_| UpdateCheckError::InvalidVersion(raw.to_string()))
}

/// 检查是否有可用更新。
///
/// - 成功且存在更高版本：返回 `Ok(Some(ReleaseInfo))`；
/// - 已是最新：返回 `Ok(None)`；
/// - 网络/解析失败：返回 `Err(UpdateCheckError)`。
///
/// GitHub API 对匿名请求限速（约 60 次/小时），超出时返回 403，
/// 调用方应按需静默处理（尤其自动检查路径）。
pub async fn check_update(current_version: &str) -> Result<Option<ReleaseInfo>, UpdateCheckError> {
    let current = parse_version(current_version)?;

    let resp = reqwest::Client::builder()
        .user_agent(concat!("s3dm/", env!("CARGO_PKG_VERSION")))
        .build()?
        .get(LATEST_RELEASE_URL)
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .send()
        .await?;

    if !resp.status().is_success() {
        return Err(UpdateCheckError::Status(resp.status().as_u16()));
    }

    let release: GithubRelease = resp.json().await?;
    let latest = parse_version(&release.tag_name)?;

    if latest > current {
        Ok(Some(ReleaseInfo {
            version: release.tag_name.clone(),
            name: release.name.unwrap_or(release.tag_name.clone()),
            html_url: release.html_url,
            body: release.body.unwrap_or_default(),
            published_at: release.published_at,
        }))
    } else {
        Ok(None)
    }
}

/// 返回更新检查所用的仓库标识，便于 GUI 在日志/调试中引用。
pub fn repo_slug() -> &'static str {
    REPO
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_version_strips_v_prefix() {
        assert_eq!(parse_version("v0.4.0").unwrap(), Version::new(0, 4, 0));
        assert_eq!(parse_version("V1.2.3").unwrap(), Version::new(1, 2, 3));
        // 预发布标签被保留（符合 semver 语义）
        assert_eq!(
            parse_version("2.0.0-rc1").unwrap(),
            Version::parse("2.0.0-rc1").unwrap()
        );
    }

    #[test]
    fn parse_version_rejects_non_semver() {
        assert!(parse_version("latest").is_err());
        assert!(parse_version("").is_err());
        assert!(parse_version("vX.Y.Z").is_err());
    }

    #[test]
    fn newer_version_detected() {
        // 模拟比较：latest > current 时应返回 Some
        let current = Version::new(0, 3, 0);
        let latest = Version::new(0, 4, 0);
        assert!(latest > current);
    }
}
