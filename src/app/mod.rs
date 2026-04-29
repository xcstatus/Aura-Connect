mod chrome;
mod connection;
mod engine_adapter;
mod message;
mod model;
mod settings_modal;
mod ssh_session_manager;
mod state;
mod subscription;
mod terminal_event;
mod terminal_host;
mod terminal_viewport;
mod terminal_widget;
mod update;
mod view;
mod widgets;

pub mod components;

pub use connection::{DirectInputParts, is_direct_candidate, parse_direct_input};
pub use model::{AppModel, ConnectErrorKind, ConnectionDraft, DraftSource, HostKeyErrorInfo};

fn app_theme(state: &state::IcedState) -> iced::Theme {
    crate::theme::rustssh_iced_theme(&state.model.settings.color_scheme)
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
        window_settings.transparent = false;
        window_settings.blur = false;
    }
    #[cfg(not(target_os = "macos"))]
    {
        window_settings.transparent = true;
    }

    iced::application(boot, update, view)
        .title(title)
        .subscription(subscription)
        .theme(app_theme)
        .style(chrome::app_chrome_style)
        .window(window_settings)
        .run()
}
