//! 应用核心模型与初始化
//!
//! 本模块定义 `App` 结构体（应用全部状态），以及：
//! - `boot()`：应用入口初始化函数
//! - `App::load_objects()`：加载 S3 对象列表的辅助方法

use iced::{Task, Theme};
use s3dm_config::ConfigStore;
use s3dm_core::{CoreError, S3Bucket, S3Manager, S3Object};

use crate::connection::ConnectionForm;
use crate::message::Message;

/// 应用主状态结构体，遵循 Elm 架构的 Model 层
///
/// 包含连接管理、桶浏览、对象 CRUD、UI 状态等全部应用数据。
pub struct App {
    /// 连接配置持久化存储
    pub config_store: ConfigStore,
    /// S3 API 管理器（连接建立后为 Some）
    pub s3_manager: Option<S3Manager>,
    /// 错误提示信息
    pub error_message: Option<String>,
    /// 当前展开显示桶列表的连接 ID
    pub expanded_connection: Option<String>,
    /// 当前已选中/连接的连接 ID
    pub selected_connection_id: Option<String>,
    /// 正在编辑/新增的连接表单数据
    pub connection_form: Option<ConnectionForm>,
    /// 是否正在测试连接
    pub connection_testing: bool,
    /// 连接表单测试结果（None 表示尚未测试）
    pub connection_test_result: Option<Result<(), CoreError>>,
    /// 当前连接下的桶列表
    pub buckets: Vec<S3Bucket>,
    /// 当前选中的桶名称
    pub current_bucket: Option<String>,
    /// 当前浏览的路径前缀
    pub current_prefix: String,
    /// 当前目录下的对象列表
    pub objects: Vec<S3Object>,
    /// 当前目录下的公共前缀（子文件夹）列表
    pub common_prefixes: Vec<String>,
    /// 是否还有更多对象可以加载（分页标志）
    pub is_truncated: bool,
    /// 分页续传令牌
    pub continuation_token: Option<String>,
    /// 是否正在加载中
    pub is_loading: bool,
    /// 正在连接中的连接名称（用于状态栏提示，None 表示未在连接）
    pub connecting_name: Option<String>,
    /// 文件下载目录路径
    pub download_dir: String,
    /// 待删除确认的连接 ID
    pub pending_delete: Option<String>,
    /// 待删除确认的对象 Key
    pub pending_delete_object: Option<String>,
    /// 待删除确认的前缀路径
    pub pending_delete_prefix: Option<String>,
    /// 新建文件夹输入框内容
    pub new_folder_input: Option<String>,
    /// 是否显示设置面板
    pub show_settings: bool,
    /// 当前应用主题
    pub theme: Theme,
    /// 当前主题名称
    pub current_theme_name: String,
}

impl App {
    /// 异步加载当前桶和前缀下的对象列表
    ///
    /// 发送 `list_objects` 请求到 S3，结果通过 `Message::ObjectsResult` 返回。
    pub fn load_objects(&mut self) -> Task<Message> {
        let bucket = match &self.current_bucket {
            Some(b) => b.clone(),
            None => return Task::none(),
        };
        let prefix = self.current_prefix.clone();
        let s3 = match &self.s3_manager {
            Some(s) => s.clone(),
            None => return Task::none(),
        };
        log::debug!("Loading objects bucket={} prefix={:?}", bucket, prefix);
        self.continuation_token = None;
        self.is_loading = true;
        Task::perform(
            async move { s3.list_objects(&bucket, &prefix, "/", 200, None) },
            Message::ObjectsResult,
        )
    }
}

/// 应用初始化入口，返回 (App, Task)
///
/// 流程：
/// 1. 通过 `sys-locale` 检测系统语言
/// 2. 设置 `rust-i18n` 的 locale
/// 3. 构造 `App` 默认实例
pub fn boot() -> (App, Task<Message>) {
    let locale = sys_locale::get_locale().unwrap_or_default();
    let lang = locale.split('-').next().unwrap_or("en");
    match lang {
        "zh" => {
            if locale.starts_with("zh-TW")
                || locale.starts_with("zh-HK")
                || locale.starts_with("zh-Hant")
            {
                rust_i18n::set_locale("zh-TW");
            } else {
                rust_i18n::set_locale("zh-CN");
            }
        }
        _ => rust_i18n::set_locale("en"),
    }
    log::info!(
        "Initializing S3DM application (locale: {})",
        &*rust_i18n::locale()
    );
    let app = App {
        config_store: ConfigStore::new(),
        s3_manager: None,
        error_message: None,
        expanded_connection: None,
        selected_connection_id: None,
        connection_form: None,
        connection_testing: false,
        connection_test_result: None,
        buckets: Vec::new(),
        current_bucket: None,
        current_prefix: String::new(),
        objects: Vec::new(),
        common_prefixes: Vec::new(),
        is_truncated: false,
        continuation_token: None,
        is_loading: false,
        connecting_name: None,
        download_dir: dirs::download_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default(),
        pending_delete: None,
        pending_delete_object: None,
        pending_delete_prefix: None,
        new_folder_input: None,
        show_settings: false,
        theme: Theme::Dark,
        current_theme_name: "Dark".to_string(),
    };
    (app, Task::none())
}
