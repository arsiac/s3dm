//! 应用核心模型与初始化
//!
//! 本模块定义 `App` 结构体（应用全部状态），以及：
//! - `boot()`：应用入口初始化函数
//! - `App::load_objects()`：加载 S3 对象列表的辅助方法

use iced::widget::combo_box;
use iced::{Task, Theme};
use s3dm_config::ConfigStore;
use s3dm_core::{CoreError, S3Bucket, S3Manager, S3Object};

use crate::connection::ConnectionForm;
use crate::constants;
use crate::message::Message;
use crate::preview::PreviewContent;

/// 复制/移动操作模式
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CopyMoveMode {
    Copy,
    Move,
}

/// 复制/移动对话框状态
#[derive(Debug, Clone)]
pub struct CopyMoveState {
    pub mode: CopyMoveMode,
    pub source_key: String,
    /// 对话框内错误提示（而不是主窗口错误条）
    pub error: Option<String>,
    /// 当前目标路径前缀
    pub target_prefix: String,
    /// 当前目标前缀下的子文件夹列表
    pub available_prefixes: Vec<String>,
    /// 是否正在加载子文件夹列表
    pub is_loading_prefixes: bool,
    pub new_name: String,
}

/// 设置面板的分类页
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsCategory {
    /// 通用（语言、下载目录）
    #[default]
    General,
    /// 外观（主题、界面字体）
    Appearance,
    /// 预览（预览编辑器字体）
    Preview,
    /// 更新检查
    Updates,
    /// 关于
    About,
}

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
    /// 连接表单参数校验/错误提示（在表单内展示，None 表示无错误）
    pub connection_form_error: Option<String>,
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
    /// 当前正在下载的文件名（None 表示未在下载，用于状态栏提示）
    pub downloading_file: Option<String>,
    /// 当前正在下载的对象 Key（用于在列表中标记对应行的下载按钮）
    pub downloading_key: Option<String>,
    /// 当前展开"更多"菜单的对象 Key（None 表示无菜单展开，互斥）
    pub open_menu_key: Option<String>,
    /// 当前展开"更多"菜单的文件夹前缀（None 表示无菜单展开，互斥）
    pub open_prefix_menu: Option<String>,
    /// 当前下载进度（已下载字节数，总大小 None 表示未知），None 表示无进行中的下载
    pub download_progress: Option<(u64, Option<u64>)>,
    /// 下载成功提示信息（None 表示无提示，用于顶部绿色通知栏）
    pub success_message: Option<String>,
    /// 待删除确认的连接 ID
    pub pending_delete: Option<String>,
    /// 待删除确认的对象 Key
    pub pending_delete_object: Option<String>,
    /// 待删除确认的前缀路径
    pub pending_delete_prefix: Option<String>,
    /// 当前预览内容（None 表示未打开预览）
    pub preview: Option<PreviewContent>,
    /// 当前正在预览的对象 Key
    pub preview_key: Option<String>,
    /// 是否正在加载预览内容
    pub preview_loading: bool,
    /// 预览文本/代码只读编辑器内容（用于支持选中与复制）
    pub preview_editor_content: Option<iced::widget::text_editor::Content<iced::Renderer>>,
    /// 当前正在查看属性的对象 Key（None 表示未打开属性对话框）
    pub show_properties: Option<String>,

    /// 新建文件夹输入框内容
    pub new_folder_input: Option<String>,
    /// 重命名对话框：(old_key, current_name)，None 表示关闭
    pub rename_input: Option<(String, String)>,
    /// 复制/移动对话框状态，None 表示关闭
    pub copy_move_input: Option<CopyMoveState>,
    /// 是否显示设置面板
    pub show_settings: bool,
    /// 当前应用主题
    pub theme: Theme,
    /// 当前主题名称
    pub current_theme_name: String,
    /// 可用更新信息（None 表示无更新或尚未检查），驱动顶部更新提示栏
    pub update_info: Option<s3dm_core::update_check::ReleaseInfo>,
    /// 是否正在检查更新（用于按钮/状态栏 loading 态）
    pub checking_update: bool,
    /// 是否已在本次会话忽略更新提示（关闭顶部栏后不再自动弹出）
    pub update_dismissed: bool,
    /// 设置项：启动时是否自动检查更新
    pub auto_check_update: bool,
    /// 界面字体系列名称（空 = 系统默认）
    pub ui_font_family: String,
    /// 界面基础字号（像素）
    pub ui_font_size: u16,
    /// 预览编辑器字体系列名称（空 = 系统默认等宽字体）
    pub preview_font_family: String,
    /// 预览编辑器字号（像素）
    pub preview_font_size: u16,
    /// 界面字体下拉框状态（可输入过滤，选项随语言重建）
    pub ui_font_combo: combo_box::State<String>,
    /// 预览编辑器字体下拉框状态
    pub preview_font_combo: combo_box::State<String>,
    /// 设置面板当前选中的分类页
    pub settings_category: SettingsCategory,
    /// 最近一次更新检查的结论（用于设置面板内反馈，None 表示尚未检查）
    pub update_check_status: Option<crate::update::UpdateCheckStatus>,
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
            async move { s3.list_objects(&bucket, &prefix, "/", 200, None).await },
            Message::ObjectsResult,
        )
    }
}

/// 应用初始化入口，返回 (App, Task)
///
/// 流程：
/// 1. 从持久化设置加载主题/语言/下载目录偏好
/// 2. 通过 `sys-locale` 检测系统语言（仅当设置中未显式指定时作为兜底）
/// 3. 设置 `rust-i18n` 的 locale
/// 4. 构造 `App` 默认实例
pub fn boot() -> (App, Task<Message>) {
    let settings = s3dm_config::AppSettings::load();
    let stored_lang = settings.language.clone();

    let locale = sys_locale::get_locale().unwrap_or_default();
    let lang = locale.split('-').next().unwrap_or("en");
    let system_locale = match lang {
        "zh" => {
            if locale.starts_with("zh-TW")
                || locale.starts_with("zh-HK")
                || locale.starts_with("zh-Hant")
            {
                "zh-TW".to_string()
            } else {
                "zh-CN".to_string()
            }
        }
        _ => "en".to_string(),
    };
    // 设置中保存的语言优先；否则回退到系统检测
    let effective_locale = if stored_lang.is_empty() {
        system_locale
    } else {
        stored_lang
    };
    rust_i18n::set_locale(&effective_locale);
    log::info!(
        "Initializing S3DM application (locale: {})",
        &*rust_i18n::locale()
    );

    // 根据持久化主题名解析 Iced Theme
    let (theme, current_theme_name) = constants::AVAILABLE_THEMES
        .iter()
        .find(|(name, _)| *name == settings.theme)
        .map(|(name, t)| (t.clone(), name.to_string()))
        .unwrap_or((iced::Theme::Dark, "Dark".to_string()));

    let download_dir = if settings.download_dir.is_empty() {
        dirs::download_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default()
    } else {
        settings.download_dir.clone()
    };

    let auto_check_update = settings.auto_check_update;

    let ui_font_combo = combo_box::State::with_selection(
        crate::font::font_options(rust_i18n::t!("settings_font_family_default").as_ref()),
        Some(&settings.ui_font_family),
    );
    let preview_font_combo = combo_box::State::with_selection(
        crate::font::font_options(rust_i18n::t!("settings_font_family_default_mono").as_ref()),
        Some(&settings.preview_font_family),
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
        connection_form_error: None,
        buckets: Vec::new(),
        current_bucket: None,
        current_prefix: String::new(),
        objects: Vec::new(),
        common_prefixes: Vec::new(),
        is_truncated: false,
        continuation_token: None,
        is_loading: false,
        connecting_name: None,
        download_dir,
        downloading_file: None,
        downloading_key: None,
        open_menu_key: None,
        open_prefix_menu: None,
        download_progress: None,
        success_message: None,
        pending_delete: None,
        pending_delete_object: None,
        pending_delete_prefix: None,
        preview: None,
        preview_key: None,
        preview_loading: false,
        preview_editor_content: None,
        show_properties: None,

        new_folder_input: None,
        rename_input: None,
        copy_move_input: None,
        show_settings: false,
        theme,
        current_theme_name,
        update_info: None,
        checking_update: false,
        update_dismissed: false,
        auto_check_update,
        ui_font_family: settings.ui_font_family,
        ui_font_size: settings.ui_font_size,
        preview_font_family: settings.preview_font_family,
        preview_font_size: settings.preview_font_size,
        ui_font_combo,
        preview_font_combo,
        settings_category: SettingsCategory::default(),
        update_check_status: None,
    };

    // 若开启自动检查，启动后在后台静默检查一次更新
    let task = if auto_check_update {
        Task::perform(
            async move { s3dm_core::update_check::check_update(constants::APP_VERSION).await },
            |r| Message::UpdateCheckResult(r.map_err(|e| e.to_string()), true),
        )
    } else {
        Task::none()
    };
    (app, task)
}
