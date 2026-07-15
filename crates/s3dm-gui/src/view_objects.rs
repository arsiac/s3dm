//! 对象浏览器视图
//!
//! 渲染当前桶和路径下的文件/文件夹列表，包括：
//! - 面包屑导航（桶名 + 路径 + 刷新/上传/新建文件夹按钮）
//! - "返回上级"导航项
//! - 文件夹列表（可点击进入，可删除）
//! - 文件列表（含大小、修改时间、下载/删除操作）
//! - "加载更多"分页按钮

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
use crate::message::Message;

/// 渲染对象/文件浏览器
///
/// 仅当 `current_bucket` 已设置时调用。
pub fn view_objects(app: &App) -> Element<'_, Message> {
    let p = constants::custom_palette(&app.theme);
    let unknown_label = t!("unknown").to_string();
    let bucket_name = app
        .current_bucket
        .as_deref()
        .unwrap_or(&unknown_label)
        .to_string();

    // 图标按钮统一样式
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

    let refresh_svg = svg(SvgHandle::from_memory(
        include_bytes!("../icons/arrow-clockwise-16-filled.svg").to_vec(),
    ))
    .width(Length::Fixed(16.0))
    .height(Length::Fixed(16.0))
    .style(svg_style);
    let upload_svg = svg(SvgHandle::from_memory(
        include_bytes!("../icons/cloud-arrow-up-16-filled.svg").to_vec(),
    ))
    .width(Length::Fixed(16.0))
    .height(Length::Fixed(16.0))
    .style(svg_style);
    let back_svg = svg(SvgHandle::from_memory(
        include_bytes!("../icons/arrow-left-16-filled.svg").to_vec(),
    ))
    .width(Length::Fixed(16.0))
    .height(Length::Fixed(16.0))
    .style(svg_style);

    // ── 面包屑导航栏 ──
    let breadcrumb = row![
        button(back_svg)
            .style(icon_btn_style)
            .on_press(Message::BackToBuckets),
        row![
            svg(SvgHandle::from_memory(
                include_bytes!("../icons/folder-16-filled.svg").to_vec()
            ))
            .width(Length::Fixed(16.0))
            .height(Length::Fixed(16.0))
            .style(svg_style),
            text(bucket_name).size(16),
        ]
        .spacing(4)
        .align_y(Alignment::Center),
        text(&app.current_prefix).size(14).color(p.text_secondary),
        container(
            button(refresh_svg)
                .style(icon_btn_style)
                .on_press(Message::RefreshObjects)
        )
        .width(Length::Fill)
        .align_x(Alignment::End),
        button(upload_svg)
            .style(icon_btn_style)
            .on_press(Message::UploadObject),
        button(
            svg(SvgHandle::from_memory(
                include_bytes!("../icons/folder-add-16-filled.svg").to_vec()
            ))
            .width(Length::Fixed(16.0))
            .height(Length::Fixed(16.0))
            .style(svg_style),
        )
        .style(icon_btn_style)
        .on_press(Message::ToggleNewFolder),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    let mut items: Vec<Element<Message>> = Vec::new();

    // 文件行按钮样式
    let row_style = |theme: &Theme, _: button::Status| -> button::Style {
        let p = constants::custom_palette(theme);
        button::Style {
            background: Some(iced::Background::Color(p.surface)),
            text_color: theme.palette().text,
            border: Border::default().rounded(4),
            shadow: iced::Shadow::default(),
            ..Default::default()
        }
    };

    // ── "返回上级"导航 ──
    if !app.current_prefix.is_empty() {
        items.push(
            button(
                row![
                    text("📂 ..").size(14),
                    container(text("")).width(Length::Fill),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            )
            .on_press(Message::NavigateUp)
            .style(row_style)
            .padding(Padding::from([8, 16]))
            .into(),
        );
    }

    // ── 文件夹列表 ──
    for prefix in &app.common_prefixes {
        let display_name = prefix
            .strip_prefix(&app.current_prefix)
            .unwrap_or(prefix)
            .trim_end_matches('/');

        let folder_delete_btn = button(
            svg(SvgHandle::from_memory(
                include_bytes!("../icons/delete-16-filled.svg").to_vec(),
            ))
            .width(Length::Fixed(16.0))
            .height(Length::Fixed(16.0))
            .style(svg_style),
        )
        .style(icon_btn_style)
        .on_press(Message::DeletePrefix(prefix.clone()));

        items.push(
            button(
                row![
                    row![
                        svg(SvgHandle::from_memory(
                            include_bytes!("../icons/folder-16-filled.svg").to_vec()
                        ))
                        .width(Length::Fixed(14.0))
                        .height(Length::Fixed(14.0))
                        .style(svg_style),
                        text(display_name).size(14),
                    ]
                    .spacing(4)
                    .align_y(Alignment::Center),
                    container(folder_delete_btn)
                        .width(Length::Fill)
                        .align_x(Alignment::End),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            )
            .on_press(Message::PrefixSelected(prefix.clone()))
            .style(row_style)
            .padding(Padding::from([8, 16]))
            .into(),
        );
    }

    // ── 文件列表 ──
    for obj in &app.objects {
        let name = obj
            .key
            .strip_prefix(&app.current_prefix)
            .unwrap_or(&obj.key);
        if name.is_empty() {
            continue;
        }

        let row_content = row![
            row![
                svg(SvgHandle::from_memory(
                    include_bytes!("../icons/document-16-filled.svg").to_vec()
                ))
                .width(Length::Fixed(14.0))
                .height(Length::Fixed(14.0))
                .style(svg_style),
                text(name).size(14),
            ]
            .spacing(4)
            .align_y(Alignment::Center),
            container(
                text(constants::format_size(obj.size))
                    .size(12)
                    .color(p.text_secondary)
            )
            .width(Length::Fill),
            text(
                obj.last_modified
                    .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_default()
            )
            .size(12)
            .color(p.text_secondary),
            button(
                svg(SvgHandle::from_memory(
                    include_bytes!("../icons/cloud-arrow-down-16-filled.svg").to_vec()
                ))
                .width(Length::Fixed(16.0))
                .height(Length::Fixed(16.0))
                .style(svg_style),
            )
            .style(icon_btn_style)
            .on_press(Message::DownloadObject(obj.key.clone())),
            button(
                svg(SvgHandle::from_memory(
                    include_bytes!("../icons/delete-16-filled.svg").to_vec()
                ))
                .width(Length::Fixed(16.0))
                .height(Length::Fixed(16.0))
                .style(svg_style),
            )
            .style(icon_btn_style)
            .on_press(Message::DeleteObject(obj.key.clone())),
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        items.push(
            container(row_content)
                .padding(Padding::from([8, 16]))
                .style(|theme: &Theme| container::Style {
                    background: Some(iced::Background::Color(
                        constants::custom_palette(theme).surface,
                    )),
                    border: Border::default().rounded(4),
                    ..Default::default()
                })
                .width(Length::Fill)
                .into(),
        );
    }

    // ── "加载更多"分页按钮 ──
    if app.is_truncated {
        items.push(
            container(button(text(t!("load_more").to_string())).on_press(Message::LoadMoreObjects))
                .padding(Padding::from([8, 16]))
                .center_x(Length::Fill)
                .width(Length::Fill)
                .into(),
        );
    }

    let list = scrollable(column(items).spacing(4));

    container(column![breadcrumb, rule::horizontal(1), list].spacing(10))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
