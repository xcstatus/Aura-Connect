use eframe::egui;

use crate::app::RustSshApp;
use crate::session::ProtocolType;
use crate::ui_egui::theme::Theme;

pub(super) mod shared;
mod general;
mod advanced;
mod port_forwarding;
mod encryption;

pub fn render_modal(app: &mut RustSshApp, ctx: &egui::Context) {
    if app.editing_session.is_none() {
        return;
    }

    // Esc closes modal (spec)
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        app.editing_session = None;
        app.session_test_in_progress = false;
        return;
    }

    let theme = Theme::dark();
    let accent = theme.accent_base;
    let text_primary = egui::Color32::WHITE;
    let text_secondary = theme.text_secondary;
    let border_subtle = egui::Color32::from_rgba_premultiplied(255, 255, 255, 10);
    let error = egui::Color32::from_rgb(255, 59, 48);

    let content_rect = ctx.available_rect();
    egui::Area::new(egui::Id::new("modal_overlay_session_editor"))
        .order(egui::Order::Foreground)
        .fixed_pos(content_rect.min)
        .show(ctx, |ui| {
            ui.painter().rect_filled(
                content_rect,
                egui::CornerRadius::ZERO,
                egui::Color32::from_rgba_premultiplied(0, 0, 0, 153),
            );

            let panel_width = (content_rect.width() * 0.85).clamp(800.0, 1200.0);
            let panel_height = (content_rect.height() * 0.8).max(600.0);
            let panel_rect =
                egui::Rect::from_center_size(content_rect.center(), egui::vec2(panel_width, panel_height));

            let frame = egui::Frame::new()
                .fill(theme.bg_header)
                // .stroke(egui::Stroke::new(1.0, border_subtle))
                .corner_radius(egui::CornerRadius::same(12))
                .inner_margin(egui::Margin::ZERO)
                .shadow(egui::Shadow {
                    color: egui::Color32::from_black_alpha(120),
                    offset: [0, 8].into(),
                    blur: 24,
                    spread: 0,
                });

            ui.scope_builder(egui::UiBuilder::new().max_rect(panel_rect), |ui| {
                frame.show(ui, |ui| {
                    let top_h = 32.0;
                    let footer_h = 64.0;

                    // Top area with close button
                    ui.allocate_ui_with_layout(
                        egui::vec2(ui.available_width(), top_h),
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            // ui.add_space(8.0);
                            let r =
                                ui.allocate_exact_size(egui::vec2(32.0, 32.0), egui::Sense::hover()).0;
                            let resp = crate::ui_egui::render_icon_button(
                                ui,
                                r,
                                "关闭",
                                &theme,
                                |ui, rect| {
                                    ui.painter().text(
                                        rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        "✕",
                                        egui::FontId::proportional(14.0),
                                        text_secondary,
                                    );
                                },
                            );
                            if resp.clicked() {
                                app.editing_session = None;
                                app.session_test_in_progress = false;
                            }
                        },
                    );

                    // Single-column content (tabs + scroll + footer)
                    let content_h = ui.available_height() - footer_h;
                    let mut first_error_rect: Option<egui::Rect> = None;
                    let mut request_test = false;
                    let mut request_save = false;

                    egui::ScrollArea::vertical()
                        .id_salt("session_editor_scroll_single_column")
                        .max_height(content_h)
                        .show(ui, |ui| {
                            ui.add_space(24.0);

                            let Some(state) = app.editing_session.as_mut() else { return; };
                            state.protocol = ProtocolType::SSH;

                            // Top fixed grid: name / folder / test button
                            ui.horizontal(|ui| {
                                ui.add_space(24.0);
                                ui.spacing_mut().item_spacing = egui::vec2(16.0, 16.0);

                                ui.label(egui::RichText::new("名称:").size(12.0).color(text_primary));
                                ui.add_sized(
                                    [260.0, 32.0],
                                    egui::TextEdit::singleline(&mut state.name)
                                        .desired_width(f32::INFINITY)
                                        .margin(egui::vec2(12.0, 10.0))
                                        .background_color(theme.surface_1),
                                );
            
                                ui.label(egui::RichText::new("分组:").size(12.0).color(text_primary));
                                ui.add_sized(
                                    [130.0, 32.0],
                                    egui::TextEdit::singleline(&mut state.folder)
                                        .desired_width(f32::INFINITY)
                                        .margin(egui::vec2(12.0, 10.0))
                                        .background_color(theme.surface_1),
                                );

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    let btn = egui::Button::new(
                                        egui::RichText::new("测试连接").size(12.0).color(text_primary),
                                    )
                                    .min_size(egui::vec2(120.0, 36.0))
                                    .fill(theme.surface_1)
                                    // .stroke(egui::Stroke::new(1.0, border_subtle))
                                    .corner_radius(egui::CornerRadius::same(6));
                                    let resp = ui.add_enabled(!app.session_test_in_progress, btn);
                                    if app.session_test_in_progress {
                                        ui.put(
                                            egui::Rect::from_center_size(
                                                resp.rect.left_center() + egui::vec2(16.0, 0.0),
                                                egui::vec2(16.0, 16.0),
                                            ),
                                            egui::Spinner::new(),
                                        );
                                    }
                                    if resp.clicked() {
                                        request_test = true;
                                    }
                                });
                            });

                            ui.add_space(20.0);

                            // Tabs bar (single-column)
                            let tab_labels = ["通用", "高级设置", "端口转发", "加密方法"];
                            ui.horizontal(|ui| {
                                ui.add_space(24.0);
                                ui.spacing_mut().item_spacing.x = 16.0;
                                for (idx, label) in tab_labels.iter().enumerate() {
                                    let is_active = app.session_editor_tab == idx;
                                    let text = egui::RichText::new(*label)
                                        .size(13.0)
                                        .strong()
                                        .color(if is_active { accent } else { text_secondary });
                                    let resp = ui.add(egui::Label::new(text).sense(egui::Sense::click()));
                                    if resp.clicked() {
                                        app.session_editor_tab = idx;
                                    }
                                }
                            });
                            ui.add_space(16.0);

                            // Tab content split by file
                            match app.session_editor_tab {
                                0 => general::render(ui, app, &theme, border_subtle, error, &mut first_error_rect),
                                1 => advanced::render(ui, app, &theme, border_subtle),
                                2 => port_forwarding::render(ui, app, &theme, border_subtle),
                                3 => encryption::render(ui, app, &theme, border_subtle),
                                _ => {}
                            }

                            if ui.ctx().input(|i| i.key_pressed(egui::Key::Enter)) {
                                request_save = true;
                            }
                        });

                    // Footer (cancel/save)
                    let mut cancel_clicked = false;
                    let mut save_clicked = false;
                    let footer_rect =
                        ui.allocate_exact_size(egui::vec2(ui.available_width(), footer_h), egui::Sense::hover()).0;
                    ui.painter().hline(footer_rect.x_range(), footer_rect.top(), egui::Stroke::new(1.0, border_subtle));
                    ui.scope_builder(egui::UiBuilder::new().max_rect(footer_rect), |ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add_space(24.0);
                            let save = ui.add_sized(
                                [80.0, 36.0],
                                egui::Button::new(
                                    egui::RichText::new("保存")
                                        .size(12.0)
                                        .strong()
                                        .color(theme.text_on_accent_bg()),
                                )
                                .fill(accent)
                                .corner_radius(egui::CornerRadius::same(6)),
                            );
                            save_clicked = save.clicked();
                            ui.add_space(12.0);
                            let cancel = ui.add_sized(
                                [80.0, 36.0],
                                egui::Button::new(egui::RichText::new("取消").size(12.0).color(theme.accent_base))
                                    .fill(theme.surface_1)
                                    // .stroke(egui::Stroke::new(1.0, border_subtle))
                                    .corner_radius(egui::CornerRadius::same(6)),
                            );
                            cancel_clicked = cancel.clicked();
                        });
                    });

                    // Toast lifecycle
                    let now = ctx.input(|i| i.time);
                    app.session_toasts.retain(|t| now - t.created_at < 3.0);

                    // Test action
                    if request_test {
                        shared::start_test_connection(app, now);
                    }
                    if app.session_test_in_progress && now - app.session_test_started_at > 3.0 {
                        app.session_test_in_progress = false;
                    }

                    // Cancel action
                    if cancel_clicked {
                        app.editing_session = None;
                        app.session_test_in_progress = false;
                    }

                    // Save action
                    if save_clicked || request_save {
                        shared::save_session_or_toast(app, now, &mut first_error_rect);
                    }

                    // Toasts overlay
                    shared::render_toasts(ctx, &theme, &app.session_toasts);
                });
            });
        });
}

