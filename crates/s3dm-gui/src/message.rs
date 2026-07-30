//! 应用消息枚举定义
//!
//! 遵循 Elm 架构的消息模式，定义所有用户交互事件和异步操作结果。
//! 每个变体对应一个用户动作或系统回调，由 `update()` 统一处理。

use std::path::PathBuf;

use crate::preview::PreviewContent;
use iced::widget::text_editor;
use s3dm_core::{CoreError, ObjectListResult, S3Bucket, S3Manager};

/// 应用消息枚举，涵盖所有用户交互与异步回调
#[derive(Debug, Clone)]
pub enum Message {
    // ── 连接管理 ──
    /// 切换连接列表的展开/折叠状态
    ToggleConnectionExpand(String),
    /// 选中某个连接，发起 S3 连接
    ConnectionSelected(String),
    /// 打开添加连接表单
    ConnectionAdd,
    /// 打开编辑连接表单
    ConnectionEdit(String),
    /// 提示删除连接确认
    ConnectionDelete(String),
    /// 连接表单字段变更
    ConnectionFormChanged { field: String, value: String },
    /// 保存连接表单
    ConnectionFormSave,
    /// 取消连接表单编辑
    ConnectionFormCancel,
    /// 测试当前连接表单配置
    ConnectionFormTest,
    /// 连接表单测试结果回调
    ConnectionTestResult(Result<(), CoreError>),

    // ── S3 连接结果 ──
    /// 连接完成回调，携带 S3Manager 和桶列表
    Connected {
        connection_id: String,
        manager: S3Manager,
        buckets: Result<Vec<S3Bucket>, CoreError>,
    },

    // ── 桶/路径导航 ──
    /// 返回当前连接的存储桶列表
    BackToBuckets,
    /// 刷新当前连接的存储桶列表
    RefreshBuckets,
    /// 选中某个桶
    BucketSelected(String),
    /// 进入某个文件夹前缀
    PrefixSelected(String),
    /// 返回上一级目录
    NavigateUp,

    // ── 对象操作 ──
    /// 刷新当前目录的对象列表
    RefreshObjects,
    /// 加载更多对象（分页）
    LoadMoreObjects,
    /// 预览单个对象（文本/代码/图片）
    PreviewObject(String),
    /// 提示删除单个对象确认
    DeleteObject(String),
    /// 切换某个对象的"更多"菜单开合（None 关闭所有菜单）
    ToggleObjectMenu(Option<String>),
    /// 切换某个文件夹的"更多"菜单开合（None 关闭所有菜单）
    TogglePrefixMenu(Option<String>),
    /// 提示删除整个前缀确认
    DeletePrefix(String),
    /// 打开上传文件选择器
    UploadObject,
    /// 下载单个对象
    DownloadObject(String),
    // ── 异步操作结果 ──
    /// 对象列表加载结果
    ObjectsResult(Result<ObjectListResult, CoreError>),
    /// 预览内容加载结果
    PreviewResult {
        key: String,
        data: Result<PreviewContent, CoreError>,
    },
    /// 关闭预览弹窗
    ClosePreview,
    /// 预览只读编辑器动作（选中/复制等交互，编辑类动作被忽略以保持只读）
    PreviewEditorAction(text_editor::Action),
    /// 删除操作结果
    DeleteResult(Result<(), CoreError>),
    /// 下载结果，包含保存路径与写入字节数
    DownloadResult {
        key: String,
        save_path: String,
        data: Result<u64, CoreError>,
    },
    /// 下载进度更新（已下载字节数，总大小 None 表示未知）
    DownloadProgress { downloaded: u64, total: Option<u64> },
    /// 上传操作结果
    UploadResult(Result<(), CoreError>),

    // ── 文件对话框 ──
    /// 文件选择器返回结果
    FileChosen(Option<PathBuf>),

    // ── 设置 ──
    /// 下载目录路径变更
    DownloadDirChanged(String),
    /// 清除错误提示
    ClearError,
    /// 清除下载成功提示
    ClearSuccessMessage,
    /// 切换设置面板显示
    ToggleSettings,
    /// 主题切换
    ThemeChanged(String),
    /// 语言切换
    LanguageChanged(String),

    // ── 删除确认对话框 ──
    /// 确认删除连接
    ConfirmDelete(String),
    /// 取消删除连接
    CancelDelete,
    /// 确认删除对象
    ConfirmDeleteObject(String),
    /// 取消删除对象
    CancelDeleteObject,
    /// 确认删除前缀
    ConfirmDeletePrefix(String),
    /// 取消删除前缀
    CancelDeletePrefix,

    // ── 新建文件夹 ──
    /// 切换新建文件夹输入框显示
    ToggleNewFolder,
    /// 打开重命名对话框
    RenameObject(String),
    /// 重命名输入框内容变更
    RenameInputChanged(String),
    /// 确认执行重命名
    ConfirmRename,
    /// 取消重命名
    CancelRename,
    /// 打开复制对话框
    CopyObject(String),
    /// 打开移动对话框
    MoveObject(String),
    /// 复制/移动对话框字段变更
    CopyMoveInputChanged { field: String, value: String },
    /// 进入目标路径下的某个子文件夹
    NavigateIntoTargetFolder(String),
    /// 返回目标路径的上一级
    NavigateUpTargetFolder,
    /// 手动输入目标前缀并立即加载子文件夹
    TargetPrefixInputChanged(String),
    /// 目标前缀的子文件夹列表加载结果
    TargetPrefixesResult(Result<Vec<String>, s3dm_core::CoreError>),
    /// 确认执行复制/移动
    ConfirmCopyMove,
    /// 取消复制/移动
    CancelCopyMove,
    /// 复制/移动操作结果
    CopyMoveResult(Result<(), s3dm_core::CoreError>),
    /// 新建文件夹输入框内容变更
    NewFolderInputChanged(String),
    /// 确认创建文件夹
    CreateNewFolder,

    // ── 更新检查 ──
    /// 手动或自动触发更新检查
    CheckForUpdates,
    /// 更新检查结果回调（Ok(Some) 表示有新版本，Ok(None) 表示已是最新，Err 为错误描述）
    /// 第二个布尔值指示是否来自启动时的自动检查，用于决定是否静默处理错误
    UpdateCheckResult(
        Result<Option<s3dm_core::update_check::ReleaseInfo>, String>,
        bool,
    ),
    /// 关闭顶部更新提示栏（保留结果，仅本次会话不再自动弹出）
    DismissUpdateNotice,
    /// 在浏览器中打开发布页下载页
    OpenReleasePage,
    /// 切换「启动时自动检查更新」设置项
    ToggleAutoCheckUpdate(bool),
}
