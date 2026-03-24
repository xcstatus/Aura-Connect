pub mod theme;
pub mod components;
pub mod settings;

use eframe::egui;
use crate::ui::theme::Theme;

/// 通用的图标按钮渲染器
/// 适用于 Header、侧边栏或模态框等任何需要图标反馈的场景
pub fn render_icon_button(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    tooltip: &str,
    theme: &Theme,
    render_icon: impl FnOnce(&mut egui::Ui, &egui::Rect),
) -> egui::Response {
    let response = ui.interact(rect, ui.id().with(tooltip), egui::Sense::click());
    
    let bg_color = if response.is_pointer_button_down_on() {
        theme.surface_2
    } else if response.hovered() {
        theme.surface_1
    } else {
        egui::Color32::TRANSPARENT
    };
    
    if bg_color != egui::Color32::TRANSPARENT {
        ui.painter()
            .rect_filled(rect, egui::CornerRadius::same(6), bg_color);
    }
    render_icon(ui, &rect);
    // 调试边框 
    // ui.painter().rect_stroke(rect, egui::CornerRadius::ZERO, egui::Stroke::new(1.0, egui::Color32::RED), egui::StrokeKind::Inside);

    response.on_hover_text(tooltip)
}