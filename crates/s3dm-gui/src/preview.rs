//! 对象预览（文本 / 代码 / 图片）
//!
//! 本模块负责：
//! - 根据对象 Key 与大小判定预览类型（文本 / 代码 / 图片 / 过大 / 不支持）
//! - 将代码内容通过 `iced_highlighter` 逐行语法高亮为带颜色的 `Span`
//! - 渲染预览弹窗（行号 + 高亮代码 / 纯文本 / 图片 / 提示）

use iced::{
    Alignment, Border, Element, Length,
    widget::{
        Theme, button, column, container, image, image::Handle, row, rule, scrollable, svg,
        svg::Handle as SvgHandle, text,
    },
};
use iced_highlighter::{Settings, Stream, Theme as HiTheme};
use rust_i18n::t;

use crate::app::App;
use crate::constants;
use crate::icon;
use crate::message::Message;

/// 预览代码/文本使用的 `Span` 类型别名（Link 固定为 `Message`）
type Span<'a> = iced::widget::text::Span<'a, Message>;

/// 预览内容分类结果
#[derive(Debug, Clone)]
pub enum PreviewContent {
    /// 纯文本
    Text(String),
    /// 代码（含语法 token 与原文）
    Code { token: String, content: String },
    /// 图片字节
    Image(Vec<u8>),
    /// SVG 矢量字节
    Svg(Vec<u8>),
    /// 文件过大，无法预览
    TooLarge,
    /// 类型不支持预览
    Unsupported,
}

/// 预览类型分类
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PreviewKind {
    Text,
    Code,
    Image,
    /// SVG 矢量图（使用 svg 部件渲染，而非位图 image 部件）
    Svg,
    /// 文件过大，仍提供预览入口，打开后提示下载
    TooLarge,
    Unsupported,
}

/// 预览大小阈值：超过 5MB 不预览，提示下载
const MAX_PREVIEW_BYTES: u64 = 5 * 1024 * 1024;

/// 根据对象 Key 与大小判定预览类型
pub fn classify(key: &str, size: i64) -> PreviewKind {
    let lower = key.to_ascii_lowercase();
    let ext = lower.rsplit('.').next().unwrap_or("");

    // 压缩包等二进制不预览（无论大小都不显示预览按钮）
    if matches!(
        ext,
        "zip" | "rar" | "7z" | "gz" | "xz" | "bz2" | "zst" | "tar" | "tgz"
    ) {
        return PreviewKind::Unsupported;
    }

    let base = match ext {
        "txt" | "log" => PreviewKind::Text,
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "ico" | "tiff" | "heic" => {
            PreviewKind::Image
        }
        "svg" => PreviewKind::Svg,
        "json" | "yaml" | "yml" | "toml" | "py" | "rs" | "c" | "h" | "hpp" | "cpp" | "cc"
        | "java" | "js" | "ts" | "tsx" | "jsx" | "go" | "rb" | "php" | "sh" | "bash" | "md"
        | "html" | "css" | "xml" | "csv" | "ini" | "cfg" | "conf" => PreviewKind::Code,
        "mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" | "mp4" | "mkv" | "avi" | "mov" | "webm"
        | "flv" => PreviewKind::Unsupported,
        // 二进制文档 / Office / 可执行 / 字体等：无法作为文本或图片预览
        "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "odt" | "ods" | "odp"
        | "exe" | "dll" | "so" | "dylib" | "bin" | "dat" | "class" | "o" | "a" | "obj" | "ttf"
        | "otf" | "woff" | "woff2" | "eot" => PreviewKind::Unsupported,
        _ => PreviewKind::Text,
    };

    // 只有本可预览的类型（文本/代码/图片/SVG）超阈值才降级为 TooLarge；
    // 明确不支持的类型（压缩包/音视频）无论大小都不显示预览按钮。
    if matches!(
        base,
        PreviewKind::Text | PreviewKind::Code | PreviewKind::Image | PreviewKind::Svg
    ) && size as u64 > MAX_PREVIEW_BYTES
    {
        PreviewKind::TooLarge
    } else {
        base
    }
}

/// 根据扩展名映射到 `iced_highlighter` 的语法 token（Sublime 语法名）
fn lang_token(key: &str) -> String {
    let lower = key.to_ascii_lowercase();
    let ext = lower.rsplit('.').next().unwrap_or("");
    let token = match ext {
        "rs" => "rust",
        "py" => "python",
        "js" => "javascript",
        "ts" => "typescript",
        "tsx" => "tsx",
        "jsx" => "jsx",
        "json" => "json",
        "yaml" | "yml" => "yaml",
        "toml" => "toml",
        "md" => "markdown",
        "html" | "htm" => "html",
        "css" => "css",
        "xml" => "xml",
        "c" | "h" => "c",
        "cpp" | "cc" | "cxx" | "hpp" => "cpp",
        "java" => "java",
        "go" => "go",
        "rb" => "ruby",
        "php" => "php",
        "sh" | "bash" => "bash",
        "csv" => "csv",
        "ini" | "cfg" | "conf" => "ini",
        "log" | "txt" => "plaintext",
        _ => "plaintext",
    };
    token.to_string()
}

/// 根据应用主题明暗选择高亮主题
fn hi_theme(theme: &Theme) -> HiTheme {
    let bg = theme.palette().background;
    let luminance = 0.299 * bg.r + 0.587 * bg.g + 0.114 * bg.b;
    if luminance > 0.5 {
        HiTheme::InspiredGitHub
    } else {
        HiTheme::SolarizedDark
    }
}

/// 将一行高亮结果转换为带颜色的 `Span` 列表
fn line_spans(line: &str, stream: &mut Stream, font: iced::Font) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    for (range, highlight) in stream.highlight_line(line) {
        let text = line.get(range.clone()).unwrap_or("").to_string();
        if text.is_empty() {
            continue;
        }
        let mut span = Span::new(text);
        if let Some(color) = highlight.color() {
            span = span.color(color);
        }
        if let Some(f) = highlight.font() {
            span = span.font(f);
        } else {
            span = span.font(font);
        }
        spans.push(span.to_static());
    }
    // 末尾换行（保留空行高度）
    spans.push(Span::new("\n").font(font).to_static());
    spans
}

/// 将代码内容渲染为带行号的高亮文本元素列表（每行一个 rich_text）
fn render_code(content: &str, token: &str, theme: &Theme) -> Element<'static, Message> {
    let font = iced::Font::MONOSPACE;
    let size = 13.0;
    let p = constants::custom_palette(theme);
    let line_no_color = p.text_secondary;
    let hi = hi_theme(theme);
    let mut stream = Stream::new(&Settings {
        theme: hi,
        token: token.to_string(),
    });

    let total_lines = content.lines().count().max(1);
    let digits = total_lines.to_string().len().max(2);

    let mut elements: Vec<Element<'static, Message>> = Vec::new();

    for (idx, line) in content.lines().enumerate() {
        let line_no = format!("{:>width$} │ ", idx + 1, width = digits);
        let line_no_span = Span::new(line_no)
            .font(font)
            .color(line_no_color)
            .to_static();

        let mut spans = vec![line_no_span];
        spans.extend(line_spans(line, &mut stream, font));
        stream.commit();

        let rt = iced::widget::rich_text(spans).font(font).size(size).into();
        elements.push(rt);
    }

    scrollable(column(elements).spacing(0))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// 渲染预览弹窗主体内容
fn preview_body<'a>(app: &'a App, content: &'a PreviewContent) -> Element<'a, Message> {
    let p = constants::custom_palette(&app.theme);
    match content {
        PreviewContent::Text(text_content) => {
            let font = iced::Font::MONOSPACE;
            let total_lines = text_content.lines().count().max(1);
            let digits = total_lines.to_string().len().max(2);
            let mut elements: Vec<Element<'_, Message>> = Vec::new();
            for (idx, line) in text_content.lines().enumerate() {
                let line_no = format!("{:>width$} │ ", idx + 1, width = digits);
                let spans = vec![
                    Span::new(line_no)
                        .font(font)
                        .color(p.text_secondary)
                        .to_static(),
                    Span::new(format!("{}\n", line)).font(font).to_static(),
                ];
                elements.push(iced::widget::rich_text(spans).font(font).size(13.0).into());
            }
            scrollable(column(elements).spacing(0))
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        }
        PreviewContent::Code { token, content } => render_code(content, token, &app.theme),
        PreviewContent::Image(bytes) => container(
            image(Handle::from_bytes(bytes.clone()))
                .content_fit(iced::ContentFit::Contain)
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into(),
        PreviewContent::Svg(bytes) => container(
            svg(SvgHandle::from_memory(bytes.clone()))
                .content_fit(iced::ContentFit::Contain)
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into(),
        PreviewContent::TooLarge => container(
            text(t!("preview_too_large").to_string())
                .size(14)
                .color(p.text_secondary),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into(),
        PreviewContent::Unsupported => container(
            text(t!("preview_unsupported").to_string())
                .size(14)
                .color(p.text_secondary),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into(),
    }
}

/// 渲染预览弹窗（半透明遮罩 + 居中面板）
pub fn view<'a>(app: &'a App, content: &'a PreviewContent, key: &'a str) -> Element<'a, Message> {
    let p = constants::custom_palette(&app.theme);
    let obj_name = key.rsplit_once('/').map(|(_, n)| n).unwrap_or(key);
    let size_label = app
        .objects
        .iter()
        .find(|o| o.key == key)
        .map(|o| constants::format_size(o.size))
        .unwrap_or_default();

    let svg_style = |t: &Theme, _: svg::Status| svg::Style {
        color: Some(constants::custom_palette(t).text_secondary),
    };

    let dismiss = svg(SvgHandle::from_memory(icon::ICON_DISMISS.to_vec()))
        .width(Length::Fixed(16.0))
        .height(Length::Fixed(16.0))
        .style(svg_style);

    let header = row![
        svg(SvgHandle::from_memory(icon::file_icon(obj_name).to_vec()))
            .width(Length::Fixed(16.0))
            .height(Length::Fixed(16.0))
            .style(svg_style),
        text(obj_name).size(15),
        text(size_label).size(12).color(p.text_secondary),
        container(
            button(dismiss)
                .style(move |_: &Theme, s: button::Status| -> button::Style {
                    let hover_bg = iced::Color::from_rgba(1.0, 1.0, 1.0, 0.12);
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
                        text_color: iced::Color::WHITE,
                        shadow: iced::Shadow::default(),
                        ..Default::default()
                    }
                })
                .on_press(Message::ClosePreview)
        )
        .width(Length::Fill)
        .align_x(Alignment::End),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let body = preview_body(app, content);

    let panel = column![header, rule::horizontal(1), body]
        .spacing(10)
        .padding(16);

    let content = container(panel)
        .width(Length::Fixed(860.0))
        .height(Length::Fixed(620.0))
        .style(move |_: &Theme| container::Style {
            background: Some(iced::Background::Color(p.surface_raised)),
            border: Border::default().rounded(8),
            ..Default::default()
        });

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

/// 渲染预览加载中遮罩
pub fn view_loading<'a>(app: &'a App) -> Element<'a, Message> {
    let p = constants::custom_palette(&app.theme);
    let overlay = container(
        text(t!("preview_loading").to_string())
            .size(14)
            .color(p.text_secondary),
    )
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

/// 将下载到的字节根据对象信息解析为 `PreviewContent`
pub fn build_preview(key: &str, size: i64, bytes: Vec<u8>) -> PreviewContent {
    match classify(key, size) {
        PreviewKind::Image => PreviewContent::Image(bytes),
        PreviewKind::Svg => PreviewContent::Svg(bytes),
        PreviewKind::Code => PreviewContent::Code {
            token: lang_token(key),
            content: String::from_utf8_lossy(&bytes).to_string(),
        },
        PreviewKind::Text => PreviewContent::Text(String::from_utf8_lossy(&bytes).to_string()),
        PreviewKind::TooLarge => PreviewContent::TooLarge,
        PreviewKind::Unsupported => PreviewContent::Unsupported,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_detects_text_code_image_svg() {
        assert_eq!(classify("a.txt", 10), PreviewKind::Text);
        assert_eq!(classify("a.log", 10), PreviewKind::Text);
        assert_eq!(classify("a.rs", 10), PreviewKind::Code);
        assert_eq!(classify("a.json", 10), PreviewKind::Code);
        assert_eq!(classify("a.png", 10), PreviewKind::Image);
        assert_eq!(classify("a.JPG", 10), PreviewKind::Image); // 大小写不敏感
        assert_eq!(classify("a.svg", 10), PreviewKind::Svg);
    }

    #[test]
    fn classify_unsupported_types_never_preview() {
        // 压缩包与音视频无论大小都不支持预览
        assert_eq!(classify("a.zip", 10), PreviewKind::Unsupported);
        assert_eq!(classify("a.tar.gz", 10), PreviewKind::Unsupported);
        assert_eq!(classify("a.mp4", 10), PreviewKind::Unsupported);
        // 二进制文档 / Office / 可执行 / 字体同样不支持预览
        assert_eq!(classify("a.pdf", 10), PreviewKind::Unsupported);
        assert_eq!(classify("report.PDF", 10), PreviewKind::Unsupported); // 大小写不敏感
        assert_eq!(classify("a.docx", 10), PreviewKind::Unsupported);
        assert_eq!(classify("a.xlsx", 10), PreviewKind::Unsupported);
        assert_eq!(classify("a.exe", 10), PreviewKind::Unsupported);
        assert_eq!(classify("a.ttf", 10), PreviewKind::Unsupported);
        // 即便超阈值也仍是 Unsupported，而非 TooLarge
        assert_eq!(
            classify("a.zip", (MAX_PREVIEW_BYTES + 1) as i64),
            PreviewKind::Unsupported
        );
        assert_eq!(
            classify("a.pdf", (MAX_PREVIEW_BYTES + 1) as i64),
            PreviewKind::Unsupported
        );
    }

    #[test]
    fn classify_downgrades_previewable_over_threshold() {
        let big = (MAX_PREVIEW_BYTES + 1) as i64;
        assert_eq!(classify("a.txt", big), PreviewKind::TooLarge);
        assert_eq!(classify("a.rs", big), PreviewKind::TooLarge);
        assert_eq!(classify("a.png", big), PreviewKind::TooLarge);
        assert_eq!(classify("a.svg", big), PreviewKind::TooLarge);
        // 恰好等于阈值不降级
        assert_eq!(
            classify("a.txt", MAX_PREVIEW_BYTES as i64),
            PreviewKind::Text
        );
    }

    #[test]
    fn classify_unknown_extension_defaults_to_text() {
        assert_eq!(classify("a.unknownext", 10), PreviewKind::Text);
        assert_eq!(classify("noext", 10), PreviewKind::Text);
    }

    #[test]
    fn build_preview_maps_kind_to_content() {
        assert!(matches!(
            build_preview("a.txt", 3, b"abc".to_vec()),
            PreviewContent::Text(_)
        ));
        assert!(matches!(
            build_preview("a.rs", 3, b"abc".to_vec()),
            PreviewContent::Code { .. }
        ));
        assert!(matches!(
            build_preview("a.png", 3, vec![0u8; 3]),
            PreviewContent::Image(_)
        ));
        assert!(matches!(
            build_preview("a.svg", 3, vec![0u8; 3]),
            PreviewContent::Svg(_)
        ));
        assert!(matches!(
            build_preview("a.zip", 3, vec![0u8; 3]),
            PreviewContent::Unsupported
        ));
        assert!(matches!(
            build_preview("a.txt", (MAX_PREVIEW_BYTES + 1) as i64, vec![]),
            PreviewContent::TooLarge
        ));
    }

    #[test]
    fn lang_token_maps_common_extensions() {
        assert_eq!(lang_token("main.rs"), "rust");
        assert_eq!(lang_token("app.py"), "python");
        assert_eq!(lang_token("data.json"), "json");
        assert_eq!(lang_token("unknown.zzz"), "plaintext");
    }
}
