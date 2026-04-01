use eframe::egui;
use crate::app::RustSshApp;
use crate::ui_egui::theme::Theme;
use crate::icons::IconRenderer;

pub fn render_header(app: &mut RustSshApp, ui: &mut egui::Ui, theme: &Theme) {
    let header_height = 32.0;
    // Keep a stable top-bar layout even when there are no tabs/buttons.
    ui.set_min_height(header_height);
    let ctx = ui.ctx();
    let header_rect = egui::Rect::from_min_size(
        ui.cursor().min,
        egui::vec2(ui.available_width(), header_height),
    );

    // 绘制顶栏背景
    ui.painter().rect_filled(
        header_rect,
        egui::CornerRadius { nw: 10, ne: 10, sw: 0, se: 0 }, 
        theme.bg_header,
    );

    // 顶栏拖拽响应区
    let header_response = ui.interact(header_rect, ui.id().with("header_area"), egui::Sense::click_and_drag());
    if header_response.double_clicked() {
        let is_maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!is_maximized));
    }
    if header_response.dragged() {
        ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }

    ui.scope_builder(egui::UiBuilder::new().max_rect(header_rect), |ui| {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
        ui.horizontal(|ui: &mut egui::Ui| {
            #[cfg(target_os = "macos")]
            ui.add_space(75.0); 

            let reserved_right = 160.0; 
            let tabs_max_width = ui.available_width() - reserved_right;
            
            let scroll_area = egui::ScrollArea::horizontal()
                .max_width(tabs_max_width)
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden);
            
            let scroll_output = scroll_area.show(ui, |ui: &mut egui::Ui| {
                ui.horizontal(|ui: &mut egui::Ui| {
                    ui.spacing_mut().item_spacing.x = 0.0; 
                    let mut tab_to_close = None;
                    let mut tab_to_select = None;

                    for (i, tab_title) in app.tabs.iter().enumerate() {
                         let is_selected = i == app.active_tab_index;
                         let (rect, response) = ui.allocate_at_least(egui::vec2(140.0, header_height), egui::Sense::click());

                         if response.clicked() { 
                             tab_to_select = Some(i);
                             // 优化：扩展目标矩形，确保滚动后避开左右两侧各 40px 的渐变遮罩区
                             let mut scroll_rect = rect;
                             scroll_rect.min.x -= 40.0;
                             scroll_rect.max.x += 40.0;
                             ui.scroll_to_rect(scroll_rect, None); 
                         }

                         let tab_bg = if is_selected {
                             theme.tab_active
                         } else if response.hovered() {
                             theme.surface_1
                         } else {
                             egui::Color32::TRANSPARENT
                         };

                         ui.painter().rect_filled(rect, egui::CornerRadius { nw: 8, ne: 8, sw: 0, se: 0 }, tab_bg);
                         if is_selected {
                             ui.painter().rect_filled(
                                 egui::Rect::from_min_max(rect.left_bottom() + egui::vec2(0.0, -2.0), rect.right_bottom()),
                                 egui::CornerRadius::ZERO,
                                 theme.accent_base,
                             );
                         }

                         let text_pos = rect.left_center() + egui::vec2(12.0, 0.0);
                         ui.painter().text(
                             text_pos,
                             egui::Align2::LEFT_CENTER,
                             tab_title,
                             egui::FontId::proportional(13.0),
                             if is_selected { theme.text_primary } else { theme.text_secondary },
                         );

                         let close_btn_rect = egui::Rect::from_center_size(rect.right_center() - egui::vec2(16.0, 0.0), egui::vec2(16.0, 16.0));
                         let close_resp = ui.interact(close_btn_rect, ui.id().with(("close", i)), egui::Sense::click());

                         let close_btn_bg = if close_resp.is_pointer_button_down_on() {
                             theme.surface_3
                         } else if close_resp.hovered() {
                             theme.surface_2
                         } else {
                             egui::Color32::TRANSPARENT
                         };

                         if close_btn_bg != egui::Color32::TRANSPARENT {
                             ui.painter().rect_filled(close_btn_rect, egui::CornerRadius::same(4), close_btn_bg);
                         }
                         if response.hovered() || close_resp.hovered() || is_selected {
                             IconRenderer::render(ui, close_btn_rect, "x", 10.0);
                         }
                         if close_resp.clicked() { tab_to_close = Some(i); }
                         
                         if !is_selected && i < app.tabs.len() - 1 {
                             ui.painter().vline(rect.right(), rect.y_range().shrink(8.0), ui.visuals().widgets.noninteractive.bg_stroke);
                         }
                    }

                    if let Some(idx) = tab_to_select { app.active_tab_index = idx; }
                    if let Some(idx) = tab_to_close {
                        app.tabs.remove(idx);
                        if app.tabs.is_empty() {
                            app.active_tab_index = 0;
                        } else if app.active_tab_index >= app.tabs.len() {
                            app.active_tab_index = app.tabs.len() - 1;
                        }
                    }
                });
            });

            // 渐变遮罩
            let mask_rect = scroll_output.inner_rect;
            if scroll_output.state.offset.x < (scroll_output.content_size.x - scroll_output.inner_rect.width() - 1.0) {
                for i in 0..10 {
                    let alpha = (i as f32 / 10.0 * 255.0) as u8;
                    let width = 4.0;
                    let r = egui::Rect::from_min_max(
                        mask_rect.right_top() - egui::vec2(width * (10 - i) as f32, 0.0),
                        mask_rect.right_bottom() - egui::vec2(width * (9 - i) as f32, 0.0)
                    );
                    ui.painter().rect_filled(r, egui::CornerRadius::ZERO, theme.bg_header.gamma_multiply(alpha as f32 / 255.0));
                }
            }
            if scroll_output.state.offset.x > 1.0 {
                 for i in 0..10 {
                     let alpha = ((10 - i) as f32 / 10.0 * 255.0) as u8;
                     let width = 4.0;
                     let r = egui::Rect::from_min_max(
                         mask_rect.left_top() + egui::vec2(width * i as f32, 0.0),
                         mask_rect.left_bottom() + egui::vec2(width * (i + 1) as f32, 0.0)
                     );
                     ui.painter().rect_filled(r, egui::CornerRadius::ZERO, theme.bg_header.gamma_multiply(alpha as f32 / 255.0));
                 }
            }

            // 操作组
            let separator_x = ui.cursor().min.x + 8.0;
            let separator_y_mid = header_rect.center().y;
            ui.painter().vline(separator_x, (separator_y_mid - 6.0)..=(separator_y_mid + 6.0), egui::Stroke::new(1.0, egui::Color32::from_gray(80)));
            
            let mut current_x = separator_x + 8.0;

            if app.show_add_button {
                let rect = egui::Rect::from_min_size(egui::pos2(current_x, header_rect.min.y), egui::vec2(32.0, header_rect.height()));
                let resp = crate::ui_egui::render_icon_button(
                    ui,
                    rect,
                    "新建标签页",
                    theme,
                    |ui, rect| {
                        IconRenderer::render(ui, *rect, "+", 18.0);
                    },
                );
                if resp.clicked() {
                    app.tabs.push(format!("session-{}", app.tabs.len() + 1));
                    app.active_tab_index = app.tabs.len().saturating_sub(1);
                }
                current_x += 32.0;
            }

            let rect = egui::Rect::from_min_size(egui::pos2(current_x, header_rect.min.y), egui::vec2(32.0, header_rect.height()));
            let resp = crate::ui_egui::render_icon_button(
                ui,
                rect,
                "快速连接",
                theme,
                |ui, rect| {
                    IconRenderer::render(ui, *rect, "⚡", 15.0);
                },
            );
            if resp.clicked() {
                app.show_quick_connect = !app.show_quick_connect;
            }

            // 控制组
            let current_right_x = header_rect.max.x - 32.0;
            let right_sep_x = current_right_x - 8.0;
            ui.painter().vline(right_sep_x, (separator_y_mid - 6.0)..=(separator_y_mid + 6.0), egui::Stroke::new(1.0, egui::Color32::from_gray(80)));
            
            let rect = egui::Rect::from_min_size(egui::pos2(current_right_x, header_rect.min.y), egui::vec2(32.0, header_rect.height()));
            let resp = crate::ui_egui::render_icon_button(
                ui,
                rect,
                "设置中心",
                theme,
                |ui, rect| {
                    IconRenderer::render(ui, *rect, "⚙", 15.0);
                },
            );
            if resp.clicked() {
                app.show_settings = true;
            }

            #[cfg(not(target_os = "macos"))]
            {
                let mut win_ctrl_x = current_right_x; 
                let win_btn_color = egui::Color32::from_gray(180);
                let mut render_win_ctrl = |ui: &mut egui::Ui, text: &str| -> egui::Response {
                    win_ctrl_x -= 32.0;
                    let rect = egui::Rect::from_min_size(egui::pos2(win_ctrl_x, header_rect.min.y), egui::vec2(32.0, header_rect.height()));
                    let response = ui.interact(rect, ui.id().with(text), egui::Sense::click());
                    if response.hovered() {
                        let bg = if text == "✕" { egui::Color32::from_rgba_premultiplied(255, 0, 0, 50) } else { egui::Color32::from_rgba_premultiplied(255, 255, 255, 15) };
                        ui.painter().rect_filled(rect, egui::CornerRadius::ZERO, bg);
                    }
                    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, text, egui::FontId::proportional(14.0), win_btn_color);
                    response
                };
                if render_win_ctrl(ui, "✕").clicked() { ctx.send_viewport_cmd(egui::ViewportCommand::Close); }
                if render_win_ctrl(ui, "⬜").clicked() { 
                    let is_max = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!is_max));
                }
                if render_win_ctrl(ui, "—").clicked() { ctx.send_viewport_cmd(egui::ViewportCommand::Minimized); }
            }
            
            if app.needs_restart {
                render_restart_banner(ui, theme, app);
            }
        });
    });
}

fn render_restart_banner(ui: &mut egui::Ui, _theme: &Theme, app: &mut RustSshApp) {
    let banner_height = 36.0;
    let width = ui.available_width();
    let (rect, _) = ui.allocate_at_least(egui::vec2(width, banner_height), egui::Sense::hover());
    
    // 背景 (黄色/橙色提示)
    ui.painter().rect_filled(rect, egui::CornerRadius::ZERO, egui::Color32::from_rgb(255, 170, 0).gamma_multiply(0.2));
    ui.painter().hline(rect.x_range(), rect.bottom(), egui::Stroke::new(1.0, egui::Color32::from_rgb(255, 170, 0).gamma_multiply(0.3)));

    ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
        ui.horizontal(|ui| {
            ui.add_space(16.0);
            ui.label(egui::RichText::new("⚠️ 部分设置需要重启应用后生效。").color(egui::Color32::from_rgb(255, 200, 0)));
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(16.0);
                if ui.button("稍后处理").clicked() {
                    app.needs_restart = false;
                }
                if ui.button(egui::RichText::new("立即重启").strong()).clicked() {
                    // 模拟重启逻辑
                    std::process::Command::new(std::env::current_exe().unwrap()).spawn().expect("failed to restart");
                    std::process::exit(0);
                }
            });
        });
    });
}
