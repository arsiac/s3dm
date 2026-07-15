//! 应用主视图编排
//!
//! 本模块是 View 层的入口，负责组合所有子视图组件并编排整体布局：
//! - 顶层布局：左面板 + 分隔线 + 右侧内容区 + 底部状态栏
//! - 错误提示栏（条件渲染）
//! - 模态叠加层（设置面板、连接表单、各确认弹窗）
//! - 加载中遮罩

use iced::{
    Alignment, Element, Length,
    widget::{Theme, button, column, container, row, rule, text},
};
use rust_i18n::t;

use crate::app::App;
use crate::message::Message;

use crate::view_buckets::view_buckets;
use crate::view_dialogs;
use crate::view_form::view_connection_form;
use crate::view_left_panel::view_left_panel;
use crate::view_objects::view_objects;
use crate::view_settings::{view_settings, view_status_bar};

/// 应用主视图入口
///
/// 组装三层布局结构：
/// 1. 错误提示栏（可选）
/// 2. 主体区域（左面板 + 内容区 + 状态栏）
/// 3. 模态叠加层（设置、表单、对话框，通过 stack 实现）
pub fn view(app: &App) -> Element<'_, Message> {
    let mut elements: Vec<Element<Message>> = Vec::new();

    // ── 错误提示栏 ──
    if let Some(err) = &app.error_message {
        let error_bar = container(
            row![
                text(t!("error", message = err.as_str()).to_string()).color(iced::Color::WHITE),
                button("×").on_press(Message::ClearError),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        )
        .padding(10)
        .style(|_: &Theme| container::Style {
            background: Some(iced::Background::Color(iced::Color::from_rgb(
                0.8, 0.2, 0.2,
            ))),
            text_color: Some(iced::Color::WHITE),
            ..Default::default()
        })
        .width(Length::Fill);
        elements.push(error_bar.into());
    }

    // ── 主布局 ──
    let side_panel = view_left_panel(app);
    let right_area = view_right_content(app);
    let status_line = view_status_bar(app);

    let body = row![
        side_panel,
        rule::vertical(1),
        container(right_area)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(20),
    ]
    .height(Length::Fill);

    elements.push(
        column![body, rule::horizontal(1), status_line]
            .spacing(0)
            .height(Length::Fill)
            .into(),
    );

    let content = container(column(elements).spacing(0))
        .width(Length::Fill)
        .height(Length::Fill);

    // ── 叠加层（stack） ──
    let mut stack_elements: Vec<Element<Message>> = vec![content.into()];

    // 设置面板
    if app.show_settings {
        stack_elements.push(overlay_element(view_settings(app)));
    }

    // 连接表单
    if app.connection_form.is_some() {
        stack_elements.push(overlay_element(view_connection_form(app)));
    }

    // 删除连接确认弹窗
    if let Some(ref del_id) = app.pending_delete {
        let conn_name = app
            .config_store
            .list()
            .iter()
            .find(|c| &c.id == del_id)
            .map(|c| c.name.as_str())
            .unwrap_or("?");
        stack_elements.push(view_dialogs::delete_connection(app, del_id, conn_name));
    }

    // 删除对象确认弹窗
    if let Some(ref del_key) = app.pending_delete_object {
        stack_elements.push(view_dialogs::delete_object(app, del_key));
    }

    // 新建文件夹输入弹窗
    if let Some(ref input) = app.new_folder_input {
        stack_elements.push(view_dialogs::new_folder(app, input));
    }

    // 删除前缀确认弹窗
    if let Some(ref prefix) = app.pending_delete_prefix {
        stack_elements.push(view_dialogs::delete_prefix(app, prefix));
    }

    iced::widget::stack(stack_elements).into()
}

/// 渲染通用模态叠加层包装（半透明遮罩 + 居中 + opaque）
fn overlay_element<'a>(child: Element<'a, Message>) -> Element<'a, Message> {
    let overlay = container(child)
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

/// 渲染右侧内容区域
///
/// - 已选中桶：显示对象浏览器
/// - 已选中连接：显示该连接下的存储桶列表
/// - 无连接：提示添加连接
/// - 其他：提示选择连接
fn view_right_content(app: &App) -> Element<'_, Message> {
    if app.current_bucket.is_some() {
        view_objects(app)
    } else if app.selected_connection_id.is_some() {
        view_buckets(app)
    } else {
        let p = crate::constants::custom_palette(&app.theme);
        let hint = if app.config_store.list().is_empty() {
            t!("no_connection")
        } else {
            t!("select_connection_hint")
        };
        container(text(hint.to_string()).size(16).color(p.text_secondary))
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    }
}
