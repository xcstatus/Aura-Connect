//! 设置中心弹窗（布局与 egui `settings/mod.rs` 对齐：左侧大类 + 右侧子页签 + 滚动内容）。

use std::collections::BTreeMap;

use iced::alignment::Alignment;
use iced::widget::{
    button, checkbox, column, container, mouse_area, pick_list, radio, row, scrollable, slider, text,
    text_input, Space, Stack,
};
use iced::widget::scrollable::{Direction as ScrollDirection, Scrollbar};
use iced::{Element, Theme};

use crate::session::{ProtocolType, SessionProfile, TransportConfig};
use crate::settings::{HostKeyPolicy, TerminalPlainTextUpdate, TerminalRenderMode};

use super::message::{Message, SettingsCategory, SettingsField};
use super::state::IcedState;
use super::widgets::chrome_button::{style_chrome_primary, style_chrome_secondary, style_top_icon};

pub(crate) fn max_sub_tab(category: SettingsCategory) -> usize {
    match category {
        SettingsCategory::General => 2,
        SettingsCategory::Terminal => 3,
        SettingsCategory::Connection => 3,
        SettingsCategory::Security => 1,
        SettingsCategory::Backup => 0,
    }
}

pub(crate) fn clamp_sub_tab(category: SettingsCategory, sub: usize) -> usize {
    sub.min(max_sub_tab(category))
}

pub(crate) fn modal_stack(state: &IcedState) -> Element<'_, Message> {
    let scrim = mouse_area(
        container(Space::new().width(iced::Length::Fill).height(iced::Length::Fill))
            .style(|_t: &Theme| {
                container::Style::default().background(iced::Color::from_rgba(0.0, 0.0, 0.0, 0.42))
            }),
    )
    .on_press(Message::SettingsDismiss);

    let mw = (state.window_size.width * 0.9).min(1000.0).max(480.0);
    let mh = (state.window_size.height * 0.85).min(800.0).max(400.0);

    let panel = container(modal_card(state))
        .width(iced::Length::Fixed(mw))
        .height(iced::Length::Fixed(mh));

    let centered = container(panel)
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center);

    Stack::with_children([scrim.into(), centered.into()])
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .into()
}

fn modal_card(state: &IcedState) -> Element<'_, Message> {
    let i18n = &state.model.i18n;
    let sidebar = settings_sidebar(state);
    let body = column![
        settings_header_row(i18n),
        settings_sub_tab_row(state),
        scrollable(settings_main_content(state))
            .direction(ScrollDirection::Vertical(Scrollbar::default()))
            .width(iced::Length::Fill)
            .height(iced::Length::Fill),
        restart_banner(state),
    ]
    .spacing(0)
    .width(iced::Length::Fill)
    .height(iced::Length::Fill);

    let row_content = row![sidebar, container(body).width(iced::Length::Fill).padding(0)]
        .spacing(0)
        .width(iced::Length::Fill)
        .height(iced::Length::Fill);

    container(row_content)
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .style(modal_frame_style)
        .into()
}

fn modal_frame_style(theme: &Theme) -> container::Style {
    let base = theme.extended_palette().background.base.color;
    let border = iced::Border {
        width: 1.0,
        color: theme.extended_palette().background.strong.color,
        radius: 12.0.into(),
    };
    container::Style::default().background(base).border(border)
}

fn sidebar_item_style(active: bool, theme: &Theme) -> container::Style {
    let t = theme.extended_palette();
    if active {
        container::Style::default().background(t.background.strong.color)
    } else {
        container::Style::default()
    }
}

fn settings_sidebar(state: &IcedState) -> Element<'_, Message> {
    let i18n = &state.model.i18n;
    let entries: [(SettingsCategory, &'static str); 5] = [
        (SettingsCategory::General, "iced.settings.cat.general"),
        (SettingsCategory::Terminal, "iced.settings.cat.terminal"),
        (SettingsCategory::Connection, "iced.settings.cat.connection"),
        (SettingsCategory::Security, "iced.settings.cat.security"),
        (SettingsCategory::Backup, "iced.settings.cat.backup"),
    ];
    let mut col = column![].spacing(4).width(iced::Length::Fixed(220.0)).padding(12);
    col = col.push(Space::new().height(iced::Length::Fixed(24.0)));
    for (cat, key) in entries {
        let label = i18n.tr(key).to_string();
        let active = state.settings_category == cat;
        let cell = button(
            container(text(label).size(13))
                .width(iced::Length::Fill)
                .padding([10, 16]),
        )
        .on_press(Message::SettingsCategoryChanged(cat))
        .width(iced::Length::Fill)
        .style(style_chrome_secondary(13.0));
        col = col.push(
            container(if active { cell } else { cell })
                .style(move |theme: &Theme| sidebar_item_style(active, theme)),
        );
    }
    container(col)
        .width(iced::Length::Fixed(220.0))
        .height(iced::Length::Fill)
        .style(|theme: &Theme| {
            let t = theme.extended_palette();
            container::Style::default().background(t.background.strong.color)
        })
        .into()
}

fn settings_header_row(i18n: &crate::i18n::I18n) -> Element<'_, Message> {
    container(
        row![
            text(i18n.tr("iced.settings.title")).size(16),
            Space::new().width(iced::Length::Fill),
            button(text("×").size(14))
                .on_press(Message::SettingsDismiss)
                .width(iced::Length::Fixed(28.0))
                .height(iced::Length::Fixed(28.0))
                .style(style_top_icon(14.0)),
        ]
        .align_y(Alignment::Center),
    )
    .width(iced::Length::Fill)
    .height(iced::Length::Fixed(36.0))
    .padding([4, 12])
    .into()
}

fn sub_tab_labels(category: SettingsCategory, i18n: &crate::i18n::I18n) -> Vec<String> {
    let keys: &[&str] = match category {
        SettingsCategory::General => &[
            "iced.settings.sub.general.basic",
            "iced.settings.sub.general.appearance",
            "iced.settings.sub.general.typography",
        ],
        SettingsCategory::Terminal => &[
            "iced.settings.sub.terminal.render",
            "iced.settings.sub.terminal.scheme",
            "iced.settings.sub.terminal.text",
            "iced.settings.sub.terminal.interaction",
        ],
        SettingsCategory::Connection => &[
            "iced.settings.sub.conn.ssh",
            "iced.settings.sub.conn.telnet",
            "iced.settings.sub.conn.serial",
            "iced.settings.sub.conn.advanced",
        ],
        SettingsCategory::Security => &[
            "iced.settings.sub.security.policy",
            "iced.settings.sub.security.hosts",
        ],
        SettingsCategory::Backup => &["iced.settings.sub.backup.main"],
    };
    keys.iter().map(|k| i18n.tr(k).to_string()).collect()
}

fn settings_sub_tab_row(state: &IcedState) -> Element<'_, Message> {
    let i18n = &state.model.i18n;
    let labels = sub_tab_labels(state.settings_category, i18n);
    let cat_i = state.settings_category as usize;
    let current = state.settings_sub_tab[cat_i].min(max_sub_tab(state.settings_category));

    let mut r = row![].spacing(16).align_y(Alignment::Center);
    for (idx, lab) in labels.iter().enumerate() {
        let active = idx == current;
        let lab_clone = lab.clone();
        let btn = button(text(lab_clone).size(13))
            .on_press(Message::SettingsSubTabChanged(idx))
            .style(move |theme: &Theme, status: iced::widget::button::Status| {
                if active {
                    style_chrome_primary(13.0)(theme, status)
                } else {
                    style_chrome_secondary(13.0)(theme, status)
                }
            });
        r = r.push(btn);
    }
    container(r)
        .width(iced::Length::Fill)
        .padding([8, 20])
        .style(|theme: &Theme| {
            container::Style::default()
                .border(iced::Border {
                    width: 0.0,
                    color: theme.extended_palette().background.strong.color,
                    radius: 0.0.into(),
                })
        })
        .into()
}

fn settings_main_content(state: &IcedState) -> Element<'_, Message> {
    let cat = state.settings_category;
    let sub = state.settings_sub_tab[cat as usize];
    let sub = clamp_sub_tab(cat, sub);
    match cat {
        SettingsCategory::General => general_pane(state, sub),
        SettingsCategory::Terminal => terminal_pane(state, sub),
        SettingsCategory::Connection => connection_pane(state, sub),
        SettingsCategory::Security => security_pane(state, sub),
        SettingsCategory::Backup => backup_pane(state),
    }
}

fn section_title(s: &str) -> Element<'_, Message> {
    container(text(s).size(18))
        .padding(iced::Padding {
            top: 8.0,
            right: 0.0,
            bottom: 16.0,
            left: 0.0,
        })
        .into()
}

fn section_title_owned(s: String) -> Element<'static, Message> {
    container(text(s).size(18))
        .padding(iced::Padding {
            top: 8.0,
            right: 0.0,
            bottom: 16.0,
            left: 0.0,
        })
        .into()
}

fn general_pane(state: &IcedState, sub: usize) -> Element<'_, Message> {
    let i18n = &state.model.i18n;
    let g = &state.model.settings.general;
    match sub {
        0 => column![
            section_title(i18n.tr("iced.settings.section.startup")),
            settings_row(
                i18n.tr("iced.settings.row.language"),
                radio(
                    i18n.tr("iced.settings.language.zh"),
                    "zh-CN",
                    (g.language == "zh-CN").then_some("zh-CN"),
                    |c| Message::SettingsFieldChanged(SettingsField::Language(c.to_string())),
                ),
            ),
            settings_row(
                "",
                radio(
                    i18n.tr("iced.settings.language.en"),
                    "en-US",
                    (g.language == "en-US").then_some("en-US"),
                    |c| Message::SettingsFieldChanged(SettingsField::Language(c.to_string())),
                ),
            ),
            checkbox(g.auto_check_update)
                .label(i18n.tr("iced.settings.row.auto_update"))
                .on_toggle(|v| Message::SettingsFieldChanged(SettingsField::AutoCheckUpdate(v))),
            text(i18n.tr("iced.settings.language.help")).size(12),
        ]
        .spacing(12)
        .padding(20)
        .width(iced::Length::Fill)
        .into(),
        1 => {
            const THEMES: [&str; 3] = ["Dark", "Light", "Warm"];
            let theme_sel = THEMES
                .iter()
                .find(|&&x| x == g.theme.as_str())
                .map(|_| g.theme.as_str());
            column![
                section_title(i18n.tr("iced.settings.section.appearance")),
                settings_row(
                    i18n.tr("iced.settings.row.theme"),
                    pick_list(
                        THEMES.as_slice(),
                        theme_sel,
                        |t: &str| Message::SettingsFieldChanged(SettingsField::Theme(t.to_string())),
                    )
                    .width(200.0),
                ),
                settings_row(
                    i18n.tr("iced.settings.row.accent"),
                    text_input("#RRGGBB", &g.accent_color)
                        .on_input(|s| Message::SettingsFieldChanged(SettingsField::AccentColor(s)))
                        .width(240.0),
                ),
            ]
            .spacing(12)
            .padding(20)
            .width(iced::Length::Fill)
            .into()
        }
        2 => column![
            section_title(i18n.tr("iced.settings.section.typography")),
            settings_row(
                i18n.tr("iced.settings.row.ui_font_size"),
                text(format!("{:.0}px", g.font_size)).size(12),
            ),
            slider(12.0..=20.0, g.font_size, |v| {
                Message::SettingsFieldChanged(SettingsField::FontSize(v))
            })
            .width(300.0),
        ]
        .spacing(12)
        .padding(20)
        .width(iced::Length::Fill)
        .into(),
        _ => Space::new().into(),
    }
}

fn terminal_pane(state: &IcedState, sub: usize) -> Element<'_, Message> {
    let i18n = &state.model.i18n;
    let t = &state.model.settings.terminal;
    match sub {
        0 => column![
            section_title(i18n.tr("iced.settings.section.render_engine")),
            checkbox(t.gpu_acceleration)
                .label(i18n.tr("iced.settings.row.gpu_accel"))
                .on_toggle(|v| Message::SettingsFieldChanged(SettingsField::GpuAcceleration(v))),
            settings_row(
                i18n.tr("iced.settings.row.target_fps"),
                text(format!("{}", t.target_fps)).size(12),
            ),
            slider(10u32..=240u32, t.target_fps, |v| {
                Message::SettingsFieldChanged(SettingsField::TargetFps(v))
            })
            .width(300.0),
            text(i18n.tr("iced.settings.section.stability")).size(14),
            checkbox(t.atlas_reset_on_pressure)
                .label(i18n.tr("iced.settings.row.atlas_reset"))
                .on_toggle(|v| {
                    Message::SettingsFieldChanged(SettingsField::AtlasResetOnPressure(v))
                }),
        ]
        .spacing(12)
        .padding(20)
        .width(iced::Length::Fill)
        .into(),
        1 => {
            let schemes = [
                ("Default", "Original Dark Scheme"),
                ("Nord", "Arctic bluish Nord palette"),
                ("Monokai", "Classic bright colors on grey"),
                ("Solarized", "Precision colors for digital clarity"),
            ];
            let mut col = column![section_title(i18n.tr("iced.settings.section.color_scheme"))].spacing(12);
            for (name, desc) in schemes {
                let active = t.color_scheme == name;
                col = col.push(
                    row![
                        column![
                            text(name).size(13),
                            text(desc).size(11).style(|theme: &Theme| text::Style {
                                color: Some(
                                    theme.extended_palette().background.base.text.scale_alpha(0.65),
                                ),
                            }),
                        ]
                        .spacing(4),
                        Space::new().width(iced::Length::Fill),
                        button(text(if active {
                            i18n.tr("iced.settings.scheme.applied")
                        } else {
                            i18n.tr("iced.settings.scheme.apply")
                        }))
                        .on_press(Message::SettingsFieldChanged(SettingsField::ColorScheme(
                            name.to_string(),
                        )))
                        .style(style_chrome_secondary(12.0)),
                    ]
                    .align_y(Alignment::Center),
                );
            }
            col.spacing(16).padding(20).width(iced::Length::Fill).into()
        }
        2 => {
            const FONTS: [&str; 3] = ["JetBrains Mono", "SF Mono", "Cascadia Code"];
            let font_sel = FONTS
                .iter()
                .find(|&&x| x == t.font_family.as_str())
                .map(|_| t.font_family.as_str());
            let gpu_path = t
                .gpu_font_path
                .clone()
                .unwrap_or_default();
            let face_raw = t
                .gpu_font_face_index
                .map(|n| n.to_string())
                .unwrap_or_default();
            let mode = t.terminal_render_mode;
            let plain_upd = t.plain_text_update;
            column![
                section_title(i18n.tr("iced.settings.section.text_render")),
                checkbox(t.apply_terminal_metrics)
                    .label(i18n.tr("iced.settings.row.apply_terminal_metrics"))
                    .on_toggle(|v| {
                        Message::SettingsFieldChanged(SettingsField::ApplyTerminalMetrics(v))
                    }),
                settings_row(
                    i18n.tr("iced.settings.row.terminal_font_size"),
                    text(format!("{:.0}", t.font_size)).size(12),
                ),
                slider(8.0..=36.0, t.font_size, |v| {
                    Message::SettingsFieldChanged(SettingsField::TerminalFontSize(v))
                })
                .width(300.0),
                settings_row(
                    i18n.tr("iced.settings.row.line_height"),
                    text(format!("{:.2}", t.line_height)).size(12),
                ),
                slider(1.0..=2.0, t.line_height, |v| {
                    Message::SettingsFieldChanged(SettingsField::LineHeight(v))
                })
                .width(300.0),
                settings_row(
                    i18n.tr("iced.settings.row.mono_font"),
                    pick_list(
                        FONTS.as_slice(),
                        font_sel,
                        |f: &str| Message::SettingsFieldChanged(SettingsField::FontFamily(f.to_string())),
                    )
                    .width(220.0),
                ),
                text(i18n.tr("iced.settings.section.gpu_font")).size(14),
                settings_row(
                    i18n.tr("iced.settings.row.gpu_font_path"),
                    text_input("TTF/TTC path", &gpu_path)
                        .on_input(|s| Message::SettingsFieldChanged(SettingsField::GpuFontPath(s)))
                        .width(iced::Length::Fill),
                ),
                settings_row(
                    i18n.tr("iced.settings.row.gpu_face_index"),
                    text_input("0", &face_raw)
                        .on_input(|s| {
                            Message::SettingsFieldChanged(SettingsField::GpuFontFaceIndex(s))
                        })
                        .width(120.0),
                ),
                text(i18n.tr("iced.settings.row.render_mode")).size(14),
                radio(
                    "styled",
                    TerminalRenderMode::Styled,
                    Some(mode),
                    |m| Message::SettingsFieldChanged(SettingsField::TerminalRenderMode(m)),
                ),
                radio(
                    "plain",
                    TerminalRenderMode::Plain,
                    Some(mode),
                    |m| Message::SettingsFieldChanged(SettingsField::TerminalRenderMode(m)),
                ),
                text(i18n.tr("iced.settings.row.plain_text_update")).size(14),
                text(i18n.tr("iced.settings.hint.plain_text_update_plain_only")).size(11),
                radio(
                    "incremental",
                    TerminalPlainTextUpdate::Incremental,
                    Some(plain_upd),
                    |m| Message::SettingsFieldChanged(SettingsField::PlainTextUpdate(m)),
                ),
                radio(
                    "full",
                    TerminalPlainTextUpdate::Full,
                    Some(plain_upd),
                    |m| Message::SettingsFieldChanged(SettingsField::PlainTextUpdate(m)),
                ),
            ]
            .spacing(12)
            .padding(20)
            .width(iced::Length::Fill)
            .into()
        }
        3 => column![
            section_title(i18n.tr("iced.settings.section.interaction")),
            checkbox(t.right_click_paste)
                .label(i18n.tr("iced.settings.row.right_paste"))
                .on_toggle(|v| Message::SettingsFieldChanged(SettingsField::RightClickPaste(v))),
            checkbox(t.bracketed_paste)
                .label(i18n.tr("iced.settings.row.bracketed_paste"))
                .on_toggle(|v| Message::SettingsFieldChanged(SettingsField::BracketedPaste(v))),
            checkbox(t.keep_selection_highlight)
                .label(i18n.tr("iced.settings.row.keep_selection"))
                .on_toggle(|v| {
                    Message::SettingsFieldChanged(SettingsField::KeepSelectionHighlight(v))
                }),
            settings_row(
                i18n.tr("iced.settings.row.scrollback"),
                text(format!("{}", t.scrollback_limit)).size(12),
            ),
            slider(1000u32..=50000u32, t.scrollback_limit as u32, |v| {
                Message::SettingsFieldChanged(SettingsField::ScrollbackLimit(v as usize))
            })
            .step(1000u32)
            .width(320.0),
            checkbox(t.history_search_enabled)
                .label(i18n.tr("iced.settings.row.history_search"))
                .on_toggle(|v| Message::SettingsFieldChanged(SettingsField::HistorySearch(v))),
            checkbox(t.local_path_completion_enabled)
                .label(i18n.tr("iced.settings.row.path_completion"))
                .on_toggle(|v| Message::SettingsFieldChanged(SettingsField::PathCompletion(v))),
        ]
        .spacing(12)
        .padding(20)
        .width(iced::Length::Fill)
        .into(),
        _ => Space::new().into(),
    }
}

fn matches_protocol(transport: &TransportConfig, protocol: ProtocolType) -> bool {
    matches!(
        (transport, protocol),
        (TransportConfig::Ssh(_), ProtocolType::SSH)
            | (TransportConfig::Telnet(_), ProtocolType::Telnet)
            | (TransportConfig::Serial(_), ProtocolType::Serial)
    )
}

fn target_of(s: &SessionProfile) -> String {
    match &s.transport {
        TransportConfig::Ssh(ssh) => format!("{}:{}", ssh.host, ssh.port),
        TransportConfig::Telnet(tel) => format!("{}:{}", tel.host, tel.port),
        TransportConfig::Serial(se) => se.port.clone(),
    }
}

fn matches_search(s: &SessionProfile, search: &str) -> bool {
    if search.is_empty() {
        return true;
    }
    let q = search.to_lowercase();
    s.name.to_lowercase().contains(&q) || target_of(s).to_lowercase().contains(&q)
}

fn connection_pane(state: &IcedState, sub: usize) -> Element<'_, Message> {
    let i18n = &state.model.i18n;
    let search = state.settings_connection_search.as_str();
    match sub {
        0 => connection_protocol_page(state, ProtocolType::SSH, "SSH", search),
        1 => connection_protocol_page(state, ProtocolType::Telnet, "TELNET", search),
        2 => connection_protocol_page(state, ProtocolType::Serial, "SERIAL", search),
        3 => {
            let single = state.model.settings.quick_connect.single_shared_session;
            column![
                section_title(i18n.tr("iced.settings.conn.session_model_title")),
                checkbox(single)
                    .label(i18n.tr("iced.settings.row.single_shared_session"))
                    .on_toggle(|v| {
                        Message::SettingsFieldChanged(SettingsField::SingleSharedSession(v))
                    }),
                text(i18n.tr("iced.settings.hint.single_shared_session")).size(12),
                section_title(i18n.tr("iced.settings.conn.advanced_title")),
                text(i18n.tr("iced.settings.conn.advanced_hint")).size(13),
            ]
            .spacing(12)
            .padding(20)
            .width(iced::Length::Fill)
            .into()
        }
        _ => Space::new().into(),
    }
}

fn connection_protocol_page<'a>(
    state: &'a IcedState,
    protocol: ProtocolType,
    title: &str,
    search: &'a str,
) -> Element<'a, Message> {
    let i18n = &state.model.i18n;
    let mut grouped: BTreeMap<String, Vec<SessionProfile>> = BTreeMap::new();
    for s in state.model.profiles() {
        if !matches_protocol(&s.transport, protocol) {
            continue;
        }
        if !matches_search(s, search) {
            continue;
        }
        let group_name = s
            .folder
            .as_ref()
            .map(|g| g.trim().to_string())
            .filter(|g| !g.is_empty())
            .unwrap_or_else(|| "Default".to_string());
        grouped.entry(group_name).or_default().push(s.clone());
    }
    for sessions in grouped.values_mut() {
        sessions.sort_by_key(|s| s.name.to_lowercase());
    }

    let head = format!("{} {}", title, i18n.tr("iced.settings.conn.manage_suffix"));
    let mut col = column![
        section_title_owned(head),
        row![
            text_input(i18n.tr("iced.settings.conn.search_hint"), search)
                .on_input(|q| Message::SettingsFieldChanged(SettingsField::ConnectionSearch(q)))
                .width(iced::Length::Fill),
            button(text(i18n.tr("iced.settings.conn.new")))
                .on_press(Message::OpenSessionEditor(None))
                .style(style_chrome_primary(12.0)),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    ]
    .spacing(12)
    .padding(20)
    .width(iced::Length::Fill);

    if grouped.is_empty() {
        col = col.push(text(i18n.tr("iced.settings.conn.empty")).size(13));
    } else {
        for (group, sessions) in grouped {
            let is_default = group == "Default" || group == "未分类";
            let header = if is_default {
                String::new()
            } else {
                group.clone()
            };
            if !header.is_empty() {
                col = col.push(text(header).size(13));
            }
            for s in sessions {
                let id = s.id.clone();
                let subtitle = target_of(&s);
                col = col.push(
                    row![
                        column![
                            text(s.name.clone()).size(13),
                            text(subtitle).size(11).style(|theme: &Theme| text::Style {
                                color: Some(
                                    theme.extended_palette().background.base.text.scale_alpha(0.6),
                                ),
                            }),
                        ]
                        .spacing(2),
                        Space::new().width(iced::Length::Fill),
                        button(text(i18n.tr("iced.settings.conn.edit")))
                            .on_press(Message::OpenSessionEditor(Some(id.clone())))
                            .style(style_chrome_secondary(11.0)),
                        button(text(i18n.tr("iced.settings.conn.delete")))
                            .on_press(Message::DeleteSessionProfile(id))
                            .style(style_chrome_secondary(11.0)),
                    ]
                    .align_y(Alignment::Center),
                );
            }
        }
    }
    col.into()
}

fn security_pane(state: &IcedState, sub: usize) -> Element<'_, Message> {
    let i18n = &state.model.i18n;
    let sec = &state.model.settings.security;
    match sub {
        0 => {
            let timeout = sec.idle_timeout_mins;
            column![
                section_title(i18n.tr("settings.security.vault.title")),
                column![
                    text(i18n.tr("settings.security.auto_lock.label")).size(14),
                    text(i18n.tr("settings.security.auto_lock.help")).size(11),
                ]
                .spacing(4),
                radio(
                    i18n.tr("settings.security.timeout.minute_1"),
                    1u32,
                    (timeout == 1).then_some(1u32),
                    |v| Message::SettingsFieldChanged(SettingsField::IdleTimeoutMins(v)),
                ),
                radio(
                    i18n.tr("settings.security.timeout.minute_5"),
                    5u32,
                    (timeout == 5).then_some(5u32),
                    |v| Message::SettingsFieldChanged(SettingsField::IdleTimeoutMins(v)),
                ),
                radio(
                    i18n.tr("settings.security.timeout.minute_10"),
                    10u32,
                    (timeout == 10).then_some(10u32),
                    |v| Message::SettingsFieldChanged(SettingsField::IdleTimeoutMins(v)),
                ),
                radio(
                    i18n.tr("settings.security.timeout.minute_30"),
                    30u32,
                    (timeout == 30).then_some(30u32),
                    |v| Message::SettingsFieldChanged(SettingsField::IdleTimeoutMins(v)),
                ),
                radio(
                    i18n.tr("settings.security.timeout.never"),
                    0u32,
                    (timeout == 0).then_some(0u32),
                    |v| Message::SettingsFieldChanged(SettingsField::IdleTimeoutMins(v)),
                ),
                checkbox(sec.lock_on_sleep)
                    .label(i18n.tr("settings.security.lock_on_sleep.label"))
                    .on_toggle(|v| Message::SettingsFieldChanged(SettingsField::LockOnSleep(v))),
                text(i18n.tr("settings.security.lock_on_sleep.help")).size(11),
                button(text(if sec.vault.is_some() {
                    i18n.tr("settings.security.master_password.change_action")
                } else {
                    i18n.tr("settings.security.master_password.init_action")
                }))
                .on_press(Message::VaultOpen)
                .style(style_chrome_secondary(12.0)),
                text(i18n.tr("settings.security.biometrics.title")).size(16),
                checkbox(sec.use_biometrics)
                    .label(i18n.tr("settings.security.biometrics.label"))
                    .on_toggle(Message::BiometricsToggle),
                text(i18n.tr("settings.security.biometrics.help")).size(11),
            ]
            .spacing(12)
            .padding(20)
            .width(iced::Length::Fill)
            .into()
        }
        1 => {
            let policy = sec.host_key_policy;
            column![
                section_title(i18n.tr("settings.security.hosts.title")),
                text(i18n.tr("settings.security.hosts.policy.label")).size(14),
                radio(
                    i18n.tr("settings.security.hosts.policy.strict"),
                    HostKeyPolicy::Strict,
                    Some(policy),
                    |p| Message::SettingsFieldChanged(SettingsField::HostKeyPolicy(p)),
                ),
                radio(
                    i18n.tr("settings.security.hosts.policy.ask"),
                    HostKeyPolicy::Ask,
                    Some(policy),
                    |p| Message::SettingsFieldChanged(SettingsField::HostKeyPolicy(p)),
                ),
                radio(
                    i18n.tr("settings.security.hosts.policy.accept_new"),
                    HostKeyPolicy::AcceptNew,
                    Some(policy),
                    |p| Message::SettingsFieldChanged(SettingsField::HostKeyPolicy(p)),
                ),
                text(i18n.tr("settings.security.hosts.policy.help")).size(11),
                text(i18n.tr("settings.security.hosts.table.title")).size(14),
                text("(mock) 10.0.0.1:22  ·  ED25519").size(12),
                text("(mock) github.com  ·  RSA").size(12),
            ]
            .spacing(12)
            .padding(20)
            .width(iced::Length::Fill)
            .into()
        }
        _ => Space::new().into(),
    }
}

fn backup_pane(state: &IcedState) -> Element<'_, Message> {
    let i18n = &state.model.i18n;
    column![
        section_title(i18n.tr("iced.settings.backup.title")),
        text(i18n.tr("iced.settings.backup.hint")).size(13),
    ]
    .spacing(12)
    .padding(20)
    .width(iced::Length::Fill)
    .into()
}

fn settings_row<'a, L: Into<Element<'a, Message>>>(
    label: &'static str,
    right: L,
) -> Element<'a, Message> {
    row![
        container(text(label).size(14))
            .width(iced::Length::FillPortion(1))
            .align_y(iced::alignment::Vertical::Top),
        container(right.into())
            .width(iced::Length::FillPortion(1))
            .align_x(iced::alignment::Horizontal::Right),
    ]
    .spacing(16)
    .align_y(Alignment::Start)
    .into()
}

fn restart_banner(state: &IcedState) -> Element<'_, Message> {
    if !state.settings_needs_restart {
        return Space::new().height(0).into();
    }
    let i18n = &state.model.i18n;
    container(
        row![
            text(i18n.tr("iced.settings.restart.banner")).size(13),
            Space::new().width(iced::Length::Fill),
            button(text(i18n.tr("iced.settings.restart.ok")))
                .on_press(Message::SettingsRestartAcknowledged)
                .style(style_chrome_primary(12.0)),
        ]
        .padding(12)
        .align_y(Alignment::Center),
    )
    .width(iced::Length::Fill)
    .style(|_t: &Theme| {
        container::Style::default().background(iced::Color::from_rgba(1.0, 0.65, 0.0, 0.15))
    })
    .into()
}
