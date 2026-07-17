//! 连接表单视图
//!
//! 渲染添加/编辑 S3 连接的模态表单，包含：
//! - 标题（新建/编辑）
//! - 连接名称、端点、区域、访问密钥、秘密密钥等输入框
//! - Path Style 切换开关
//! - 保存/取消按钮

use iced::{
    Alignment, Border, Element, Length,
    widget::{
        Theme, button, column, container, row, rule, svg, svg::Handle as SvgHandle, text,
        text_input, toggler,
    },
};
use rust_i18n::t;

use crate::app::App;
use crate::constants;
use crate::message::Message;

/// 渲染连接表单面板（不含遮罩 overlay）
///
/// 返回表单内容元素，由 `view()` 中的 overlay 逻辑包装。
pub fn view_connection_form(app: &App) -> Element<'_, Message> {
    let form = app.connection_form.as_ref().unwrap();
    let title = if form.id.is_some() {
        t!("edit_connection_title").to_string()
    } else {
        t!("add_connection_title").to_string()
    };

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
            text(title).size(18),
            container(
                button(dismiss)
                    .style(btn_style)
                    .on_press(Message::ConnectionFormCancel)
            )
            .width(Length::Fill)
            .align_x(Alignment::End),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
        rule::horizontal(1),
        text_input(&t!("name"), &form.name).on_input(|v| {
            Message::ConnectionFormChanged {
                field: "name".into(),
                value: v,
            }
        }),
        text_input(&t!("endpoint_hint"), &form.endpoint).on_input(|v| {
            Message::ConnectionFormChanged {
                field: "endpoint".into(),
                value: v,
            }
        }),
        text_input(&t!("region"), &form.region).on_input(|v| {
            Message::ConnectionFormChanged {
                field: "region".into(),
                value: v,
            }
        }),
        text_input(&t!("access_key_id"), &form.access_key_id).on_input(|v| {
            Message::ConnectionFormChanged {
                field: "access_key_id".into(),
                value: v,
            }
        }),
        text_input(&t!("secret_access_key"), &form.secret_access_key).on_input(|v| {
            Message::ConnectionFormChanged {
                field: "secret_access_key".into(),
                value: v,
            }
        }),
        toggler(form.force_path_style)
            .label(t!("force_path_style_label").to_string())
            .on_toggle(|b| Message::ConnectionFormChanged {
                field: "force_path_style".into(),
                value: b.to_string(),
            }),
        row![
            button(text(t!("test_connection").to_string()))
                .style(btn_style)
                .on_press(Message::ConnectionFormTest),
            container(row![
                button(text(t!("save").to_string())).on_press(Message::ConnectionFormSave),
                button(text(t!("cancel").to_string())).on_press(Message::ConnectionFormCancel),
            ]
            .spacing(10))
            .width(Length::Fill)
            .align_x(Alignment::End),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
        {
            let pal = constants::custom_palette(&app.theme);
            let msg = if app.connection_testing {
                Some((t!("testing_connection").to_string(), pal.text_secondary))
            } else if let Some(result) = &app.connection_test_result {
                match result {
                    Ok(()) => Some((
                        t!("test_connection_success").to_string(),
                        iced::Color::from_rgb(0.3, 0.7, 0.3),
                    )),
                    Err(e) => Some((
                        t!("test_connection_failed", error = e.to_string()).to_string(),
                        iced::Color::from_rgb(0.8, 0.3, 0.3),
                    )),
                }
            } else {
                None
            };
            let result_widget: Element<'_, Message> = match msg {
                Some((text_str, color)) => text(text_str)
                    .size(13)
                    .style(move |_: &Theme| text::Style { color: Some(color) })
                    .into(),
                None => text("").size(13).into(),
            };
            result_widget
        },
    ]
    .spacing(10)
    .padding(20);

    container(panel)
        .width(420)
        .style(|theme: &Theme| container::Style {
            background: Some(iced::Background::Color(
                constants::custom_palette(theme).surface_raised,
            )),
            border: Border::default().rounded(8),
            ..Default::default()
        })
        .into()
}
