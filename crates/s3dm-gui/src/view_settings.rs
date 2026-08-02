//! 设置面板与状态栏视图
//!
//! - `view_settings()`：分类式设置面板（通用 / 外观 / 预览 / 更新 / 关于），
//!   左侧分类导航 + 右侧内容区，含界面与预览编辑器字体配置及实时预览
//! - `view_status_bar()`：底部状态栏，显示当前连接/桶/对象计数信息

use iced::{
    Alignment, Border, Element, Length, Padding,
    widget::{
        Theme, button, checkbox, column, combo_box, container, image, pick_list, progress_bar, row,
        rule, scrollable, slider, svg, svg::Handle as SvgHandle, text, text_input,
    },
};
use rust_i18n::t;

use crate::app::{App, SettingsCategory};
use crate::constants;
use crate::font;
use crate::icon;
use crate::message::Message;
use crate::update::UpdateCheckStatus;

/// 设置面板尺寸（固定像素，保证分类导航与内容区布局稳定）
const SETTINGS_W: f32 = 760.0;
const SETTINGS_H: f32 = 540.0;
/// 控件行左侧标签宽度
const LABEL_W: f32 = 130.0;

/// 渲染设置面板（不含遮罩 overlay）
///
/// 布局：标题栏 + 左侧分类导航 + 右侧内容区。
pub fn view_settings(app: &App) -> Element<'_, Message> {
    let p = constants::custom_palette(&app.theme);

    let dismiss = svg(SvgHandle::from_memory(icon::ICON_DISMISS.to_vec()))
        .width(Length::Fixed(16.0))
        .height(Length::Fixed(16.0))
        .style(move |_: &Theme, _: svg::Status| svg::Style {
            color: Some(p.text_secondary),
        });

    let header = row![
        text(t!("settings").to_string())
            .font(font::ui_font(app))
            .size(font::ui_size(app, 20)),
        container(
            button(dismiss)
                .style(icon_button_style(app))
                .on_press(Message::ToggleSettings)
        )
        .width(Length::Fill)
        .align_x(Alignment::End),
    ]
    .spacing(10)
    .align_y(Alignment::Center)
    .padding(Padding {
        top: 20.0,
        right: 20.0,
        bottom: 16.0,
        left: 20.0,
    });

    let body = row![
        nav_pane(app),
        rule::vertical(1),
        container(content_pane(app))
            .width(Length::Fill)
            .height(Length::Fill),
    ]
    .height(Length::Fill);

    let panel = column![header, rule::horizontal(1), body].spacing(0);

    container(panel)
        .width(Length::Fixed(SETTINGS_W))
        .height(Length::Fixed(SETTINGS_H))
        .style(|theme: &Theme| container::Style {
            background: Some(iced::Background::Color(
                constants::custom_palette(theme).surface_raised,
            )),
            border: Border::default().rounded(8),
            ..Default::default()
        })
        .into()
}

// ─────────────────────────── 左侧分类导航 ───────────────────────────

/// 分类导航列表：图标 + 标签，选中项高亮
fn nav_pane<'a>(app: &'a App) -> Element<'a, Message> {
    let categories: [(SettingsCategory, &'static [u8], &str); 5] = [
        (
            SettingsCategory::General,
            icon::ICON_SETTINGS,
            "settings_category_general",
        ),
        (
            SettingsCategory::Appearance,
            icon::ICON_IMAGE,
            "settings_category_appearance",
        ),
        (
            SettingsCategory::Preview,
            icon::ICON_PREVIEW,
            "settings_category_preview",
        ),
        (
            SettingsCategory::Updates,
            icon::ICON_REFRESH,
            "settings_category_updates",
        ),
        (
            SettingsCategory::About,
            icon::ICON_INFO,
            "settings_category_about",
        ),
    ];

    let items = categories.into_iter().map(|(category, icon_bytes, key)| {
        nav_item(app, category, icon_bytes, t!(key).to_string())
    });

    container(column(items).spacing(4).padding(Padding::from([20, 12])))
        .width(Length::Fixed(172.0))
        .height(Length::Fill)
        .align_y(Alignment::Start)
        .into()
}

/// 单个分类导航项
fn nav_item<'a>(
    app: &'a App,
    category: SettingsCategory,
    icon_bytes: &'static [u8],
    label: String,
) -> Element<'a, Message> {
    let p = constants::custom_palette(&app.theme);
    let selected = app.settings_category == category;
    let primary = app.theme.palette().primary;
    let (fg, bg) = if selected {
        (
            primary,
            Some(iced::Background::Color(iced::Color::from_rgba(
                primary.r, primary.g, primary.b, 0.16,
            ))),
        )
    } else {
        (p.text_secondary, None)
    };
    let hover_bg = iced::Color::from_rgba(1.0, 1.0, 1.0, 0.07);

    let content = row![
        svg(SvgHandle::from_memory(icon_bytes.to_vec()))
            .width(Length::Fixed(16.0))
            .height(Length::Fixed(16.0))
            .style(move |_: &Theme, _: svg::Status| svg::Style { color: Some(fg) }),
        text(label)
            .font(font::ui_font(app))
            .size(font::ui_size(app, 13))
            .color(fg),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    button(content)
        .on_press(Message::SettingsCategorySelected(category))
        .style(move |_: &Theme, s: button::Status| {
            let background = match s {
                button::Status::Hovered | button::Status::Pressed => {
                    Some(iced::Background::Color(hover_bg))
                }
                _ => bg,
            };
            button::Style {
                background,
                text_color: fg,
                border: Border::default().rounded(6),
                shadow: iced::Shadow::default(),
                ..Default::default()
            }
        })
        .padding(Padding::from([8, 12]))
        .width(Length::Fill)
        .into()
}

// ─────────────────────────── 右侧内容区 ───────────────────────────

/// 根据当前选中的分类渲染对应内容页（外层滚动容器）
fn content_pane<'a>(app: &'a App) -> Element<'a, Message> {
    let pane: Element<Message> = match app.settings_category {
        SettingsCategory::General => general_pane(app),
        SettingsCategory::Appearance => appearance_pane(app),
        SettingsCategory::Preview => preview_pane(app),
        SettingsCategory::Updates => updates_pane(app),
        SettingsCategory::About => about_pane(app),
    };

    container(scrollable(container(pane).padding(Padding::from([24, 28]))))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// 页面标题
fn pane_title<'a>(app: &'a App, key: &str) -> Element<'a, Message> {
    text(t!(key).to_string())
        .font(font::ui_font(app))
        .size(font::ui_size(app, 17))
        .into()
}

/// 页面说明文字
fn pane_hint<'a>(app: &'a App, key: &str) -> Element<'a, Message> {
    let p = constants::custom_palette(&app.theme);
    text(t!(key).to_string())
        .font(font::ui_font(app))
        .size(font::ui_size(app, 12))
        .color(p.text_secondary)
        .into()
}

/// 小节标题（如「界面字体」「预览编辑器字体」）
fn section_label<'a>(app: &'a App, key: &str) -> Element<'a, Message> {
    let p = constants::custom_palette(&app.theme);
    text(t!(key).to_string())
        .font(font::ui_font(app))
        .size(font::ui_size(app, 13))
        .color(p.text_secondary)
        .into()
}

/// 标签 + 控件的水平行
fn control_row<'a>(
    app: &'a App,
    label: impl Into<Element<'a, Message>>,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    let _ = app;
    row![
        container(label.into())
            .width(Length::Fixed(LABEL_W))
            .align_x(Alignment::Start),
        container(control.into()).width(Length::Fill),
    ]
    .spacing(12)
    .align_y(Alignment::Center)
    .into()
}

/// 字号滑块：滑杆 + 当前值
fn size_slider<'a>(
    app: &'a App,
    value: u16,
    range: std::ops::RangeInclusive<u16>,
    on_change: fn(u16) -> Message,
) -> Element<'a, Message> {
    let p = constants::custom_palette(&app.theme);
    row![
        slider(range, value, on_change)
            .step(1u16)
            .width(Length::Fill),
        container(
            text(format!("{}px", value))
                .font(font::ui_font(app))
                .size(font::ui_size(app, 12))
                .color(p.text_secondary)
        )
        .width(Length::Fixed(44.0))
        .align_x(Alignment::End),
    ]
    .spacing(10)
    .align_y(Alignment::Center)
    .into()
}

/// 字体家族下拉框（可输入过滤，忽略大小写）
///
/// 选项为「系统默认 + 已安装字体」；输入时按子串过滤并高亮候选项，
/// 键入任意文本也会实时生效（等同自定义字体名）。
fn font_combo<'a>(
    app: &'a App,
    state: &'a iced::widget::combo_box::State<String>,
    value: &'a str,
    default_label: &str,
    placeholder: &str,
    on_select: fn(String) -> Message,
) -> Element<'a, Message> {
    // 空值显示「系统默认」；否则显示当前家族名
    let selection = if value.trim().is_empty() {
        default_label.to_string()
    } else {
        value.to_string()
    };
    let default_owned = default_label.to_string();

    combo_box(state, placeholder, Some(&selection), move |name: String| {
        if name == default_owned {
            on_select(String::new())
        } else {
            on_select(name)
        }
    })
    .on_input(on_select)
    .font(font::ui_font(app))
    .size(font::ui_size(app, 14))
    .padding(Padding::from([6, 10]))
    .width(Length::Fill)
    .into()
}

/// 圆角底色样本容器
fn sample_box<'a>(app: &'a App, children: Vec<Element<'a, Message>>) -> Element<'a, Message> {
    let p = constants::custom_palette(&app.theme);
    container(column(children).spacing(8))
        .width(Length::Fill)
        .padding(16)
        .style(move |_: &Theme| container::Style {
            background: Some(iced::Background::Color(p.surface)),
            border: Border::default().rounded(6),
            ..Default::default()
        })
        .into()
}

// ── 通用：语言 / 下载目录 ──

fn general_pane<'a>(app: &'a App) -> Element<'a, Message> {
    let lang_names: Vec<String> = constants::LANGUAGES
        .iter()
        .map(|(n, _)| n.to_string())
        .collect();
    let current_locale = rust_i18n::locale().to_string();
    let current_lang = constants::LANGUAGES
        .iter()
        .find(|(_, code)| *code == current_locale)
        .map(|(name, _)| name.to_string())
        .unwrap_or_else(|| "English".to_string());

    let language_pick = pick_list(lang_names, Some(current_lang), |name| {
        let code = constants::LANGUAGES
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, c)| c.to_string())
            .unwrap_or_else(|| "en".to_string());
        Message::LanguageChanged(code)
    })
    .font(font::ui_font(app))
    .text_size(font::ui_size(app, 14));

    let dir_input = text_input(&t!("download_dir_hint"), &app.download_dir)
        .on_input(Message::DownloadDirChanged)
        .font(font::ui_font(app))
        .size(font::ui_size(app, 14))
        .padding(Padding::from([7, 10]));

    column![
        pane_title(app, "settings_category_general"),
        pane_hint(app, "settings_general_hint"),
        rule::horizontal(1),
        control_row(
            app,
            text(t!("language").to_string()).font(font::ui_font(app)),
            language_pick,
        ),
        control_row(
            app,
            text(t!("download_dir").to_string()).font(font::ui_font(app)),
            dir_input,
        ),
    ]
    .spacing(16)
    .into()
}

// ── 外观：主题 / 界面字体 ──

fn appearance_pane<'a>(app: &'a App) -> Element<'a, Message> {
    let theme_names: Vec<String> = constants::AVAILABLE_THEMES
        .iter()
        .map(|(n, _)| n.to_string())
        .collect();

    let theme_pick = pick_list(
        theme_names,
        Some(app.current_theme_name.clone()),
        Message::ThemeChanged,
    )
    .font(font::ui_font(app))
    .text_size(font::ui_size(app, 14));

    let ui_sample = sample_box(
        app,
        vec![
            text(t!("settings_font_sample_ui").to_string())
                .font(font::ui_font(app))
                .size(font::ui_size(app, 12))
                .color(palette_secondary(app))
                .into(),
            text("The quick brown fox jumps over the lazy dog.")
                .font(font::ui_font(app))
                .size(font::ui_size(app, 15))
                .into(),
            text("S3 Desktop Manager 界面字体预览 0123456789")
                .font(font::ui_font(app))
                .size(font::ui_size(app, 14))
                .into(),
        ],
    );

    column![
        pane_title(app, "settings_category_appearance"),
        pane_hint(app, "settings_appearance_hint"),
        rule::horizontal(1),
        control_row(
            app,
            text(t!("theme").to_string()).font(font::ui_font(app)),
            theme_pick,
        ),
        rule::horizontal(1),
        section_label(app, "settings_ui_font"),
        control_row(
            app,
            text(t!("settings_ui_font_family_label").to_string()).font(font::ui_font(app)),
            font_combo(
                app,
                &app.ui_font_combo,
                &app.ui_font_family,
                t!("settings_font_family_default").as_ref(),
                t!("settings_font_search_placeholder").as_ref(),
                Message::UiFontFamilyChanged,
            )
        ),
        control_row(
            app,
            text(t!("settings_ui_font_size_label").to_string()).font(font::ui_font(app)),
            size_slider(app, app.ui_font_size, 10..=24, Message::UiFontSizeChanged)
        ),
        ui_sample,
    ]
    .spacing(16)
    .into()
}

// ── 预览：预览编辑器字体 ──

fn preview_pane<'a>(app: &'a App) -> Element<'a, Message> {
    let code_sample = sample_box(
        app,
        vec![
            text(t!("settings_font_sample_preview").to_string())
                .font(font::ui_font(app))
                .size(font::ui_size(app, 12))
                .color(palette_secondary(app))
                .into(),
            text("fn main() {")
                .font(font::preview_font(app))
                .size(f32::from(app.preview_font_size))
                .into(),
            text("    println!(\"Hello, S3 Desktop Manager!\");")
                .font(font::preview_font(app))
                .size(f32::from(app.preview_font_size))
                .into(),
            text("    // 中文注释：预览编辑器字体示例 0123456789")
                .font(font::preview_font(app))
                .size(f32::from(app.preview_font_size))
                .into(),
            text("}")
                .font(font::preview_font(app))
                .size(f32::from(app.preview_font_size))
                .into(),
        ],
    );

    column![
        pane_title(app, "settings_category_preview"),
        pane_hint(app, "settings_preview_hint"),
        rule::horizontal(1),
        section_label(app, "settings_preview_editor_font"),
        control_row(
            app,
            text(t!("settings_ui_font_family_label").to_string()).font(font::ui_font(app)),
            font_combo(
                app,
                &app.preview_font_combo,
                &app.preview_font_family,
                t!("settings_font_family_default_mono").as_ref(),
                t!("settings_font_search_placeholder").as_ref(),
                Message::PreviewFontFamilyChanged,
            )
        ),
        control_row(
            app,
            text(t!("settings_ui_font_size_label").to_string()).font(font::ui_font(app)),
            size_slider(
                app,
                app.preview_font_size,
                10..=22,
                Message::PreviewFontSizeChanged
            )
        ),
        code_sample,
    ]
    .spacing(16)
    .into()
}

// ── 更新：自动检查 + 手动检查 ──

fn updates_pane<'a>(app: &'a App) -> Element<'a, Message> {
    let check_label = if app.checking_update {
        t!("update_checking").to_string()
    } else {
        t!("update_check_now").to_string()
    };
    let check_btn = button(
        text(check_label)
            .font(font::ui_font(app))
            .size(font::ui_size(app, 13)),
    )
    .style(icon_button_style(app))
    .on_press_maybe(if app.checking_update {
        None
    } else {
        Some(Message::CheckForUpdates)
    });

    let auto_box = checkbox(app.auto_check_update)
        .label(t!("update_auto_check").to_string())
        .font(font::ui_font(app))
        .text_size(font::ui_size(app, 14))
        .on_toggle(Message::ToggleAutoCheckUpdate);

    let warn_color = iced::Color::from_rgb(0.9, 0.55, 0.2);
    let status: Element<Message> = match &app.update_check_status {
        Some(UpdateCheckStatus::UpToDate) => text(t!("update_no_update").to_string())
            .font(font::ui_font(app))
            .size(font::ui_size(app, 12))
            .color(palette_secondary(app))
            .into(),
        Some(UpdateCheckStatus::Error(e)) => {
            text(t!("update_error_in_panel", error = e.as_str()).to_string())
                .font(font::ui_font(app))
                .size(font::ui_size(app, 12))
                .color(warn_color)
                .into()
        }
        None => {
            if let Some(info) = &app.update_info {
                if app.update_dismissed {
                    text(t!("update_available_short", version = info.version.as_str()).to_string())
                        .font(font::ui_font(app))
                        .size(font::ui_size(app, 12))
                        .color(palette_secondary(app))
                        .into()
                } else {
                    button(
                        text(t!("update_view").to_string())
                            .font(font::ui_font(app))
                            .size(font::ui_size(app, 12)),
                    )
                    .style(icon_button_style(app))
                    .on_press(Message::OpenReleasePage)
                    .into()
                }
            } else {
                text(t!("update_unknown").to_string())
                    .font(font::ui_font(app))
                    .size(font::ui_size(app, 12))
                    .color(palette_secondary(app))
                    .into()
            }
        }
    };

    column![
        pane_title(app, "settings_category_updates"),
        pane_hint(app, "settings_updates_hint"),
        rule::horizontal(1),
        auto_box,
        control_row(
            app,
            text(t!("update_check_now").to_string()).font(font::ui_font(app)),
            row![
                check_btn,
                container(status)
                    .width(Length::Fill)
                    .align_x(Alignment::End)
            ]
            .spacing(10)
            .align_y(Alignment::Center)
        ),
    ]
    .spacing(16)
    .into()
}

// ── 关于：程序信息 ──

fn about_pane<'a>(app: &'a App) -> Element<'a, Message> {
    let app_icon = image(iced::widget::image::Handle::from_bytes(
        icon::WINDOW_ICON.to_vec(),
    ))
    .width(Length::Fixed(56.0))
    .height(Length::Fixed(56.0));

    let header = row![
        app_icon,
        column![
            text(t!("app_name").to_string())
                .font(font::ui_font(app))
                .size(font::ui_size(app, 18)),
            text(format!("{} {}", t!("version"), constants::APP_VERSION))
                .font(font::ui_font(app))
                .size(font::ui_size(app, 13))
                .color(palette_secondary(app)),
        ]
        .spacing(4)
        .align_x(Alignment::Start),
    ]
    .spacing(12)
    .align_y(Alignment::Center);

    let description = text(t!("app_description").to_string())
        .font(font::ui_font(app))
        .size(font::ui_size(app, 13))
        .color(palette_secondary(app))
        .width(Length::Fill);

    let meta = row![
        text(format!("{}: {}", t!("author"), constants::APP_AUTHOR))
            .font(font::ui_font(app))
            .size(font::ui_size(app, 12))
            .color(palette_secondary(app)),
        container(
            text(format!("{}: {}", t!("license"), constants::APP_LICENSE))
                .font(font::ui_font(app))
                .size(font::ui_size(app, 12))
                .color(palette_secondary(app))
        )
        .width(Length::Fill)
        .align_x(Alignment::End),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    column![
        pane_title(app, "settings_category_about"),
        rule::horizontal(1),
        header,
        description,
        meta,
    ]
    .spacing(18)
    .into()
}

// ─────────────────────────── 通用小工具 ───────────────────────────

/// 当前主题下的次要文本色
fn palette_secondary(app: &App) -> iced::Color {
    constants::custom_palette(&app.theme).text_secondary
}

/// 图标按钮样式（悬停淡色背景，与其余视图保持一致）
fn icon_button_style(app: &App) -> impl Fn(&Theme, button::Status) -> button::Style + '_ {
    let p = constants::custom_palette(&app.theme);
    move |_: &Theme, s: button::Status| {
        let hbg = iced::Color::from_rgba(1.0, 1.0, 1.0, 0.08);
        let (bg, border) = match s {
            button::Status::Hovered | button::Status::Pressed => (
                Some(iced::Background::Color(hbg)),
                Border {
                    color: hbg,
                    width: 1.0,
                    radius: 4.0.into(),
                },
            ),
            _ => (None, Border::default().width(0)),
        };
        button::Style {
            background: bg,
            border,
            text_color: p.text_secondary,
            shadow: iced::Shadow::default(),
            ..Default::default()
        }
    }
}

/// 渲染底部状态栏
///
/// 显示当前连接名称、桶名称、对象/桶数量等信息。
/// 未连接时显示"就绪"状态。
pub fn view_status_bar(app: &App) -> Element<'_, Message> {
    let p = constants::custom_palette(&app.theme);

    let status_text = if let Some(name) = &app.connecting_name {
        // 正在连接：优先显示连接进度，避免用户以为软件卡住
        t!("status_connecting", name = name.as_str()).to_string()
    } else if app.selected_connection_id.is_some() {
        let conn_name = app
            .config_store
            .list()
            .iter()
            .find(|c| Some(&c.id) == app.selected_connection_id.as_ref())
            .map(|c| c.name.as_str())
            .unwrap_or("?");
        let bucket_info = app
            .current_bucket
            .as_deref()
            .map(|b| format!(" | bucket: {}", b))
            .unwrap_or_default();
        let obj_count = if !app.objects.is_empty() {
            format!(" | {} {}", app.objects.len(), t!("status_objects"))
        } else if !app.buckets.is_empty() {
            format!(" | {} {}", app.buckets.len(), t!("status_buckets"))
        } else {
            String::new()
        };
        format!(
            "{}: {}{}{}",
            t!("status_connected"),
            conn_name,
            bucket_info,
            obj_count
        )
    } else {
        t!("status_ready").to_string()
    };

    let mut bar = row![
        text(status_text)
            .font(font::ui_font(app))
            .size(font::ui_size(app, 11))
            .color(p.text_secondary)
    ]
    .spacing(10)
    .padding(Padding::from([6, 16]))
    .align_y(Alignment::Center);

    // 加载中指示器：紧凑地显示在状态栏右侧，不占用额外空间
    if app.is_loading {
        // 下载中：优先展示带文件名的进度条；否则回退到通用"加载中..."
        let indicator: Element<Message> = match (&app.downloading_file, &app.download_progress) {
            // 已知总大小：显示进度条 + 已下载/总大小
            (Some(name), Some((downloaded, Some(total)))) if *total > 0 => {
                let ratio = (*downloaded as f32 / *total as f32).clamp(0.0, 1.0);
                row![
                    text(t!("downloading_progress", name = name.as_str()).to_string())
                        .font(font::ui_font(app))
                        .size(font::ui_size(app, 11))
                        .color(p.text_secondary),
                    progress_bar(0.0..=1.0, ratio)
                        .length(Length::Fixed(120.0))
                        .girth(Length::Fixed(8.0)),
                    text(format!(
                        "{}/{}",
                        constants::format_size(*downloaded as i64),
                        constants::format_size(*total as i64)
                    ))
                    .font(font::ui_font(app))
                    .size(font::ui_size(app, 11))
                    .color(p.text_secondary),
                ]
                .spacing(8)
                .align_y(Alignment::Center)
                .into()
            }
            // 总大小未知：不确定态文字，仅显示已下载字节数
            (Some(name), Some((downloaded, None))) => text(
                t!(
                    "downloading_unknown",
                    name = name.as_str(),
                    size = constants::format_size(*downloaded as i64)
                )
                .to_string(),
            )
            .font(font::ui_font(app))
            .size(font::ui_size(app, 11))
            .color(p.text_secondary)
            .into(),
            // 下载中但尚无进度数据：显示文件名
            (Some(name), _) => text(t!("downloading_status", name = name.as_str()).to_string())
                .font(font::ui_font(app))
                .size(font::ui_size(app, 11))
                .color(p.text_secondary)
                .into(),
            // 非下载类加载
            (None, _) => text(t!("loading").to_string())
                .font(font::ui_font(app))
                .size(font::ui_size(app, 11))
                .color(p.text_secondary)
                .into(),
        };
        bar = bar.push(
            container(indicator)
                .width(Length::Fill)
                .align_x(Alignment::End),
        );
    }

    bar.into()
}
