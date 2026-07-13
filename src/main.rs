fn main() -> iced::Result {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("Starting S3 Desktop Manager");

    iced::application(s3dm_gui::boot, s3dm_gui::update, s3dm_gui::view)
        .theme(iced::Theme::Dark)
        .centered()
        .run()
}
