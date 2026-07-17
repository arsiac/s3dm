//! 左侧面板视图
//!
//! 包含连接列表和桶列表的渲染，是应用的主要导航区域。
//! 每个连接可展开查看其桶列表，支持添加/编辑/删除连接操作。

use iced::{
    Alignment, Border, Element, Length, Padding,
    widget::{
        Theme, button, column, container, row, rule, scrollable, svg, svg::Handle as SvgHandle,
        text,
    },
};
use rust_i18n::t;

use crate::app::App;
use crate::constants;
use crate::icon;
use crate::message::Message;

/// 渲染左侧面板
///
/// 结构：
/// - 标题栏（Storage Browser + 添加按钮 + 设置按钮）
/// - 连接列表（可展开/折叠，内联编辑/删除按钮）
/// - 展开后显示每个连接下的桶列表
pub fn view_left_panel(app: &App) -> Element<'_, Message> {
    let p = constants::custom_palette(&app.theme);
    let palette = app.theme.palette();
    let connections = app.config_store.list();

    // 通用悬停背景色
    let hover_bg = iced::Color::from_rgba(1.0, 1.0, 1.0, 0.08);

    // 图标按钮样式：仅悬停时显示微弱的背景
    let icon_btn_style = move |_: &Theme, s: button::Status| -> button::Style {
        let (bg, border) = match s {
            button::Status::Hovered | button::Status::Pressed => (
                Some(iced::Background::Color(hover_bg)),
                Border {
                    color: hover_bg,
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

    // SVG 图标着色样式
    let svg_style = |theme: &Theme, _: svg::Status| svg::Style {
        color: Some(constants::custom_palette(theme).text_secondary),
    };

    // ── 标题栏 ──
    let header = container(
        row![
            text(t!("storage_browser").to_string())
                .size(13)
                .color(p.text_secondary),
            container(
                button(
                    svg(SvgHandle::from_memory(icon::ICON_ADD.to_vec(),))
                        .width(Length::Fixed(16.0))
                        .height(Length::Fixed(16.0))
                        .style(svg_style),
                )
                .style(icon_btn_style)
                .on_press(Message::ConnectionAdd)
                .padding(Padding::from([2, 6]))
            )
            .width(Length::Fill)
            .align_x(Alignment::End),
            button(
                svg(SvgHandle::from_memory(icon::ICON_SETTINGS.to_vec(),))
                    .width(Length::Fixed(16.0))
                    .height(Length::Fixed(16.0))
                    .style(svg_style),
            )
            .style(icon_btn_style)
            .on_press(Message::ToggleSettings)
            .padding(Padding::from([2, 6])),
        ]
        .spacing(2)
        .align_y(Alignment::Center),
    )
    .padding(Padding::from([12, 16]))
    .width(Length::Fill);

    let mut items: Vec<Element<Message>> = Vec::new();

    // 空状态：无连接时显示提示
    if connections.is_empty() {
        items.push(
            container(
                text(t!("no_connection").to_string())
                    .size(12)
                    .color(p.text_secondary),
            )
            .padding(Padding::from([8, 16]))
            .width(Length::Fill)
            .into(),
        );
    }

    // ── 遍历连接列表 ──
    for conn in connections.iter() {
        let is_connected = app.selected_connection_id.as_ref() == Some(&conn.id);

        // 连接行上的操作按钮（编辑、删除）
        let action_btn_style = move |_: &Theme, s: button::Status| -> button::Style {
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
        let action_svg = |data: &[u8]| {
            svg(SvgHandle::from_memory(data.to_vec()))
                .width(Length::Fixed(14.0))
                .height(Length::Fixed(14.0))
                .style(|t: &Theme, _: svg::Status| svg::Style {
                    color: Some(constants::custom_palette(t).text_secondary),
                })
        };

        let edit_btn = button(action_svg(icon::ICON_EDIT))
            .style(action_btn_style)
            .on_press(Message::ConnectionEdit(conn.id.clone()))
            .padding(Padding::from([2, 4]));
        let delete_btn = button(action_svg(icon::ICON_DELETE))
            .style(action_btn_style)
            .on_press(Message::ConnectionDelete(conn.id.clone()))
            .padding(Padding::from([2, 4]));

        let conn_row = row![
            text(&conn.name).size(13),
            container(edit_btn)
                .width(Length::Fill)
                .align_x(Alignment::End),
            delete_btn,
            text("●").size(8).color(if is_connected {
                iced::Color::from_rgb(0.3, 0.8, 0.3)
            } else {
                p.text_secondary
            }),
        ]
        .spacing(2)
        .align_y(Alignment::Center);

        // 点击行为：选中连接并在右侧加载桶列表
        let msg = Message::ConnectionSelected(conn.id.clone());

        let row_bg = if is_connected {
            Some(iced::Background::Color(iced::Color::from_rgba(
                1.0, 1.0, 1.0, 0.04,
            )))
        } else {
            None
        };

        items.push(
            button(conn_row)
                .on_press(msg)
                .style(move |_: &Theme, s: button::Status| {
                    let bg = match s {
                        button::Status::Hovered | button::Status::Pressed => Some(
                            iced::Background::Color(iced::Color::from_rgba(1.0, 1.0, 1.0, 0.08)),
                        ),
                        _ => row_bg,
                    };
                    button::Style {
                        background: bg,
                        text_color: palette.text,
                        border: Border::default(),
                        shadow: iced::Shadow::default(),
                        ..Default::default()
                    }
                })
                .padding(Padding::from([8, 16]))
                .width(Length::Fill)
                .into(),
        );

        // ── 桶列表已移至右侧内容区展示（view_buckets） ──
    }

    let mut list_elements: Vec<Element<Message>> = vec![header.into(), rule::horizontal(1).into()];
    list_elements.extend(items);

    container(scrollable(column(list_elements).spacing(0)))
        .width(260)
        .style(|theme: &Theme| {
            let p = constants::custom_palette(theme);
            container::Style {
                background: Some(iced::Background::Color(p.surface)),
                ..Default::default()
            }
        })
        .height(Length::Fill)
        .into()
}
