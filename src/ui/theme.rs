use eframe::egui;

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub bg_primary: egui::Color32,
    pub bg_secondary: egui::Color32,
    pub bg_header: egui::Color32,
    pub surface_1: egui::Color32,
    pub surface_2: egui::Color32,
    pub surface_3: egui::Color32,
    pub tab_active: egui::Color32,
    pub accent_base: egui::Color32,
    pub accent_hover: egui::Color32,
    pub accent_active: egui::Color32,
    pub text_primary: egui::Color32,
    pub text_secondary: egui::Color32,
    pub border_subtle: egui::Color32,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            bg_primary: egui::Color32::from_rgb(28, 28, 30),     // Gray_900
            bg_secondary: egui::Color32::from_rgb(35, 35, 38),   // Gray_800
            bg_header: egui::Color32::from_rgb(40, 40, 45),      // Gray_700
            surface_1: egui::Color32::from_rgb(44, 44, 46),     // Gray_600
            surface_2: egui::Color32::from_rgb(58, 58, 60),     // Gray_500
            surface_3: egui::Color32::from_rgb(72, 72, 74),     // Gray_400
            tab_active: egui::Color32::from_rgb(0, 0, 0),       // Black
            accent_base: egui::Color32::from_rgb(0, 255, 128),  // #00FF80
            accent_hover: egui::Color32::from_rgb(51, 255, 153), // #33FF99
            accent_active: egui::Color32::from_rgb(0, 204, 102), // #00CC66
            text_primary: egui::Color32::WHITE,
            text_secondary: egui::Color32::from_rgb(160, 160, 165),
            border_subtle: egui::Color32::from_rgba_premultiplied(255, 255, 255, 10), // Border_Subtle_Dark
        }
    }
}
