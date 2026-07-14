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
    widget::{Theme, button, column, container, row, rule, text, text_input},
};
use rust_i18n::t;

use crate::app::App;
use crate::constants;
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
