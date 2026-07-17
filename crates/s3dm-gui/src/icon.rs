//! 应用图标资源（桌面端 windows/macos/linux）
//!
//! 图标源来自 `s3dm-icons/web/`，由该目录的通用尺寸（favicon / chrome 图标）
//! 派生生成，分别适配各桌面平台：
//! - `icon-256.png` / `icon-512.png`：Linux 窗口与任务栏图标
//! - `icon.ico`：Windows（含 16/32/256 多尺寸）
//! - `icon.icns`：macOS（含 128~1024 各档）
//!
//! 运行期窗口图标使用 256 PNG，在 `main.rs` 中通过本模块导出加载。

/// 应用窗口图标（256×256 PNG），用于运行期窗口与任务栏显示。
pub const WINDOW_ICON: &[u8] = include_bytes!("../icons/app/icon-256.png");
