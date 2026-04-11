use iced::widget::{column, container, Space, Stack};
use iced::Element;

use super::chrome::unified_titlebar_padding;
use super::components::helpers::tokens_for_state;
use super::components;
use super::components::overlays;
use super::components::quick_connect;
use super::message::Message;
use super::settings_modal;
use super::state::IcedState;
use super::terminal_viewport;

pub(crate) fn view(state: &IcedState) -> Element<'_, Message> {
    let tick_ms = state.tick_ms();
    let top_bar = if state.show_welcome {
        components::top_bar::title_bar(state, tick_ms)
    } else {
        components::top_bar::top_bar(state, tick_ms)
    };

    // Welcome page: no terminal, just centered welcome content
    if state.show_welcome {
        let welcome_content = components::welcome::welcome_view(state);
        let welcome_with_overlays: Element<'_, Message> = {
            let mut layers: Vec<Element<'_, Message>> = vec![welcome_content];
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
            layers.push(components::modals::vault_unlock_modal(state));
            Stack::with_children(layers)
                .width(iced::Length::Fill)
                .height(iced::Length::Fill)
                .into()
        };
        let tokens = tokens_for_state(state);
        let main_chrome = crate::app::chrome::main_chrome_style(tokens);
        let chrome = column![top_bar, container(welcome_with_overlays)
            .width(iced::Length::Fill)
            .height(iced::Length::Fill)
            .style(main_chrome)]
        .height(iced::Length::Fill);
        return container(chrome)
            .padding(unified_titlebar_padding())
            .width(iced::Length::Fill)
            .height(iced::Length::Fill)
            .into();
    }

    // 防御：欢迎页关闭但 tabs 尚未创建时（如连接消息到达但页签还在创建中），显示占位
    if !state.show_welcome && state.tab_panes.is_empty() {
        let tokens = tokens_for_state(state);
        let main_chrome = crate::app::chrome::main_chrome_style(tokens);
        return container(
            column![top_bar, container(Space::new().width(iced::Length::Fill).height(iced::Length::Fill))
                .width(iced::Length::Fill)
                .height(iced::Length::Fill)
                .style(main_chrome)]
            .height(iced::Length::Fill),
        )
        .padding(unified_titlebar_padding())
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .into();
    }

    let mut term_vp =
        terminal_viewport::terminal_viewport_spec_for_settings(&state.model.settings.terminal);

    // 根据 breadcrumb 状态更新 viewport spec
    let breadcrumb_visible = state.breadcrumb_pinned || state.breadcrumb_temp_visible;
    term_vp.breadcrumb_visible = breadcrumb_visible;

    // 根据 breadcrumb_pinned 和 breadcrumb_temp_visible 决定是否显示 breadcrumb
    let breadcrumb_visible = state.breadcrumb_pinned || state.breadcrumb_temp_visible;
    let breadcrumb = if breadcrumb_visible {
        Some(components::breadcrumb::breadcrumb(state))
    } else {
        None
    };

    let bottom_bar = components::status_bar::status_bar(state);
    let terminal_panel = components::terminal_view::terminal_panel(state);

    // 构建主体内容区域（breadcrumb + 终端，终端弹性填满中间空间）
    let below_top_fill: Element<'_, Message> = if let Some(bc) = breadcrumb {
        column![bc, terminal_panel]
            .spacing(term_vp.main_column_spacing())
            .height(iced::Length::Fill)
            .into()
    } else {
        terminal_panel.into()
    };

    // 底栏固定在底部，主体区域填满剩余空间
    let content_with_bottom: Element<'_, Message> = column![below_top_fill, bottom_bar]
        .spacing(0)
        .height(iced::Length::Fill)
        .into();

    let under_top_bar: Element<'_, Message> = {
        let mut layers: Vec<Element<'_, Message>> = vec![content_with_bottom.into()];

        // Terminal-inline overlay: connection progress bar (shown while modal is closed).
        layers.push(overlays::inline_connecting_overlay(state));
        // Terminal-inline overlay: password/passphrase input (modal closed, need credential).
        layers.push(overlays::inline_password_overlay(state));

        // 浮动 breadcrumb 图标（仅在 breadcrumb 未固定且未临时显示时显示）
        if !state.breadcrumb_pinned && !state.breadcrumb_temp_visible {
            layers.push(components::breadcrumb::breadcrumb_float_icon(state));
        }

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

    let tokens = tokens_for_state(state);
    let main_chrome = crate::app::chrome::main_chrome_style(tokens);
    let terminal_region = container(under_top_bar)
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .style(main_chrome);

    let chrome = column![top_bar, terminal_region].height(iced::Length::Fill);

    container(chrome)
        .padding(unified_titlebar_padding())
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .into()
}
