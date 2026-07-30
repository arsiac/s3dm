//! 模态对话框渲染组件
//!
//! 提供应用中所有模态弹窗的渲染函数：
//! - 删除连接确认对话框
//! - 删除对象确认对话框
//! - 删除前缀确认对话框
//! - 新建文件夹输入对话框
//!
//! 每个函数返回已包装 `opaque` 的完整 overlay 元素，可直接加入 stack 中。

use iced::{
    Alignment, Border, Element, Length,
    Padding,
    widget::{Theme, button, column, container, row, rule, scrollable, svg, svg::Handle as SvgHandle, text, text_input},
};
use rust_i18n::t;

use crate::app::App;
use crate::constants;
use crate::icon;
use crate::message::Message;

/// 渲染删除连接确认对话框
pub fn delete_connection<'a>(
    app: &'a App,
    del_id: &'a str,
    conn_name: &'a str,
) -> Element<'a, Message> {
    let p = constants::custom_palette(&app.theme);
    let panel = column![
        text(t!("delete_confirm_title").to_string()).size(18),
        rule::horizontal(1),
        text(t!("delete_confirm_message", name = conn_name).to_string()).size(14),
        row![
            container(
                button(text(t!("confirm").to_string()))
                    .on_press(Message::ConfirmDelete(del_id.to_string()))
            )
            .width(Length::Fill)
            .align_x(Alignment::End),
            button(text(t!("cancel").to_string())).on_press(Message::CancelDelete),
        ]
        .spacing(10),
    ]
    .spacing(16)
    .padding(20);

    let content = container(panel)
        .width(360)
        .style(move |_: &Theme| container::Style {
            background: Some(iced::Background::Color(p.surface_raised)),
            border: Border::default().rounded(8),
            ..Default::default()
        });

    overlay_wrap(content)
}

/// 渲染删除对象确认对话框
pub fn delete_object<'a>(app: &'a App, del_key: &'a str) -> Element<'a, Message> {
    let obj_name = del_key.rsplit_once('/').map(|(_, n)| n).unwrap_or(del_key);
    let p = constants::custom_palette(&app.theme);
    let panel = column![
        text(t!("delete_object_confirm_title").to_string()).size(18),
        rule::horizontal(1),
        text(t!("delete_object_confirm_message", name = obj_name).to_string()).size(14),
        row![
            container(
                button(text(t!("confirm").to_string()))
                    .on_press(Message::ConfirmDeleteObject(del_key.to_string()))
            )
            .width(Length::Fill)
            .align_x(Alignment::End),
            button(text(t!("cancel").to_string())).on_press(Message::CancelDeleteObject),
        ]
        .spacing(10),
    ]
    .spacing(16)
    .padding(20);

    let content = container(panel)
        .width(360)
        .style(move |_: &Theme| container::Style {
            background: Some(iced::Background::Color(p.surface_raised)),
            border: Border::default().rounded(8),
            ..Default::default()
        });

    overlay_wrap(content)
}

/// 渲染删除前缀（文件夹）确认对话框
pub fn delete_prefix<'a>(app: &'a App, prefix: &'a str) -> Element<'a, Message> {
    let folder_name = prefix
        .trim_end_matches('/')
        .rsplit_once('/')
        .map(|(_, n)| n)
        .unwrap_or(prefix.trim_end_matches('/'));
    let p = constants::custom_palette(&app.theme);
    let panel = column![
        text(t!("delete_prefix_confirm_title").to_string()).size(18),
        rule::horizontal(1),
        text(t!("delete_prefix_confirm_message", name = folder_name).to_string()).size(14),
        row![
            container(
                button(text(t!("confirm").to_string()))
                    .on_press(Message::ConfirmDeletePrefix(prefix.to_string()))
            )
            .width(Length::Fill)
            .align_x(Alignment::End),
            button(text(t!("cancel").to_string())).on_press(Message::CancelDeletePrefix),
        ]
        .spacing(10),
    ]
    .spacing(16)
    .padding(20);

    let content = container(panel)
        .width(360)
        .style(move |_: &Theme| container::Style {
            background: Some(iced::Background::Color(p.surface_raised)),
            border: Border::default().rounded(8),
            ..Default::default()
        });

    overlay_wrap(content)
}

/// 渲染新建文件夹输入对话框
pub fn new_folder<'a>(app: &'a App, input: &'a str) -> Element<'a, Message> {
    let p = constants::custom_palette(&app.theme);
    let panel = column![
        text(t!("new_folder_title").to_string()).size(18),
        rule::horizontal(1),
        text_input(&t!("new_folder_placeholder"), input).on_input(Message::NewFolderInputChanged),
        row![
            container(button(text(t!("confirm").to_string())).on_press(Message::CreateNewFolder))
                .width(Length::Fill)
                .align_x(Alignment::End),
            button(text(t!("cancel").to_string())).on_press(Message::ToggleNewFolder),
        ]
        .spacing(10),
    ]
    .spacing(16)
    .padding(20);

    let content = container(panel)
        .width(360)
        .style(move |_: &Theme| container::Style {
            background: Some(iced::Background::Color(p.surface_raised)),
            border: Border::default().rounded(8),
            ..Default::default()
        });

    overlay_wrap(content)
}

/// 渲染重命名文件对话框
pub fn rename_dialog<'a>(app: &'a App, _old_key: &'a str, current_name: &'a str) -> Element<'a, Message> {
    let p = constants::custom_palette(&app.theme);
    let panel = column![
        text(t!("rename_title").to_string()).size(18),
        rule::horizontal(1),
        text_input(&t!("rename_placeholder"), current_name)
            .on_input(Message::RenameInputChanged),
        row![
            container(
                button(text(t!("confirm").to_string()))
                    .on_press(Message::ConfirmRename)
            )
            .width(Length::Fill)
            .align_x(Alignment::End),
            button(text(t!("cancel").to_string())).on_press(Message::CancelRename),
        ]
        .spacing(10),
    ]
    .spacing(16)
    .padding(20);

    let content = container(panel)
        .width(360)
        .style(move |_: &Theme| container::Style {
            background: Some(iced::Background::Color(p.surface_raised)),
            border: Border::default().rounded(8),
            ..Default::default()
        });

    overlay_wrap(content)
}

/// 渲染复制/移动文件对话框
pub fn copy_move_dialog<'a>(app: &'a App, state: &'a crate::app::CopyMoveState) -> Element<'a, Message> {
    let p = constants::custom_palette(&app.theme);
    let svg_style = move |_: &Theme, _: svg::Status| svg::Style {
        color: Some(p.text_secondary),
    };
    let title = match state.mode {
        crate::app::CopyMoveMode::Copy => t!("copy_title").to_string(),
        crate::app::CopyMoveMode::Move => t!("move_title").to_string(),
    };

    // ── 文件夹列表（条件渲染） ──
    let folder_list: Vec<Element<Message>> = if state.is_loading_prefixes {
        vec![
            container(text(t!("loading").to_string()).size(13).color(p.text_secondary))
                .width(Length::Fill)
                .center_x(Length::Fill)
                .padding(Padding::from([16, 0]))
                .into(),
        ]
    } else {
        let mut items: Vec<Element<Message>> = Vec::new();
        // "返回上级"导航（仅当不在根路径时显示）
        if !state.target_prefix.is_empty() {
            items.push(
                button(
                    row![
                        text("📂 ..").size(13),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center),
                )
                .on_press(Message::NavigateUpTargetFolder)
                .style(move |_: &Theme, _: button::Status| button::Style {
                    background: None,
                    text_color: p.text_secondary,
                    border: Border::default(),
                    shadow: iced::Shadow::default(),
                    ..Default::default()
                })
                .padding(Padding::from([6, 0]))
                .width(Length::Fill)
                .into(),
            );
        }
        // 子文件夹列表
        for prefix in &state.available_prefixes {
            let display_name = prefix.trim_end_matches('/');
            let folder_clone = prefix.clone();
            items.push(
                button(
                    row![
                        svg(SvgHandle::from_memory(icon::ICON_FOLDER.to_vec()))
                            .width(Length::Fixed(14.0))
                            .height(Length::Fixed(14.0))
                            .style(svg_style),
                        text(display_name).size(13),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center),
                )
                .on_press(Message::NavigateIntoTargetFolder(folder_clone))
                .style(move |_: &Theme, _: button::Status| button::Style {
                    background: None,
                    text_color: p.text_secondary,
                    border: Border::default(),
                    shadow: iced::Shadow::default(),
                    ..Default::default()
                })
                .padding(Padding::from([6, 0]))
                .width(Length::Fill)
                .into(),
            );
        }
        items
    };

    let panel = container(
        column![
            text(title).size(18),
            rule::horizontal(1),
            // 源文件信息
            text(t!("copy_source_label", path = state.source_key.as_str()).to_string())
                .size(12)
                .color(p.text_secondary),
            // 目标路径输入
            text(t!("copy_target_prefix").to_string()).size(13),
            text_input("", &state.target_prefix)
                .on_input(Message::TargetPrefixInputChanged),
            // 文件夹浏览器
            container(
                scrollable(
                    column(folder_list).spacing(2)
                )
                .height(Length::Fixed(180.0))
            )
            .style(|_: &Theme| container::Style {
                background: Some(iced::Background::Color(iced::Color::from_rgba(0.0, 0.0, 0.0, 0.06))),
                border: Border::default().rounded(4),
                ..Default::default()
            })
            .padding(8)
            .width(Length::Fill),
            // 新文件名
            text(t!("copy_target_name").to_string()).size(13),
            text_input(&state.new_name, &state.new_name)
                .on_input(|v| Message::CopyMoveInputChanged {
                    field: "new_name".to_string(),
                    value: v,
                }),
            // 对话框内错误提示
            if let Some(ref err) = state.error {
                let e: Element<Message> = container(text(err).size(12).color(iced::Color::from_rgb(0.9, 0.3, 0.3)))
                    .padding(Padding::from([4, 0]))
                    .width(Length::Fill)
                    .into();
                e
            } else {
                let e: Element<Message> = iced::widget::Space::new().into();
                e
            },
            // 按钮
            row![
                container(
                    button(text(t!("confirm").to_string()))
                        .on_press(Message::ConfirmCopyMove)
                )
                .width(Length::Fill)
                .align_x(Alignment::End),
                button(text(t!("cancel").to_string())).on_press(Message::CancelCopyMove),
            ]
            .spacing(10),
        ]
        .spacing(10)
        .padding(20)
    );

    let content = container(panel)
        .width(440)
        .style(move |_: &Theme| container::Style {
            background: Some(iced::Background::Color(p.surface_raised)),
            border: Border::default().rounded(8),
            ..Default::default()
        })
        .max_height(560);

    overlay_wrap(content)
}

/// 通用半透明遮罩 + 居中容器包装
///
/// 将面板内容居中放置在暗色半透明背景上，形成模态弹窗效果。
fn overlay_wrap<'a>(content: container::Container<'a, Message>) -> Element<'a, Message> {
    let overlay = container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_: &Theme| container::Style {
            background: Some(iced::Background::Color(iced::Color::from_rgba(
                0.0, 0.0, 0.0, 0.6,
            ))),
            ..Default::default()
        })
        .center_x(Length::Fill)
        .center_y(Length::Fill);

    iced::widget::opaque(overlay)
}
