mod chrome;
mod engine_adapter;
mod message;
mod session_manager;
mod settings_modal;
mod state;
mod subscription;
mod terminal_event;
mod terminal_host;
mod terminal_rich;
mod terminal_viewport;
mod update;
mod view;
mod widgets;

fn app_theme(_: &state::IcedState) -> iced::Theme {
    crate::theme::default_rustssh_iced_theme()
}

pub fn run() -> iced::Result {
    use state::boot;
    use subscription::{subscription, title};
    use update::update;
    use view::view;

    let mut window_settings = iced::window::Settings::default();
    #[cfg(target_os = "macos")]
    {
        window_settings.platform_specific.titlebar_transparent = true;
        window_settings.platform_specific.fullsize_content_view = true;
        window_settings.platform_specific.title_hidden = true;
        window_settings.transparent = true;
        window_settings.blur = true;
    }

    iced::application(boot, update, view)
        .title(title)
        .subscription(subscription)
        .theme(app_theme)
        .style(chrome::app_chrome_style)
        .window(window_settings)
        .run()
}
