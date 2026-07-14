use iced::{
    Element, Padding, Task, Theme,
    widget::{button, column, container, pick_list, row, rule, scrollable, text, text_input, toggler},
};
use rust_i18n::t;

rust_i18n::i18n!("locales");
use s3dm_config::ConfigStore;
use s3dm_core::{CoreError, ObjectListResult, S3Bucket, S3Manager, S3Object};

const AVAILABLE_THEMES: &[(&str, Theme)] = &[
    ("Dark", Theme::Dark),
    ("Light", Theme::Light),
    ("Dracula", Theme::Dracula),
    ("Nord", Theme::Nord),
    ("Solarized Light", Theme::SolarizedLight),
    ("Solarized Dark", Theme::SolarizedDark),
    ("Gruvbox Light", Theme::GruvboxLight),
    ("Gruvbox Dark", Theme::GruvboxDark),
    ("Catppuccin Latte", Theme::CatppuccinLatte),
    ("Catppuccin Frappé", Theme::CatppuccinFrappe),
    ("Catppuccin Macchiato", Theme::CatppuccinMacchiato),
    ("Catppuccin Mocha", Theme::CatppuccinMocha),
    ("Tokyo Night", Theme::TokyoNight),
    ("Tokyo Night Storm", Theme::TokyoNightStorm),
    ("Tokyo Night Light", Theme::TokyoNightLight),
    ("Kanagawa Wave", Theme::KanagawaWave),
    ("Kanagawa Dragon", Theme::KanagawaDragon),
    ("Kanagawa Lotus", Theme::KanagawaLotus),
    ("Moonfly", Theme::Moonfly),
    ("Nightfly", Theme::Nightfly),
    ("Oxocarbon", Theme::Oxocarbon),
    ("Ferra", Theme::Ferra),
];

const LANGUAGES: &[(&str, &str)] = &[
    ("English", "en"),
    ("简体中文", "zh-CN"),
    ("繁體中文", "zh-TW"),
];

struct CustomPalette {
    surface: iced::Color,
    surface_raised: iced::Color,
    text_secondary: iced::Color,
}

fn custom_palette(theme: &Theme) -> CustomPalette {
    let bg = theme.palette().background;
    let luminance = 0.299 * bg.r + 0.587 * bg.g + 0.114 * bg.b;
    if luminance > 0.5 {
        CustomPalette {
            surface: iced::Color::from_rgb(
                (bg.r - 0.06).max(0.0),
                (bg.g - 0.06).max(0.0),
                (bg.b - 0.06).max(0.0),
            ),
            surface_raised: iced::Color::from_rgb(
                (bg.r - 0.10).max(0.0),
                (bg.g - 0.10).max(0.0),
                (bg.b - 0.10).max(0.0),
            ),
            text_secondary: iced::Color::from_rgb(0.45, 0.45, 0.45),
        }
    } else {
        CustomPalette {
            surface: iced::Color::from_rgb(
                (bg.r + 0.08).min(1.0),
                (bg.g + 0.08).min(1.0),
                (bg.b + 0.08).min(1.0),
            ),
            surface_raised: iced::Color::from_rgb(
                (bg.r + 0.12).min(1.0),
                (bg.g + 0.12).min(1.0),
                (bg.b + 0.12).min(1.0),
            ),
            text_secondary: iced::Color::from_rgb(0.6, 0.6, 0.6),
        }
    }
}

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
    ConnectionFormChanged {
        field: String,
        value: String,
    },
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
    ToggleSettings,
    ThemeChanged(String),
    LanguageChanged(String),
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
    pub show_settings: bool,
    pub theme: Theme,
    pub current_theme_name: String,
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
        show_settings: false,
        theme: Theme::Dark,
        current_theme_name: "Dark".to_string(),
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
                    log::info!("Connected successfully, got {} buckets", list.len());
                    app.s3_manager = Some(manager);
                    app.buckets = list;
                    app.selected_connection_id = Some(connection_id);
                    app.current_page = Page::Buckets;
                }
                Err(e) => {
                    log::error!("Connection failed: {}", e);
                    app.error_message =
                        Some(t!("connection_failed", error = e.to_string()).to_string());
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
                app.error_message =
                    Some(t!("delete_connection_failed", error = e.to_string()).to_string());
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
                    app.error_message =
                        Some(t!("save_connection_failed", error = e.to_string()).to_string());
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
                    app.error_message =
                        Some(t!("load_objects_failed", error = e.to_string()).to_string());
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
                app.error_message = Some(t!("delete_failed", error = e.to_string()).to_string());
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
                app.error_message = Some(t!("select_file_path").to_string());
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
                            Err(CoreError::S3(
                                t!("read_file_failed", error = e.to_string()).to_string(),
                            ))
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
                    app.error_message =
                        Some(t!("upload_failed", error = e.to_string()).to_string());
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
                app.error_message = Some(t!("set_download_path").to_string());
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
                        format!(
                            "{}{}",
                            path,
                            key.rsplit_once('/').map(|(_, f)| f).unwrap_or(&key)
                        )
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
                    app.error_message =
                        Some(t!("download_failed", error = e.to_string()).to_string());
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
        Message::ToggleSettings => {
            app.show_settings = !app.show_settings;
            Task::none()
        }
        Message::ThemeChanged(name) => {
            if let Some((_, theme)) = AVAILABLE_THEMES.iter().find(|(n, _)| *n == name) {
                app.theme = theme.clone();
                app.current_theme_name = name;
            }
            Task::none()
        }
        Message::LanguageChanged(code) => {
            rust_i18n::set_locale(&code);
            Task::none()
        }
    }
}

pub fn view(app: &App) -> Element<'_, Message> {
    let mut elements: Vec<Element<Message>> = Vec::new();

    if let Some(err) = &app.error_message {
        let error_bar = container(
            row![
                text(t!("error", message = err.as_str()).to_string()).color(iced::Color::WHITE),
                button("×").on_press(Message::ClearError),
            ]
            .spacing(10)
            .align_y(iced::Alignment::Center),
        )
        .padding(10)
        .style(|_: &Theme| container::Style {
            background: Some(iced::Background::Color(iced::Color::from_rgb(
                0.8, 0.2, 0.2,
            ))),
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
        let loading = container(text(t!("loading").to_string()).size(24))
            .width(iced::Length::Fill)
            .height(iced::Length::Fill)
            .center_x(iced::Length::Fill)
            .center_y(iced::Length::Fill);
        elements.push(loading.into());
    }

    let content = container(column(elements).padding(20).spacing(10))
        .width(iced::Length::Fill)
        .height(iced::Length::Fill);

    if app.show_settings {
        let overlay = container(view_settings(app))
            .width(iced::Length::Fill)
            .height(iced::Length::Fill)
            .style(|_: &Theme| container::Style {
                background: Some(iced::Background::Color(iced::Color::from_rgba(
                    0.0, 0.0, 0.0, 0.6,
                ))),
                ..Default::default()
            })
            .center_x(iced::Length::Fill)
            .center_y(iced::Length::Fill);

        iced::widget::stack(vec![content.into(), iced::widget::opaque(overlay).into()]).into()
    } else {
        content.into()
    }
}

fn view_connections(app: &App) -> Element<'_, Message> {
    let p = custom_palette(&app.theme);
    let header = row![
        text(t!("connections").to_string()).size(24),
        button(text(t!("add_connection").to_string())).on_press(Message::ConnectionAdd),
        container(button("⚙").on_press(Message::ToggleSettings))
            .width(iced::Length::Fill)
            .align_x(iced::Alignment::End),
    ]
    .spacing(10)
    .align_y(iced::Alignment::Center);

    let mut content = column![header].spacing(10);

    if let Some(form) = &app.connection_form {
        let placeholder_name = t!("name").to_string();
        let placeholder_endpoint = t!("endpoint_hint").to_string();
        let placeholder_region = t!("region").to_string();
        let placeholder_ak = t!("access_key_id").to_string();
        let placeholder_sk = t!("secret_access_key").to_string();

        let fields = column![
            text(if form.id.is_some() {
                t!("edit_connection_title").to_string()
            } else {
                t!("add_connection_title").to_string()
            })
            .size(18),
            text_input(&placeholder_name, &form.name).on_input(|v| {
                Message::ConnectionFormChanged {
                    field: "name".into(),
                    value: v,
                }
            }),
            text_input(&placeholder_endpoint, &form.endpoint).on_input(|v| {
                Message::ConnectionFormChanged {
                    field: "endpoint".into(),
                    value: v,
                }
            },),
            text_input(&placeholder_region, &form.region).on_input(|v| {
                Message::ConnectionFormChanged {
                    field: "region".into(),
                    value: v,
                }
            }),
            text_input(&placeholder_ak, &form.access_key_id).on_input(|v| {
                Message::ConnectionFormChanged {
                    field: "access_key_id".into(),
                    value: v,
                }
            }),
            text_input(&placeholder_sk, &form.secret_access_key).on_input(|v| {
                Message::ConnectionFormChanged {
                    field: "secret_access_key".into(),
                    value: v,
                }
            }),
            toggler(form.force_path_style)
                .label(t!("force_path_style_label").to_string())
                .on_toggle(|b| Message::ConnectionFormChanged {
                    field: "force_path_style".into(),
                    value: b.to_string(),
                }),
            row![
                button(text(t!("save").to_string())).on_press(Message::ConnectionFormSave),
                button(text(t!("cancel").to_string())).on_press(Message::ConnectionFormCancel),
            ]
            .spacing(10),
        ]
        .spacing(8)
        .padding(15);

        content = column![
            content,
            container(fields)
                .style(|theme: &Theme| container::Style {
                    background: Some(iced::Background::Color(custom_palette(theme).surface_raised)),
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
                    .color(p.text_secondary),
            ]
            .spacing(4);

            row![
                container(card).width(iced::Length::Fill),
                button(text(t!("connect").to_string()))
                    .on_press(Message::ConnectionSelected(conn.id.clone())),
                button(text(t!("edit").to_string()))
                    .on_press(Message::ConnectionEdit(conn.id.clone())),
                button(text(t!("delete").to_string()))
                    .on_press(Message::ConnectionDelete(conn.id.clone())),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center)
            .into()
        })
        .collect();

    let content = if items.is_empty() {
        content
    } else {
        let list = container(column(items).spacing(6).padding(10))
            .style(|theme: &Theme| container::Style {
                background: Some(iced::Background::Color(custom_palette(theme).surface)),
                border: iced::Border::default().rounded(4),
                ..Default::default()
            })
            .width(iced::Length::Fill);

        column![content, list].spacing(10)
    };

    container(content).width(iced::Length::Fill).into()
}

fn view_buckets(app: &App) -> Element<'_, Message> {
    let p = custom_palette(&app.theme);
    let header = row![
        button(text(t!("back").to_string())).on_press(Message::GoToConnections),
        text(t!("buckets").to_string()).size(24),
        container(button("⚙").on_press(Message::ToggleSettings))
            .width(iced::Length::Fill)
            .align_x(iced::Alignment::End),
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
                    .color(p.text_secondary),
                )
                .width(iced::Length::Fill),
                button(text(t!("open").to_string()))
                    .on_press(Message::BucketSelected(b.name.clone())),
            ]
            .spacing(10)
            .align_y(iced::Alignment::Center);

            container(card)
                .padding(10)
                .style(|theme: &Theme| container::Style {
                    background: Some(iced::Background::Color(custom_palette(theme).surface)),
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
    let p = custom_palette(&app.theme);
    let unknown_label = t!("unknown").to_string();
    let bucket_name = app.current_bucket.as_deref().unwrap_or(&unknown_label);
    let placeholder_local_path = t!("local_file_path").to_string();
    let placeholder_download_path = t!("download_save_path").to_string();

    let breadcrumb = row![
        button(text(t!("back").to_string())).on_press(Message::NavigateUp),
        text(format!("📁 {}", bucket_name)).size(24),
        text(&app.current_prefix)
            .size(14)
            .color(p.text_secondary),
        button(text(t!("refresh").to_string())).on_press(Message::RefreshObjects),
        container(button("⚙").on_press(Message::ToggleSettings))
            .width(iced::Length::Fill)
            .align_x(iced::Alignment::End),
    ]
    .spacing(10)
    .align_y(iced::Alignment::Center);

    let upload_row = row![
        text_input(&placeholder_local_path, &app.upload_path)
            .on_input(Message::UploadPathChanged)
            .width(iced::Length::Fill),
        button(text(t!("upload").to_string())).on_press(Message::UploadObject),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center);

    let download_row = row![
        text_input(&placeholder_download_path, &app.download_path)
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
                        text(t!("folder").to_string())
                    .size(12)
                    .color(p.text_secondary),
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
        let name = obj
            .key
            .strip_prefix(&app.current_prefix)
            .unwrap_or(&obj.key);
        if name.is_empty() {
            continue;
        }

        let row_content = row![
            text(format!("📄 {}", name)).size(14),
            container(
                text(format_size(obj.size))
                    .size(12)
                    .color(p.text_secondary),
            )
            .width(iced::Length::Fill),
            text(
                obj.last_modified
                    .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_default()
            )
            .size(12)
            .color(p.text_secondary),
            button(text(t!("download").to_string()))
                .on_press(Message::DownloadObject(obj.key.clone())),
            button(text(t!("delete").to_string())).on_press(Message::DeleteObject(obj.key.clone())),
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center);

        items.push(
            container(row_content)
                .padding(Padding::new(8.0).right(24.0))
                .width(iced::Length::Fill)
                .into(),
        );
        items.push(rule::horizontal(1).into());
    }

    if app.is_truncated {
        items.push(
            container(button(text(t!("load_more").to_string())).on_press(Message::LoadMoreObjects))
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

fn view_settings(app: &App) -> Element<'_, Message> {
    let theme_names: Vec<String> = AVAILABLE_THEMES.iter().map(|(n, _)| n.to_string()).collect();
    let lang_names: Vec<String> = LANGUAGES.iter().map(|(n, _)| n.to_string()).collect();
    let current_locale = rust_i18n::locale().to_string();
    let current_lang = LANGUAGES
        .iter()
        .find(|(_, code)| *code == current_locale)
        .map(|(name, _)| name.to_string())
        .unwrap_or_else(|| "English".to_string());

    let panel = column![
        row![
            text(t!("settings").to_string()).size(20),
            container(button("×").on_press(Message::ToggleSettings))
                .width(iced::Length::Fill)
                .align_x(iced::Alignment::End),
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center),
        iced::widget::rule::horizontal(1),
        text(t!("theme").to_string()).size(16),
        pick_list(theme_names, Some(app.current_theme_name.clone()), Message::ThemeChanged),
        text(t!("language").to_string()).size(16),
        pick_list(
            lang_names,
            Some(current_lang),
            |name| {
                let code = LANGUAGES
                    .iter()
                    .find(|(n, _)| *n == name)
                    .map(|(_, c)| c.to_string())
                    .unwrap_or_else(|| "en".to_string());
                Message::LanguageChanged(code)
            },
        ),
    ]
    .spacing(15)
    .padding(20);

    container(panel)
        .width(360)
        .style(|theme: &Theme| container::Style {
            background: Some(iced::Background::Color(custom_palette(theme).surface_raised)),
            border: iced::Border::default().rounded(8),
            ..Default::default()
        })
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
