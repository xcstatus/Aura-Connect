use eframe::egui;
use crate::app::RustSshApp;
use crate::ui_egui::theme::Theme;
use crate::ui_egui::settings::{render_row, render_switch};

pub fn render(app: &mut RustSshApp, ui: &mut egui::Ui, theme: &Theme, tab_idx: usize) {
    match tab_idx {
        0 => render_basic(app, ui, theme),
        1 => render_appearance(app, ui, theme),
        2 => render_typography(app, ui, theme),
        _ => {}
    }
}

fn render_basic(app: &mut RustSshApp, ui: &mut egui::Ui, theme: &Theme) {
    ui.add_space(8.0);
    ui.label(egui::RichText::new("启动与更新").size(18.0).strong().color(egui::Color32::WHITE));
    ui.add_space(24.0);
    
    render_row(ui, "界面显示语言", theme, |ui| {
        let mut lang = app.settings.general.language.clone();
        if egui::ComboBox::from_id_salt("lang_combo")
            .selected_text(if lang == "zh-CN" { "简体中文" } else { "English" })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut lang, "zh-CN".to_string(), "简体中文");
                ui.selectable_value(&mut lang, "en-US".to_string(), "English");
            }).response.changed() {
            app.settings.general.language = lang;
            app.i18n
                .set_locale(crate::i18n::Locale::from_language_code(&app.settings.general.language));
            let _ = app.settings.save();
            ui.ctx().request_repaint();
        }
    });

    render_row(ui, "启动时自动检查更新", theme, |ui| {
        if render_switch(ui, &mut app.settings.general.auto_check_update).changed() {
            let _ = app.settings.save();
        }
    });
}

fn render_appearance(app: &mut RustSshApp, ui: &mut egui::Ui, theme: &Theme) {
    ui.add_space(8.0);
    ui.label(egui::RichText::new("外观定制").size(18.0).strong().color(egui::Color32::WHITE));
    ui.add_space(24.0);
    
    render_row(ui, "应用主题模式", theme, |ui| {
        let mut current_theme = app.settings.general.theme.clone();
        if egui::ComboBox::from_id_salt("theme_combo")
            .selected_text(&current_theme)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut current_theme, "Dark".to_string(), "深色模式");
                ui.selectable_value(&mut current_theme, "Light".to_string(), "浅色模式");
                ui.selectable_value(&mut current_theme, "Warm".to_string(), "暖色护眼");
            }).response.changed() {
            app.settings.general.theme = current_theme;
            let _ = app.settings.save();
        }
    });

    render_row(ui, "主强调色", theme, |ui| {
        let color = theme.accent_base; // 从当前主题读取初始值
        // 使用 color_edit_button_srgba 直接操作并检测变更
        let mut rgba = [color.r(), color.g(), color.b(), color.a()];
        if ui.color_edit_button_srgba_unmultiplied(&mut rgba).changed() {
            let selected_color = egui::Color32::from_rgba_unmultiplied(rgba[0], rgba[1], rgba[2], rgba[3]);
            app.settings.general.accent_color = format!("#{:02X}{:02X}{:02X}", selected_color.r(), selected_color.g(), selected_color.b());
            let _ = app.settings.save();
        }
    });
}

fn render_typography(app: &mut RustSshApp, ui: &mut egui::Ui, theme: &Theme) {
    ui.add_space(8.0);
    ui.label(egui::RichText::new("文本渲染").size(18.0).strong().color(egui::Color32::WHITE));
    ui.add_space(24.0);
    
    render_row(ui, "界面全局字号", theme, |ui| {
        let mut size = app.settings.general.font_size;
        if ui.add(egui::Slider::new(&mut size, 12.0..=20.0).suffix("px")).changed() {
            app.settings.general.font_size = size;
            let _ = app.settings.save();
        }
    });
}
