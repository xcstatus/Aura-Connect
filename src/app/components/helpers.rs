use iced::{Color, Theme};
use iced::widget::container;

/// Modal scrim style with fixed alpha.
pub fn modal_scrim_style(theme: &Theme) -> container::Style {
    let bg = theme.extended_palette().background.base.color;
    container::Style::default()
        .background(Color::from_rgba(bg.r, bg.g, bg.b, 0.42))
}

/// Modal scrim style with dynamic alpha.
pub fn modal_scrim_alpha(alpha: f32) -> impl Fn(&Theme) -> container::Style + 'static {
    move |theme: &Theme| {
        let bg = theme.extended_palette().background.base.color;
        container::Style::default()
            .background(Color::from_rgba(bg.r, bg.g, bg.b, alpha))
    }
}

/// Group header style for quick connect panels.
pub fn quick_connect_group_header_style(theme: &Theme) -> container::Style {
    let t = theme.extended_palette();
    container::Style::default().background(t.background.strong.color)
}
