use iced::{Color, Theme};
use iced::widget::container;

/// Modal scrim style with fixed alpha.
pub fn modal_scrim_style(theme: &Theme) -> container::Style {
    layered_scrim_style(theme, 0)
}

/// Modal scrim style with dynamic alpha (for animations).
pub fn modal_scrim_alpha(alpha: f32) -> impl Fn(&Theme) -> container::Style + 'static {
    move |theme: &Theme| {
        let bg = theme.extended_palette().background.base.color;
        container::Style::default()
            .background(Color::from_rgba(bg.r, bg.g, bg.b, alpha))
    }
}

/// Layered scrim style - adjusts transparency based on modal stack depth.
/// level: 0 = first modal, 1 = second modal (nested), etc.
/// Each additional layer adds 0.15 to alpha, capped at 0.75.
pub fn layered_scrim_style(theme: &Theme, level: usize) -> container::Style {
    let bg = theme.extended_palette().background.base.color;
    let base_alpha = 0.35_f32;
    let alpha = (base_alpha + level as f32 * 0.15).min(0.75);
    container::Style::default()
        .background(Color::from_rgba(bg.r, bg.g, bg.b, alpha))
}

/// Layered scrim style with dynamic alpha (for animations + layered effect).
pub fn layered_scrim_alpha(alpha: f32, level: usize) -> impl Fn(&Theme) -> container::Style + 'static {
    move |theme: &Theme| {
        let bg = theme.extended_palette().background.base.color;
        let level_adjustment = level as f32 * 0.1;
        let adjusted_alpha = (alpha - level_adjustment).max(0.2).min(0.85);
        container::Style::default()
            .background(Color::from_rgba(bg.r, bg.g, bg.b, adjusted_alpha))
    }
}

/// Group header style for quick connect panels.
pub fn quick_connect_group_header_style(theme: &Theme) -> container::Style {
    let t = theme.extended_palette();
    container::Style::default().background(t.background.strong.color)
}
