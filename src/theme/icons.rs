//! 图标系统：SVG 资源管理、主题适配、绘制渲染（见 `doc/图标风格规范设计.md`）。
//!
//! ## 设计原则
//!
//! 1. **状态驱动颜色**：图标颜色完全继承 `DesignTokens`，不硬编码色值
//! 2. **统一资源管理**：所有图标通过 `IconId` 枚举统一引用
//! 3. **动画支持**：支持旋转动画（加载图标）

use std::collections::HashMap;
use std::sync::LazyLock;

use iced::Color;
use iced::widget::svg::{Svg, Style};
use iced::widget::Space;
use iced::Length;

/// 图标唯一标识（与 `assets/icon/` 下的 SVG 文件对应）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IconId {
    // 功能图标
    QuickConnect, // link-1.svg - 快速连接
    Reload,       // reload.svg - 旋转加载
    Update,       // update.svg - 加载帧
    Plus,         // plus.svg - 新建
    Gear,         // gear.svg - 设置
    Close,        // cross-2.svg - 关闭
    Delete,       // trash.svg - 删除

    // Breadcrumb 图标
    Sftp,         // file.svg - SFTP 文件传输
    Pin,          // pin-top.svg - 固定 breadcrumb
    Unpin,        // pin-bottom.svg - 取消固定
    PortForward,  // port-forward.svg - 端口转发

    // SFTP 专用图标
    Download,     // download.svg - 下载
    NewFolder,    // folder-plus.svg - 新建文件夹

    // 布局与导航图标
    Layout,       // 切换布局
    ArrowRight,   // 向右箭头（左右布局指示）
    ArrowDown,    // 向下箭头（上下布局指示）
    ArrowLeft,    // 向左箭头
    Folder,       // 文件夹
    Document,     // 文档/文件

    // Status icons (extended)
    Connected,     // Connection established
    Connecting,    // Connecting (with animation)
    Disconnected,  // Not connected
    Error,         // Error state
    Locked,        // Locked
    Unlocked,      // Unlocked
    Eye,           // Show password
    EyeOff,        // Hide password

    File,
}

/// 图标元数据
pub struct IconMeta {
    pub path: &'static str,
    pub width: u32,
    pub height: u32,
}

/// SVG 资源注册表（启动时加载一次）
static ICON_REGISTRY: LazyLock<HashMap<IconId, IconMeta>> = LazyLock::new(|| {
    let mut map = HashMap::new();

    // 功能图标
    map.insert(IconId::QuickConnect, IconMeta {
        path: "assets/icon/link-1.svg",
        width: 15,
        height: 15,
    });
    map.insert(IconId::Reload, IconMeta {
        path: "assets/icon/reload.svg",
        width: 15,
        height: 15,
    });
    map.insert(IconId::Update, IconMeta {
        path: "assets/icon/update.svg",
        width: 15,
        height: 15,
    });
    map.insert(IconId::Plus, IconMeta {
        path: "assets/icon/plus.svg",
        width: 15,
        height: 15,
    });
    map.insert(IconId::Gear, IconMeta {
        path: "assets/icon/gear.svg",
        width: 15,
        height: 15,
    });
    map.insert(IconId::Close, IconMeta {
        path: "assets/icon/cross-2.svg",
        width: 15,
        height: 15,
    });

    // Breadcrumb 图标
    map.insert(IconId::Sftp, IconMeta {
        path: "assets/icon/folder-open-fill.svg",
        width: 15,
        height: 15,
    });
    map.insert(IconId::Pin, IconMeta {
        path: "assets/icon/pin-top.svg",
        width: 15,
        height: 15,
    });
    map.insert(IconId::Unpin, IconMeta {
        path: "assets/icon/pin-bottom.svg",
        width: 15,
        height: 15,
    });
    map.insert(IconId::Delete, IconMeta {
        path: "assets/icon/trash.svg",
        width: 15,
        height: 15,
    });
    map.insert(IconId::PortForward, IconMeta {
        path: "assets/icon/port-forward.svg",
        width: 15,
        height: 15,
    });

    // 布局与导航图标
    map.insert(IconId::Layout, IconMeta {
        path: "assets/icon/layout.svg",
        width: 15,
        height: 15,
    });
    map.insert(IconId::ArrowRight, IconMeta {
        path: "assets/icon/arrow-right.svg",
        width: 15,
        height: 15,
    });
    map.insert(IconId::ArrowDown, IconMeta {
        path: "assets/icon/arrow-down.svg",
        width: 15,
        height: 15,
    });
    map.insert(IconId::ArrowLeft, IconMeta {
        path: "assets/icon/arrow-left.svg",
        width: 15,
        height: 15,
    });
    map.insert(IconId::Folder, IconMeta {
        path: "assets/icon/folder-2-fill.svg",
        width: 15,
        height: 15,
    });

    map.insert(IconId::File, IconMeta {
        path: "assets/icon/file-text.svg",
        width: 15,
        height: 15,
    });

    map.insert(IconId::Document, IconMeta {
        path: "assets/icon/document.svg",
        width: 15,
        height: 15,
    });

    // SFTP 专用图标
    map.insert(IconId::Download, IconMeta {
        path: "assets/icon/download.svg",
        width: 15,
        height: 15,
    });
    map.insert(IconId::NewFolder, IconMeta {
        path: "assets/icon/folder-plus.svg",
        width: 15,
        height: 15,
    });

    // Status icons (extended)
    map.insert(IconId::Connected, IconMeta {
        path: "assets/icon/check-circle.svg",
        width: 15,
        height: 15,
    });
    map.insert(IconId::Connecting, IconMeta {
        path: "assets/icon/loader.svg",
        width: 15,
        height: 15,
    });
    map.insert(IconId::Disconnected, IconMeta {
        path: "assets/icon/x-circle.svg",
        width: 15,
        height: 15,
    });
    map.insert(IconId::Error, IconMeta {
        path: "assets/icon/alert-circle.svg",
        width: 15,
        height: 15,
    });
    map.insert(IconId::Locked, IconMeta {
        path: "assets/icon/lock-closed.svg",
        width: 15,
        height: 15,
    });
    map.insert(IconId::Unlocked, IconMeta {
        path: "assets/icon/lock-open-1.svg",
        width: 15,
        height: 15,
    });

    // Password visibility icons
    map.insert(IconId::Eye, IconMeta {
        path: "assets/icon/eye-line.svg",
        width: 15,
        height: 10,
    });
    map.insert(IconId::EyeOff, IconMeta {
        path: "assets/icon/eye-off-line.svg",
        width: 15,
        height: 10,
    });

    map
});

/// 图标状态（决定颜色）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconState {
    /// 默认状态：使用 text_secondary
    Default,
    /// 悬停状态：使用 text_primary
    Hovered,
    /// 激活/选中状态：使用 accent_base
    Active,
    /// 禁用状态：使用 text_disabled
    Disabled,
    /// 错误状态：使用 error 色
    Error,
    /// 警告状态：使用 warning 色
    Warning,
    /// 成功状态：使用 success 色
    Success,
}

impl IconState {
    /// 根据状态获取对应颜色
    pub fn color(&self, tokens: &crate::theme::DesignTokens) -> Color {
        match self {
            IconState::Default | IconState::Hovered => tokens.text_secondary,
            IconState::Active => tokens.accent_base,
            IconState::Disabled => tokens.text_disabled,
            IconState::Error => tokens.error,
            IconState::Warning => tokens.warning,
            IconState::Success => tokens.success,
        }
    }
}

/// 图标渲染选项
#[derive(Debug, Clone)]
pub struct IconOptions {
    /// 图标 ID
    pub id: IconId,
    /// 渲染尺寸
    pub width: u32,
    pub height: u32,
    /// 主题颜色
    pub color: Color,
    /// 旋转角度（0-360），用于动画
    pub rotation: Option<f32>,
}

impl Default for IconOptions {
    fn default() -> Self {
        Self {
            id: IconId::Plus,
            width: 15,
            height: 15,
            color: Color::from_rgb8(0xa0, 0xa0, 0xa5),
            rotation: None,
        }
    }
}

impl IconOptions {
    pub fn new(id: IconId) -> Self {
        Self { id, ..Default::default() }
    }

    pub fn with_size(mut self, size: u32) -> Self {
        self.width = size;
        self.height = size;
        self
    }

    pub fn with_width(mut self, width: u32) -> Self {
        self.width = width;
        self
    }

    pub fn with_height(mut self, height: u32) -> Self {
        self.height = height;
        self
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn with_rotation(mut self, rotation: f32) -> Self {
        self.rotation = Some(rotation);
        self
    }

    /// 从 DesignTokens 和 IconState 创建
    pub fn from_state(id: IconId, tokens: &crate::theme::DesignTokens, state: IconState) -> Self {
        Self {
            id,
            width: 15,
            height: 15,
            color: state.color(tokens),
            rotation: None,
        }
    }
}

// ============================================================================
// SVG 内容处理
// ============================================================================

/// 加载 SVG 内容并处理 currentColor
pub fn load_svg_content(id: IconId) -> Option<String> {
    let meta = ICON_REGISTRY.get(&id)?;
    let full_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(meta.path);

    std::fs::read_to_string(&full_path).ok()
}

/// 将 SVG 中的 currentColor 替换为指定颜色
pub fn colorize_svg(svg_content: &str, color: &Color) -> String {
    let r = (color.r * 255.0).round() as u8;
    let g = (color.g * 255.0).round() as u8;
    let b = (color.b * 255.0).round() as u8;

    svg_content.replace("currentColor", &format!("#{:02X}{:02X}{:02X}", r, g, b))
}

/// 旋转 SVG 内容
pub fn rotate_svg_content(svg_content: &str, degrees: f32, width: u32, height: u32) -> String {
    let cx = width as f32 / 2.0;
    let cy = height as f32 / 2.0;

    if let Some(start) = svg_content.find('>') {
        if let Some(end) = svg_content.rfind("</svg>") {
            let inner = &svg_content[start + 1..end];
            let transform = format!(
                r#"<g transform="rotate({:.1} {} {})">"#,
                degrees, cx, cy
            );
            return format!(
                r#"<svg width="{}" height="{}" viewBox="0 0 {} {}" fill="none" xmlns="http://www.w3.org/2000/svg">{}{}</g></svg>"#,
                width, height, width, height, transform, inner
            );
        }
    }

    String::from(svg_content)
}

/// 获取 SVG 元数据
pub fn get_icon_meta(id: IconId) -> Option<IconMeta> {
    ICON_REGISTRY.get(&id).copied()
}

/// 获取所有注册的图标 ID
pub fn all_icon_ids() -> Vec<IconId> {
    ICON_REGISTRY.keys().copied().collect()
}

// ============================================================================
// SVG 图标渲染
// ============================================================================

/// 获取图标的完整路径
pub fn icon_path(id: IconId) -> Option<String> {
    let meta = ICON_REGISTRY.get(&id)?;
    let full_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(meta.path);
    Some(full_path.to_string_lossy().to_string())
}

/// 创建图标视图
pub fn icon_view(options: IconOptions) -> iced::Element<'static, ()> {
    let path = match icon_path(options.id) {
        Some(p) => p,
        None => {
            return Space::new()
                .width(Length::Fixed(options.width as f32))
                .height(Length::Fixed(options.height as f32))
                .into();
        }
    };

    let mut svg = Svg::from_path(&path)
        .width(Length::Fixed(options.width as f32))
        .height(Length::Fixed(options.height as f32));

    // 设置颜色滤镜
    svg = svg.style(move |_theme, _status| Style {
        color: Some(options.color),
    });

    // 设置旋转角度
    if let Some(rotation) = options.rotation {
        svg = svg.rotation(iced::Radians(rotation * std::f32::consts::PI / 180.0));
    }

    svg.into()
}

/// 创建带泛型消息的图标视图
pub fn icon_view_with<Message: 'static + Clone>(
    options: IconOptions,
    msg: Message,
) -> iced::Element<'static, Message> {
    let msg = std::sync::Arc::new(msg);
    icon_view(options).map(move |()| (*msg).clone())
}

// ============================================================================
// 便捷构造器
// ============================================================================

/// 创建带状态的图标视图
pub fn icon(id: IconId, tokens: &crate::theme::DesignTokens, state: IconState) -> iced::Element<'static, ()> {
    icon_view(IconOptions::from_state(id, tokens, state))
}

/// 创建默认状态的图标
pub fn icon_default(id: IconId, tokens: &crate::theme::DesignTokens) -> iced::Element<'static, ()> {
    icon(id, tokens, IconState::Default)
}

/// 创建激活状态的图标
pub fn icon_active(id: IconId, tokens: &crate::theme::DesignTokens) -> iced::Element<'static, ()> {
    icon(id, tokens, IconState::Active)
}

// ============================================================================
// 动画支持
// ============================================================================

/// 动画状态管理器
#[derive(Debug, Clone, Default)]
pub struct IconAnimationState {
    /// 图标旋转角度映射
    rotations: HashMap<IconId, f32>,
}

impl IconAnimationState {
    pub fn new() -> Self {
        Self::default()
    }

    /// 更新旋转角度
    pub fn update_rotation(&mut self, id: IconId, degrees: f32) {
        self.rotations.insert(id, degrees);
    }

    /// 获取当前旋转角度
    pub fn get_rotation(&self, id: IconId) -> Option<f32> {
        self.rotations.get(&id).copied()
    }

    /// 移除动画状态
    pub fn clear_rotation(&mut self, id: IconId) {
        self.rotations.remove(&id);
    }

    /// 是否正在动画
    pub fn is_animating(&self, id: IconId) -> bool {
        self.rotations.contains_key(&id)
    }

    /// 开始动画
    pub fn start_animation(&mut self, id: IconId) {
        if !self.rotations.contains_key(&id) {
            self.rotations.insert(id, 0.0);
        }
    }

    /// 停止动画
    pub fn stop_animation(&mut self, id: IconId) {
        self.rotations.remove(&id);
    }
}

/// 加载图标旋转速度（每毫秒旋转角度，1秒转一圈）
pub const RELOAD_ROTATION_SPEED: f32 = 360.0 / 1000.0;

/// 计算动画旋转角度
pub fn compute_rotation(degrees_per_ms: f32, elapsed_ms: f32) -> f32 {
    (degrees_per_ms * elapsed_ms) % 360.0
}

/// 更新加载动画状态
pub fn tick_loading_animation(state: &mut IconAnimationState, delta_ms: f32) {
    // Reload 图标旋转
    if state.is_animating(IconId::Reload) {
        let current = state.get_rotation(IconId::Reload).unwrap_or(0.0);
        state.update_rotation(IconId::Reload, (current + RELOAD_ROTATION_SPEED * delta_ms) % 360.0);
    }

    // Update 图标旋转
    if state.is_animating(IconId::Update) {
        let current = state.get_rotation(IconId::Update).unwrap_or(0.0);
        state.update_rotation(IconId::Update, (current + RELOAD_ROTATION_SPEED * delta_ms) % 360.0);
    }
}

// ============================================================================
// 主题适配
// ============================================================================

/// 图标样式配置
#[derive(Debug, Clone, Copy)]
pub struct IconStyleConfig {
    /// 默认颜色
    pub default_color: Color,
    /// 悬停颜色
    pub hover_color: Color,
    /// 激活颜色
    pub active_color: Color,
    /// 禁用颜色
    pub disabled_color: Color,
    /// 图标尺寸映射
    pub sizes: IconSizes,
}

/// 图标尺寸
#[derive(Debug, Clone, Copy)]
pub struct IconSizes {
    pub small: u32,
    pub medium: u32,
    pub large: u32,
}

impl Default for IconStyleConfig {
    fn default() -> Self {
        Self::dark()
    }
}

impl IconStyleConfig {
    pub fn dark() -> Self {
        Self {
            default_color: Color::from_rgb8(0xa0, 0xa0, 0xa5),
            hover_color: Color::WHITE,
            active_color: Color::from_rgb8(0x00, 0xff, 0x80),
            disabled_color: Color::from_rgb8(0x6c, 0x6c, 0x70),
            sizes: IconSizes {
                small: 12,
                medium: 15,
                large: 24,
            },
        }
    }

    pub fn light() -> Self {
        Self {
            default_color: Color::from_rgb8(0x6e, 0x6e, 0x73),
            hover_color: Color::from_rgb8(0x1c, 0x1c, 0x1e),
            active_color: Color::from_rgb8(0x00, 0x7a, 0xff),
            disabled_color: Color::from_rgb8(0xa1, 0xa1, 0xa6),
            sizes: IconSizes {
                small: 12,
                medium: 15,
                large: 24,
            },
        }
    }

    pub fn warm() -> Self {
        Self {
            default_color: Color::from_rgb8(0xb7, 0xa9, 0x99),
            hover_color: Color::from_rgb8(0xd4, 0xbe, 0xab),
            active_color: Color::from_rgb8(0xe0, 0x99, 0x5e),
            disabled_color: Color::from_rgb8(0x8a, 0x7d, 0x72),
            sizes: IconSizes {
                small: 12,
                medium: 15,
                large: 24,
            },
        }
    }

    pub fn github() -> Self {
        Self {
            default_color: Color::from_rgb8(0x8B, 0x94, 0x9E),
            hover_color: Color::from_rgb8(0xE6, 0xED, 0xF3),
            active_color: Color::from_rgb8(0x23, 0x86, 0x36),
            disabled_color: Color::from_rgb8(0x6E, 0x76, 0x81),
            sizes: IconSizes {
                small: 12,
                medium: 15,
                large: 24,
            },
        }
    }

    /// 从 DesignTokens 获取样式配置
    pub fn from_tokens(tokens: &crate::theme::DesignTokens) -> Self {
        Self {
            default_color: tokens.text_secondary,
            hover_color: tokens.text_primary,
            active_color: tokens.accent_base,
            disabled_color: tokens.text_disabled,
            sizes: IconSizes {
                small: 12,
                medium: 15,
                large: 24,
            },
        }
    }
}

/// 图标主题适配器
pub struct IconThemeAdapter;

impl IconThemeAdapter {
    /// 根据主题是否为暗色返回不同的图标样式配置
    pub fn style_for_theme(is_dark: bool) -> IconStyleConfig {
        if is_dark {
            IconStyleConfig::dark()
        } else {
            IconStyleConfig::light()
        }
    }

    /// 从 DesignTokens 获取样式配置
    pub fn from_tokens(tokens: &crate::theme::DesignTokens) -> IconStyleConfig {
        IconStyleConfig::from_tokens(tokens)
    }
}

// ============================================================================
// 辅助类型
// ============================================================================

impl IconMeta {
    pub fn path(&self) -> &str {
        self.path
    }

    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

impl Copy for IconMeta {}

impl Clone for IconMeta {
    fn clone(&self) -> Self {
        *self
    }
}
