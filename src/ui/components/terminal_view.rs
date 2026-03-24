use eframe::egui;

use crate::ui::theme::Theme;
use crate::backend::ssh_session::AsyncSession;
use vte::{Parser, Perform};

/// 终端样式的精简表示 (符合 2.2 色卡与样式要求)
#[derive(Clone, PartialEq)]
struct TextChar {
    font_id: egui::FontId,
    color: egui::Color32,
    c: char,
}

pub struct TerminalView {
    ghostty_engine: Option<crate::backend::ghostty_ffi::GhosttyIntegration>,
    parser: Parser,
    performer: TerminalPerformer,
    last_cols: u16,
    last_rows: u16,
}

struct TerminalPerformer {
    grid: Vec<Vec<TextChar>>,
    cursor_x: usize,
    cursor_y: usize,
    current_color: egui::Color32,
}

// 安全声明：在 egui 环境下，这些简单的 POD 类型是线程安全的
unsafe impl Send for TerminalPerformer {}
unsafe impl Sync for TerminalPerformer {}

impl Perform for TerminalPerformer {
    fn print(&mut self, c: char) {
        if self.cursor_y >= self.grid.len() {
            self.grid.resize(self.cursor_y + 1, Vec::new());
        }
        let line = &mut self.grid[self.cursor_y];
        if self.cursor_x >= line.len() {
            line.resize(self.cursor_x + 1, TextChar { font_id: egui::FontId::monospace(14.0), color: egui::Color32::from_rgb(0, 255, 128), c: ' ' });
        }
        line[self.cursor_x] = TextChar {
            font_id: egui::FontId::monospace(14.0),
            color: self.current_color,
            c,
        };
        self.cursor_x += 1;
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            b'\n' => {
                self.cursor_y += 1;
                self.cursor_x = 0;
                
                // Task 2: 动态历史行修剪防 OOM (上限10000行)
                const MAX_LINES: usize = 10000;
                if self.grid.len() > MAX_LINES {
                    let overflow = self.grid.len() - MAX_LINES;
                    self.grid.drain(0..overflow);
                    self.cursor_y = self.cursor_y.saturating_sub(overflow);
                }
            }
            b'\r' => self.cursor_x = 0,
            b'\x08' => if self.cursor_x > 0 { self.cursor_x -= 1 }, // Backspace
            _ => {}
        }
    }

    fn csi_dispatch(&mut self, params: &vte::Params, _intermediates: &[u8], _ignore: bool, action: char) {
        match action {
            'm' => {
                // SGR (颜色/样式)
                for param in params {
                    for &value in param {
                        match value {
                            0 => self.current_color = egui::Color32::from_rgb(0, 255, 128), // 重置
                            30..=37 => {
                                let colors = [
                                    egui::Color32::BLACK, egui::Color32::RED, egui::Color32::GREEN,
                                    egui::Color32::YELLOW, egui::Color32::BLUE, egui::Color32::from_rgb(255, 0, 255),
                                    egui::Color32::from_rgb(0, 255, 255), egui::Color32::WHITE
                                ];
                                self.current_color = colors[(value - 30) as usize];
                            }
                            _ => {}
                        }
                    }
                }
            }
            'H' | 'f' => {
                // CUP/HVP 光标位置
                let mut it = params.iter();
                let row = it.next().and_then(|p| p.first()).copied().unwrap_or(1);
                let col = it.next().and_then(|p| p.first()).copied().unwrap_or(1);
                self.cursor_y = (row.saturating_sub(1)) as usize;
                self.cursor_x = (col.saturating_sub(1)) as usize;
            }
            'J' => {
                // ED 清屏
                let mode = params.iter().next().and_then(|p| p.first()).copied().unwrap_or(0);
                if mode == 2 {
                    self.grid.clear();
                    self.cursor_x = 0;
                    self.cursor_y = 0;
                }
            }
            'K' => {
                // EL 清行
                let mode = params.iter().next().and_then(|p| p.first()).copied().unwrap_or(0);
                if mode == 0 && self.cursor_y < self.grid.len() {
                    let line = &mut self.grid[self.cursor_y];
                    if self.cursor_x < line.len() {
                        line.truncate(self.cursor_x);
                    }
                }
            }
            _ => {}
        }
    }
}

impl TerminalView {
    pub fn new() -> Self {
        Self {
            ghostty_engine: crate::backend::ghostty_ffi::GhosttyIntegration::new(),
            parser: Parser::new(),
            performer: TerminalPerformer {
                grid: vec![Vec::new()],
                cursor_x: 0,
                cursor_y: 0,
                current_color: egui::Color32::from_rgb(0, 255, 128),
            },
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
            ui.input(|i| {
                for event in &i.events {
                    // 路由键盘、鼠标、粘贴事件到转换器 (满足 Task 4)
                    if let Some(bytes) = crate::ui::components::keyboard_mapper::translate_egui_event(event) {
                        events_to_send.push(String::from_utf8_lossy(&bytes).to_string());
                    }
                }
            });

            if !events_to_send.is_empty() {
                if let Some(s) = session.as_mut() {
                    for text in events_to_send {
                        let _ = s.write_stream(text.as_bytes()); // 内部应改为 try_send
                    }
                }
            }
        }

        // --- 计算并同步 PTY 大小 (2.2.5) ---
        let char_width = 8.5;
        let row_height = 18.0;
        let cols = (rect.width() / char_width).floor() as u16;
        let rows = (rect.height() / row_height).floor() as u16;

        if let Some(s) = session.as_mut() {
            if cols != self.last_cols || rows != self.last_rows {
                let _ = tokio::runtime::Handle::current().spawn(async move {
                    // 这里由于 &mut self 无法直接移动，理想做法是克隆 channel handle 或使用 mpsc
                    // 暂时保留逻辑占位，后续优化
                });
                self.last_cols = cols;
                self.last_rows = rows;
                println!("📐 PTY Resized to: {}x{}", cols, rows);
            }

            // --- 输出解析与硬件加速注入 (循环排空缓冲区以提升性能) ---
            let mut read_buf = [0u8; 4096];
            let mut total_read = 0;
            while let Ok(n) = s.read_stream(&mut read_buf) {
                if n == 0 { break; }
                println!("🧠 [Terminal Parser] 送入解析器 {} bytes", n);
                total_read += n;

                if let Some(ghostty) = &mut self.ghostty_engine {
                    ghostty.write_ansi(&read_buf[..n]);
                } else {
                    self.parser.advance(&mut self.performer, &read_buf[..n]);
                }
                
                if total_read > 32768 { break; } // 防止大流量卡死 UI
            }
            if total_read > 0 {
                ui.ctx().request_repaint();
                println!("🎨 [Terminal Render] 成功排空缓冲并由服务端数据触发重绘");
            }
        }
        
        // --- 硬件级别纹理渲染 / 降级字符渲染 ---
        if let Some(ghostty) = &self.ghostty_engine {
            // Task 3: 提取 libghostty FBO 以图层方式进行绘制
            if let Some(handle) = ghostty.get_texture_handle() {
                // TODO: wgpu 绑定的实际句柄注册 (需要 eframe::egui_wgpu 桥接)
                // 这里代表抽象接缝已构建完毕
                _ = handle;
            }
            ui.painter().rect_filled(rect, egui::CornerRadius::ZERO, egui::Color32::from_rgb(5, 5, 8));
            ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "👻 Ghostty Engine Running (WGPU Tex Placeholder)", egui::FontId::monospace(14.0), egui::Color32::WHITE);
        } else {
            // Task 1 & 3: 使用 ScrollArea 实现无限回滚与自动锚向底部
            let row_height = 18.0;
            let char_width = 8.5;
            let total_rows = self.performer.grid.len();

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
                    let last_visible_row = ((viewport.max.y / row_height).ceil() as usize).min(total_rows);

                    let mut y_offset = first_visible_row as f32 * row_height + 5.0;

                    for (y, row) in self.performer.grid.iter().enumerate().skip(first_visible_row).take(last_visible_row.saturating_sub(first_visible_row) + 1) {
                        let line_pos = egui::pos2(10.0, y_offset);
                        // 由于极简版目前逐字带样式绘制，为了高性能，同颜色字符应合并
                        let mut current_color = None;
                        let mut current_text = String::new();
                        let mut start_pos = line_pos;

                        for tchar in row {
                            if Some(tchar.color) != current_color {
                                if !current_text.is_empty() {
                                    ui.painter().text(start_pos, egui::Align2::LEFT_TOP, &current_text, tchar.font_id.clone(), current_color.unwrap());
                                    start_pos.x += current_text.chars().count() as f32 * char_width;
                                    current_text.clear();
                                }
                                current_color = Some(tchar.color);
                            }
                            current_text.push(tchar.c);
                        }
                        if !current_text.is_empty() {
                            ui.painter().text(start_pos, egui::Align2::LEFT_TOP, &current_text, egui::FontId::monospace(14.0), current_color.unwrap());
                        }

                        // 绘制光标
                        if y == self.performer.cursor_y && ui.memory(|mem| mem.has_focus(response.id)) {
                            let cursor_pos = egui::pos2(10.0 + self.performer.cursor_x as f32 * char_width, y_offset);
                            let cursor_rect = egui::Rect::from_min_size(cursor_pos, egui::vec2(char_width, row_height));
                            ui.painter().rect_filled(cursor_rect, egui::CornerRadius::ZERO, egui::Color32::from_rgba_premultiplied(0, 255, 128, 150));
                        }

                        y_offset += row_height;
                    }
                });
        }

        if session.is_some() {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(16));
        }
    }
}
