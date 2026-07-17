fn main() -> iced::Result {
    let project_level = if cfg!(debug_assertions) { "debug" } else { "info" };
    let filter = format!("warn,s3dm={},s3dm_config={},s3dm_core={},s3dm_gui={}", project_level, project_level, project_level, project_level);

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(&filter)).init();

    log::info!("Starting S3 Desktop Manager");

    let window_icon = iced::window::icon::from_file_data(s3dm_gui::icon::WINDOW_ICON, None).ok();

    iced::application(s3dm_gui::boot, s3dm_gui::update, s3dm_gui::view)
        .theme(|app: &s3dm_gui::App| app.theme.clone())
        .window(iced::window::Settings {
            icon: window_icon,
            ..Default::default()
        })
        .centered()
        .run()
}
