//! 将 [`DesignTokens`] 映射为 Iced [`Theme`](iced::Theme) 的 [`Palette`] 与 [`Extended`](iced::theme::palette::Extended)。

use iced::Theme;
use iced::theme::palette::{
    Background, Danger, Extended, Pair, Palette, Primary, Secondary, Success, Warning,
};

use super::tokens::{DesignTokens, RustSshThemeId};

fn is_dark_bg(c: iced::Color) -> bool {
    c.r * 0.2126 + c.g * 0.7152 + c.b * 0.0722 < 0.45
}

/// 用于 Iced 简色板：背景用主背景，主色用 Accent，语义色对齐文档。
pub fn iced_palette(tokens: &DesignTokens) -> Palette {
    Palette {
        background: tokens.bg_primary,
        text: tokens.text_primary,
        primary: tokens.accent_base,
        success: tokens.success,
        warning: tokens.warning,
        danger: tokens.error,
    }
}

/// 扩展色板：Background / Primary 与 Surface_1/2/3、Accent 三态严格对齐设计系统。
pub fn iced_extended(tokens: &DesignTokens) -> Extended {
    let bg = tokens.bg_primary;
    let txt = tokens.text_primary;
    Extended {
        background: Background {
            base: Pair::new(tokens.bg_primary, tokens.text_primary),
            weakest: Pair::new(tokens.bg_primary, tokens.text_primary),
            weaker: Pair::new(tokens.bg_secondary, tokens.text_primary),
            weak: Pair::new(tokens.surface_1, tokens.text_primary),
            neutral: Pair::new(tokens.bg_header, tokens.text_primary),
            strong: Pair::new(tokens.surface_2, tokens.text_primary),
            stronger: Pair::new(tokens.surface_3, tokens.text_primary),
            strongest: Pair::new(tokens.bg_header, tokens.text_primary),
        },
        primary: Primary {
            base: Pair::new(tokens.accent_base, tokens.on_accent_label),
            weak: Pair::new(tokens.accent_hover, tokens.on_accent_label),
            strong: Pair::new(tokens.accent_active, tokens.on_accent_label),
        },
        secondary: Secondary::generate(tokens.surface_1, tokens.text_secondary),
        success: Success::generate(tokens.success, bg, txt),
        warning: Warning::generate(tokens.warning, bg, txt),
        danger: Danger::generate(tokens.error, bg, txt),
        is_dark: is_dark_bg(tokens.bg_primary),
    }
}

/// 构建 Iced `Theme::Custom`，名称用于调试与未来的主题选择器。
pub fn rustssh_iced_theme(id: RustSshThemeId) -> Theme {
    let tokens = DesignTokens::for_id(id);
    let palette = iced_palette(&tokens);
    let name = match id {
        RustSshThemeId::Dark => "RustSsh Dark",
        RustSshThemeId::Light => "RustSsh Light",
        RustSshThemeId::Warm => "RustSsh Warm",
        RustSshThemeId::GitHub => "RustSsh GitHub",
    };
    Theme::custom_with_fn(name, palette, move |p| {
        let _ = p;
        iced_extended(&tokens)
    })
}

/// 默认与产品主力模式一致：Dark。
pub fn default_rustssh_iced_theme() -> Theme {
    rustssh_iced_theme(RustSshThemeId::Dark)
}
