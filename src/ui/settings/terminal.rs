use eframe::egui;
use crate::app::RustSshApp;
use crate::ui::theme::Theme;
use crate::ui::settings::{render_row, render_switch};

pub fn render(app: &mut RustSshApp, ui: &mut egui::Ui, theme: &Theme, tab_idx: usize) {
    match tab_idx {
        0 => render_rendering(app, ui, theme),
        1 => render_themes(app, ui, theme),
        2 => render_text(app, ui, theme),
        3 => render_interaction(app, ui, theme),
        _ => { ui.label("Coming soon..."); }
    }
}

fn render_rendering(app: &mut RustSshApp, ui: &mut egui::Ui, theme: &Theme) {
    ui.add_space(8.0);
    ui.label(egui::RichText::new("渲染引擎").size(18.0).strong().color(egui::Color32::WHITE));
    ui.add_space(24.0);
    
    render_row(ui, "启用 GPU 加速渲染", theme, |ui| {
        if render_switch(ui, &mut app.settings.terminal.gpu_acceleration).changed() {
            let _ = app.settings.save();
            app.needs_restart = true;
        }
    });
    
    render_row(ui, "目标渲染帧率", theme, |ui| {
        let mut fps = app.settings.terminal.target_fps;
        if ui.add(egui::Slider::new(&mut fps, 10..=240).suffix(" FPS")).changed() {
            app.settings.terminal.target_fps = fps;
            let _ = app.settings.save();
        }
    });
}

fn render_themes(app: &mut RustSshApp, ui: &mut egui::Ui, theme: &Theme) {
    ui.add_space(8.0);
    ui.label(egui::RichText::new("内置配色方案").size(18.0).strong().color(egui::Color32::WHITE));
    ui.add_space(24.0);
    
    let schemes = [
        ("Default", "Original Dark Scheme"),
        ("Nord", "Arctic bluish Nord palette"),
        ("Monokai", "Classic bright colors on grey"),
        ("Solarized", "Precision colors for digital clarity"),
    ];
    
    for (name, desc) in schemes {
        let is_active = app.settings.terminal.color_scheme == name;
        
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(name).strong().color(if is_active { egui::Color32::from_rgb(0, 255, 128) } else { egui::Color32::WHITE }));
                ui.label(egui::RichText::new(desc).size(11.0).color(theme.text_secondary));
            });
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.selectable_label(is_active, if is_active { "已应用" } else { "应用" }).clicked() {
                    app.settings.terminal.color_scheme = name.to_string();
                    let _ = app.settings.save();
                }
            });
        });
        ui.add_space(16.0);
    }
}

fn render_text(app: &mut RustSshApp, ui: &mut egui::Ui, theme: &Theme) {
    ui.add_space(8.0);
    ui.label(egui::RichText::new("文本渲染策略").size(18.0).strong().color(egui::Color32::WHITE));
    ui.add_space(24.0);
    
    render_row(ui, "终端文本行间距", theme, |ui| {
        let mut h = app.settings.terminal.line_height;
        if ui.add(egui::Slider::new(&mut h, 1.0..=2.0)).changed() {
            app.settings.terminal.line_height = h;
            let _ = app.settings.save();
        }
    });
    
    render_row(ui, "首选等宽字体", theme, |ui| {
        let mut font = app.settings.terminal.font_family.clone();
        if egui::ComboBox::from_id_salt("font_combo")
            .selected_text(&font)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut font, "JetBrains Mono".to_string(), "JetBrains Mono");
                ui.selectable_value(&mut font, "SF Mono".to_string(), "SF Mono");
                ui.selectable_value(&mut font, "Cascadia Code".to_string(), "Cascadia Code");
            }).response.changed() {
            app.settings.terminal.font_family = font;
            let _ = app.settings.save();
        }
    });
}

fn render_interaction(app: &mut RustSshApp, ui: &mut egui::Ui, theme: &Theme) {
    ui.add_space(8.0);
    ui.label(egui::RichText::new("交互行为").size(18.0).strong().color(egui::Color32::WHITE));
    ui.add_space(24.0);
    
    render_row(ui, "右键自动粘贴剪贴板内容", theme, |ui| {
        if render_switch(ui, &mut app.settings.terminal.right_click_paste).changed() {
            let _ = app.settings.save();
        }
    });
    
    render_row(ui, "回滚缓冲区限制 (行数)", theme, |ui| {
        let mut limit = app.settings.terminal.scrollback_limit;
        if ui.add(egui::Slider::new(&mut limit, 1000..=50000).step_by(1000.0)).changed() {
            app.settings.terminal.scrollback_limit = limit;
            let _ = app.settings.save();
        }
    });
}
