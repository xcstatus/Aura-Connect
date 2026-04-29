//! RustSSH 设计系统 → 代码：色票、布局 token、Iced 主题桥接（`doc/设计系统_色卡.md`，`doc/组件设计规范.md`）。

pub mod animation;
pub mod iced_bridge;
pub mod icons;
pub mod import_export;
pub mod layout;
pub mod liquid_glass;
pub mod tokens;
pub mod user_scheme;

pub use animation::{
    anim_done, anim_t, anim_t_bidir, ease_in, ease_in_out, ease_out, lerp, lerp_color,
};
pub use iced_bridge::{
    default_rustssh_iced_theme, iced_extended, iced_palette, rustssh_iced_theme,
};
pub use icons::{
    IconAnimationState, IconButtonSize, IconId, IconMeta, IconOptions, IconSize, IconState,
    RELOAD_ROTATION_SPEED,
    colorize_svg, compute_rotation, get_icon_meta,
    icon, icon_with,
    icon_button, icon_button_auto, icon_button_with_tooltip,
    icon_button_tooltip_auto,
    icon_view, icon_view_with, load_svg_content, tick_loading_animation,
};
pub use liquid_glass::{
    GlassButtonVariant, GlassContainerConfig, GlassIntensity, glass_container_style,
    glass_danger_button_style, glass_edge_container_style, glass_icon_button_style,
    glass_primary_button_style, glass_secondary_button_style, glass_tab_button_style,
};
pub use tokens::{COLOR_SCHEMES, ColorScheme, DebugTokens, DesignTokens, color};

pub use import_export::{export_user_scheme_to_json, import_from_json};

pub use user_scheme::{
    COLOR_FIELDS, ColorFieldCategory, ColorFieldMeta, ExportedScheme, UserColorScheme,
    UserColorSchemes, get_fields_by_category,
};
