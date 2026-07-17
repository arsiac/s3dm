//! 应用更新逻辑（状态机核心）
//!
//! 本模块包含 `update()` 函数，是 Elm 架构中的 Update 层。
//! 处理所有 `Message` 变体，更新 `App` 状态并返回副作用 `Task`。

use iced::Task;
use std::path::{Path, PathBuf};

use crate::app::App;
use crate::connection::ConnectionForm;
use crate::constants::AVAILABLE_THEMES;
use crate::message::Message;

/// 清理文件名，移除/替换对文件系统不安全的字符。
///
/// 替换路径分隔符与 Windows 保留字符，避免下载时写出非法路径。
fn sanitize_filename(name: &str) -> String {
    let reserved = ['/', '\\', '\0', ':', '*', '?', '"', '<', '>', '|'];
    let mut out: String = name
        .chars()
        .map(|c| if reserved.contains(&c) { '_' } else { c })
        .collect();
    if out.trim().is_empty() || out == "." || out == ".." {
        out = "_".to_string();
    }
    out
}

/// 若目标路径已存在，则追加 `_N` 后缀生成不冲突的唯一路径，避免静默覆盖。
fn unique_save_path(base: &Path) -> PathBuf {
    if !base.exists() {
        return base.to_path_buf();
    }
    let parent = base.parent();
    let stem = base
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let ext = base
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()));
    let mut n = 1;
    loop {
        let candidate = match parent {
            Some(p) if !p.as_os_str().is_empty() => {
                p.join(format!("{}_{}{}", stem, n, ext.as_deref().unwrap_or("")))
            }
            _ => PathBuf::from(format!("{}_{}{}", stem, n, ext.as_deref().unwrap_or(""))),
        };
        if !candidate.exists() {
            return candidate;
        }
        n += 1;
    }
}

/// 应用状态更新入口
///
/// 根据收到的 `Message` 更新 `App` 状态，并返回需要执行的异步任务。
/// 这是整个应用的状态机核心，所有用户交互和异步回调均汇聚于此。
pub fn update(app: &mut App, message: Message) -> Task<Message> {
    match message {
        // ── 连接展开/折叠 ──
        Message::ToggleConnectionExpand(id) => {
            if app.expanded_connection.as_ref() == Some(&id) {
                app.expanded_connection = None;
            }
            Task::none()
        }

        // ── 选择连接 → 发起 S3 连接 ──
        Message::ConnectionSelected(conn_id) => {
            if app.selected_connection_id.as_ref() == Some(&conn_id) {
                app.expanded_connection = Some(conn_id.clone());
                return Task::none();
            }
            app.expanded_connection = Some(conn_id.clone());
            connect_to(app, conn_id)
        }

        // ── 返回当前连接的存储桶列表 ──
        Message::BackToBuckets => {
            log::info!("Back to bucket list");
            app.current_bucket = None;
            app.current_prefix.clear();
            app.objects.clear();
            app.common_prefixes.clear();
            Task::none()
        }

        // ── 刷新当前连接的存储桶列表 ──
        Message::RefreshBuckets => {
            if let Some(conn_id) = app.selected_connection_id.clone() {
                connect_to(app, conn_id)
            } else {
                Task::none()
            }
        }

        // ── 连接完成回调 ──
        Message::Connected {
            connection_id,
            manager,
            buckets,
        } => {
            app.is_loading = false;
            app.connecting_name = None;
            match buckets {
                Ok(list) => {
                    log::info!("Connected successfully, got {} buckets", list.len());
                    app.s3_manager = Some(manager);
                    app.buckets = list;
                    app.selected_connection_id = Some(connection_id);
                }
                Err(e) => {
                    log::error!("Connection failed: {}", e);
                    app.expanded_connection = None;
                    app.error_message =
                        Some(rust_i18n::t!("connection_failed", error = e.to_string()).to_string());
                }
            }
            Task::none()
        }

        // ── 新建连接表单 ──
        Message::ConnectionAdd => {
            log::info!("Opening add connection form");
            app.connection_form = Some(ConnectionForm {
                id: None,
                name: String::new(),
                endpoint: String::new(),
                region: String::new(),
                access_key_id: String::new(),
                secret_access_key: String::new(),
                force_path_style: true,
            });
            app.connection_testing = false;
            app.connection_test_result = None;
            Task::none()
        }

        // ── 编辑连接表单 ──
        Message::ConnectionEdit(id) => {
            log::info!("Editing connection: id={}", id);
            if let Some(config) = app.config_store.get(&id) {
                app.connection_form = Some(ConnectionForm::from_config(config));
            } else {
                log::error!("Edit failed: connection id={} not found", id);
            }
            app.connection_testing = false;
            app.connection_test_result = None;
            Task::none()
        }

        // ── 提示删除连接确认 ──
        Message::ConnectionDelete(id) => {
            log::info!("Prompting delete confirmation: id={}", id);
            app.pending_delete = Some(id);
            Task::none()
        }

        // ── 确认删除连接 ──
        Message::ConfirmDelete(id) => {
            log::info!("Confirming delete: id={}", id);
            if let Err(e) = app.config_store.delete(&id) {
                log::error!("Delete connection failed: {}", e);
                app.error_message = Some(
                    rust_i18n::t!("delete_connection_failed", error = e.to_string()).to_string(),
                );
            }
            app.pending_delete = None;
            if app.selected_connection_id.as_ref() == Some(&id) {
                app.selected_connection_id = None;
                app.expanded_connection = None;
                app.s3_manager = None;
                app.buckets.clear();
                app.current_bucket = None;
                app.current_prefix.clear();
                app.objects.clear();
                app.common_prefixes.clear();
            }
            Task::none()
        }

        // ── 取消删除连接 ──
        Message::CancelDelete => {
            app.pending_delete = None;
            Task::none()
        }

        // ── 连接表单字段变更 ──
        Message::ConnectionFormChanged { field, value } => {
            if let Some(form) = &mut app.connection_form {
                match field.as_str() {
                    "name" => form.name = value,
                    "endpoint" => form.endpoint = value,
                    "region" => form.region = value,
                    "access_key_id" => form.access_key_id = value,
                    "secret_access_key" => form.secret_access_key = value,
                    "force_path_style" => form.force_path_style = value == "true",
                    _ => {}
                }
            }
            Task::none()
        }

        // ── 保存连接表单 ──
        Message::ConnectionFormSave => {
            if let Some(form) = app.connection_form.take() {
                let config = form.to_config();
                let result = if form.id.is_some() {
                    app.config_store.update(config)
                } else {
                    app.config_store.add(config)
                };
                if let Err(e) = result {
                    log::error!("Save connection failed: {}", e);
                    app.error_message = Some(
                        rust_i18n::t!("save_connection_failed", error = e.to_string()).to_string(),
                    );
                }
            }
            app.connection_testing = false;
            app.connection_test_result = None;
            Task::none()
        }

        // ── 测试连接配置 ──
        Message::ConnectionFormTest => {
            let form = match &app.connection_form {
                Some(f) => f.clone(),
                None => return Task::none(),
            };
            if form.endpoint.is_empty() {
                app.connection_test_result = Some(Err(s3dm_core::CoreError::Connection(
                    rust_i18n::t!("test_connection_empty_endpoint").to_string(),
                )));
                return Task::none();
            }
            log::info!("Testing connection form: endpoint={}", form.endpoint);
            app.connection_testing = true;
            app.connection_test_result = None;
            Task::perform(
                async move {
                    s3dm_core::S3Manager::test_connection(
                        &form.endpoint,
                        &form.region,
                        &form.access_key_id,
                        &form.secret_access_key,
                        form.force_path_style,
                    )
                    .await
                },
                Message::ConnectionTestResult,
            )
        }

        // ── 连接测试结果回调 ──
        Message::ConnectionTestResult(result) => {
            app.connection_testing = false;
            app.connection_test_result = Some(result);
            Task::none()
        }

        // ── 取消连接表单 ──
        Message::ConnectionFormCancel => {
            log::debug!("Cancelling connection edit");
            app.connection_form = None;
            Task::none()
        }

        // ── 选中桶 → 加载对象列表 ──
        Message::BucketSelected(bucket) => {
            log::info!("Bucket selected: {}", bucket);
            app.current_bucket = Some(bucket);
            app.current_prefix = String::new();
            app.objects.clear();
            app.common_prefixes.clear();
            app.load_objects()
        }

        // ── 进入文件夹 ──
        Message::PrefixSelected(prefix) => {
            log::info!("Entering folder: {}", prefix);
            app.current_prefix = prefix;
            app.load_objects()
        }

        // ── 返回上一级 ──
        Message::NavigateUp => {
            if app.current_prefix.is_empty() {
                log::info!("Navigating back to bucket list");
                app.current_bucket = None;
                Task::none()
            } else {
                let trimmed = app.current_prefix.trim_end_matches('/');
                let mut parts: Vec<&str> = trimmed.split('/').collect();
                parts.pop();
                app.current_prefix = if parts.is_empty() {
                    String::new()
                } else {
                    format!("{}/", parts.join("/"))
                };
                log::info!("Navigating up to: {}", app.current_prefix);
                app.load_objects()
            }
        }

        // ── 刷新对象列表 ──
        Message::RefreshObjects => {
            log::info!("Refreshing object list");
            app.load_objects()
        }

        // ── 分页加载更多对象 ──
        Message::LoadMoreObjects => {
            log::info!("Loading more objects");
            let bucket = match &app.current_bucket {
                Some(b) => b.clone(),
                None => return Task::none(),
            };
            let prefix = app.current_prefix.clone();
            let token = app.continuation_token.clone();
            let s3 = match &app.s3_manager {
                Some(s) => s.clone(),
                None => return Task::none(),
            };
            app.is_loading = true;
            Task::perform(
                async move {
                    s3.list_objects(&bucket, &prefix, "/", 200, token.as_deref())
                        .await
                },
                Message::ObjectsResult,
            )
        }

        // ── 对象列表加载结果 ──
        Message::ObjectsResult(result) => {
            app.is_loading = false;
            match result {
                Ok(list) => {
                    if app.continuation_token.is_some() {
                        let prev = app.objects.len();
                        app.objects.extend(list.objects);
                        app.common_prefixes = list.common_prefixes;
                        log::info!(
                            "Loaded {} more objects, total {}",
                            app.objects.len() - prev,
                            app.objects.len()
                        );
                    } else {
                        app.objects = list.objects;
                        app.common_prefixes = list.common_prefixes;
                        log::info!(
                            "Objects loaded: {} files, {} folders",
                            app.objects.len(),
                            app.common_prefixes.len()
                        );
                    }
                    app.is_truncated = list.is_truncated;
                    app.continuation_token = list.continuation_token;
                }
                Err(e) => {
                    log::error!("Failed to load objects: {}", e);
                    app.error_message = Some(
                        rust_i18n::t!("load_objects_failed", error = e.to_string()).to_string(),
                    );
                }
            }
            Task::none()
        }

        // ── 提示删除对象确认 ──
        Message::DeleteObject(key) => {
            log::info!("Prompting delete object confirmation: {}", key);
            app.pending_delete_object = Some(key);
            Task::none()
        }

        // ── 确认删除对象 ──
        Message::ConfirmDeleteObject(key) => {
            log::info!("Confirming delete object: {}", key);
            let bucket = match &app.current_bucket {
                Some(b) => b.clone(),
                None => return Task::none(),
            };
            let s3 = match &app.s3_manager {
                Some(s) => s.clone(),
                None => return Task::none(),
            };
            app.pending_delete_object = None;
            Task::perform(
                async move { s3.delete_object(&bucket, &key).await },
                Message::DeleteResult,
            )
        }

        // ── 取消删除对象 ──
        Message::CancelDeleteObject => {
            app.pending_delete_object = None;
            Task::none()
        }

        // ── 切换新建文件夹输入框 ──
        Message::ToggleNewFolder => {
            if app.new_folder_input.is_some() {
                app.new_folder_input = None;
            } else {
                app.new_folder_input = Some(String::new());
            }
            Task::none()
        }

        // ── 新建文件夹名称输入 ──
        Message::NewFolderInputChanged(val) => {
            if let Some(ref mut v) = app.new_folder_input {
                *v = val;
            }
            Task::none()
        }

        // ── 确认创建文件夹 ──
        Message::CreateNewFolder => {
            let name = match &app.new_folder_input {
                Some(n) if !n.is_empty() => n.clone(),
                _ => return Task::none(),
            };
            app.new_folder_input = None;
            let bucket = match &app.current_bucket {
                Some(b) => b.clone(),
                None => return Task::none(),
            };
            let prefix = app.current_prefix.clone();
            let key = format!("{}{}/", prefix, name);
            let s3 = match &app.s3_manager {
                Some(s) => s.clone(),
                None => return Task::none(),
            };
            log::info!("Creating folder: {}", key);
            app.is_loading = true;
            Task::perform(
                async move { s3.create_folder(&bucket, &key).await },
                Message::UploadResult,
            )
        }

        // ── 提示删除前缀确认 ──
        Message::DeletePrefix(prefix) => {
            log::info!("Prompting delete prefix confirmation: {}", prefix);
            app.pending_delete_prefix = Some(prefix);
            Task::none()
        }

        // ── 确认删除前缀 ──
        Message::ConfirmDeletePrefix(prefix) => {
            log::info!("Confirming delete prefix: {}", prefix);
            let bucket = match &app.current_bucket {
                Some(b) => b.clone(),
                None => return Task::none(),
            };
            let s3 = match &app.s3_manager {
                Some(s) => s.clone(),
                None => return Task::none(),
            };
            app.pending_delete_prefix = None;
            app.is_loading = true;
            Task::perform(
                async move { s3.delete_prefix(&bucket, &prefix).await },
                Message::DeleteResult,
            )
        }

        // ── 取消删除前缀 ──
        Message::CancelDeletePrefix => {
            app.pending_delete_prefix = None;
            Task::none()
        }

        // ── 删除操作结果 ──
        Message::DeleteResult(result) => match result {
            Ok(()) => {
                log::info!("Object deleted successfully");
                app.load_objects()
            }
            Err(e) => {
                log::error!("Failed to delete object: {}", e);
                app.error_message =
                    Some(rust_i18n::t!("delete_failed", error = e.to_string()).to_string());
                Task::none()
            }
        },

        // ── 打开文件选择器上传 ──
        Message::UploadObject => {
            log::info!("Opening file picker for upload");
            Task::perform(
                async {
                    rfd::AsyncFileDialog::new()
                        .pick_file()
                        .await
                        .map(|f| f.path().to_path_buf())
                },
                Message::FileChosen,
            )
        }

        // ── 文件选择完成 → 读取并上传 ──
        Message::FileChosen(Some(path)) => {
            let bucket = match &app.current_bucket {
                Some(b) => b.clone(),
                None => return Task::none(),
            };
            let prefix = app.current_prefix.clone();
            let s3 = match &app.s3_manager {
                Some(s) => s.clone(),
                None => return Task::none(),
            };
            let key = format!(
                "{}{}",
                prefix,
                sanitize_filename(
                    path.file_name()
                        .map(|n| n.to_string_lossy())
                        .unwrap_or_default()
                        .as_ref()
                )
            );
            let src_path = path.clone();
            log::info!("Uploading file: {:?} -> {}", path, key);
            app.is_loading = true;
            Task::perform(
                async move { s3.put_object_from_file(&bucket, &key, &src_path).await },
                Message::UploadResult,
            )
        }
        Message::FileChosen(None) => Task::none(),

        // ── 上传结果 ──
        Message::UploadResult(result) => {
            app.is_loading = false;
            match result {
                Ok(()) => {
                    log::info!("Upload succeeded");
                    return app.load_objects();
                }
                Err(e) => {
                    log::error!("Upload failed: {}", e);
                    app.error_message =
                        Some(rust_i18n::t!("upload_failed", error = e.to_string()).to_string());
                }
            }
            Task::none()
        }

        // ── 下载对象 ──
        Message::DownloadObject(key) => {
            let bucket = match &app.current_bucket {
                Some(b) => b.clone(),
                None => return Task::none(),
            };
            let s3 = match &app.s3_manager {
                Some(s) => s.clone(),
                None => return Task::none(),
            };
            let dir = app.download_dir.clone();
            let fname = sanitize_filename(key.rsplit_once('/').map(|(_, n)| n).unwrap_or(&key));
            let base = format!("{}/{}", dir.trim_end_matches('/'), fname);
            let save_path = unique_save_path(std::path::Path::new(&base))
                .to_string_lossy()
                .to_string();
            if base != save_path {
                log::warn!(
                    "Download target exists, renamed to avoid overwrite: {} -> {}",
                    base,
                    save_path
                );
            }
            log::info!("Downloading object: {} -> {}", key, save_path);
            app.is_loading = true;
            let key_c = key.clone();
            Task::perform(
                async move {
                    let data = s3
                        .get_object_to_file(&bucket, &key, std::path::Path::new(&save_path))
                        .await;
                    (key_c, save_path, data)
                },
                |(key, save_path, data)| Message::DownloadResult {
                    key,
                    save_path,
                    data,
                },
            )
        }

        // ── 下载结果 ──
        Message::DownloadResult {
            key: _,
            save_path,
            data,
        } => {
            app.is_loading = false;
            match data {
                Ok(bytes) => {
                    log::info!("Download saved to: {} ({} bytes)", save_path, bytes);
                    Task::none()
                }
                Err(e) => {
                    log::error!("Failed to download object: {}", e);
                    app.error_message =
                        Some(rust_i18n::t!("download_failed", error = e.to_string()).to_string());
                    Task::none()
                }
            }
        }

        // ── 下载目录变更 ──
        Message::DownloadDirChanged(path) => {
            app.download_dir = path;
            Task::none()
        }

        // ── 清除错误 ──
        Message::ClearError => {
            app.error_message = None;
            Task::none()
        }

        // ── 切换设置面板 ──
        Message::ToggleSettings => {
            app.show_settings = !app.show_settings;
            Task::none()
        }

        // ── 主题切换 ──
        Message::ThemeChanged(name) => {
            if let Some((_, theme)) = AVAILABLE_THEMES.iter().find(|(n, _)| *n == name) {
                app.theme = theme.clone();
                app.current_theme_name = name;
            }
            Task::none()
        }

        // ── 语言切换 ──
        Message::LanguageChanged(code) => {
            rust_i18n::set_locale(&code);
            Task::none()
        }
    }
}

/// 发起指定连接的 S3 连接并拉取桶列表
///
/// 设置 `is_loading` 后异步创建 `S3Manager` 并调用 `list_buckets`，
/// 结果通过 `Message::Connected` 回调回写状态。
fn connect_to(app: &mut App, conn_id: String) -> Task<Message> {
    log::info!("Connection selected: id={}", conn_id);
    app.is_loading = true;
    app.current_bucket = None;
    app.current_prefix.clear();
    app.objects.clear();
    app.common_prefixes.clear();
    app.continuation_token = None;
    if let Some(config) = app.config_store.get(&conn_id).cloned() {
        app.connecting_name = Some(config.name.clone());
        let endpoint = config.endpoint;
        let region = config.region;
        let ak = config.access_key_id;
        let sk = config.secret_access_key;
        let fps = config.force_path_style;
        Task::perform(
            async move {
                log::info!("Connecting to S3 endpoint={} region={}", endpoint, region);
                let manager = s3dm_core::S3Manager::new(&endpoint, &region, &ak, &sk, fps);
                let buckets = manager.list_buckets().await;
                (manager, buckets)
            },
            |(manager, buckets)| Message::Connected {
                connection_id: conn_id,
                manager,
                buckets,
            },
        )
    } else {
        log::error!("Connection config not found: id={}", conn_id);
        app.is_loading = false;
        app.connecting_name = None;
        Task::none()
    }
}
