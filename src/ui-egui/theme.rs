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

    /// 根据背景色自动返回可读性更高的文本颜色（黑/白）。
    pub fn text_on_background(bg: egui::Color32) -> egui::Color32 {
        let white = egui::Color32::WHITE;
        let black = egui::Color32::BLACK;
        if Self::contrast_ratio(bg, white) >= Self::contrast_ratio(bg, black) {
            white
        } else {
            black
        }
    }

    /// 根据背景色自动返回“次级文本色”（在最佳主文本色基础上做透明度衰减）。
    pub fn subtle_text_on_background(bg: egui::Color32) -> egui::Color32 {
        let primary = Self::text_on_background(bg);
        egui::Color32::from_rgba_premultiplied(primary.r(), primary.g(), primary.b(), 180)
    }

    /// 常用语义：主背景上的主文本色。
    pub fn text_on_primary_bg(&self) -> egui::Color32 {
        Self::text_on_background(self.bg_primary)
    }

    /// 常用语义：次背景上的主文本色。
    pub fn text_on_secondary_bg(&self) -> egui::Color32 {
        Self::text_on_background(self.bg_secondary)
    }

    /// 常用语义：强调色背景（例如主按钮）上的主文本色。
    pub fn text_on_accent_bg(&self) -> egui::Color32 {
        Self::text_on_background(self.accent_base)
    }

    fn contrast_ratio(bg: egui::Color32, fg: egui::Color32) -> f32 {
        let l1 = Self::relative_luminance(bg);
        let l2 = Self::relative_luminance(fg);
        let (max_l, min_l) = if l1 >= l2 { (l1, l2) } else { (l2, l1) };
        (max_l + 0.05) / (min_l + 0.05)
    }

    fn relative_luminance(c: egui::Color32) -> f32 {
        let r = Self::srgb_to_linear(c.r() as f32 / 255.0);
        let g = Self::srgb_to_linear(c.g() as f32 / 255.0);
        let b = Self::srgb_to_linear(c.b() as f32 / 255.0);
        0.2126 * r + 0.7152 * g + 0.0722 * b
    }

    fn srgb_to_linear(v: f32) -> f32 {
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    }
}
