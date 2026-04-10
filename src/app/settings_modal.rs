//! 设置中心弹窗（布局与 egui `settings/mod.rs` 对齐：左侧大类 + 右侧子页签 + 滚动内容）。

use std::collections::BTreeMap;

use iced::alignment::Alignment;
use iced::widget::scrollable::{Direction as ScrollDirection, Scrollbar};
use iced::widget::{
    Space, Stack, button, checkbox, column, container, pick_list, radio, row,
    scrollable, slider, text, text_input,
};
use iced::{Border, Color, Element, Theme};

use crate::session::{ProtocolType, SessionProfile, TransportConfig};
use crate::settings::HostKeyPolicy;
use crate::theme::layout;
use crate::theme::{COLOR_SCHEMES, DesignTokens};
use crate::theme::icons::{icon_view_with, IconId, IconOptions};

use super::message::{Message, SettingsCategory, SettingsField};
use super::state::IcedState;
use super::widgets::chrome_button::{style_chrome_primary, style_chrome_secondary, style_tab_strip, style_top_icon};

fn settings_tokens(state: &IcedState) -> DesignTokens {
    DesignTokens::for_color_scheme(&state.model.settings.color_scheme)
}

pub(crate) fn max_sub_tab(category: SettingsCategory) -> usize {
    match category {
        SettingsCategory::General => 1,
        SettingsCategory::ColorScheme => 0,
        SettingsCategory::Terminal => 2,
        SettingsCategory::Connection => 3,
        SettingsCategory::Security => 1,
        SettingsCategory::Backup => 0,
    }
}

pub(crate) fn clamp_sub_tab(category: SettingsCategory, sub: usize) -> usize {
    sub.min(max_sub_tab(category))
}

pub(crate) fn modal_stack(state: &IcedState) -> Element<'_, Message> {
    let tokens = settings_tokens(state);

    // 遮罩层不再拦截点击事件，用户点击模态框外部区域不会关闭设置中心
    // ESC 键关闭功能在其他地方处理
    let scrim = container(Space::new().width(iced::Length::Fill).height(iced::Length::Fill))
        .style(move |_t: &Theme| {
            container::Style::default().background(tokens.scrim())
        });

    let mw = (state.window_size.width * layout::SETTINGS_MODAL_WIDTH_RATIO)
        .clamp(layout::SETTINGS_MODAL_MIN_WIDTH, layout::SETTINGS_MODAL_MAX_WIDTH);
    let mh = (state.window_size.height * layout::SETTINGS_MODAL_HEIGHT_RATIO)
        .max(layout::SETTINGS_MODAL_MIN_HEIGHT);

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
    let tokens = settings_tokens(state);
    let sidebar = settings_sidebar(state, tokens);
    let body = column![
        settings_header_row(i18n, tokens),
        settings_sub_tab_row(state, tokens),
        scrollable(settings_main_content(state, tokens))
            .direction(ScrollDirection::Vertical(Scrollbar::default()))
            .width(iced::Length::Fill)
            .height(iced::Length::Fill),
        restart_banner(state, tokens),
    ]
    .spacing(0)
    .width(iced::Length::Fill)
    .height(iced::Length::Fill);

    // 侧边栏与内容区之间添加 1px 分隔线
    let divider = container(Space::new())
        .width(iced::Length::Fixed(1.0))
        .height(iced::Length::Fill)
        .style(move |_t: &Theme| {
            container::Style::default().background(tokens.border_subtle)
        });

    let row_content = row![
        sidebar,
        divider,
        container(body).width(iced::Length::Fill).padding(0)
    ]
    .spacing(0)
    .width(iced::Length::Fill)
    .height(iced::Length::Fill);

    container(row_content)
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .style(move |_t: &Theme| {
            container::Style::default()
                .background(tokens.bg_primary)
                .border(Border {
                    width: 1.0,
                    color: tokens.border_default,
                    radius: 12.0.into(),
                })
        })
        .into()
}

fn sidebar_item_style(active: bool, tokens: DesignTokens) -> container::Style {
    let bg = if active { tokens.surface_2 } else { tokens.bg_secondary };
    let border_color = if active { tokens.accent_base } else { Color::TRANSPARENT };
    container::Style::default()
        .background(bg)
        .border(Border {
            width: 3.0,
            color: border_color,
            radius: 4.0.into(),
        })
}

fn sidebar_item_style_clone(active: bool, bg: Color, accent: Color) -> impl Fn(&Theme) -> container::Style + 'static {
    move |_theme: &Theme| {
        container::Style::default()
            .background(bg)
            .border(Border {
                width: if active { 3.0 } else { 0.0 },
                color: if active { accent } else { Color::TRANSPARENT },
                radius: 4.0.into(),
            })
    }
}

fn settings_sidebar(state: &IcedState, tokens: DesignTokens) -> Element<'_, Message> {
    let i18n = &state.model.i18n;
    let entries: [(SettingsCategory, &'static str); 6] = [
        (SettingsCategory::General, "iced.settings.cat.general"),
        (SettingsCategory::ColorScheme, "iced.settings.cat.color_scheme"),
        (SettingsCategory::Terminal, "iced.settings.cat.terminal"),
        (SettingsCategory::Connection, "iced.settings.cat.connection"),
        (SettingsCategory::Security, "iced.settings.cat.security"),
        (SettingsCategory::Backup, "iced.settings.cat.backup"),
    ];
    let accent_base = tokens.accent_base;
    let bg_secondary = tokens.bg_secondary;
    let surface_2 = tokens.surface_2;
    let text_secondary = tokens.text_secondary;
    let mut col = column![]
        .spacing(4)
        .width(iced::Length::Fixed(layout::SETTINGS_SIDEBAR_WIDTH))
        .padding(layout::SETTINGS_SIDEBAR_PADDING);
    col = col.push(Space::new().height(iced::Length::Fixed(24.0)));
    for (cat, key) in entries {
        let label = i18n.tr(key).to_string();
        let active = state.settings_category == cat;
        let text_color = if active { accent_base } else { text_secondary };
        let btn_style = style_tab_strip(tokens);
        let item_bg = if active { surface_2 } else { bg_secondary };
        let item_accent = if active { accent_base } else { Color::TRANSPARENT };
        let cell = button(
            container(text(label).size(13).style(move |_t: &Theme| text::Style {
                color: Some(text_color),
            }))
                .width(iced::Length::Fill)
                .padding([10, 16]),
        )
        .on_press(Message::SettingsCategoryChanged(cat))
        .width(iced::Length::Fill)
        .style(btn_style);
        col = col.push(
            container(cell)
                .style(sidebar_item_style_clone(active, item_bg, item_accent)),
        );
    }
    container(col)
        .width(iced::Length::Fixed(layout::SETTINGS_SIDEBAR_WIDTH))
        .height(iced::Length::Fill)
        .style(move |_t: &Theme| {
            container::Style::default().background(bg_secondary)
        })
        .into()
}

fn settings_header_row(i18n: &crate::i18n::I18n, tokens: DesignTokens) -> Element<'_, Message> {
    let text_primary = tokens.text_primary;
    let btn_style = style_top_icon(tokens);
    container(
        row![
            text(i18n.tr("iced.settings.title")).size(16).style(move |_t: &Theme| text::Style {
                color: Some(text_primary),
            }),
            Space::new().width(iced::Length::Fill),
            icon_close_button(tokens, btn_style),
        ]
        .align_y(Alignment::Center),
    )
    .width(iced::Length::Fill)
    .height(iced::Length::Fixed(layout::SETTINGS_HEADER_HEIGHT))
    .padding([4, 12])
    .into()
}

fn sub_tab_labels(category: SettingsCategory, i18n: &crate::i18n::I18n) -> Vec<String> {
    let keys: &[&str] = match category {
        SettingsCategory::General => &[
            "iced.settings.sub.general.basic",
            "iced.settings.sub.general.typography",
        ],
        SettingsCategory::ColorScheme => &[
            "iced.settings.sub.color_scheme.presets",
        ],
        SettingsCategory::Terminal => &[
            "iced.settings.sub.terminal.render",
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

fn settings_sub_tab_row(state: &IcedState, tokens: DesignTokens) -> Element<'_, Message> {
    let i18n = &state.model.i18n;
    let labels = sub_tab_labels(state.settings_category, i18n);
    let cat_i = state.settings_category as usize;
    let current = state.settings_sub_tab[cat_i].min(max_sub_tab(state.settings_category));
    let accent_base = tokens.accent_base;
    let text_secondary = tokens.text_secondary;
    let border_subtle = tokens.border_subtle;

    let mut r = row![].spacing(layout::SETTINGS_TAB_SPACING).align_y(Alignment::Center);
    for (idx, lab) in labels.iter().enumerate() {
        let active = idx == current;
        let lab_clone = lab.clone();
        let btn_color = if active { accent_base } else { text_secondary };
        let btn = button(
            container(
                text(lab_clone).size(13).style(move |_t: &Theme| text::Style {
                    color: Some(btn_color),
                })
            )
        )
            .on_press(Message::SettingsSubTabChanged(idx))
            .style(move |theme: &Theme, status: iced::widget::button::Status| {
                // 所有 Tab 按钮都使用极简样式，选中态通过文字颜色区分
                style_tab_strip(tokens)(theme, status)
            });
        r = r.push(btn);
    }

    // Tab 栏容器，底部添加 1px border_subtle 分隔线
    container(r)
        .width(iced::Length::Fill)
        .height(iced::Length::Fixed(layout::SETTINGS_TAB_HEIGHT))
        .padding([8, 24])
        .style(move |_t: &Theme| {
            container::Style::default()
                .border(Border {
                    width: 1.0,
                    color: border_subtle,
                    radius: 0.0.into(),
                })
        })
        .into()
}

fn settings_main_content(state: &IcedState, tokens: DesignTokens) -> Element<'_, Message> {
    let cat = state.settings_category;
    let sub = state.settings_sub_tab[cat as usize];
    let sub = clamp_sub_tab(cat, sub);
    match cat {
        SettingsCategory::General => general_pane(state, sub, tokens),
        SettingsCategory::ColorScheme => color_scheme_pane(state, tokens),
        SettingsCategory::Terminal => terminal_pane(state, sub, tokens),
        SettingsCategory::Connection => connection_pane(state, sub, tokens),
        SettingsCategory::Security => security_pane(state, sub, tokens),
        SettingsCategory::Backup => backup_pane(state, tokens),
    }
}

fn section_title(s: &str, tokens: DesignTokens) -> Element<'_, Message> {
    let text_primary = tokens.text_primary;
    container(
        text(s).size(18).style(move |_t: &Theme| text::Style {
            color: Some(text_primary),
        })
    )
        .padding(iced::Padding {
            top: 8.0,
            right: 0.0,
            bottom: 16.0,
            left: 0.0,
        })
        .into()
}

fn section_title_owned(s: String, tokens: DesignTokens) -> Element<'static, Message> {
    let text_primary = tokens.text_primary;
    container(
        text(s).size(18).style(move |_t: &Theme| text::Style {
            color: Some(text_primary),
        })
    )
        .padding(iced::Padding {
            top: 8.0,
            right: 0.0,
            bottom: 16.0,
            left: 0.0,
        })
        .into()
}

fn general_pane(state: &IcedState, sub: usize, tokens: DesignTokens) -> Element<'_, Message> {
    let i18n = &state.model.i18n;
    let g = &state.model.settings.general;
    let text_secondary = tokens.text_secondary;

    match sub {
        0 => column![
            section_title(i18n.tr("iced.settings.section.startup"), tokens),
            settings_row(
                i18n.tr("iced.settings.row.language"),
                radio(
                    i18n.tr("iced.settings.language.zh"),
                    "zh-CN",
                    (g.language == "zh-CN").then_some("zh-CN"),
                    |c| Message::SettingsFieldChanged(SettingsField::Language(c.to_string())),
                ),
                tokens,
            ),
            settings_row(
                "",
                radio(
                    i18n.tr("iced.settings.language.en"),
                    "en-US",
                    (g.language == "en-US").then_some("en-US"),
                    |c| Message::SettingsFieldChanged(SettingsField::Language(c.to_string())),
                ),
                tokens,
            ),
            checkbox(g.auto_check_update)
                .label(i18n.tr("iced.settings.row.auto_update"))
                .on_toggle(|v| Message::SettingsFieldChanged(SettingsField::AutoCheckUpdate(v))),
            text(i18n.tr("iced.settings.language.help")).size(12).style(move |_t: &Theme| text::Style {
                color: Some(text_secondary),
            }),
        ]
        .spacing(layout::SETTINGS_ITEM_SPACING)
        .padding(layout::SETTINGS_CONTENT_PADDING as u16)
        .width(iced::Length::Fill)
        .into(),
        1 => column![
            section_title(i18n.tr("iced.settings.section.typography"), tokens),
            settings_row(
                i18n.tr("iced.settings.row.ui_font_size"),
                text(format!("{:.0}px", g.font_size)).size(12).style(move |_t: &Theme| text::Style {
                    color: Some(text_secondary),
                }),
                tokens,
            ),
            slider(12.0..=20.0, g.font_size, |v| {
                Message::SettingsFieldChanged(SettingsField::FontSize(v))
            })
            .width(300.0),
        ]
        .spacing(layout::SETTINGS_ITEM_SPACING)
        .padding(layout::SETTINGS_CONTENT_PADDING as u16)
        .width(iced::Length::Fill)
        .into(),
        _ => Space::new().into(),
    }
}

fn color_scheme_pane(state: &IcedState, tokens: DesignTokens) -> Element<'_, Message> {
    let i18n = &state.model.i18n;
    let current_scheme_id = &state.model.settings.color_scheme;
    let text_primary = tokens.text_primary;

    let mut col = column![section_title(i18n.tr("iced.settings.section.color_scheme"), tokens)]
        .spacing(layout::SETTINGS_ITEM_SPACING);

    // 分两列显示预设方案
    for chunk in COLOR_SCHEMES.chunks(2) {
        let mut row_elements: Vec<Element<'_, Message>> = Vec::new();
        for scheme in chunk.iter() {
            let active = current_scheme_id == scheme.id;

            // 配色预览框 - 使用 effective_* 方法获取衍生值
            let preview = container(
                column![
                    text("Aa").size(12).style(move |_| text::Style {
                        color: Some(scheme.effective_term_fg()),
                    }),
                ]
                .spacing(4),
            )
            .width(iced::Length::Fixed(60.0))
            .height(iced::Length::Fixed(40.0))
            .padding(4)
            .style(move |_| container::Style {
                background: Some(iced::Background::Color(scheme.effective_term_bg())),
                border: iced::Border {
                    width: 1.0,
                    color: if active { tokens.accent_base } else { tokens.border_default },
                    radius: 4.0.into(),
                },
                ..Default::default()
            });

            let applied_text: Element<'_, Message> = if active {
                text(i18n.tr("iced.settings.scheme.applied"))
                    .size(10)
                    .style(move |_t: &Theme| text::Style {
                        color: Some(tokens.accent_base),
                    })
                    .into()
            } else {
                Space::new().height(iced::Length::Fixed(12.0)).into()
            };

            let scheme_content = column![
                text(scheme.name).size(13).style(move |_t: &Theme| text::Style {
                    color: Some(text_primary),
                }),
                preview,
                applied_text,
            ]
            .spacing(layout::SETTINGS_LABEL_DESC_SPACING);

            let scheme_btn = button(scheme_content)
                .on_press(Message::SettingsFieldChanged(SettingsField::ColorScheme(
                    scheme.id.to_string(),
                )))
                .padding(8)
                .style(move |_theme: &Theme, _status: iced::widget::button::Status| {
                    iced::widget::button::Style {
                        background: Some(if active {
                            tokens.accent_base.into()
                        } else {
                            tokens.surface_1.into()
                        }),
                        border: iced::Border {
                            width: if active { 2.0 } else { 1.0 },
                            color: if active {
                                tokens.accent_base
                            } else {
                                tokens.border_default
                            },
                            radius: 8.0.into(),
                        },
                        ..Default::default()
                    }
                });

            row_elements.push(scheme_btn.into());
        }

        // 如果只有奇数个，补一个空占位
        while row_elements.len() < 2 {
            row_elements.push(
                Space::new()
                    .width(iced::Length::Fill)
                    .height(iced::Length::Fixed(80.0))
                    .into(),
            );
        }

        col = col.push(row(row_elements).spacing(layout::SETTINGS_ITEM_SPACING));
    }

    col.spacing(layout::SETTINGS_ITEM_SPACING)
        .padding(layout::SETTINGS_CONTENT_PADDING as u16)
        .width(iced::Length::Fill)
        .into()
}

fn terminal_pane(state: &IcedState, sub: usize, tokens: DesignTokens) -> Element<'_, Message> {
    let i18n = &state.model.i18n;
    let t = &state.model.settings.terminal;
    let text_secondary = tokens.text_secondary;

    match sub {
        // 渲染
        0 => column![
            section_title(i18n.tr("iced.settings.section.render_engine"), tokens),
            settings_row(
                i18n.tr("iced.settings.row.target_fps"),
                text(format!("{}", t.target_fps)).size(12).style(move |_t: &Theme| text::Style {
                    color: Some(text_secondary),
                }),
                tokens,
            ),
            slider(10u32..=240u32, t.target_fps, |v| {
                Message::SettingsFieldChanged(SettingsField::TargetFps(v))
            })
            .width(300.0),
        ]
        .spacing(layout::SETTINGS_ITEM_SPACING)
        .padding(layout::SETTINGS_CONTENT_PADDING as u16)
        .width(iced::Length::Fill)
        .into(),
        // 文字
        1 => {
            const FONTS: [&str; 3] = ["JetBrains Mono", "SF Mono", "Cascadia Code"];
            let font_sel = FONTS
                .iter()
                .find(|&&x| x == t.font_family.as_str())
                .map(|_| t.font_family.as_str());
            column![
                section_title(i18n.tr("iced.settings.section.text_render"), tokens),
                checkbox(t.apply_terminal_metrics)
                    .label(i18n.tr("iced.settings.row.apply_terminal_metrics"))
                    .on_toggle(|v| {
                        Message::SettingsFieldChanged(SettingsField::ApplyTerminalMetrics(v))
                    }),
                settings_row(
                    i18n.tr("iced.settings.row.terminal_font_size"),
                    text(format!("{:.0}", t.font_size)).size(12).style(move |_t: &Theme| text::Style {
                        color: Some(text_secondary),
                    }),
                    tokens,
                ),
                slider(8.0..=36.0, t.font_size, |v| {
                    Message::SettingsFieldChanged(SettingsField::TerminalFontSize(v))
                })
                .width(300.0),
                settings_row(
                    i18n.tr("iced.settings.row.line_height"),
                    text(format!("{:.2}", t.line_height)).size(12).style(move |_t: &Theme| text::Style {
                        color: Some(text_secondary),
                    }),
                    tokens,
                ),
                slider(1.0..=2.0, t.line_height, |v| {
                    Message::SettingsFieldChanged(SettingsField::LineHeight(v))
                })
                .width(300.0),
                settings_row(
                    i18n.tr("iced.settings.row.mono_font"),
                    pick_list(FONTS.as_slice(), font_sel, |f: &str| {
                        Message::SettingsFieldChanged(SettingsField::FontFamily(f.to_string()))
                    },)
                    .width(220.0),
                    tokens,
                ),
            ]
            .spacing(layout::SETTINGS_ITEM_SPACING)
            .padding(layout::SETTINGS_CONTENT_PADDING as u16)
            .width(iced::Length::Fill)
            .into()
        }
        // 交互
        2 => column![
            section_title(i18n.tr("iced.settings.section.interaction"), tokens),
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
                text(format!("{}", t.scrollback_limit)).size(12).style(move |_t: &Theme| text::Style {
                    color: Some(text_secondary),
                }),
                tokens,
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
        .spacing(layout::SETTINGS_ITEM_SPACING)
        .padding(layout::SETTINGS_CONTENT_PADDING as u16)
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

fn connection_pane(state: &IcedState, sub: usize, tokens: DesignTokens) -> Element<'_, Message> {
    let i18n = &state.model.i18n;
    let search = state.settings_connection_search.as_str();
    let text_secondary = tokens.text_secondary;

    match sub {
        0 => connection_protocol_page(state, ProtocolType::SSH, "SSH", search, tokens),
        1 => connection_protocol_page(state, ProtocolType::Telnet, "TELNET", search, tokens),
        2 => connection_protocol_page(state, ProtocolType::Serial, "SERIAL", search, tokens),
        3 => {
            let single = state.model.settings.quick_connect.single_shared_session;
            column![
                section_title(i18n.tr("iced.settings.conn.session_model_title"), tokens),
                checkbox(single)
                    .label(i18n.tr("iced.settings.row.single_shared_session"))
                    .on_toggle(|v| {
                        Message::SettingsFieldChanged(SettingsField::SingleSharedSession(v))
                    }),
                text(i18n.tr("iced.settings.hint.single_shared_session")).size(12).style(move |_t: &Theme| text::Style {
                    color: Some(text_secondary),
                }),
                section_title(i18n.tr("iced.settings.conn.advanced_title"), tokens),
                text(i18n.tr("iced.settings.conn.advanced_hint")).size(13).style(move |_t: &Theme| text::Style {
                    color: Some(text_secondary),
                }),
            ]
            .spacing(layout::SETTINGS_ITEM_SPACING)
            .padding(layout::SETTINGS_CONTENT_PADDING as u16)
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
    tokens: DesignTokens,
) -> Element<'a, Message> {
    let i18n = &state.model.i18n;
    let text_primary = tokens.text_primary;
    let text_secondary = tokens.text_secondary;
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
        section_title_owned(head, tokens),
        row![
            text_input(i18n.tr("iced.settings.conn.search_hint"), search)
                .on_input(|q| Message::SettingsFieldChanged(SettingsField::ConnectionSearch(q)))
                .width(iced::Length::Fill),
            button(text(i18n.tr("iced.settings.conn.new")))
                .on_press(Message::OpenSessionEditor(None))
                .style(style_chrome_primary(tokens)),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    ]
    .spacing(layout::SETTINGS_ITEM_SPACING)
    .padding(layout::SETTINGS_CONTENT_PADDING as u16)
    .width(iced::Length::Fill);

    if grouped.is_empty() {
        col = col.push(text(i18n.tr("iced.settings.conn.empty")).size(13).style(move |_t: &Theme| text::Style {
            color: Some(text_secondary),
        }));
    } else {
        for (group, sessions) in grouped {
            let is_default = group == "Default" || group == "未分类";
            let header = if is_default {
                String::new()
            } else {
                group.clone()
            };
            if !header.is_empty() {
                col = col.push(text(header).size(13).style(move |_t: &Theme| text::Style {
                    color: Some(text_primary),
                }));
            }
            for s in sessions {
                let id = s.id.clone();
                let subtitle = target_of(&s);
                col = col.push(
                    row![
                        column![
                            text(s.name.clone()).size(13).style(move |_t: &Theme| text::Style {
                                color: Some(text_primary),
                            }),
                            text(subtitle).size(11).style(move |_t: &Theme| text::Style {
                                color: Some(text_secondary),
                            }),
                        ]
                        .spacing(layout::SETTINGS_LABEL_DESC_SPACING),
                        Space::new().width(iced::Length::Fill),
                        button(text(i18n.tr("iced.settings.conn.edit")))
                            .on_press(Message::OpenSessionEditor(Some(id.clone())))
                            .style(style_chrome_secondary(tokens)),
                        button(text(i18n.tr("iced.settings.conn.delete")))
                            .on_press(Message::DeleteSessionProfile(id))
                            .style(style_chrome_secondary(tokens)),
                    ]
                    .align_y(Alignment::Center),
                );
            }
        }
    }
    col.into()
}

fn security_pane(state: &IcedState, sub: usize, tokens: DesignTokens) -> Element<'_, Message> {
    let i18n = &state.model.i18n;
    let sec = &state.model.settings.security;
    let text_primary = tokens.text_primary;
    let text_secondary = tokens.text_secondary;

    match sub {
        0 => {
            let timeout = sec.idle_timeout_mins;
            column![
                section_title(i18n.tr("settings.security.vault.title"), tokens),
                column![
                    text(i18n.tr("settings.security.auto_lock.label")).size(14).style(move |_t: &Theme| text::Style {
                        color: Some(text_primary),
                    }),
                    text(i18n.tr("settings.security.auto_lock.help")).size(11).style(move |_t: &Theme| text::Style {
                        color: Some(text_secondary),
                    }),
                ]
                .spacing(layout::SETTINGS_LABEL_DESC_SPACING),
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
                text(i18n.tr("settings.security.lock_on_sleep.help")).size(11).style(move |_t: &Theme| text::Style {
                    color: Some(text_secondary),
                }),
                Space::new().height(iced::Length::Fixed(8.0)),
                section_title(i18n.tr("settings.security.kdf.title"), tokens),
                text(i18n.tr("settings.security.kdf.help")).size(11).style(move |_t: &Theme| text::Style {
                    color: Some(text_secondary),
                }),
                radio(
                    i18n.tr("settings.security.kdf.balanced"),
                    crate::settings::KdfMemoryLevel::Balanced,
                    Some(sec.kdf_memory_level),
                    |v| Message::SettingsFieldChanged(SettingsField::KdfMemoryLevel(v)),
                ),
                radio(
                    i18n.tr("settings.security.kdf.security"),
                    crate::settings::KdfMemoryLevel::Security,
                    Some(sec.kdf_memory_level),
                    |v| Message::SettingsFieldChanged(SettingsField::KdfMemoryLevel(v)),
                ),
                Space::new().height(iced::Length::Fixed(8.0)),
                button(text(if sec.vault.is_some() {
                    i18n.tr("settings.security.master_password.change_action")
                } else {
                    i18n.tr("settings.security.master_password.init_action")
                }))
                .on_press(Message::VaultOpen)
                .style(style_chrome_secondary(tokens)),
                text(i18n.tr("settings.security.biometrics.title")).size(16).style(move |_t: &Theme| text::Style {
                    color: Some(text_primary),
                }),
                checkbox(sec.use_biometrics)
                    .label(i18n.tr("settings.security.biometrics.label"))
                    .on_toggle(Message::BiometricsToggle),
                text(i18n.tr("settings.security.biometrics.help")).size(11).style(move |_t: &Theme| text::Style {
                    color: Some(text_secondary),
                }),
            ]
            .spacing(layout::SETTINGS_ITEM_SPACING)
            .padding(layout::SETTINGS_CONTENT_PADDING as u16)
            .width(iced::Length::Fill)
            .into()
        }
        1 => {
            let policy = sec.host_key_policy;
            column![
                section_title(i18n.tr("settings.security.hosts.title"), tokens),
                text(i18n.tr("settings.security.hosts.policy.label")).size(14).style(move |_t: &Theme| text::Style {
                    color: Some(text_primary),
                }),
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
                text(i18n.tr("settings.security.hosts.policy.help")).size(11).style(move |_t: &Theme| text::Style {
                    color: Some(text_secondary),
                }),
                text(i18n.tr("settings.security.hosts.table.title")).size(14).style(move |_t: &Theme| text::Style {
                    color: Some(text_primary),
                }),
                text("(mock) 10.0.0.1:22  ·  ED25519").size(12).style(move |_t: &Theme| text::Style {
                    color: Some(text_secondary),
                }),
                text("(mock) github.com  ·  RSA").size(12).style(move |_t: &Theme| text::Style {
                    color: Some(text_secondary),
                }),
            ]
            .spacing(layout::SETTINGS_ITEM_SPACING)
            .padding(layout::SETTINGS_CONTENT_PADDING as u16)
            .width(iced::Length::Fill)
            .into()
        }
        _ => Space::new().into(),
    }
}

fn backup_pane(state: &IcedState, tokens: DesignTokens) -> Element<'_, Message> {
    let i18n = &state.model.i18n;
    let text_secondary = tokens.text_secondary;
    column![
        section_title(i18n.tr("iced.settings.backup.title"), tokens),
        text(i18n.tr("iced.settings.backup.hint")).size(13).style(move |_t: &Theme| text::Style {
            color: Some(text_secondary),
        }),
    ]
    .spacing(layout::SETTINGS_ITEM_SPACING)
    .padding(layout::SETTINGS_CONTENT_PADDING as u16)
    .width(iced::Length::Fill)
    .into()
}

fn settings_row<'a, L: Into<Element<'a, Message>>>(
    label: &'static str,
    right: L,
    tokens: DesignTokens,
) -> Element<'a, Message> {
    let text_primary = tokens.text_primary;
    row![
        container(
            text(label).size(14).style(move |_t: &Theme| text::Style {
                color: Some(text_primary),
            })
        )
            .width(iced::Length::FillPortion(1))
            .align_y(iced::alignment::Vertical::Top),
        container(right.into())
            .width(iced::Length::FillPortion(1))
            .align_x(iced::alignment::Horizontal::Right),
    ]
    .spacing(layout::SETTINGS_ITEM_SPACING)
    .align_y(Alignment::Start)
    .into()
}

fn restart_banner(state: &IcedState, tokens: DesignTokens) -> Element<'_, Message> {
    if !state.settings_needs_restart {
        return Space::new().height(0).into();
    }
    let i18n = &state.model.i18n;
    let text_primary = tokens.text_primary;
    let warning_color = tokens.warning;

    container(
        row![
            text(i18n.tr("iced.settings.restart.banner")).size(13).style(move |_t: &Theme| text::Style {
                color: Some(text_primary),
            }),
            Space::new().width(iced::Length::Fill),
            button(text(i18n.tr("iced.settings.restart.ok")))
                .on_press(Message::SettingsRestartAcknowledged)
                .style(style_chrome_primary(tokens)),
        ]
        .padding(12)
        .align_y(Alignment::Center),
    )
    .width(iced::Length::Fill)
    .style(move |_t: &Theme| {
        container::Style::default()
            .background(iced::Color::from_rgba(
                warning_color.r,
                warning_color.g,
                warning_color.b,
                0.15,
            ))
    })
    .into()
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 创建关闭图标按钮
fn icon_close_button(
    tokens: DesignTokens,
    btn_style: impl Fn(&Theme, button::Status) -> button::Style + 'static,
) -> Element<'static, Message> {
    let close_icon = icon_view_with(
        IconOptions::new(IconId::Close)
            .with_size(14)
            .with_color(tokens.text_secondary),
        Message::SettingsDismiss,
    );
    button(close_icon)
        .on_press(Message::SettingsDismiss)
        .width(iced::Length::Fixed(28.0))
        .height(iced::Length::Fixed(28.0))
        .style(btn_style)
        .into()
}
