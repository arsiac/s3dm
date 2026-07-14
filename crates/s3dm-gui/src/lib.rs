use std::path::PathBuf;

use iced::widget::svg;
use iced::widget::svg::Handle as SvgHandle;
use iced::{
    Alignment, Element, Length, Padding, Task, Theme,
    widget::{
        button, column, container, pick_list, row, rule, scrollable, text, text_input, toggler,
    },
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
    ToggleConnectionExpand(String),
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
    DeletePrefix(String),
    UploadObject,
    DownloadObject(String),
    ObjectsResult(Result<ObjectListResult, CoreError>),
    DeleteResult(Result<(), CoreError>),
    DownloadResult {
        key: String,
        save_path: String,
        data: Result<Vec<u8>, CoreError>,
    },
    UploadResult(Result<(), CoreError>),
    FileChosen(Option<PathBuf>),
    DownloadDirChanged(String),
    ClearError,
    ToggleSettings,
    ThemeChanged(String),
    LanguageChanged(String),
    ConfirmDelete(String),
    CancelDelete,
    ConfirmDeleteObject(String),
    CancelDeleteObject,
}

pub struct App {
    config_store: ConfigStore,
    s3_manager: Option<S3Manager>,
    error_message: Option<String>,
    expanded_connection: Option<String>,
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
    download_dir: String,
    pending_delete: Option<String>,
    pending_delete_object: Option<String>,
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
        s3_manager: None,
        error_message: None,
        expanded_connection: None,
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
        download_dir: dirs::download_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
        pending_delete: None,
        pending_delete_object: None,
        show_settings: false,
        theme: Theme::Dark,
        current_theme_name: "Dark".to_string(),
    };
    (app, Task::none())
}

pub fn update(app: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::ToggleConnectionExpand(id) => {
            if app.expanded_connection.as_ref() == Some(&id) {
                app.expanded_connection = None;
            }
            Task::none()
        }
        Message::ConnectionSelected(conn_id) => {
            log::info!("Connection selected: id={}", conn_id);
            app.expanded_connection = Some(conn_id.clone());
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
                }
                Err(e) => {
                    log::error!("Connection failed: {}", e);
                    app.expanded_connection = None;
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
            log::info!("Prompting delete confirmation: id={}", id);
            app.pending_delete = Some(id);
            Task::none()
        }
        Message::ConfirmDelete(id) => {
            log::info!("Confirming delete: id={}", id);
            if let Err(e) = app.config_store.delete(&id) {
                log::error!("Delete connection failed: {}", e);
                app.error_message =
                    Some(t!("delete_connection_failed", error = e.to_string()).to_string());
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
        Message::CancelDelete => {
            app.pending_delete = None;
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
            log::info!("Prompting delete object confirmation: {}", key);
            app.pending_delete_object = Some(key);
            Task::none()
        }
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
                async move { s3.delete_object(&bucket, &key) },
                Message::DeleteResult,
            )
        }
        Message::CancelDeleteObject => {
            app.pending_delete_object = None;
            Task::none()
        }
        Message::DeletePrefix(prefix) => {
            log::info!("Deleting prefix: {}", prefix);
            let bucket = match &app.current_bucket {
                Some(b) => b.clone(),
                None => return Task::none(),
            };
            let s3 = match &app.s3_manager {
                Some(s) => s.clone(),
                None => return Task::none(),
            };
            app.is_loading = true;
            Task::perform(
                async move { s3.delete_prefix(&bucket, &prefix) },
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
                path.file_name().map(|n| n.to_string_lossy()).unwrap_or_default()
            );
            log::info!("Uploading file: {:?} -> {}", path, key);
            app.is_loading = true;
            Task::perform(
                async move {
                    match std::fs::read(&path) {
                        Ok(data) => s3.put_object(&bucket, &key, data),
                        Err(e) => {
                            log::error!("Failed to read file: {:?}: {}", path, e);
                            Err(CoreError::S3(
                                t!("read_file_failed", error = e.to_string()).to_string(),
                            ))
                        }
                    }
                },
                Message::UploadResult,
            )
        }
        Message::FileChosen(None) => Task::none(),
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
            let s3 = match &app.s3_manager {
                Some(s) => s.clone(),
                None => return Task::none(),
            };
            let dir = app.download_dir.clone();
            let fname = key.rsplit_once('/').map(|(_, n)| n).unwrap_or(&key);
            let save_path = format!("{}/{}", dir.trim_end_matches('/'), fname);
            log::info!("Downloading object: {} -> {}", key, save_path);
            app.is_loading = true;
            let key_c = key.clone();
            Task::perform(
                async move {
                    let data = s3.get_object(&bucket, &key);
                    (key_c, save_path, data)
                },
                |(key, save_path, data)| Message::DownloadResult { key, save_path, data },
            )
        }
        Message::DownloadResult { key: _, save_path, data } => {
            app.is_loading = false;
            match data {
                Ok(bytes) => {
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
        Message::DownloadDirChanged(path) => {
            app.download_dir = path;
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
            .align_y(Alignment::Center),
        )
        .padding(10)
        .style(|_: &Theme| container::Style {
            background: Some(iced::Background::Color(iced::Color::from_rgb(
                0.8, 0.2, 0.2,
            ))),
            text_color: Some(iced::Color::WHITE),
            ..Default::default()
        })
        .width(Length::Fill);
        elements.push(error_bar.into());
    }

    let side_panel = view_left_panel(app);
    let right_area = view_right_content(app);
    let status_line = view_status_bar(app);

    let body = row![
        side_panel,
        rule::vertical(1),
        container(right_area)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(20),
    ]
    .height(Length::Fill);

    elements.push(
        column![body, rule::horizontal(1), status_line,]
            .spacing(0)
            .height(Length::Fill)
            .into(),
    );

    if app.is_loading {
        let loading = container(text(t!("loading").to_string()).size(24))
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill);
        elements.push(loading.into());
    }

    let content = container(column(elements).spacing(0))
        .width(Length::Fill)
        .height(Length::Fill);

    let mut stack_elements: Vec<Element<Message>> = vec![content.into()];

    if app.show_settings {
        let overlay = container(view_settings(app))
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

        stack_elements.push(iced::widget::opaque(overlay).into());
    }

    if app.connection_form.is_some() {
        let overlay = container(view_connection_form(app))
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

        stack_elements.push(iced::widget::opaque(overlay).into());
    }

    if let Some(ref del_id) = app.pending_delete {
        let conn_name = app.config_store.list().iter()
            .find(|c| &c.id == del_id).map(|c| c.name.as_str()).unwrap_or("?");
        let p = custom_palette(&app.theme);
        let panel = column![
            text(t!("delete_confirm_title").to_string()).size(18),
            rule::horizontal(1),
            text(t!("delete_confirm_message", name = conn_name).to_string()).size(14),
            row![
                container(button(text(t!("confirm").to_string())).on_press(Message::ConfirmDelete(del_id.clone())))
                    .width(Length::Fill)
                    .align_x(Alignment::End),
                button(text(t!("cancel").to_string())).on_press(Message::CancelDelete),
            ]
            .spacing(10),
        ]
        .spacing(16)
        .padding(20);

        let content = container(panel)
            .width(360)
            .style(move |_: &Theme| container::Style {
                background: Some(iced::Background::Color(p.surface_raised)),
                border: iced::Border::default().rounded(8),
                ..Default::default()
            });

        let overlay = container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_: &Theme| container::Style {
                background: Some(iced::Background::Color(iced::Color::from_rgba(0.0, 0.0, 0.0, 0.6))),
                ..Default::default()
            })
            .center_x(Length::Fill)
            .center_y(Length::Fill);

        stack_elements.push(iced::widget::opaque(overlay).into());
    }

    if let Some(ref del_key) = app.pending_delete_object {
        let obj_name = del_key.rsplit_once('/').map(|(_, n)| n).unwrap_or(del_key);
        let p = custom_palette(&app.theme);
        let panel = column![
            text(t!("delete_object_confirm_title").to_string()).size(18),
            rule::horizontal(1),
            text(t!("delete_object_confirm_message", name = obj_name).to_string()).size(14),
            row![
                container(button(text(t!("confirm").to_string())).on_press(Message::ConfirmDeleteObject(del_key.clone())))
                    .width(Length::Fill)
                    .align_x(Alignment::End),
                button(text(t!("cancel").to_string())).on_press(Message::CancelDeleteObject),
            ]
            .spacing(10),
        ]
        .spacing(16)
        .padding(20);

        let content = container(panel)
            .width(360)
            .style(move |_: &Theme| container::Style {
                background: Some(iced::Background::Color(p.surface_raised)),
                border: iced::Border::default().rounded(8),
                ..Default::default()
            });

        let overlay = container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_: &Theme| container::Style {
                background: Some(iced::Background::Color(iced::Color::from_rgba(0.0, 0.0, 0.0, 0.6))),
                ..Default::default()
            })
            .center_x(Length::Fill)
            .center_y(Length::Fill);

        stack_elements.push(iced::widget::opaque(overlay).into());
    }

    iced::widget::stack(stack_elements).into()
}

fn view_left_panel(app: &App) -> Element<'_, Message> {
    let p = custom_palette(&app.theme);
    let palette = app.theme.palette();
    let connections = app.config_store.list();

    let hover_bg = iced::Color::from_rgba(1.0, 1.0, 1.0, 0.08);
    let icon_btn_style = move |_: &Theme, s: button::Status| -> button::Style {
        let (bg, border) = match s {
            button::Status::Hovered | button::Status::Pressed => (
                Some(iced::Background::Color(hover_bg)),
                iced::Border {
                    color: hover_bg,
                    width: 1.0,
                    radius: 4.0.into(),
                },
            ),
            _ => (None, iced::Border::default().width(0)),
        };
        button::Style {
            background: bg,
            border,
            text_color: p.text_secondary,
            shadow: iced::Shadow::default(),
            ..Default::default()
        }
    };

    let svg_style = |theme: &Theme, _: svg::Status| svg::Style {
        color: Some(custom_palette(theme).text_secondary),
    };

    let header = container(
        row![
            text(t!("storage_browser").to_string())
                .size(13)
                .color(p.text_secondary),
            container(
                button(
                    svg(SvgHandle::from_memory(
                        include_bytes!("../icons/add-16-filled.svg").to_vec(),
                    ))
                    .width(Length::Fixed(16.0))
                    .height(Length::Fixed(16.0))
                    .style(svg_style),
                )
                .style(icon_btn_style)
                .on_press(Message::ConnectionAdd)
                .padding(Padding::from([2, 6]))
            )
            .width(Length::Fill)
            .align_x(Alignment::End),
            button(
                svg(SvgHandle::from_memory(
                    include_bytes!("../icons/settings-16-filled.svg").to_vec(),
                ))
                .width(Length::Fixed(16.0))
                .height(Length::Fixed(16.0))
                .style(svg_style),
            )
            .style(icon_btn_style)
            .on_press(Message::ToggleSettings)
            .padding(Padding::from([2, 6])),
        ]
        .spacing(2)
        .align_y(Alignment::Center),
    )
    .padding(Padding::from([12, 16]))
    .width(Length::Fill);

    let mut items: Vec<Element<Message>> = Vec::new();

    if connections.is_empty() {
        items.push(
            container(
                text(t!("no_connection").to_string())
                    .size(12)
                    .color(p.text_secondary),
            )
            .padding(Padding::from([8, 16]))
            .width(Length::Fill)
            .into(),
        );
    }

    for conn in connections.iter() {
        let is_expanded = app.expanded_connection.as_ref() == Some(&conn.id);
        let is_connected = app.selected_connection_id.as_ref() == Some(&conn.id);

        let icon = if is_expanded { "▼" } else { "▶" };

        let action_btn_style = move |_: &Theme, s: button::Status| -> button::Style {
            let hbg = iced::Color::from_rgba(1.0, 1.0, 1.0, 0.08);
            let (bg, border) = match s {
                button::Status::Hovered | button::Status::Pressed => (
                    Some(iced::Background::Color(hbg)),
                    iced::Border { color: hbg, width: 1.0, radius: 4.0.into() },
                ),
                _ => (None, iced::Border::default().width(0)),
            };
            button::Style { background: bg, border, text_color: p.text_secondary, shadow: iced::Shadow::default(), ..Default::default() }
        };
        let action_svg = |data: &[u8]| {
            svg(SvgHandle::from_memory(data.to_vec()))
                .width(Length::Fixed(14.0)).height(Length::Fixed(14.0))
                .style(|t: &Theme, _: svg::Status| svg::Style { color: Some(custom_palette(t).text_secondary) })
        };

        let edit_btn = button(action_svg(include_bytes!("../icons/edit-16-filled.svg")))
            .style(action_btn_style).on_press(Message::ConnectionEdit(conn.id.clone())).padding(Padding::from([2, 4]));
        let delete_btn = button(action_svg(include_bytes!("../icons/delete-16-filled.svg")))
            .style(action_btn_style).on_press(Message::ConnectionDelete(conn.id.clone())).padding(Padding::from([2, 4]));

        let conn_row = row![
            text(icon).size(10).color(p.text_secondary),
            text(&conn.name).size(13),
            container(edit_btn).width(Length::Fill).align_x(Alignment::End),
            delete_btn,
            text("●").size(8).color(if is_connected {
                iced::Color::from_rgb(0.3, 0.8, 0.3)
            } else {
                p.text_secondary
            }),
        ]
        .spacing(2)
        .align_y(Alignment::Center);

        let msg = if is_expanded {
            Message::ToggleConnectionExpand(conn.id.clone())
        } else {
            Message::ConnectionSelected(conn.id.clone())
        };

        let row_bg = if is_expanded {
            Some(iced::Background::Color(iced::Color::from_rgba(
                1.0, 1.0, 1.0, 0.04,
            )))
        } else {
            None
        };

        items.push(
            button(conn_row)
                .on_press(msg)
                .style(move |_: &Theme, s: button::Status| {
                    let bg = match s {
                        button::Status::Hovered | button::Status::Pressed => Some(
                            iced::Background::Color(iced::Color::from_rgba(1.0, 1.0, 1.0, 0.08)),
                        ),
                        _ => row_bg,
                    };
                    button::Style {
                        background: bg,
                        text_color: palette.text,
                        border: iced::Border::default(),
                        shadow: iced::Shadow::default(),
                        ..Default::default()
                    }
                })
                .padding(Padding::from([8, 16]))
                .width(Length::Fill)
                .into(),
        );

        if is_expanded {
            if app.buckets.is_empty() && app.is_loading {
                items.push(
                    container(text("  ...").size(12).color(p.text_secondary))
                        .padding(Padding::from([4, 16]))
                        .into(),
                );
            }
            for bucket in &app.buckets {
                let is_active = app.current_bucket.as_deref() == Some(&bucket.name);
                let bucket_bg = if is_active {
                    Some(iced::Background::Color(iced::Color::from_rgba(
                        1.0, 1.0, 1.0, 0.08,
                    )))
                } else {
                    None
                };
                items.push(
                    button(
                        row![
                            svg(SvgHandle::from_memory(include_bytes!("../icons/folder-16-filled.svg").to_vec()))
                                .width(Length::Fixed(14.0)).height(Length::Fixed(14.0))                                .style(svg_style),
                            text(&bucket.name).size(12),
                        ]
                            .spacing(6)
                            .align_y(Alignment::Center),
                    )
                    .on_press(Message::BucketSelected(bucket.name.clone()))
                    .style(move |_: &Theme, s: button::Status| {
                        let bg = match s {
                            button::Status::Hovered | button::Status::Pressed => {
                                Some(iced::Background::Color(iced::Color::from_rgba(
                                    1.0, 1.0, 1.0, 0.08,
                                )))
                            }
                            _ => bucket_bg,
                        };
                        button::Style {
                            background: bg,
                            text_color: palette.text,
                            border: iced::Border::default(),
                            shadow: iced::Shadow::default(),
                            ..Default::default()
                        }
                    })
                    .padding(Padding::from([6, 16]))
                    .width(Length::Fill)
                    .into(),
                );
            }
        }
    }

    let mut list_elements: Vec<Element<Message>> = vec![header.into(), rule::horizontal(1).into()];
    list_elements.extend(items);

    container(scrollable(column(list_elements).spacing(0)))
        .width(260)
        .style(|theme: &Theme| {
            let p = custom_palette(theme);
            container::Style {
                background: Some(iced::Background::Color(p.surface)),
                ..Default::default()
            }
        })
        .height(Length::Fill)
        .into()
}

fn view_right_content(app: &App) -> Element<'_, Message> {
    if app.current_bucket.is_some() {
        view_objects(app)
    } else {
        let p = custom_palette(&app.theme);
        let hint = if app.expanded_connection.is_some() {
            t!("select_bucket_hint")
        } else if app.config_store.list().is_empty() {
            t!("no_connection")
        } else {
            t!("select_connection_hint")
        };
        container(
            text(hint.to_string())
                .size(16)
                .color(p.text_secondary),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
    }
}

fn view_connection_form(app: &App) -> Element<'_, Message> {
    let form = app.connection_form.as_ref().unwrap();
    let title = if form.id.is_some() {
        t!("edit_connection_title").to_string()
    } else {
        t!("add_connection_title").to_string()
    };

    let p = custom_palette(&app.theme);
    let btn_style = move |_: &Theme, s: button::Status| -> button::Style {
        let hbg = iced::Color::from_rgba(1.0, 1.0, 1.0, 0.08);
        let (bg, border) = match s {
            button::Status::Hovered | button::Status::Pressed => (
                Some(iced::Background::Color(hbg)),
                iced::Border { color: hbg, width: 1.0, radius: 4.0.into() },
            ),
            _ => (None, iced::Border::default().width(0)),
        };
        button::Style { background: bg, border, text_color: p.text_secondary, shadow: iced::Shadow::default(), ..Default::default() }
    };
    let svg_style = move |_: &Theme, _: svg::Status| svg::Style { color: Some(p.text_secondary) };
    let dismiss = svg(SvgHandle::from_memory(include_bytes!("../icons/dismiss-16-filled.svg").to_vec()))
        .width(Length::Fixed(16.0)).height(Length::Fixed(16.0)).style(svg_style);
    let panel = column![
        row![
            text(title).size(18),
            container(button(dismiss).style(btn_style).on_press(Message::ConnectionFormCancel))
                .width(Length::Fill)
                .align_x(Alignment::End),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
        rule::horizontal(1),
        text_input(&t!("name").to_string(), &form.name).on_input(|v| {
            Message::ConnectionFormChanged {
                field: "name".into(),
                value: v,
            }
        }),
        text_input(&t!("endpoint_hint").to_string(), &form.endpoint).on_input(|v| {
            Message::ConnectionFormChanged {
                field: "endpoint".into(),
                value: v,
            }
        }),
        text_input(&t!("region").to_string(), &form.region).on_input(|v| {
            Message::ConnectionFormChanged {
                field: "region".into(),
                value: v,
            }
        }),
        text_input(&t!("access_key_id").to_string(), &form.access_key_id).on_input(|v| {
            Message::ConnectionFormChanged {
                field: "access_key_id".into(),
                value: v,
            }
        }),
        text_input(
            &t!("secret_access_key").to_string(),
            &form.secret_access_key
        )
        .on_input(|v| {
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
    .spacing(10)
    .padding(20);

    container(panel)
        .width(420)
        .style(|theme: &Theme| container::Style {
            background: Some(iced::Background::Color(
                custom_palette(theme).surface_raised,
            )),
            border: iced::Border::default().rounded(8),
            ..Default::default()
        })
        .into()
}

fn view_objects(app: &App) -> Element<'_, Message> {
    let p = custom_palette(&app.theme);
    let unknown_label = t!("unknown").to_string();
    let bucket_name = app.current_bucket.as_deref().unwrap_or(&unknown_label).to_string();
    let icon_btn_style = move |_: &Theme, s: button::Status| -> button::Style {
        let hbg = iced::Color::from_rgba(1.0, 1.0, 1.0, 0.08);
        let (bg, border) = match s {
            button::Status::Hovered | button::Status::Pressed => (
                Some(iced::Background::Color(hbg)),
                iced::Border { color: hbg, width: 1.0, radius: 4.0.into() },
            ),
            _ => (None, iced::Border::default().width(0)),
        };
        button::Style { background: bg, border, text_color: p.text_secondary, shadow: iced::Shadow::default(), ..Default::default() }
    };
    let svg_style = |t: &Theme, _: svg::Status| svg::Style { color: Some(custom_palette(t).text_secondary) };
    let refresh_svg = svg(SvgHandle::from_memory(include_bytes!("../icons/arrow-clockwise-16-filled.svg").to_vec()))
        .width(Length::Fixed(16.0)).height(Length::Fixed(16.0)).style(svg_style);
    let upload_svg = svg(SvgHandle::from_memory(include_bytes!("../icons/cloud-arrow-up-16-filled.svg").to_vec()))
        .width(Length::Fixed(16.0)).height(Length::Fixed(16.0)).style(svg_style);

    let breadcrumb = row![
        row![
            svg(SvgHandle::from_memory(include_bytes!("../icons/folder-16-filled.svg").to_vec()))
                .width(Length::Fixed(16.0)).height(Length::Fixed(16.0)).style(svg_style),
            text(bucket_name).size(16),
        ]
        .spacing(4)
        .align_y(Alignment::Center),
        text(&app.current_prefix).size(14).color(p.text_secondary),
        container(button(refresh_svg).style(icon_btn_style).on_press(Message::RefreshObjects))
            .width(Length::Fill)
            .align_x(Alignment::End),
        button(upload_svg).style(icon_btn_style).on_press(Message::UploadObject),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    let mut items: Vec<Element<Message>> = Vec::new();

    let row_style = |theme: &Theme, _: button::Status| -> button::Style {
        let p = custom_palette(theme);
        button::Style {
            background: Some(iced::Background::Color(p.surface)),
            text_color: theme.palette().text,
            border: iced::Border::default().rounded(4),
            shadow: iced::Shadow::default(),
            ..Default::default()
        }
    };

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

    for prefix in &app.common_prefixes {
        let display_name = prefix
            .strip_prefix(&app.current_prefix)
            .unwrap_or(prefix)
            .trim_end_matches('/');

        let folder_delete_btn = button(
            svg(SvgHandle::from_memory(include_bytes!("../icons/delete-16-filled.svg").to_vec()))
                .width(Length::Fixed(16.0)).height(Length::Fixed(16.0)).style(svg_style),
        )
        .style(icon_btn_style)
        .on_press(Message::DeletePrefix(prefix.clone()));

        items.push(
            button(
                row![
                    row![
                        svg(SvgHandle::from_memory(include_bytes!("../icons/folder-16-filled.svg").to_vec()))
                            .width(Length::Fixed(14.0)).height(Length::Fixed(14.0)).style(svg_style),
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

    for obj in &app.objects {
        let name = obj
            .key
            .strip_prefix(&app.current_prefix)
            .unwrap_or(&obj.key);
        if name.is_empty() {
            continue;
        }

        let row_content = row![
            row![
                svg(SvgHandle::from_memory(include_bytes!("../icons/document-16-filled.svg").to_vec()))
                    .width(Length::Fixed(14.0)).height(Length::Fixed(14.0)).style(svg_style),
                text(name).size(14),
            ]
            .spacing(4)
            .align_y(Alignment::Center),
            container(text(format_size(obj.size)).size(12).color(p.text_secondary),)
                .width(Length::Fill),
            text(
                obj.last_modified
                    .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_default()
            )
            .size(12)
            .color(p.text_secondary),
            button(
                svg(SvgHandle::from_memory(include_bytes!("../icons/cloud-arrow-down-16-filled.svg").to_vec()))
                    .width(Length::Fixed(16.0)).height(Length::Fixed(16.0)).style(svg_style),
            )
            .style(icon_btn_style)
            .on_press(Message::DownloadObject(obj.key.clone())),
            button(
                svg(SvgHandle::from_memory(include_bytes!("../icons/delete-16-filled.svg").to_vec()))
                    .width(Length::Fixed(16.0)).height(Length::Fixed(16.0)).style(svg_style),
            )
            .style(icon_btn_style)
            .on_press(Message::DeleteObject(obj.key.clone())),
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        items.push(
            container(row_content)
                .padding(Padding::from([8, 16]))
                .style(|theme: &Theme| container::Style {
                    background: Some(iced::Background::Color(custom_palette(theme).surface)),
                    border: iced::Border::default().rounded(4),
                    ..Default::default()
                })
                .width(Length::Fill)
                .into(),
        );
    }

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

    container(
        column![
            breadcrumb,
            rule::horizontal(1),
            list,
        ]
        .spacing(10),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn view_status_bar(app: &App) -> Element<'_, Message> {
    let p = custom_palette(&app.theme);

    let status_text = if app.selected_connection_id.is_some() {
        let conn_name = app
            .config_store
            .list()
            .iter()
            .find(|c| Some(&c.id) == app.selected_connection_id.as_ref())
            .map(|c| c.name.as_str())
            .unwrap_or("?");
        let bucket_info = app
            .current_bucket
            .as_deref()
            .map(|b| format!(" | bucket: {}", b))
            .unwrap_or_default();
        let obj_count = if !app.objects.is_empty() {
            format!(" | {} {}", app.objects.len(), t!("status_objects"))
        } else if !app.buckets.is_empty() {
            format!(" | {} {}", app.buckets.len(), t!("status_buckets"))
        } else {
            String::new()
        };
        format!(
            "{}: {}{}{}",
            t!("status_connected"),
            conn_name,
            bucket_info,
            obj_count
        )
    } else {
        t!("status_ready").to_string()
    };

    row![text(status_text).size(11).color(p.text_secondary),]
        .padding(Padding::from([6, 16]))
        .align_y(Alignment::Center)
        .into()
}

fn view_settings(app: &App) -> Element<'_, Message> {
    let theme_names: Vec<String> = AVAILABLE_THEMES
        .iter()
        .map(|(n, _)| n.to_string())
        .collect();
    let lang_names: Vec<String> = LANGUAGES.iter().map(|(n, _)| n.to_string()).collect();
    let current_locale = rust_i18n::locale().to_string();
    let current_lang = LANGUAGES
        .iter()
        .find(|(_, code)| *code == current_locale)
        .map(|(name, _)| name.to_string())
        .unwrap_or_else(|| "English".to_string());

    let p = custom_palette(&app.theme);
    let btn_style = move |_: &Theme, s: button::Status| -> button::Style {
        let hbg = iced::Color::from_rgba(1.0, 1.0, 1.0, 0.08);
        let (bg, border) = match s {
            button::Status::Hovered | button::Status::Pressed => (
                Some(iced::Background::Color(hbg)),
                iced::Border { color: hbg, width: 1.0, radius: 4.0.into() },
            ),
            _ => (None, iced::Border::default().width(0)),
        };
        button::Style { background: bg, border, text_color: p.text_secondary, shadow: iced::Shadow::default(), ..Default::default() }
    };
    let svg_style = move |_: &Theme, _: svg::Status| svg::Style { color: Some(p.text_secondary) };
    let dismiss = svg(SvgHandle::from_memory(include_bytes!("../icons/dismiss-16-filled.svg").to_vec()))
        .width(Length::Fixed(16.0)).height(Length::Fixed(16.0)).style(svg_style);
    let panel = column![
        row![
            text(t!("settings").to_string()).size(20),
            container(button(dismiss).style(btn_style).on_press(Message::ToggleSettings))
                .width(Length::Fill)
                .align_x(Alignment::End),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
        rule::horizontal(1),
        text(t!("theme").to_string()).size(16),
        pick_list(
            theme_names,
            Some(app.current_theme_name.clone()),
            Message::ThemeChanged
        ),
        text(t!("language").to_string()).size(16),
        pick_list(lang_names, Some(current_lang), |name| {
            let code = LANGUAGES
                .iter()
                .find(|(n, _)| *n == name)
                .map(|(_, c)| c.to_string())
                .unwrap_or_else(|| "en".to_string());
            Message::LanguageChanged(code)
        },),
        text(t!("download_dir").to_string()).size(16),
        text_input(&t!("download_dir_hint").to_string(), &app.download_dir)
            .on_input(Message::DownloadDirChanged),
    ]
    .spacing(15)
    .padding(20);

    container(panel)
        .width(360)
        .style(|theme: &Theme| container::Style {
            background: Some(iced::Background::Color(
                custom_palette(theme).surface_raised,
            )),
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
