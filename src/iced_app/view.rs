use iced::widget::{Stack, column, container};
use iced::Element;

use super::chrome::{main_chrome_style, unified_titlebar_padding};
use super::components;
use super::components::overlays;
use super::components::quick_connect;
use super::message::Message;
use super::settings_modal;
use super::state::IcedState;
use super::terminal_viewport;

pub(crate) fn view(state: &IcedState) -> Element<'_, Message> {
    let term_vp =
        terminal_viewport::terminal_viewport_spec_for_settings(&state.model.settings.terminal);

    let tick_ms = state.tick_ms();

    let top_bar = components::top_bar::top_bar(state, tick_ms);

    let breadcrumb = components::breadcrumb::breadcrumb(state);

    let terminal_panel = components::terminal_view::terminal_panel(state);

    let main_body: Element<'_, Message> = terminal_panel;

    let bottom_bar = components::status_bar::status_bar(state);

    // 顶栏 32px | 终端区（面包屑 + 主内容 + 底栏，快速连接浮层叠在终端区内）
    let below_top_fill = column![breadcrumb, main_body, bottom_bar,]
        .spacing(term_vp.main_column_spacing())
        .height(iced::Length::Fill);

    let under_top_bar: Element<'_, Message> = {
        let mut layers: Vec<Element<'_, Message>> = vec![below_top_fill.into()];

        // Terminal-inline overlay: connection progress bar (shown while modal is closed).
        layers.push(overlays::inline_connecting_overlay(state));
        // Terminal-inline overlay: password/passphrase input (modal closed, need credential).
        layers.push(overlays::inline_password_overlay(state));

        // Render quick connect modal when anim phase is opening/open/closing
        // (not yet fully closed). This allows the close animation to play.
        if state.quick_connect_anim.phase != super::state::ModalAnimPhase::Closed {
            layers.push(quick_connect::quick_connect_modal_stack(state));
        }
        if state.settings_anim.phase != super::state::ModalAnimPhase::Closed {
            layers.push(settings_modal::modal_stack(state));
            layers.push(components::modals::session_editor_modal(state));
            layers.push(components::modals::vault_modal(state));
            layers.push(components::modals::auto_probe_consent_modal(state));
        }
        layers.push(components::modals::host_key_prompt_modal(state));
        // Vault unlock modal must be above all other modals.
        // (e.g. connect-from-saved requiring vault unlock, or post-connect save-credential prompt)
        layers.push(components::modals::vault_unlock_modal(state));
        // Debug overlay: toggled with Ctrl+Shift+D; renders on top of everything.
        if state.perf.debug_overlay_enabled {
            layers.push(super::widgets::debug_overlay::make_debug_overlay(state));
        }
        Stack::with_children(layers)
            .width(iced::Length::Fill)
            .height(iced::Length::Fill)
            .into()
    };

    let terminal_region = container(under_top_bar)
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .style(main_chrome_style);

    let chrome = column![top_bar, terminal_region].height(iced::Length::Fill);

    container(chrome)
        .padding(unified_titlebar_padding())
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .into()
}
