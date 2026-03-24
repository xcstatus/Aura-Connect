use eframe::egui;

/// 图标管理器：负责处理所有顶栏图标的光学偏移与几何绘制、
pub struct IconRenderer;

impl IconRenderer {
    /// 渲染指定的图标到给定的矩形区域
    pub fn render(ui: &mut egui::Ui, rect: egui::Rect, text: &str, size: f32) {
        let text_color = egui::Color32::from_rgb(220, 220, 225);
        
        if text == "+" {
            // 手动几何绘制：确保绝对居中与视觉重量感一致 (1.5px 粗细)
            let center = rect.center();
            let h_line = (center + egui::vec2(-5.0, 0.0), center + egui::vec2(5.0, 0.0));
            let v_line = (center + egui::vec2(0.0, -5.0), center + egui::vec2(0.0, 5.0));
            let stroke = egui::Stroke::new(1.5, text_color);
            
            ui.painter().line_segment([h_line.0, h_line.1], stroke);
            ui.painter().line_segment([v_line.0, v_line.1], stroke);
        } else if text == "x" {
            // 手动绘制关闭按钮 (X)：旋转 45 度的两条对称线段
            let center = rect.center();
            let size = 4.0; // 较小的关闭按钮，更显精致
            let line1 = (center + egui::vec2(-size, -size), center + egui::vec2(size, size));
            let line2 = (center + egui::vec2(-size, size), center + egui::vec2(size, -size));
            let stroke = egui::Stroke::new(1.2, text_color); // 稍细一点，避免臃肿
            
            ui.painter().line_segment([line1.0, line1.1], stroke);
            ui.painter().line_segment([line2.0, line2.1], stroke);
        } else {
            // Emoji 字符：应用专属的光学重心补偿值 (Optical Offsets)
            let y_offset = match text {
                "⚡" => -1.5, // 闪电符号：上提抵消基线偏移
                "⚙" => -1.0, // 设置齿轮：轻微上提
                _ => 0.0,
            };

            ui.painter().text(
                rect.center() + egui::vec2(0.0, y_offset),
                egui::Align2::CENTER_CENTER, 
                text,
                egui::FontId::proportional(size),
                text_color,
            );
        }
    }
}
