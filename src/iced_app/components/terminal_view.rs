use iced::Element;
use iced::widget::{column, container};

use crate::iced_app::engine_adapter::EngineAdapter;
use crate::iced_app::message::Message;
use crate::iced_app::state::IcedState;
use crate::iced_app::terminal_widget;
use crate::iced_app::terminal_viewport;

/// Build the terminal panel (main content area).
pub fn terminal_panel(state: &IcedState) -> Element<'_, Message> {
    let term_vp = terminal_viewport::terminal_viewport_spec_for_settings(&state.model.settings.terminal);
    let cell_w_hit = terminal_viewport::terminal_scroll_cell_geometry(
        state.window_size,
        &term_vp,
        EngineAdapter::active(state).grid_size().0,
    )
    .1;
    let tick_count = state.tick_count;
    let term_font_px = term_vp.term_font_px;
    let term_cell_h = iced::Pixels(term_vp.term_cell_h().max(1.0));
    let term_font = terminal_widget::iced_terminal_font(&state.model.settings.terminal);

    let selection = EngineAdapter::active(state).selection();
    let terminal = &*state.active_terminal();
    let cache = &state.tab_panes[state.active_tab].styled_row_cache;

    container(
        column![container({
            terminal_widget::styled_terminal(
                terminal,
                cache,
                selection,
                term_font_px,
                term_cell_h,
                term_font,
                cell_w_hit,
                tick_count,
            )
        })
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .style(container::bordered_box)]
        .spacing(term_vp.terminal_panel_inner_spacing())
        .height(iced::Length::Fill),
    )
    .padding(term_vp.terminal_panel_padding())
    .height(iced::Length::Fill)
    .into()
}
