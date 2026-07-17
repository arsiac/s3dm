//! S3DM GUI — S3 兼容存储 GUI 客户端
//!
//! 基于 Iced GUI 框架（Elm 架构），提供连接管理、桶浏览、
//! 对象 CRUD、主题/语言切换等功能。
//!
//! # 模块结构
//!
//! | 模块 | 职责 |
//! |------|------|
//! | `constants` | 常量定义、自定义调色板、`format_size` 辅助函数 |
//! | `connection` | `ConnectionForm` 连接表单模型 |
//! | `message` | `Message` 枚举（所有用户交互/异步回调事件） |
//! | `app` | `App` 结构体（应用状态）、`boot()` 初始化 |
//! | `update` | `update()` 状态机核心 |
//! | `view` | `view()` 主视图编排、`view_right_content()` |
//! | `view_left_panel` | 左侧面板（连接列表） |
//! | `view_buckets` | 右侧存储桶列表视图 |
//! | `view_objects` | 对象浏览器（文件/文件夹列表） |
//! | `view_form` | 连接编辑表单 |
//! | `view_dialogs` | 模态对话框（删除确认、新建文件夹） |
//! | `view_settings` | 设置面板 + 状态栏 |
//! | `preview` | 对象预览（文本 / 代码语法高亮 + 行号 / 图片） |

// ── 国际化初始化 ──
rust_i18n::i18n!("locales");

// ── 模块声明 ──
pub mod app;
pub mod connection;
pub mod constants;
pub mod icon;
pub mod message;
pub mod preview;
pub mod update;
pub mod view;
pub mod view_buckets;
pub mod view_dialogs;
pub mod view_form;
pub mod view_left_panel;
pub mod view_objects;
pub mod view_settings;

// ── 重新导出应用入口函数与核心类型 ──
pub use app::App;
pub use app::boot;
pub use update::update;
pub use view::view;
