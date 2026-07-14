//! 设置面板与状态栏视图
//!
//! - `view_settings()`：主题、语言、下载目录设置的模态面板
//! - `view_status_bar()`：底部状态栏，显示当前连接/桶/对象计数信息

use iced::{
    Alignment, Border, Element, Length, Padding,
    widget::{
        Theme, button, column, container, pick_list, row, rule, svg, svg::Handle as SvgHandle,
        text, text_input,
    },
};
use rust_i18n::t;

use crate::app::App;
use crate::constants;
use crate::message::Message;

/// 渲染设置面板（不含遮罩 overlay）
///
/// 包含主题选择、语言切换、下载目录配置。
pub fn view_settings(app: &App) -> Element<'_, Message> {
    let theme_names: Vec<String> = constants::AVAILABLE_THEMES
        .iter()
        .map(|(n, _)| n.to_string())
        .collect();
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

    let p = constants::custom_palette(&app.theme);
    let btn_style = move |_: &Theme, s: button::Status| -> button::Style {
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
    };
    let svg_style = move |_: &Theme, _: svg::Status| svg::Style {
        color: Some(p.text_secondary),
    };
    let dismiss = svg(SvgHandle::from_memory(
        include_bytes!("../icons/dismiss-16-filled.svg").to_vec(),
    ))
    .width(Length::Fixed(16.0))
    .height(Length::Fixed(16.0))
    .style(svg_style);

    let panel = column![
        row![
            text(t!("settings").to_string()).size(20),
            container(
                button(dismiss)
                    .style(btn_style)
                    .on_press(Message::ToggleSettings)
            )
            .width(Length::Fill)
            .align_x(Alignment::End),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
        rule::horizontal(1),
        text(t!("theme").to_string()).size(16),
        pick_list(
            theme_names,
            Some(app.current_theme_name.clone()),
            Message::ThemeChanged
        ),
        text(t!("language").to_string()).size(16),
        pick_list(lang_names, Some(current_lang), |name| {
            let code = constants::LANGUAGES
                .iter()
                .find(|(n, _)| *n == name)
                .map(|(_, c)| c.to_string())
                .unwrap_or_else(|| "en".to_string());
            Message::LanguageChanged(code)
        }),
        text(t!("download_dir").to_string()).size(16),
        text_input(&t!("download_dir_hint"), &app.download_dir)
            .on_input(Message::DownloadDirChanged),
    ]
    .spacing(15)
    .padding(20);

    container(panel)
        .width(360)
        .style(|theme: &Theme| container::Style {
            background: Some(iced::Background::Color(
                constants::custom_palette(theme).surface_raised,
            )),
            border: Border::default().rounded(8),
            ..Default::default()
        })
        .into()
}

/// 渲染底部状态栏
///
/// 显示当前连接名称、桶名称、对象/桶数量等信息。
/// 未连接时显示"就绪"状态。
pub fn view_status_bar(app: &App) -> Element<'_, Message> {
    let p = constants::custom_palette(&app.theme);

    let status_text = if app.selected_connection_id.is_some() {
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

    row![text(status_text).size(11).color(p.text_secondary)]
        .padding(Padding::from([6, 16]))
        .align_y(Alignment::Center)
        .into()
}
