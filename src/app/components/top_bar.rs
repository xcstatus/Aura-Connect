use iced::alignment::Alignment;
use iced::widget::Id;
use iced::widget::scrollable::{Direction as ScrollDirection, Scrollbar};
use iced::widget::{button, column, container, mouse_area, row, scrollable, text, Space, tooltip};
use iced::{Element, Theme};

use crate::app::chrome::{
    TAB_STRIP_SCROLLABLE_ID, TOP_BAR_EDGE_PAD, TOP_BAR_H, TOP_CONTROL_GROUP_W, TOP_ICON_BTN,
    main_chrome_style, top_bar_material_style, top_bar_vertical_rule,
};
use crate::app::message::Message;
use crate::app::state::IcedState;
use crate::app::widgets::chrome_button::{style_tab_strip, style_top_icon};

/// Build the top bar (tab strip + action buttons).
pub fn top_bar(state: &IcedState, tick_ms: f32) -> Element<'_, Message> {
    let tabs_row = build_tab_strip(state, tick_ms);
    let action_group = build_action_group(state);
    let control_group = build_control_group(state);

    let tab_scroll_core = build_tab_scroll_area(state, tabs_row);
    let tab_scroll_host = mouse_area(tab_scroll_core).on_scroll(Message::TabStripWheel);

    let scroll_control_gutter = container(Space::new().height(iced::Length::Fixed(TOP_BAR_H)))
        .width(crate::app::chrome::SCROLL_TO_CONTROL_GUTTER_W)
        .style(top_bar_material_style);

    let mut top_bar_row = row![].spacing(0).align_y(Alignment::Center);
    #[cfg(target_os = "macos")]
    {
        top_bar_row = top_bar_row.push(
            container(Space::new().height(iced::Length::Fixed(TOP_BAR_H)))
                .width(crate::app::chrome::TRAFFIC_LIGHT_BAND_W)
                .style(top_bar_material_style),
        );
    }
    top_bar_row = top_bar_row
        .push(tab_scroll_host)
        .push(action_group)
        .push(scroll_control_gutter)
        .push(control_group);

    container(top_bar_row)
        .height(iced::Length::Fixed(TOP_BAR_H))
        .padding(iced::Padding::from([0.0, TOP_BAR_EDGE_PAD]))
        .into()
}

fn build_tab_strip(state: &IcedState, tick_ms: f32) -> Element<'_, Message> {
    let mut tabs_row = row![].spacing(4).align_y(Alignment::Center);
    for (i, tab) in state.tabs.iter().enumerate() {
        let tab_label = tab.title.clone();
        let tab_w = state.tab_animated_width(i, tick_ms);
        let select_btn = button(text(tab_label).size(11))
            .on_press(Message::TabSelected(i))
            .width(iced::Length::Fill)
            .style(style_tab_strip(11.0));
        let close_slot: Element<'_, Message> = if state.tab_hover_index == Some(i) {
            button(text("×").size(12))
                .on_press(Message::TabClose(i))
                .width(iced::Length::Fixed(22.0))
                .style(style_tab_strip(12.0))
                .into()
        } else {
            Space::new()
                .width(iced::Length::Fixed(22.0))
                .height(iced::Length::Fixed(1.0))
                .into()
        };
        let body_h = if i == state.active_tab { TOP_BAR_H - 2.0 } else { TOP_BAR_H };
        let top_line = container(
            Space::new()
                .width(iced::Length::Fill)
                .height(iced::Length::Fixed(if i == state.active_tab { 2.0 } else { 0.0 })),
        )
        .style(move |theme: &Theme| {
            if i == state.active_tab {
                let c = theme.extended_palette().primary.base.color;
                container::Style::default().background(c)
            } else {
                container::Style::default()
            }
        });
        let chip = mouse_area(
            container(
                column![
                    top_line,
                    container(row![select_btn, close_slot].spacing(0).align_y(Alignment::Center))
                        .width(iced::Length::Fixed(tab_w))
                        .height(iced::Length::Fixed(body_h))
                        .align_y(Alignment::Center),
                ]
                .spacing(0),
            )
            .padding([0, 4])
            .width(iced::Length::Fixed(tab_w + 8.0))
            .height(iced::Length::Fixed(TOP_BAR_H)),
        )
        .on_enter(Message::TabChipHover(Some(i)))
        .on_exit(Message::TabChipHover(None));
        tabs_row = tabs_row.push(chip);
    }
    tabs_row.into()
}

fn build_action_group(state: &IcedState) -> Element<'_, Message> {
    let i18n = &state.model.i18n;
    let btn_quick = button(text("⚡").size(14))
        .on_press(Message::TopQuickConnect)
        .width(iced::Length::Fixed(TOP_ICON_BTN))
        .height(iced::Length::Fixed(TOP_ICON_BTN))
        .style(style_top_icon(TOP_ICON_BTN));
    let btn_new = button(text("+").size(18))
        .on_press(Message::TopAddTab)
        .width(iced::Length::Fixed(TOP_ICON_BTN))
        .height(iced::Length::Fixed(TOP_ICON_BTN))
        .style(style_top_icon(TOP_ICON_BTN));
    let quick_tip = text(i18n.tr("iced.topbar.quick_connect")).size(12);
    let new_tip = text(i18n.tr("iced.topbar.new_tab")).size(12);

    container(
        row![
            tooltip(btn_quick, quick_tip, iced::widget::tooltip::Position::Bottom),
            tooltip(btn_new, new_tip, iced::widget::tooltip::Position::Bottom),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .height(iced::Length::Fixed(TOP_BAR_H))
    .padding([0, 8])
    .style(top_bar_material_style)
    .align_y(Alignment::Center)
    .into()
}

fn build_tab_scroll_area<'a>(state: &'a IcedState, tabs_row: Element<'a, Message>) -> Element<'a, Message> {
    let tabs_only_scroll = scrollable(
        row![tabs_row, top_bar_vertical_rule()].spacing(0).align_y(Alignment::Center),
    )
    .id(Id::new(TAB_STRIP_SCROLLABLE_ID))
    .direction(ScrollDirection::Horizontal(Scrollbar::hidden()))
    .height(iced::Length::Fixed(TOP_BAR_H))
    .width(iced::Length::Fill);

    container(tabs_only_scroll)
        .width(iced::Length::Fill)
        .height(iced::Length::Fixed(TOP_BAR_H))
        .style(main_chrome_style)
        .into()
}

fn build_control_group(state: &IcedState) -> Element<'_, Message> {
    let i18n = &state.model.i18n;
    let btn_settings = button(text("⚙").size(15))
        .on_press(Message::TopOpenSettings)
        .width(iced::Length::Fixed(TOP_ICON_BTN))
        .height(iced::Length::Fixed(TOP_ICON_BTN))
        .style(style_top_icon(TOP_ICON_BTN));
    let settings_tip = text(i18n.tr("iced.topbar.settings_center")).size(12);
    let settings_ctrl = tooltip(btn_settings, settings_tip, iced::widget::tooltip::Position::Bottom);

    let win_controls: Element<'_, Message> = {
        #[cfg(not(target_os = "macos"))]
        {
            row![
                button(text("—").size(12))
                    .on_press(Message::WinMinimize)
                    .width(iced::Length::Fixed(28.0))
                    .height(iced::Length::Fixed(26.0))
                    .style(style_top_icon(12.0)),
                button(text("□").size(11))
                    .on_press(Message::WinToggleMaximize)
                    .width(iced::Length::Fixed(28.0))
                    .height(iced::Length::Fixed(26.0))
                    .style(style_top_icon(11.0)),
                button(text("×").size(12))
                    .on_press(Message::WinClose)
                    .width(iced::Length::Fixed(28.0))
                    .height(iced::Length::Fixed(26.0))
                    .style(style_top_icon(12.0)),
            ]
            .spacing(2)
            .align_y(Alignment::Center)
            .into()
        }
        #[cfg(target_os = "macos")]
        {
            Space::new().into()
        }
    };

    container(
        row![Space::new().width(iced::Length::Fill), settings_ctrl, win_controls]
            .spacing(4)
            .align_y(Alignment::Center),
    )
    .width(iced::Length::Fixed(TOP_CONTROL_GROUP_W))
    .height(iced::Length::Fixed(TOP_BAR_H))
    .padding([0, 0])
    .style(top_bar_material_style)
    .into()
}
