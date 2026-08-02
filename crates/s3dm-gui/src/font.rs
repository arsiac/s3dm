//! 字体配置辅助
//!
//! 将设置中的字体偏好（家族名称 + 字号）转换为 Iced 可用的 `Font` 与
//! 像素尺寸：
//! - 界面字体：空家族名回退到系统默认无衬线字体，字号按基础值等比缩放
//! - 预览编辑器字体：空家族名回退到系统默认等宽字体
//!
//! 由于 Iced 的 `Font::Family::Name` 需要 `'static` 借用，这里用一个
//! 进程级缓存将用户输入的名称转换为 `'static str`（仅首次出现时泄漏，
//! 单次会话内重复输入不会继续增长）。

use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};

use iced::font::Family;
use iced::{Font, Pixels};

use crate::app::App;

/// 界面基础字号默认值（显式字号按 `ui_font_size / 14` 缩放）
pub const DEFAULT_UI_FONT_SIZE: u16 = 14;
/// 预览编辑器字号默认值
pub const DEFAULT_PREVIEW_FONT_SIZE: u16 = 13;
/// 界面字号缩放基准
const UI_FONT_SIZE_BASE: f32 = DEFAULT_UI_FONT_SIZE as f32;

/// 家族名称 → `'static str` 缓存
static NAME_CACHE: OnceLock<Mutex<HashMap<String, &'static str>>> = OnceLock::new();

fn name_cache() -> &'static Mutex<HashMap<String, &'static str>> {
    NAME_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// 将家族名提升为 `'static str`（进程级缓存，同一名称只泄漏一次）。
///
/// Iced 的 `Family::Name` 需要 `'static` 借用；下拉框选项与用户输入
/// 的名称都经由此处转换。
pub fn leak_name(name: &str) -> &'static str {
    if name.trim().is_empty() {
        return "";
    }
    let mut cache = name_cache().lock().expect("font name cache poisoned");
    cache
        .entry(name.to_string())
        .or_insert_with(|| Box::leak(name.to_string().into_boxed_str()))
}

/// 将用户输入的家族名解析为 Iced `Family`；空/空白名称返回默认家族。
fn family_from_str(name: &str) -> Family {
    if name.trim().is_empty() {
        Family::SansSerif
    } else {
        Family::Name(leak_name(name))
    }
}

/// 系统已安装的字体家族列表（去重、排序；进程级缓存，仅首次扫描系统字体）。
pub fn installed_families() -> &'static [&'static str] {
    static FAMILIES: OnceLock<Vec<&'static str>> = OnceLock::new();
    FAMILIES.get_or_init(|| {
        let mut db = fontdb::Database::new();
        db.load_system_fonts();
        let mut names: HashSet<String> = HashSet::new();
        for face in db.faces() {
            // 每个字面只取主家族名，避免同一字体的本地化别名重复出现
            if let Some((family, _)) = face.families.first() {
                let trimmed = family.trim();
                if !trimmed.is_empty() {
                    names.insert(trimmed.to_string());
                }
            }
        }
        let mut list: Vec<&'static str> = names.into_iter().map(|s| leak_name(&s)).collect();
        list.sort_unstable();
        list
    })
}

/// 字体下拉框选项：默认标签 + 系统已安装字体列表。
///
/// 默认标签随语言切换变化，因此每次调用时按当前语言构造。
pub fn font_options(default_label: &str) -> Vec<String> {
    let mut options = vec![default_label.to_string()];
    options.extend(installed_families().iter().map(|s| s.to_string()));
    options
}

/// 根据家族名构造字体，空名称时使用 `fallback`。
fn font_from_str(name: &str, fallback: Font) -> Font {
    if name.trim().is_empty() {
        fallback
    } else {
        Font {
            family: family_from_str(name),
            ..fallback
        }
    }
}

/// 当前界面字体（空家族名 → 系统默认无衬线字体）
pub fn ui_font(app: &App) -> Font {
    font_from_str(&app.ui_font_family, Font::DEFAULT)
}

/// 当前预览编辑器字体（空家族名 → 系统默认等宽字体）
pub fn preview_font(app: &App) -> Font {
    font_from_str(&app.preview_font_family, Font::MONOSPACE)
}

/// 界面字号缩放系数（以 14px 为基准）
pub fn scale(font_size: u16) -> f32 {
    font_size as f32 / UI_FONT_SIZE_BASE
}

/// 将某个显式字号按界面字号设置等比缩放（最小 8px，避免缩放过小）。
///
/// 返回 `u32` 以便直接作为 Iced `Pixels`（`From<u32>`）传入 `.size()`。
pub fn ui_size_for(font_size: u16, base: u16) -> u32 {
    let scaled = (base as f32 * scale(font_size)).round() as i64;
    scaled.clamp(8, i64::from(u16::MAX)) as u32
}

/// 应用当前的界面字号设置缩放某个显式字号
pub fn ui_size(app: &App, base: u16) -> u32 {
    ui_size_for(app.ui_font_size, base)
}

/// 解析启动时的字体偏好，返回应用级 `iced::Settings` 所需的字段组合。
///
/// 返回 (default_font, default_text_size)；空家族名时 default_font 保持
/// Iced 默认，从而由各平台字体回退机制自行选择。
pub fn startup_font(family: &str, size: u16) -> (Font, Pixels) {
    let font = font_from_str(family, Font::DEFAULT);
    (font, Pixels(size as f32))
}

/// 从持久化设置加载字体偏好，构造应用级启动 `iced::Settings`。
///
/// `default_font` / `default_text_size` 决定未显式指定字体与字号的控件；
/// 显式字号在视图层通过 [`ui_size`] 按同一配置实时缩放。
pub fn startup_settings() -> iced::Settings {
    let settings = s3dm_config::AppSettings::load();
    let (default_font, default_text_size) =
        startup_font(&settings.ui_font_family, settings.ui_font_size);
    log::debug!(
        "Font settings: ui_font_family={:?} ui_font_size={}",
        settings.ui_font_family,
        settings.ui_font_size
    );
    iced::Settings {
        default_font,
        default_text_size,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_family_uses_fallback() {
        assert_eq!(font_from_str("", Font::MONOSPACE), Font::MONOSPACE);
        assert_eq!(font_from_str("   ", Font::MONOSPACE), Font::MONOSPACE);
    }

    #[test]
    fn named_family_is_preserved() {
        let font = font_from_str("JetBrains Mono", Font::DEFAULT);
        assert_eq!(font.family, Family::Name("JetBrains Mono"));
        // 未配置等宽标志时保持 fallback 的 monospaced 属性
        let mono = font_from_str("JetBrains Mono", Font::MONOSPACE);
        assert_eq!(mono.family, Family::Name("JetBrains Mono"));
        assert_eq!(mono.weight, Font::MONOSPACE.weight);
    }

    #[test]
    fn scale_matches_configured_size() {
        // 14px 基准：不缩放
        assert_eq!(ui_size_for(14, 14), 14);
        assert_eq!(ui_size_for(14, 13), 13);

        // 16px：约 1.14 倍，13 → 15
        assert_eq!(ui_size_for(16, 14), 16);
        assert_eq!(ui_size_for(16, 13), 15);
    }

    #[test]
    fn size_never_below_floor() {
        assert!(ui_size_for(10, 8) >= 8);
        assert!(ui_size_for(10, 11) >= 8);
    }

    #[test]
    fn leak_name_is_stable_and_empty_safe() {
        assert_eq!(leak_name(""), "");
        assert_eq!(leak_name("   "), "");
        let a = leak_name("JetBrains Mono");
        let b = leak_name("JetBrains Mono");
        assert_eq!(a, b);
        assert!(std::ptr::eq(a, b));
    }
}
