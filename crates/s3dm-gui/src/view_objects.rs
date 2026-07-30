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
use iced_aw::widget::drop_down::DropDown;
use iced_aw::core::alignment::Alignment as DropDownAlignment;
use rust_i18n::t;

use crate::app::App;
use crate::constants;
use crate::icon;
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
    // 禁用态预览按钮的图标颜色（更暗，表达“不可用”）
    let preview_disabled_svg_style = |t: &Theme, _: svg::Status| svg::Style {
        color: Some(disabled_icon_color(&constants::custom_palette(t))),
    };

    let refresh_svg = svg(SvgHandle::from_memory(icon::ICON_REFRESH.to_vec()))
        .width(Length::Fixed(16.0))
        .height(Length::Fixed(16.0))
        .style(svg_style);
    let upload_svg = svg(SvgHandle::from_memory(icon::ICON_CLOUD_UPLOAD.to_vec()))
        .width(Length::Fixed(16.0))
        .height(Length::Fixed(16.0))
        .style(svg_style);
    let back_svg = svg(SvgHandle::from_memory(icon::ICON_ARROW_LEFT.to_vec()))
        .width(Length::Fixed(16.0))
        .height(Length::Fixed(16.0))
        .style(svg_style);

    // ── 面包屑导航栏 ──
    let breadcrumb = row![
        button(back_svg)
            .style(icon_btn_style)
            .on_press(Message::BackToBuckets),
        row![
            svg(SvgHandle::from_memory(icon::ICON_FOLDER.to_vec()))
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
            svg(SvgHandle::from_memory(icon::ICON_FOLDER_ADD.to_vec()))
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
            svg(SvgHandle::from_memory(icon::ICON_DELETE.to_vec()))
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
                        svg(SvgHandle::from_memory(icon::ICON_FOLDER.to_vec()))
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

        // 下载图标：正在下载的对象显示 cloud-link 图标（用于"更多"菜单中的下载项）
        let is_downloading = app.downloading_key.as_deref() == Some(obj.key.as_str());
        let download_icon: &[u8] = if is_downloading {
            icon::ICON_CLOUD_LINK
        } else {
            icon::ICON_CLOUD_DOWNLOAD
        };

        // 预览按钮：所有文件都显示，但不可预览类型显示为禁用态（灰化、不可点）
        let can_preview = crate::preview::classify(&obj.key, obj.size)
            != crate::preview::PreviewKind::Unsupported;
        let is_previewing =
            app.preview_key.as_deref() == Some(obj.key.as_str()) && app.preview.is_some();
        let preview_svg_style = if can_preview {
            svg_style
        } else {
            preview_disabled_svg_style
        };
        let mut preview_btn = button(
            svg(SvgHandle::from_memory(icon::ICON_PREVIEW.to_vec()))
                .width(Length::Fixed(16.0))
                .height(Length::Fixed(16.0))
                .style(preview_svg_style),
        )
        .style(icon_btn_style);
        // 可预览且未在预览中时才绑定点击；否则按钮自动进入 Disabled 态
        if can_preview && !is_previewing {
            preview_btn = preview_btn.on_press(Message::PreviewObject(obj.key.clone()));
        }

        let row_children: Vec<Element<Message>> = vec![
            row![
                svg(SvgHandle::from_memory(icon::file_icon(name).to_vec()))
                    .width(Length::Fixed(14.0))
                    .height(Length::Fixed(14.0))
                    .style(svg_style),
                text(name).size(14),
            ]
            .spacing(4)
            .align_y(Alignment::Center)
            .into(),
            container(
                text(constants::format_size(obj.size))
                    .size(12)
                    .color(p.text_secondary),
            )
            .width(Length::Fill)
            .into(),
            text(
                obj.last_modified
                    .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_default(),
            )
            .size(12)
            .color(p.text_secondary)
            .into(),
            preview_btn.into(),
            // "更多"按钮：点击展开下拉菜单，内含下载与删除操作
            {
                // 菜单项样式（左对齐、带图标 + 文字）
                let menu_item_style =
                    move |_: &Theme, s: button::Status| -> button::Style {
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

                // 菜单项图标统一使用确定的次级文字色（避免浮层取色异常导致空白）
                let menu_svg_style = svg::Style {
                    color: Some(p.text_secondary),
                };

                // 预览菜单项
                let preview_menu_icon_color = if can_preview && !is_previewing {
                    p.text_secondary
                } else {
                    disabled_icon_color(&p)
                };

                let mut preview_item = button(
                    row![
                        svg(SvgHandle::from_memory(icon::ICON_PREVIEW.to_vec()))
                            .width(Length::Fixed(16.0))
                            .height(Length::Fixed(16.0))
                            .style(move |_: &Theme, _: svg::Status| svg::Style {
                                color: Some(preview_menu_icon_color),
                            }),
                        text(t!("preview").to_string()).size(14).color(p.text_secondary),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                )
                .style(menu_item_style)
                .width(Length::Fill);
                if can_preview && !is_previewing {
                    preview_item = preview_item.on_press(Message::PreviewObject(obj.key.clone()));
                }

                // 下载菜单项
                let mut download_item = button(
                    row![
                        svg(SvgHandle::from_memory(download_icon.to_vec()))
                            .width(Length::Fixed(16.0))
                            .height(Length::Fixed(16.0))
                            .style(move |_: &Theme, _: svg::Status| menu_svg_style),
                        text(t!("download").to_string()).size(14).color(p.text_secondary),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                )
                .style(menu_item_style)
                .width(Length::Fill);
                if !is_downloading {
                    download_item =
                        download_item.on_press(Message::DownloadObject(obj.key.clone()));
                }

                // 删除菜单项
                let delete_item = button(
                    row![
                        svg(SvgHandle::from_memory(icon::ICON_DELETE.to_vec()))
                            .width(Length::Fixed(16.0))
                            .height(Length::Fixed(16.0))
                            .style(move |_: &Theme, _: svg::Status| menu_svg_style),
                        text(t!("delete").to_string()).size(14).color(p.text_secondary),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                )
                .style(menu_item_style)
                .width(Length::Fill)
                .on_press(Message::DeleteObject(obj.key.clone()));

                // 菜单浮层内容
                let menu_overlay = container(column![preview_item, download_item, delete_item].spacing(2))
                    .padding(4)
                    .style(|theme: &Theme| container::Style {
                        background: Some(iced::Background::Color(constants::custom_palette(theme).surface)),
                        border: Border {
                            color: iced::Color::from_rgba(1.0, 1.0, 1.0, 0.12),
                            width: 1.0,
                            radius: 6.0.into(),
                        },
                        ..Default::default()
                    });

                // "更多"触发按钮
                let more_btn = button(
                    svg(SvgHandle::from_memory(icon::ICON_MORE_VERTICAL.to_vec()))
                        .width(Length::Fixed(16.0))
                        .height(Length::Fixed(16.0))
                        .style(svg_style),
                )
                .style(icon_btn_style)
                .on_press(Message::ToggleObjectMenu(Some(obj.key.clone())));

                DropDown::new(
                    more_btn,
                    menu_overlay,
                    app.open_menu_key.as_deref() == Some(obj.key.as_str()),
                )
                .alignment(DropDownAlignment::End)
                .offset(iced_aw::core::offset::Offset { x: 0.0, y: 4.0 })
                .on_dismiss(Message::ToggleObjectMenu(None))
                .width(Length::Fixed(140.0))
                .into()
            },
        ];

        let row_content = row(row_children).spacing(10).align_y(Alignment::Center);

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

    // ── 空状态提示（整体居中） ──
    if !app.is_truncated
        && app.current_prefix.is_empty()
        && app.common_prefixes.is_empty()
        && app.objects.is_empty()
    {
        let empty = container(
            text(t!("empty_bucket").to_string())
                .size(14)
                .color(p.text_secondary),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill);

        return container(column![breadcrumb, rule::horizontal(1), empty].spacing(10))
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
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

/// 计算禁用态图标颜色：在 `text_secondary` 基础上压暗，表达“不可用”
fn disabled_icon_color(p: &constants::CustomPalette) -> iced::Color {
    let c = p.text_secondary;
    iced::Color::from_rgba(c.r * 0.6, c.g * 0.6, c.b * 0.6, c.a.min(0.45))
}
