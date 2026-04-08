mod chrome;
mod connection;
mod engine_adapter;
mod message;
mod model;
mod ssh_session_manager;
mod settings_modal;
mod state;
mod subscription;
mod terminal_event;
mod terminal_host;
mod terminal_widget;
mod terminal_viewport;
mod update;
mod view;
mod widgets;

pub mod components;

pub use connection::{DirectInputParts, is_direct_candidate, parse_direct_input};
pub use model::{AppModel, ConnectErrorKind, ConnectionDraft, DraftSource, HostKeyErrorInfo};

fn app_theme(state: &state::IcedState) -> iced::Theme {
    use crate::theme::RustSshThemeId;
    let theme_id = match state.model.settings.general.theme.as_str() {
        "Light" => RustSshThemeId::Light,
        "Warm" => RustSshThemeId::Warm,
        "GitHub" => RustSshThemeId::GitHub,
        _ => RustSshThemeId::Dark,
    };
    crate::theme::rustssh_iced_theme(theme_id)
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
