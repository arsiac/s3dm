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

/// 文件类型图标（16×16 填充 SVG），用于对象浏览器中按扩展名标记文件。
pub const FILE_TEXT: &[u8] = include_bytes!("../icons/document-bullet-list-16-filled.svg");
pub const FILE_CODE: &[u8] = include_bytes!("../icons/document-code-16-filled.svg");
pub const FILE_IMAGE: &[u8] = include_bytes!("../icons/image-16-filled.svg");
pub const FILE_AUDIO: &[u8] = include_bytes!("../icons/music-note-2-16-filled.svg");
pub const FILE_VIDEO: &[u8] = include_bytes!("../icons/video-16-filled.svg");
pub const FILE_ARCHIVE: &[u8] = include_bytes!("../icons/folder-zip-16-filled.svg");
pub const FILE_DEFAULT: &[u8] = include_bytes!("../icons/document-16-filled.svg");

/// 界面操作图标（16×16 填充 SVG），统一在此导入供各视图复用。
pub const ICON_DISMISS: &[u8] = include_bytes!("../icons/dismiss-16-filled.svg");
pub const ICON_ADD: &[u8] = include_bytes!("../icons/add-16-filled.svg");
pub const ICON_SETTINGS: &[u8] = include_bytes!("../icons/settings-16-filled.svg");
pub const ICON_EDIT: &[u8] = include_bytes!("../icons/edit-16-filled.svg");
pub const ICON_DELETE: &[u8] = include_bytes!("../icons/delete-16-filled.svg");
pub const ICON_REFRESH: &[u8] = include_bytes!("../icons/arrow-clockwise-16-filled.svg");
pub const ICON_FOLDER: &[u8] = include_bytes!("../icons/folder-16-filled.svg");
pub const ICON_CLOUD_UPLOAD: &[u8] = include_bytes!("../icons/cloud-arrow-up-16-filled.svg");
pub const ICON_ARROW_LEFT: &[u8] = include_bytes!("../icons/arrow-left-16-filled.svg");
pub const ICON_FOLDER_ADD: &[u8] = include_bytes!("../icons/folder-add-16-filled.svg");
pub const ICON_CLOUD_LINK: &[u8] = include_bytes!("../icons/cloud-link-16-filled.svg");
pub const ICON_CLOUD_DOWNLOAD: &[u8] = include_bytes!("../icons/cloud-arrow-down-16-filled.svg");

/// 根据文件名返回对应的文件类型图标字节。
///
/// 映射规则：
/// - 文本：`txt`, `log`
/// - 代码/配置/文档：`json`, `yaml`, `yml`, `toml`, `py`, `rs`, `c`, `h`, `java`, `js`, `ts`, `md` 等
/// - 图片：`png`, `jpg`, `jpeg`, `gif`, `svg`, `webp`, `bmp` 等
/// - 音频：`mp3`, `wav`, `flac`, `aac`, `ogg`, `m4a` 等
/// - 视频：`mp4`, `mkv`, `avi`, `mov`, `webm`, `flv` 等
/// - 压缩：`zip`, `tar.gz`, `tar.xz`, `tgz`, `rar`, `7z` 等
/// - 其余回退到 `FILE_DEFAULT`
pub fn file_icon(name: &str) -> &'static [u8] {
    let lower = name.to_ascii_lowercase();

    // 压缩包（含多段扩展名，需优先 ends_with 判断）
    if lower.ends_with(".tar.gz")
        || lower.ends_with(".tar.xz")
        || lower.ends_with(".tar.bz2")
        || lower.ends_with(".tar.zst")
        || lower.ends_with(".tgz")
        || matches!(
            extension(&lower).as_deref(),
            Some("zip" | "rar" | "7z" | "gz" | "xz" | "bz2" | "zst")
        )
    {
        return FILE_ARCHIVE;
    }

    match extension(&lower).as_deref() {
        Some("txt" | "log") => FILE_TEXT,
        Some(
            "json" | "yaml" | "yml" | "toml" | "py" | "rs" | "c" | "h" | "hpp" | "cpp" | "cc"
            | "java" | "js" | "ts" | "tsx" | "jsx" | "go" | "rb" | "php" | "sh" | "bash" | "md"
            | "html" | "css" | "xml" | "csv" | "ini" | "cfg" | "conf",
        ) => FILE_CODE,
        Some("png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" | "bmp" | "ico" | "tiff" | "heic") => {
            FILE_IMAGE
        }
        Some("mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" | "wma" | "opus") => FILE_AUDIO,
        Some("mp4" | "mkv" | "avi" | "mov" | "webm" | "flv" | "wmv" | "m4v" | "mpg" | "mpeg") => {
            FILE_VIDEO
        }
        _ => FILE_DEFAULT,
    }
}

/// 提取文件名的小写扩展名（不含点），无扩展名返回 `None`。
fn extension(name: &str) -> Option<String> {
    let base = name.rsplit('/').next().unwrap_or(name);
    base.rsplit('.').next().map(|s| s.to_string())
}
