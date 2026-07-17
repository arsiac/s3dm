//! 存储桶列表视图
//!
//! 在右侧内容区展示当前已连接账户下的存储桶列表。
//! 选中某个桶后切换到对象浏览器；刷新按钮可重新拉取桶列表。

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

/// 渲染存储桶列表（右侧内容区）
pub fn view_buckets(app: &App) -> Element<'_, Message> {
    let p = constants::custom_palette(&app.theme);
    let palette = app.theme.palette();

    // 图标按钮悬停样式
    let icon_btn_style = move |_: &Theme, s: button::Status| -> button::Style {
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
    let svg_style = |t: &Theme, _: svg::Status| svg::Style {
        color: Some(constants::custom_palette(t).text_secondary),
    };

    let refresh_svg = svg(SvgHandle::from_memory(icon::ICON_REFRESH.to_vec()))
        .width(Length::Fixed(16.0))
        .height(Length::Fixed(16.0))
        .style(svg_style);

    // ── 标题栏 ──
    let header = row![
        row![
            svg(SvgHandle::from_memory(icon::ICON_FOLDER.to_vec()))
                .width(Length::Fixed(16.0))
                .height(Length::Fixed(16.0))
                .style(svg_style),
            text(t!("buckets").to_string()).size(16),
        ]
        .spacing(4)
        .align_y(Alignment::Center),
        text(format!("({})", app.buckets.len()))
            .size(14)
            .color(p.text_secondary),
        container(
            button(refresh_svg)
                .style(icon_btn_style)
                .on_press(Message::RefreshBuckets)
        )
        .width(Length::Fill)
        .align_x(Alignment::End),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    let mut items: Vec<Element<Message>> = Vec::new();

    // 空状态：加载中或没有桶
    if app.buckets.is_empty() {
        let label = if app.is_loading {
            t!("loading").to_string()
        } else {
            t!("no_buckets").to_string()
        };
        items.push(
            container(text(label).size(14).color(p.text_secondary))
                .padding(Padding::from([8, 16]))
                .into(),
        );
    }

    // ── 桶列表 ──
    for bucket in &app.buckets {
        let is_active = app.current_bucket.as_deref() == Some(&bucket.name);
        let bucket_bg = if is_active {
            Some(iced::Background::Color(iced::Color::from_rgba(
                1.0, 1.0, 1.0, 0.08,
            )))
        } else {
            None
        };
        items.push(
            button(
                row![
                    svg(SvgHandle::from_memory(icon::ICON_FOLDER.to_vec()))
                        .width(Length::Fixed(14.0))
                        .height(Length::Fixed(14.0))
                        .style(svg_style),
                    text(&bucket.name).size(14),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            )
            .on_press(Message::BucketSelected(bucket.name.clone()))
            .style(move |_: &Theme, s: button::Status| {
                let bg = match s {
                    button::Status::Hovered | button::Status::Pressed => Some(
                        iced::Background::Color(iced::Color::from_rgba(1.0, 1.0, 1.0, 0.08)),
                    ),
                    _ => bucket_bg,
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
    }

    let list = scrollable(column(items).spacing(4));

    container(column![header, rule::horizontal(1), list].spacing(10))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
