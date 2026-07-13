
use iced::{
    widget::{button, column, container, row, rule, scrollable, text, text_input, toggler},
    Element, Padding, Task, Theme,
};
use s3dm_config::ConfigStore;
use s3dm_core::{CoreError, ObjectListResult, S3Bucket, S3Manager, S3Object};

#[derive(Debug, Clone, PartialEq)]
enum Page {
    Connections,
    Buckets,
    Objects,
}

#[derive(Debug, Clone)]
struct ConnectionForm {
    id: Option<String>,
    name: String,
    endpoint: String,
    region: String,
    access_key_id: String,
    secret_access_key: String,
    force_path_style: bool,
}

impl ConnectionForm {
    fn to_config(&self) -> s3dm_config::ConnectionConfig {
        match &self.id {
            Some(id) => s3dm_config::ConnectionConfig {
                id: id.clone(),
                name: self.name.clone(),
                endpoint: self.endpoint.clone(),
                region: self.region.clone(),
                access_key_id: self.access_key_id.clone(),
                secret_access_key: self.secret_access_key.clone(),
                force_path_style: self.force_path_style,
            },
            None => s3dm_config::ConnectionConfig::new(
                self.name.clone(),
                self.endpoint.clone(),
                self.region.clone(),
                self.access_key_id.clone(),
                self.secret_access_key.clone(),
                self.force_path_style,
            ),
        }
    }

    fn from_config(config: &s3dm_config::ConnectionConfig) -> Self {
        Self {
            id: Some(config.id.clone()),
            name: config.name.clone(),
            endpoint: config.endpoint.clone(),
            region: config.region.clone(),
            access_key_id: config.access_key_id.clone(),
            secret_access_key: config.secret_access_key.clone(),
            force_path_style: config.force_path_style,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    GoToConnections,
    ConnectionSelected(String),
    ConnectionAdd,
    ConnectionEdit(String),
    ConnectionDelete(String),
    ConnectionFormChanged { field: String, value: String },
    ConnectionFormSave,
    ConnectionFormCancel,
    Connected {
        connection_id: String,
        manager: S3Manager,
        buckets: Result<Vec<S3Bucket>, CoreError>,
    },
    BucketSelected(String),
    PrefixSelected(String),
    NavigateUp,
    RefreshObjects,
    LoadMoreObjects,
    DeleteObject(String),
    UploadObject,
    DownloadObject(String),
    ObjectsResult(Result<ObjectListResult, CoreError>),
    DeleteResult(Result<(), CoreError>),
    DownloadResult {
        key: String,
        data: Result<Vec<u8>, CoreError>,
    },
    UploadResult(Result<(), CoreError>),
    UploadPathChanged(String),
    DownloadPathChanged(String),
    ClearError,
}

pub struct App {
    config_store: ConfigStore,
    current_page: Page,
    s3_manager: Option<S3Manager>,
    error_message: Option<String>,
    selected_connection_id: Option<String>,
    connection_form: Option<ConnectionForm>,
    buckets: Vec<S3Bucket>,
    current_bucket: Option<String>,
    current_prefix: String,
    objects: Vec<S3Object>,
    common_prefixes: Vec<String>,
    is_truncated: bool,
    continuation_token: Option<String>,
    is_loading: bool,
    upload_path: String,
    download_path: String,
}

impl App {
    fn load_objects(&mut self) -> Task<Message> {
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

pub fn boot() -> (App, Task<Message>) {
    log::info!("Initializing S3DM application");
    let app = App {
        config_store: ConfigStore::new(),
        current_page: Page::Connections,
        s3_manager: None,
        error_message: None,
        selected_connection_id: None,
        connection_form: None,
        buckets: Vec::new(),
        current_bucket: None,
        current_prefix: String::new(),
        objects: Vec::new(),
        common_prefixes: Vec::new(),
        is_truncated: false,
        continuation_token: None,
        is_loading: false,
        upload_path: String::new(),
        download_path: String::new(),
    };
    (app, Task::none())
}

pub fn update(app: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::GoToConnections => {
            log::info!("Navigating back to connections page");
            app.current_page = Page::Connections;
            app.s3_manager = None;
            app.buckets.clear();
            app.current_bucket = None;
            app.current_prefix.clear();
            app.objects.clear();
            app.common_prefixes.clear();
            Task::none()
        }
        Message::ConnectionSelected(conn_id) => {
            log::info!("Connection selected: id={}", conn_id);
            if let Some(config) = app.config_store.get(&conn_id).cloned() {
                app.is_loading = true;
                let endpoint = config.endpoint;
                let region = config.region;
                let ak = config.access_key_id;
                let sk = config.secret_access_key;
                let fps = config.force_path_style;
                Task::perform(
                    async move {
                        log::info!("Connecting to S3 endpoint={} region={}", endpoint, region);
                        let manager = S3Manager::new(&endpoint, &region, &ak, &sk, fps);
                        let buckets = manager.list_buckets();
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
                Task::none()
            }
        }
        Message::Connected {
            connection_id,
            manager,
            buckets,
        } => {
            app.is_loading = false;
            match buckets {
                Ok(list) => {
                    log::info!(
                        "Connected successfully, got {} buckets",
                        list.len()
                    );
                    app.s3_manager = Some(manager);
                    app.buckets = list;
                    app.selected_connection_id = Some(connection_id);
                    app.current_page = Page::Buckets;
                }
                Err(e) => {
                    log::error!("Connection failed: {}", e);
                    app.error_message = Some(format!("连接失败: {}", e));
                }
            }
            Task::none()
        }
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
            Task::none()
        }
        Message::ConnectionEdit(id) => {
            log::info!("Editing connection: id={}", id);
            if let Some(config) = app.config_store.get(&id) {
                app.connection_form = Some(ConnectionForm::from_config(config));
            } else {
                log::error!("Edit failed: connection id={} not found", id);
            }
            Task::none()
        }
        Message::ConnectionDelete(id) => {
            log::info!("Deleting connection: id={}", id);
            if let Err(e) = app.config_store.delete(&id) {
                log::error!("Delete connection failed: {}", e);
                app.error_message = Some(format!("删除连接失败: {}", e));
            }
            Task::none()
        }
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
                    app.error_message = Some(format!("保存连接失败: {}", e));
                }
            }
            Task::none()
        }
        Message::ConnectionFormCancel => {
            log::debug!("Cancelling connection edit");
            app.connection_form = None;
            Task::none()
        }
        Message::BucketSelected(bucket) => {
            log::info!("Bucket selected: {}", bucket);
            app.current_bucket = Some(bucket);
            app.current_prefix = String::new();
            app.objects.clear();
            app.common_prefixes.clear();
            app.current_page = Page::Objects;
            app.load_objects()
        }
        Message::PrefixSelected(prefix) => {
            log::info!("Entering folder: {}", prefix);
            app.current_prefix = prefix;
            app.load_objects()
        }
        Message::NavigateUp => {
            if app.current_prefix.is_empty() {
                log::info!("Navigating back to bucket list");
                app.current_page = Page::Buckets;
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
        Message::RefreshObjects => {
            log::info!("Refreshing object list");
            app.load_objects()
        }
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
                async move { s3.list_objects(&bucket, &prefix, "/", 200, token.as_deref()) },
                Message::ObjectsResult,
            )
        }
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
                    app.error_message = Some(format!("加载对象失败: {}", e));
                }
            }
            Task::none()
        }
        Message::DeleteObject(key) => {
            log::info!("Requesting delete object: {}", key);
            let bucket = match &app.current_bucket {
                Some(b) => b.clone(),
                None => return Task::none(),
            };
            let s3 = match &app.s3_manager {
                Some(s) => s.clone(),
                None => return Task::none(),
            };
            Task::perform(
                async move { s3.delete_object(&bucket, &key) },
                Message::DeleteResult,
            )
        }
        Message::DeleteResult(result) => match result {
            Ok(()) => {
                log::info!("Object deleted successfully");
                app.load_objects()
            }
            Err(e) => {
                log::error!("Failed to delete object: {}", e);
                app.error_message = Some(format!("删除失败: {}", e));
                Task::none()
            }
        },
        Message::UploadObject => {
            let bucket = match &app.current_bucket {
                Some(b) => b.clone(),
                None => return Task::none(),
            };
            let prefix = app.current_prefix.clone();
            let path = app.upload_path.clone();
            let s3 = match &app.s3_manager {
                Some(s) => s.clone(),
                None => return Task::none(),
            };
            if path.is_empty() {
                log::warn!("Upload file path is empty");
                app.error_message = Some("请选择文件路径".into());
                return Task::none();
            }
            let key = format!(
                "{}{}",
                prefix,
                path.rsplit_once('/').map(|(_, f)| f).unwrap_or(&path)
            );
            log::info!("Uploading file: {} -> {}", path, key);
            app.is_loading = true;
            Task::perform(
                async move {
                    match std::fs::read(&path) {
                        Ok(data) => s3.put_object(&bucket, &key, data),
                        Err(e) => {
                            log::error!("Failed to read local file: {}: {}", path, e);
                            Err(CoreError::S3(format!("读取文件失败: {}", e)))
                        }
                    }
                },
                Message::UploadResult,
            )
        }
        Message::UploadResult(result) => {
            app.is_loading = false;
            match result {
                Ok(()) => {
                    log::info!("Upload succeeded");
                    app.upload_path.clear();
                    return app.load_objects();
                }
                Err(e) => {
                    log::error!("Upload failed: {}", e);
                    app.error_message = Some(format!("上传失败: {}", e));
                }
            }
            Task::none()
        }
        Message::DownloadObject(key) => {
            let bucket = match &app.current_bucket {
                Some(b) => b.clone(),
                None => return Task::none(),
            };
            let path = app.download_path.clone();
            let s3 = match &app.s3_manager {
                Some(s) => s.clone(),
                None => return Task::none(),
            };
            if path.is_empty() {
                log::warn!("Download save path is empty");
                app.error_message = Some("请设置下载保存路径".into());
                return Task::none();
            }
            log::info!("Downloading object: {} -> {}", key, path);
            app.is_loading = true;
            let key_clone = key.clone();
            Task::perform(
                async move {
                    let data = s3.get_object(&bucket, &key);
                    (key_clone, data)
                },
                |(key, data)| Message::DownloadResult { key, data },
            )
        }
        Message::DownloadResult { key, data } => {
            app.is_loading = false;
            match data {
                Ok(bytes) => {
                    let path = app.download_path.clone();
                    let save_path = if path.ends_with('/') {
                        format!("{}{}", path, key.rsplit_once('/').map(|(_, f)| f).unwrap_or(&key))
                    } else {
                        path
                    };
                    match std::fs::write(&save_path, bytes) {
                        Ok(()) => log::info!("Download saved to: {}", save_path),
                        Err(e) => log::error!("Failed to save file: {}: {}", save_path, e),
                    }
                    Task::none()
                }
                Err(e) => {
                    log::error!("Failed to download object: {}", e);
                    app.error_message = Some(format!("下载失败: {}", e));
                    Task::none()
                }
            }
        }
        Message::UploadPathChanged(path) => {
            app.upload_path = path;
            Task::none()
        }
        Message::DownloadPathChanged(path) => {
            app.download_path = path;
            Task::none()
        }
        Message::ClearError => {
            app.error_message = None;
            Task::none()
        }
    }
}

pub fn view(app: &App) -> Element<'_, Message> {
    let mut elements: Vec<Element<Message>> = Vec::new();

    if let Some(err) = &app.error_message {
        let error_bar = container(
            row![
                text(format!("Error: {}", err)).color(iced::Color::WHITE),
                button("×").on_press(Message::ClearError),
            ]
            .spacing(10)
            .align_y(iced::Alignment::Center),
        )
        .padding(10)
        .style(|_theme: &Theme| container::Style {
            background: Some(iced::Background::Color(iced::Color::from_rgb(0.8, 0.2, 0.2))),
            text_color: Some(iced::Color::WHITE),
            ..Default::default()
        })
        .width(iced::Length::Fill);
        elements.push(error_bar.into());
    }

    let page_content = match app.current_page {
        Page::Connections => view_connections(app),
        Page::Buckets => view_buckets(app),
        Page::Objects => view_objects(app),
    };
    elements.push(page_content);

    if app.is_loading {
        let loading = container(text("Loading...").size(24))
            .width(iced::Length::Fill)
            .height(iced::Length::Fill)
            .center_x(iced::Length::Fill)
            .center_y(iced::Length::Fill);
        elements.push(loading.into());
    }

    container(column(elements).padding(20).spacing(10))
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .into()
}

fn view_connections(app: &App) -> Element<'_, Message> {
    let header = row![
        text("Connections").size(24),
        button("+ Add Connection").on_press(Message::ConnectionAdd),
    ]
    .spacing(10)
    .align_y(iced::Alignment::Center);

    let mut content = column![header].spacing(10);

    if let Some(form) = &app.connection_form {
        let fields = column![
            text(if form.id.is_some() {
                "Edit Connection"
            } else {
                "Add Connection"
            })
            .size(18),
            text_input("Name", &form.name).on_input(|v| {
                Message::ConnectionFormChanged {
                    field: "name".into(),
                    value: v,
                }
            }),
            text_input("Endpoint (e.g. https://s3.example.com)", &form.endpoint).on_input(
                |v| Message::ConnectionFormChanged {
                    field: "endpoint".into(),
                    value: v,
                },
            ),
            text_input("Region", &form.region).on_input(|v| {
                Message::ConnectionFormChanged {
                    field: "region".into(),
                    value: v,
                }
            }),
            text_input("Access Key ID", &form.access_key_id).on_input(|v| {
                Message::ConnectionFormChanged {
                    field: "access_key_id".into(),
                    value: v,
                }
            }),
            text_input("Secret Access Key", &form.secret_access_key).on_input(|v| Message::ConnectionFormChanged {
                    field: "secret_access_key".into(),
                    value: v,
                }),
            toggler(form.force_path_style)
                .label("Path-style URL (recommended for S3-compatible storage)")
                .on_toggle(|b| Message::ConnectionFormChanged {
                    field: "force_path_style".into(),
                    value: b.to_string(),
                }),
            row![
                button("Save").on_press(Message::ConnectionFormSave),
                button("Cancel").on_press(Message::ConnectionFormCancel),
            ]
            .spacing(10),
        ]
        .spacing(8)
        .padding(15);

        content = column![
            content,
            container(fields)
                .style(|_theme: &Theme| container::Style {
                    background: Some(iced::Background::Color(iced::Color::from_rgb(
                        0.15, 0.15, 0.2,
                    ))),
                    border: iced::Border::default().rounded(4),
                    ..Default::default()
                })
                .width(iced::Length::Fill),
        ]
        .spacing(10);
    }

    let items: Vec<Element<Message>> = app
        .config_store
        .list()
        .iter()
        .map(|conn| {
            let card = column![
                text(&conn.name).size(16),
                text(&conn.endpoint)
                    .size(12)
                    .color(iced::Color::from_rgb(0.6, 0.6, 0.6)),
            ]
            .spacing(4);

            row![
                container(card).width(iced::Length::Fill),
                button("Connect").on_press(Message::ConnectionSelected(conn.id.clone())),
                button("Edit").on_press(Message::ConnectionEdit(conn.id.clone())),
                button("Delete").on_press(Message::ConnectionDelete(conn.id.clone())),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center)
            .into()
        })
        .collect();

    let list = container(column(items).spacing(6).padding(10))
        .style(|_theme: &Theme| container::Style {
            background: Some(iced::Background::Color(iced::Color::from_rgb(
                0.1, 0.1, 0.15,
            ))),
            border: iced::Border::default().rounded(4),
            ..Default::default()
        })
        .width(iced::Length::Fill);

    container(column![content, list].spacing(10))
        .width(iced::Length::Fill)
        .into()
}

fn view_buckets(app: &App) -> Element<'_, Message> {
    let header = row![
        button("← Back").on_press(Message::GoToConnections),
        text("Buckets").size(24),
    ]
    .spacing(10)
    .align_y(iced::Alignment::Center);

    let items: Vec<Element<Message>> = app
        .buckets
        .iter()
        .map(|b| {
            let card = row![
                text(format!("📁 {}", b.name)).size(16),
                container(
                    text(
                        b.creation_date
                            .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
                            .unwrap_or_default()
                    )
                    .size(12)
                    .color(iced::Color::from_rgb(0.6, 0.6, 0.6)),
                )
                .width(iced::Length::Fill),
                button("Open").on_press(Message::BucketSelected(b.name.clone())),
            ]
            .spacing(10)
            .align_y(iced::Alignment::Center);

            container(card)
                .padding(10)
                .style(|_theme: &Theme| container::Style {
                    background: Some(iced::Background::Color(iced::Color::from_rgb(
                        0.12, 0.12, 0.18,
                    ))),
                    border: iced::Border::default().rounded(4),
                    ..Default::default()
                })
                .width(iced::Length::Fill)
                .into()
        })
        .collect();

    let list = scrollable(column(items).spacing(6));

    container(column![header, rule::horizontal(1), list].spacing(10))
        .width(iced::Length::Fill)
        .into()
}

fn view_objects(app: &App) -> Element<'_, Message> {
    let bucket_name = app.current_bucket.as_deref().unwrap_or("unknown");

    let breadcrumb = row![
        button("← Back").on_press(Message::NavigateUp),
        text(format!("📁 {}", bucket_name)).size(24),
        text(&app.current_prefix)
            .size(14)
            .color(iced::Color::from_rgb(0.6, 0.6, 0.6)),
        button("Refresh").on_press(Message::RefreshObjects),
    ]
    .spacing(10)
    .align_y(iced::Alignment::Center);

    let upload_row = row![
        text_input("Local file path", &app.upload_path)
            .on_input(Message::UploadPathChanged)
            .width(iced::Length::Fill),
        button("Upload").on_press(Message::UploadObject),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center);

    let download_row = row![
        text_input("Download save path", &app.download_path)
            .on_input(Message::DownloadPathChanged)
            .width(iced::Length::Fill),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center);

    let mut items: Vec<Element<Message>> = Vec::new();

    if !app.current_prefix.is_empty() {
        items.push(
            button(
                row![
                    text("📂 ..").size(16),
                    container(text("").size(12)).width(iced::Length::Fill),
                ]
                .spacing(10)
                .align_y(iced::Alignment::Center),
            )
            .on_press(Message::NavigateUp)
            .into(),
        );
    }

    for prefix in &app.common_prefixes {
        let display_name = prefix
            .strip_prefix(&app.current_prefix)
            .unwrap_or(prefix)
            .trim_end_matches('/');

        items.push(
            button(
                row![
                    text(format!("📁 {}", display_name)).size(16),
                    container(
                        text("Folder")
                            .size(12)
                            .color(iced::Color::from_rgb(0.6, 0.6, 0.6)),
                    )
                    .width(iced::Length::Fill),
                ]
                .spacing(10)
                .align_y(iced::Alignment::Center),
            )
            .on_press(Message::PrefixSelected(prefix.clone()))
            .into(),
        );
        items.push(rule::horizontal(1).into());
    }

    for obj in &app.objects {
        let name = obj.key.strip_prefix(&app.current_prefix).unwrap_or(&obj.key);
        if name.is_empty() {
            continue;
        }

        let row_content = row![
            text(format!("📄 {}", name)).size(14),
            container(
                text(format_size(obj.size))
                    .size(12)
                    .color(iced::Color::from_rgb(0.6, 0.6, 0.6)),
            )
            .width(iced::Length::Fill),
            text(
                obj.last_modified
                    .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_default()
            )
            .size(12)
            .color(iced::Color::from_rgb(0.6, 0.6, 0.6)),
            button("Download").on_press(Message::DownloadObject(obj.key.clone())),
            button("Delete").on_press(Message::DeleteObject(obj.key.clone())),
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center);

        items.push(container(row_content).padding(Padding::new(8.0).right(24.0)).width(iced::Length::Fill).into());
        items.push(rule::horizontal(1).into());
    }

    if app.is_truncated {
        items.push(
            container(button("Load more...").on_press(Message::LoadMoreObjects))
                .padding(Padding::new(8.0).right(24.0))
                .center_x(iced::Length::Fill)
                .width(iced::Length::Fill)
                .into(),
        );
    }

    let list = scrollable(column(items).spacing(4));

    container(
        column![
            breadcrumb,
            rule::horizontal(1),
            upload_row,
            download_row,
            rule::horizontal(1),
            list,
        ]
        .spacing(10),
    )
    .width(iced::Length::Fill)
    .into()
}

fn format_size(size: i64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = size as f64;
    let mut unit_idx = 0;
    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }
    format!("{:.1} {}", size, UNITS[unit_idx])
}
