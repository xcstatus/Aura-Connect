use eframe::egui;
use crate::app::{RustSshApp, SettingTab};
use crate::ui::theme::Theme;

mod general;
mod terminal;
mod security;

pub fn render_settings_modal(app: &mut RustSshApp, ctx: &egui::Context, theme: &Theme) {
    if !app.show_settings {
        return;
    }

    let screen_rect = ctx.content_rect();
    // 强制按照原图比例 1000x800
    let modal_width = 1000.0f32.min(screen_rect.width() * 0.9);
    let modal_height = 800.0f32.min(screen_rect.height() * 0.85);

    egui::Window::new("Settings")
        .title_bar(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .fixed_size(egui::vec2(modal_width, modal_height))
        .frame(egui::Frame::NONE.fill(egui::Color32::from_rgb(28, 28, 30)).corner_radius(egui::CornerRadius::same(12)))
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
            
            ui.horizontal(|ui| {
                // 1. Sidebar (Width 220px, bg #232326)
                ui.vertical(|ui| {
                    ui.set_width(220.0);
                    ui.set_height(modal_height);
                    render_sidebar(app, ui, theme, modal_height);
                });
                
                // 2. Right Content (Width 780px, bg #1C1C1E)
                ui.vertical(|ui| {
                    ui.set_width(modal_width - 220.0);
                    ui.set_height(modal_height);
                    
                    render_header(app, ui, theme);
                    render_tab_bar(app, ui, theme);
                    
                    // Main Content Scroll Area
                    egui::ScrollArea::vertical()
                        .id_salt("settings_main_scroll")
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.add_space(24.0); // Side padding
                                ui.vertical(|ui| {
                                    ui.set_width(modal_width - 220.0 - 48.0);
                                    ui.add_space(38.0); // y=110 relative to top (32+40=72)
                                    render_current_tab_content(app, ui, theme);
                                });
                                ui.add_space(24.0);
                            });
                        });
                        
                    // Sticky Restart Banner
                    if app.needs_restart {
                        ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                            render_restart_banner(app, ui, theme);
                        });
                    }
                });
            });
        });
}

fn render_sidebar(app: &mut RustSshApp, ui: &mut egui::Ui, _theme: &Theme, _height: f32) {
    // Background #232326
    let bg_rect = ui.available_rect_before_wrap();
    ui.painter().rect_filled(bg_rect, egui::CornerRadius::ZERO, egui::Color32::from_rgb(35, 35, 38));
    
    // Right border #3A3A3F
    ui.painter().vline(bg_rect.right() - 1.0, bg_rect.y_range(), egui::Stroke::new(1.0, egui::Color32::from_rgb(58, 58, 63)));

    ui.vertical(|ui| {
        ui.add_space(36.0); // y=36
        
        let tabs = [
            (SettingTab::General, "通用设置"),
            (SettingTab::Terminal, "终端配置"),
            (SettingTab::Security, "安全隐私"),
            (SettingTab::Backup, "备份同步"),
        ];

        for (tab, label) in tabs {
            let is_active = app.active_settings_tab == tab;
            let mut rect = ui.available_rect_before_wrap();
            rect.set_height(40.0);
            
            let response = ui.interact(rect, ui.id().with(label), egui::Sense::click());
            
            if is_active {
                // Background Highlight (x=8, width=184)
                let mut bg_highlight = rect;
                bg_highlight.min.x += 8.0;
                bg_highlight.max.x -= 8.0;
                ui.painter().rect_filled(bg_highlight, egui::CornerRadius::same(6), egui::Color32::from_rgb(58, 58, 63));
                
                // Active Strip (x=4, width=2)
                let mut strip = rect;
                strip.min.x = rect.min.x + 4.0;
                strip.max.x = strip.min.x + 2.0;
                ui.painter().rect_filled(strip, egui::CornerRadius::ZERO, egui::Color32::from_rgb(0, 255, 128));
            } else if response.hovered() {
                let mut bg_hover = rect;
                bg_hover.min.x += 8.0;
                bg_hover.max.x -= 8.0;
                ui.painter().rect_filled(bg_hover, egui::CornerRadius::same(6), egui::Color32::from_rgba_premultiplied(255, 255, 255, 5));
            }

            ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                ui.horizontal_centered(|ui| {
                    ui.add_space(24.0); // text x=24
                    let text_color = if is_active { egui::Color32::WHITE } else { egui::Color32::from_rgb(160, 160, 165) };
                    ui.label(egui::RichText::new(label).size(13.0).color(text_color));
                });
            });

            if response.clicked() {
                app.active_settings_tab = tab;
            }
            ui.add_space(8.0);
        }
    });
}

fn render_header(app: &mut RustSshApp, ui: &mut egui::Ui, theme: &Theme) {
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), 32.0),
        egui::Layout::right_to_left(egui::Align::Center),
        |ui| {
            ui.add_space(8.0); // Right padding
            let (rect, _response) = ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::click());
            
            // 使用公共组件 render_icon_button 处理交互与反馈
            if crate::ui::render_icon_button(ui, rect, "close_settings", theme, |ui, rect| {
                // 绘制图标逻辑
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "✕",
                    egui::FontId::proportional(12.0),
                    egui::Color32::from_rgb(160, 160, 165)
                );
            }).clicked() {
                app.show_settings = false;
            }
        }
    );
}

fn render_tab_bar(app: &mut RustSshApp, ui: &mut egui::Ui, _theme: &Theme) {
    let tab_items = match app.active_settings_tab {
        SettingTab::General => vec!["基础设置", "外观定制", "文本渲染"],
        SettingTab::Terminal => vec!["渲染引擎", "内置方案", "文本策略", "交互行为"],
        SettingTab::Security => vec!["安全策略", "主机指纹"],
        _ => vec!["默认页签"],
    };

    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), 40.0),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.add_space(24.0); // x=244 (sidebar 220 + space 24)
            let current_tab_idx = *app.active_tab_indices.entry(app.active_settings_tab).or_insert(0);
            
            for (idx, label) in tab_items.iter().enumerate() {
                let is_active = idx == current_tab_idx;
                let response = ui.selectable_label(false, egui::RichText::new(*label)
                    .size(13.0)
                    .color(if is_active { egui::Color32::from_rgb(0, 255, 128) } else { egui::Color32::from_rgb(160, 160, 165) })
                    .strong());
                
                if is_active {
                    let mut line_rect = response.rect;
                    line_rect.min.y = line_rect.max.y + 10.0;
                    line_rect.max.y = line_rect.min.y + 2.0;
                    // 指示条宽度 60px 左右
                    line_rect.min.x = response.rect.center().x - 30.0;
                    line_rect.max.x = response.rect.center().x + 30.0;
                    ui.painter().rect_filled(line_rect, egui::CornerRadius::ZERO, egui::Color32::from_rgb(0, 255, 128));
                }
                
                if response.clicked() {
                    app.active_tab_indices.insert(app.active_settings_tab, idx);
                }
                ui.add_space(20.0);
            }
            
            // Bottom Border #3A3A3F
            let border_y = ui.max_rect().bottom();
            ui.painter().hline(ui.max_rect().x_range(), border_y, egui::Stroke::new(1.0, egui::Color32::from_rgb(58, 58, 63)));
        }
    );
}

fn render_restart_banner(app: &mut RustSshApp, ui: &mut egui::Ui, _theme: &Theme) {
    let amber = egui::Color32::from_rgb(255, 170, 0);
    let rect = ui.allocate_exact_size(egui::vec2(ui.available_width(), 50.0), egui::Sense::hover()).0;
    
    // Background rgba(255,170,0,0.15)
    ui.painter().rect_filled(rect, egui::CornerRadius::ZERO, amber.gamma_multiply(0.15));
    ui.painter().hline(rect.x_range(), rect.top(), egui::Stroke::new(1.0, amber.gamma_multiply(0.3)));
    
    ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
        ui.horizontal_centered(|ui| {
            ui.add_space(24.0);
            ui.label(egui::RichText::new("⚠️ 部分设置需要重启应用后生效。").color(amber).size(13.0));
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(24.0);
                // 立即重启 (96x32)
                let r_rect = ui.allocate_exact_size(egui::vec2(96.0, 32.0), egui::Sense::click()).0;
                let r_resp = ui.interact(r_rect, ui.id().with("restart_confirm"), egui::Sense::click());
                ui.painter().rect_filled(r_rect, egui::CornerRadius::same(6), amber);
                ui.painter().text(r_rect.center(), egui::Align2::CENTER_CENTER, "立即重启", egui::FontId::proportional(13.0), egui::Color32::BLACK);
                
                if r_resp.clicked() { app.needs_restart = false; }
                
                ui.add_space(12.0);
                
                // 稍后处理 (88x32)
                let l_rect = ui.allocate_exact_size(egui::vec2(88.0, 32.0), egui::Sense::click()).0;
                let l_resp = ui.interact(l_rect, ui.id().with("restart_later"), egui::Sense::click());
                ui.painter().rect_stroke(l_rect, egui::CornerRadius::same(6), egui::Stroke::new(1.0, egui::Color32::from_rgb(58, 58, 63)), egui::StrokeKind::Inside);
                ui.painter().text(l_rect.center(), egui::Align2::CENTER_CENTER, "稍后处理", egui::FontId::proportional(13.0), egui::Color32::from_rgb(160, 160, 165));
                
                if l_resp.clicked() { app.needs_restart = false; }
            });
        });
    });
}

fn render_current_tab_content(app: &mut RustSshApp, ui: &mut egui::Ui, theme: &Theme) {
    let tab_idx = *app.active_tab_indices.entry(app.active_settings_tab).or_insert(0);
    match app.active_settings_tab {
        SettingTab::General => general::render(app, ui, theme, tab_idx),
        SettingTab::Terminal => terminal::render(app, ui, theme, tab_idx),
        SettingTab::Security => security::render(app, ui, theme, tab_idx),
        _ => { ui.label("WIP..."); }
    }
}

// --- 外部通用的布局组件 (辅助函数) ---

pub fn render_row(ui: &mut egui::Ui, label: &str, _theme: &Theme, add_contents: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).size(14.0).color(egui::Color32::WHITE));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), add_contents);
    });
    ui.add_space(14.0);
    let rect = ui.available_rect_before_wrap();
    ui.painter().hline(rect.min.x..=rect.max.x, ui.next_widget_position().y, egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 45, 50)));
    ui.add_space(14.0);
}

pub fn render_switch(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let size = egui::vec2(44.0, 24.0);
    let (rect, mut response) = ui.allocate_exact_size(size, egui::Sense::click());
    if response.clicked() { *on = !*on; response.mark_changed(); }
    let how_on = ui.ctx().animate_bool(response.id.with("sw"), *on);
    let bg = if *on { egui::Color32::from_rgb(0, 255, 128) } else { egui::Color32::from_rgb(58, 58, 63) };
    ui.painter().rect_filled(rect, egui::CornerRadius::same(12), bg);
    let circle_x = egui::lerp(rect.left() + 12.0..=rect.right() - 12.0, how_on);
    ui.painter().circle_filled(egui::pos2(circle_x, rect.center().y), 10.0, egui::Color32::WHITE);
    response
}
