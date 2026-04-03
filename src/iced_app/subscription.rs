use std::time::Duration;

use iced::Subscription;

use super::message::Message;
use super::state::IcedState;

pub(crate) fn subscription(state: &IcedState) -> Subscription<Message> {
    let escape_overlay = if state.settings_modal_open {
        iced::keyboard::listen().filter_map(|ev| {
            if let iced::keyboard::Event::KeyPressed { key, .. } = ev {
                if matches!(
                    key,
                    iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape)
                ) {
                    return Some(Message::SettingsDismiss);
                }
            }
            None
        })
    } else if state.quick_connect_open {
        iced::keyboard::listen().filter_map(|ev| {
            if let iced::keyboard::Event::KeyPressed { key, .. } = ev {
                if matches!(
                    key,
                    iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape)
                ) {
                    return Some(Message::QuickConnectDismiss);
                }
            }
            None
        })
    } else {
        Subscription::none()
    };

    // Dynamic tick:
    // - Active (recent I/O or input): 16ms
    // - Idle: 150ms (background pump cadence)
    // - Unfocused: 250ms
    let now = crate::settings::unix_time_ms();
    let idle_ms = now.saturating_sub(state.last_activity_ms).max(0) as u64;
    let tick_ms = if !state.window_focused {
        250
    } else if idle_ms <= 1_000 {
        16
    } else if idle_ms <= 5_000 {
        100
    } else {
        200
    };

    Subscription::batch([
        iced::time::every(Duration::from_millis(tick_ms)).map(|_| Message::Tick),
        iced::window::resize_events().map(|(_id, size)| Message::WindowResized(size)),
        iced::event::listen().map(Message::EventOccurred),
        escape_overlay,
    ])
}

pub(crate) fn title(state: &IcedState) -> String {
    format!(
        "{} — {}",
        state.model.i18n.tr("iced.title.brand"),
        state.model.i18n.tr("iced.tab.terminal")
    )
}
