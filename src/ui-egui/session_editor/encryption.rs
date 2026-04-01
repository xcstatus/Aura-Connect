use eframe::egui;

use crate::app::RustSshApp;
use crate::ui_egui::theme::Theme;

use super::shared::session_group;

pub fn render(ui: &mut egui::Ui, _app: &mut RustSshApp, theme: &Theme, border_subtle: egui::Color32) {
    session_group(ui, theme, border_subtle, "加密方法（Encryption）", |ui| {
        ui.label(
            egui::RichText::new("（本页签已拆分文件，后续按设计稿实现算法选择/压缩/HMAC）")
                .size(12.0)
                .color(theme.text_secondary),
        );
    });
}

