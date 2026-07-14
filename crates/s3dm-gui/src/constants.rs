//! 常量定义、自定义调色板及通用辅助函数
//!
//! 本模块集中管理以下内容：
//! - `AVAILABLE_THEMES`：应用支持的所有 Iced 主题列表
//! - `LANGUAGES`：应用支持的国际化语言列表
//! - `CustomPalette`：基于主题背景色计算的自定义颜色体系
//! - `custom_palette()`：根据当前主题生成自定义调色板
//! - `format_size()`：将字节数格式化为人类可读的字符串

use iced::{Color, Theme};

/// 可用主题列表，每项为 (显示名称, Theme 枚举)
pub const AVAILABLE_THEMES: &[(&str, Theme)] = &[
    ("Dark", Theme::Dark),
    ("Light", Theme::Light),
    ("Dracula", Theme::Dracula),
    ("Nord", Theme::Nord),
    ("Solarized Light", Theme::SolarizedLight),
    ("Solarized Dark", Theme::SolarizedDark),
    ("Gruvbox Light", Theme::GruvboxLight),
    ("Gruvbox Dark", Theme::GruvboxDark),
    ("Catppuccin Latte", Theme::CatppuccinLatte),
    ("Catppuccin Frappé", Theme::CatppuccinFrappe),
    ("Catppuccin Macchiato", Theme::CatppuccinMacchiato),
    ("Catppuccin Mocha", Theme::CatppuccinMocha),
    ("Tokyo Night", Theme::TokyoNight),
    ("Tokyo Night Storm", Theme::TokyoNightStorm),
    ("Tokyo Night Light", Theme::TokyoNightLight),
    ("Kanagawa Wave", Theme::KanagawaWave),
    ("Kanagawa Dragon", Theme::KanagawaDragon),
    ("Kanagawa Lotus", Theme::KanagawaLotus),
    ("Moonfly", Theme::Moonfly),
    ("Nightfly", Theme::Nightfly),
    ("Oxocarbon", Theme::Oxocarbon),
    ("Ferra", Theme::Ferra),
];

/// 可用语言列表，每项为 (显示名称, 语言代码)
pub const LANGUAGES: &[(&str, &str)] = &[
    ("English", "en"),
    ("简体中文", "zh-CN"),
    ("繁體中文", "zh-TW"),
];

/// 自定义调色板，在主题默认颜色之外补充 UI 专用颜色
pub struct CustomPalette {
    /// 表面背景色（用于面板、卡片等）
    pub surface: Color,
    /// 抬升表面色（用于弹窗、悬浮元素等）
    pub surface_raised: Color,
    /// 次要文本色
    pub text_secondary: Color,
}

/// 根据当前主题计算自定义调色板
///
/// 通过感知亮度公式 `0.299R + 0.587G + 0.114B` 判断背景深浅，
/// 深色主题时提亮表面色，浅色主题时压暗表面色。
pub fn custom_palette(theme: &Theme) -> CustomPalette {
    let bg = theme.palette().background;
    let luminance = 0.299 * bg.r + 0.587 * bg.g + 0.114 * bg.b;
    if luminance > 0.5 {
        CustomPalette {
            surface: Color::from_rgb(
                (bg.r - 0.06).max(0.0),
                (bg.g - 0.06).max(0.0),
                (bg.b - 0.06).max(0.0),
            ),
            surface_raised: Color::from_rgb(
                (bg.r - 0.10).max(0.0),
                (bg.g - 0.10).max(0.0),
                (bg.b - 0.10).max(0.0),
            ),
            text_secondary: Color::from_rgb(0.45, 0.45, 0.45),
        }
    } else {
        CustomPalette {
            surface: Color::from_rgb(
                (bg.r + 0.08).min(1.0),
                (bg.g + 0.08).min(1.0),
                (bg.b + 0.08).min(1.0),
            ),
            surface_raised: Color::from_rgb(
                (bg.r + 0.12).min(1.0),
                (bg.g + 0.12).min(1.0),
                (bg.b + 0.12).min(1.0),
            ),
            text_secondary: Color::from_rgb(0.6, 0.6, 0.6),
        }
    }
}

/// 将字节数格式化为人类可读的字符串（B/KB/MB/GB/TB）
pub fn format_size(size: i64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = size as f64;
    let mut unit_idx = 0;
    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }
    format!("{:.1} {}", size, UNITS[unit_idx])
}
