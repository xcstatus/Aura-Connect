use eframe::egui;

use crate::ui::theme::Theme;
use crate::backend::ssh_session::AsyncSession;
use crate::backend::ghostty_ffi::GhosttyIntegration;

pub struct TerminalView {
    ghostty_engine: GhosttyIntegration,
    last_cols: u16,
    last_rows: u16,
}

impl TerminalView {
    pub fn new() -> Self {
        let mut engine = GhosttyIntegration::new().expect("Failed to initialize Ghostty engine!");
        engine.write_ansi(b"\x1b[31;1mHello Ghostty Native VT Debug!\x1b[0m\r\n");
        Self {
            ghostty_engine: engine,
            last_cols: 0,
            last_rows: 0,
        }
    }

    pub fn render(&mut self, ui: &mut egui::Ui, _theme: &Theme, session: &mut Option<Box<dyn AsyncSession>>) {
        let (rect, response) = ui.allocate_at_least(ui.available_size(), egui::Sense::click());
        
        if response.clicked() {
            ui.memory_mut(|mem| mem.request_focus(response.id));
        }
        
        // --- 输入处理 ---
        if ui.memory(|mem| mem.has_focus(response.id)) {
            let mut events_to_send = Vec::new();
            let events = ui.input(|i| i.events.clone());
            let modifiers = ui.input(|i| i.modifiers);
            let mods = Self::map_modifiers(&modifiers);
            
            for event in &events {
                match event {
                    egui::Event::Key { key, pressed, repeat, .. } => {
                        let action = if *repeat {
                            crate::backend::ghostty_ffi::GhosttyKeyAction_GHOSTTY_KEY_ACTION_REPEAT
                        } else if *pressed {
                            crate::backend::ghostty_ffi::GhosttyKeyAction_GHOSTTY_KEY_ACTION_PRESS
                        } else {
                            crate::backend::ghostty_ffi::GhosttyKeyAction_GHOSTTY_KEY_ACTION_RELEASE
                        };
                        let g_key = Self::map_key(*key);
                        if let Some(bytes) = self.ghostty_engine.process_key(action, g_key, mods, None) {
                            events_to_send.push(bytes);
                        }
                    }
                    egui::Event::Text(text) => {
                        let action = crate::backend::ghostty_ffi::GhosttyKeyAction_GHOSTTY_KEY_ACTION_PRESS;
                        if let Some(bytes) = self.ghostty_engine.process_key(action, 0, mods, Some(text)) {
                            events_to_send.push(bytes);
                        }
                    }
                    _ => {}
                }
            }

            if !events_to_send.is_empty() {
                if let Some(s) = session.as_mut() {
                    for bytes in events_to_send {
                        let _ = s.write_stream(&bytes); // 内部应改为 try_send
                    }
                }
            }
        }

        // --- 计算并同步 PTY 大小 (2.2.5) ---
        let char_width = 8.5;
        let row_height = 18.0;
        let cols = (rect.width() / char_width).floor() as u16;
        let rows = (rect.height() / row_height).floor() as u16;

        if cols != self.last_cols || rows != self.last_rows {
            self.ghostty_engine.resize(cols, rows);
            if let Some(s) = session.as_mut() {
                let _ = tokio::runtime::Handle::current().spawn(async move {
                    // resize pty on remote host...
                });
            }
            self.last_cols = cols;
            self.last_rows = rows;
            println!("📐 Ghostty Target Resized to: {}x{}", cols, rows);
        }

        if let Some(s) = session.as_mut() {
            // --- 输出解析与硬件加速注入 ---
            let mut read_buf = [0u8; 4096];
            let mut total_read = 0;
            while let Ok(n) = s.read_stream(&mut read_buf) {
                if n == 0 { break; }
                total_read += n;
                self.ghostty_engine.write_ansi(&read_buf[..n]);
                if total_read > 32768 { break; } // 防止大流量卡死 UI
            }
            if total_read > 0 {
                ui.ctx().request_repaint();
            }
        }
        
        let (total_rows, _offset, _len) = self.ghostty_engine.get_scrollbar().unwrap_or((rows as u64, 0, rows as u64));
        let (cursor_x, cursor_y) = self.ghostty_engine.get_cursor();
        
        // Debug
        // println!("⚡️ Scrollbar: total={}, offset={}, len={}", total_rows, _offset, _len);

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .stick_to_bottom(true)
            .show_viewport(ui, |ui, viewport| {
                
                // 背景
                ui.painter().rect_filled(ui.max_rect(), egui::CornerRadius::ZERO, egui::Color32::from_rgb(10, 10, 15));
                
                // 撑开内容总高度以便 Scrollbar 能真实反映进度
                ui.set_height(total_rows as f32 * row_height);

                // 视口切片裁剪 (Frustum Culling)
                let first_visible_row = (viewport.min.y / row_height).floor().max(0.0) as usize;
                let last_visible_row = ((viewport.max.y / row_height).ceil() as usize).min(total_rows as usize);

                let mut y_offset = first_visible_row as f32 * row_height + 5.0;

                let mut debug_rendered = false;

                for y in first_visible_row..=last_visible_row {
                    let line_pos = egui::pos2(ui.min_rect().min.x + 10.0, ui.min_rect().min.y + y_offset);
                    
                    // 通过 Ghostty 引擎直接获取当前格式化行 (Task 2 & 4)
                    let row_cells = self.ghostty_engine.get_line(y as u32, cols);
                    
                    if !debug_rendered {
                        // Print the first non-space characters from line to console
                        let l: String = row_cells.iter().map(|c| c.c).collect();
                        if l.trim().len() > 0 {
                            println!("📝 Render Line {}: {}", y, l);
                            debug_rendered = true; // only print first non-empty line per frame
                        }
                    }
                            
                    // 由于极简版目前逐字带样式绘制，为了高性能，同颜色字符应合并
                    let mut current_color = None;
                    let mut current_text = String::new();
                    let mut start_pos = line_pos;

                    for cell in row_cells {
                        let egui_fg = egui::Color32::from_rgb(cell.fg[0], cell.fg[1], cell.fg[2]);
                        if Some(egui_fg) != current_color {
                            if !current_text.is_empty() {
                                ui.painter().text(start_pos, egui::Align2::LEFT_TOP, &current_text, egui::FontId::monospace(14.0), current_color.unwrap());
                                start_pos.x += current_text.chars().count() as f32 * char_width;
                                current_text.clear();
                            }
                            current_color = Some(egui_fg);
                        }
                        current_text.push(cell.c);
                    }
                    if !current_text.is_empty() {
                        ui.painter().text(start_pos, egui::Align2::LEFT_TOP, &current_text, egui::FontId::monospace(14.0), current_color.unwrap());
                    }

                    // 绘制光标 (只在可见的主视窗底部渲染，这里简化为当 y 匹配且是最后部分时渲染)
                    // TODO: 将 cursor_y 精确匹配到 viewport 上的 active area Row
                    if (total_rows as usize).saturating_sub(rows as usize) + cursor_y as usize == y && ui.memory(|mem| mem.has_focus(response.id)) {
                        let cursor_pos = egui::pos2(ui.min_rect().min.x + 10.0 + cursor_x as f32 * char_width, ui.min_rect().min.y + y_offset);
                        let cursor_rect = egui::Rect::from_min_size(cursor_pos, egui::vec2(char_width, row_height));
                        ui.painter().rect_filled(cursor_rect, egui::CornerRadius::ZERO, egui::Color32::from_rgba_premultiplied(0, 255, 128, 150));
                    }

                    y_offset += row_height;
                }
            });

        if session.is_some() {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(16));
        }
    }

    fn map_modifiers(m: &egui::Modifiers) -> u16 {
        let mut mods = 0;
        if m.shift { mods |= crate::backend::ghostty_ffi::GHOSTTY_MODS_SHIFT as u16; }
        if m.ctrl { mods |= crate::backend::ghostty_ffi::GHOSTTY_MODS_CTRL as u16; }
        if m.alt { mods |= crate::backend::ghostty_ffi::GHOSTTY_MODS_ALT as u16; }
        if m.mac_cmd || m.command { mods |= crate::backend::ghostty_ffi::GHOSTTY_MODS_SUPER as u16; }
        mods
    }

    fn map_key(k: egui::Key) -> u32 {
        use crate::backend::ghostty_ffi::*;
        match k {
            egui::Key::A => GhosttyKey_GHOSTTY_KEY_A,
            egui::Key::B => GhosttyKey_GHOSTTY_KEY_B,
            egui::Key::C => GhosttyKey_GHOSTTY_KEY_C,
            egui::Key::D => GhosttyKey_GHOSTTY_KEY_D,
            egui::Key::E => GhosttyKey_GHOSTTY_KEY_E,
            egui::Key::F => GhosttyKey_GHOSTTY_KEY_F,
            egui::Key::G => GhosttyKey_GHOSTTY_KEY_G,
            egui::Key::H => GhosttyKey_GHOSTTY_KEY_H,
            egui::Key::I => GhosttyKey_GHOSTTY_KEY_I,
            egui::Key::J => GhosttyKey_GHOSTTY_KEY_J,
            egui::Key::K => GhosttyKey_GHOSTTY_KEY_K,
            egui::Key::L => GhosttyKey_GHOSTTY_KEY_L,
            egui::Key::M => GhosttyKey_GHOSTTY_KEY_M,
            egui::Key::N => GhosttyKey_GHOSTTY_KEY_N,
            egui::Key::O => GhosttyKey_GHOSTTY_KEY_O,
            egui::Key::P => GhosttyKey_GHOSTTY_KEY_P,
            egui::Key::Q => GhosttyKey_GHOSTTY_KEY_Q,
            egui::Key::R => GhosttyKey_GHOSTTY_KEY_R,
            egui::Key::S => GhosttyKey_GHOSTTY_KEY_S,
            egui::Key::T => GhosttyKey_GHOSTTY_KEY_T,
            egui::Key::U => GhosttyKey_GHOSTTY_KEY_U,
            egui::Key::V => GhosttyKey_GHOSTTY_KEY_V,
            egui::Key::W => GhosttyKey_GHOSTTY_KEY_W,
            egui::Key::X => GhosttyKey_GHOSTTY_KEY_X,
            egui::Key::Y => GhosttyKey_GHOSTTY_KEY_Y,
            egui::Key::Z => GhosttyKey_GHOSTTY_KEY_Z,
            egui::Key::Num0 => GhosttyKey_GHOSTTY_KEY_DIGIT_0,
            egui::Key::Num1 => GhosttyKey_GHOSTTY_KEY_DIGIT_1,
            egui::Key::Num2 => GhosttyKey_GHOSTTY_KEY_DIGIT_2,
            egui::Key::Num3 => GhosttyKey_GHOSTTY_KEY_DIGIT_3,
            egui::Key::Num4 => GhosttyKey_GHOSTTY_KEY_DIGIT_4,
            egui::Key::Num5 => GhosttyKey_GHOSTTY_KEY_DIGIT_5,
            egui::Key::Num6 => GhosttyKey_GHOSTTY_KEY_DIGIT_6,
            egui::Key::Num7 => GhosttyKey_GHOSTTY_KEY_DIGIT_7,
            egui::Key::Num8 => GhosttyKey_GHOSTTY_KEY_DIGIT_8,
            egui::Key::Num9 => GhosttyKey_GHOSTTY_KEY_DIGIT_9,
            egui::Key::Enter => GhosttyKey_GHOSTTY_KEY_ENTER,
            egui::Key::Space => GhosttyKey_GHOSTTY_KEY_SPACE,
            egui::Key::Backspace => GhosttyKey_GHOSTTY_KEY_BACKSPACE,
            egui::Key::Tab => GhosttyKey_GHOSTTY_KEY_TAB,
            egui::Key::Escape => GhosttyKey_GHOSTTY_KEY_ESCAPE,
            egui::Key::ArrowDown => GhosttyKey_GHOSTTY_KEY_ARROW_DOWN,
            egui::Key::ArrowLeft => GhosttyKey_GHOSTTY_KEY_ARROW_LEFT,
            egui::Key::ArrowRight => GhosttyKey_GHOSTTY_KEY_ARROW_RIGHT,
            egui::Key::ArrowUp => GhosttyKey_GHOSTTY_KEY_ARROW_UP,
            egui::Key::Home => GhosttyKey_GHOSTTY_KEY_HOME,
            egui::Key::End => GhosttyKey_GHOSTTY_KEY_END,
            egui::Key::PageUp => GhosttyKey_GHOSTTY_KEY_PAGE_UP,
            egui::Key::PageDown => GhosttyKey_GHOSTTY_KEY_PAGE_DOWN,
            egui::Key::Insert => GhosttyKey_GHOSTTY_KEY_INSERT,
            egui::Key::Delete => GhosttyKey_GHOSTTY_KEY_DELETE,
            _ => GhosttyKey_GHOSTTY_KEY_UNIDENTIFIED,
        }
    }
}
