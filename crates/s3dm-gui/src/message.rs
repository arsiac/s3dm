//! 应用消息枚举定义
//!
//! 遵循 Elm 架构的消息模式，定义所有用户交互事件和异步操作结果。
//! 每个变体对应一个用户动作或系统回调，由 `update()` 统一处理。

use std::path::PathBuf;

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
    /// 提示删除单个对象确认
    DeleteObject(String),
    /// 提示删除整个前缀确认
    DeletePrefix(String),
    /// 打开上传文件选择器
    UploadObject,
    /// 下载单个对象
    DownloadObject(String),
    // ── 异步操作结果 ──
    /// 对象列表加载结果
    ObjectsResult(Result<ObjectListResult, CoreError>),
    /// 删除操作结果
    DeleteResult(Result<(), CoreError>),
    /// 下载结果，包含保存路径与写入字节数
    DownloadResult {
        key: String,
        save_path: String,
        data: Result<u64, CoreError>,
    },
    /// 下载进度更新（已下载字节数，总大小 None 表示未知）
    DownloadProgress {
        downloaded: u64,
        total: Option<u64>,
    },
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
    /// 新建文件夹输入框内容变更
    NewFolderInputChanged(String),
    /// 确认创建文件夹
    CreateNewFolder,
}
