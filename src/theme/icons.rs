//! 图标系统：SVG 资源管理、主题适配、绘制渲染（见 `doc/设计规范/图标规范.md`）。

use std::collections::HashMap;
use std::sync::LazyLock;

use iced::Color;
use iced::Length;
use iced::widget::Space;
use iced::widget::svg::{Style, Svg};

use crate::theme::layout::{ICON_BUTTON_SIZE, ICON_BUTTON_SIZE_COMPACT};
use crate::theme::liquid_glass::glass_icon_button_style;

// ============================================================================
// IconId
// ============================================================================

/// 图标唯一标识（与 `assets/icon/` 下的 SVG 文件对应）。
///
/// 命名规范：`{分类前缀}{名称}`，分类前缀含义：
/// - `Fn` — 功能图标（新建、设置、关闭等操作）
/// - `Nav` — 导航图标（箭头方向）
/// - `Mod` — 模块专用图标（SFTP、端口转发等）
/// - `Status` — 状态指示图标（连接、锁定等）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IconId {
    // --- Fn: 功能图标 ---
    FnPlus,
    FnGear,
    FnClose,
    FnDelete,
    FnReload,
    FnLayout,
    FnQuickConnect,
    FnRestart,
    FnFingerprint,

    // --- Nav: 导航图标 ---
    NavArrowLeft,
    NavArrowRight,
    NavArrowDown,

    // --- Mod: 模块专用 ---
    ModSftp,
    ModPortForward,
    ModPin,
    ModUnpin,

    // --- Status: 状态图标 ---
    StatusCircleDot,

    // --- File: 文件/目录图标 ---
    FileFolder,
    FileFolderFill,
    FileFolderOpen,
    FileFolderPlus,
    FileDocumentFill,
    File,
    FileCode,
    FileImage,
    FileArchive,
    FileMedia,

    // --- Action: 动作图标 ---
    ActionDownload,
    ActionDownload2Fill,
    ActionDownload2Line,
    ActionUpload2Fill,
    ActionUpload2Line,

    // --- Eye: 密码可见性 ---
    Eye,
    EyeOff,
}

// ============================================================================
// IconMeta
// ============================================================================

/// 图标元数据
#[derive(Debug, Clone, Copy)]
pub struct IconMeta {
    pub path: &'static str,
    pub width: u32,
    pub height: u32,
}

impl IconMeta {
    pub fn path(&self) -> &str {
        self.path
    }
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

// ============================================================================
// ICON_REGISTRY
// ============================================================================

macro_rules! reg {
    ($map:ident, $id:expr, $path:expr, $w:expr, $h:expr) => {
        $map.insert(
            $id,
            IconMeta {
                path: $path,
                width: $w,
                height: $h,
            },
        );
    };
}

/// SVG 资源注册表（启动时加载一次）。
/// 所有图标统一使用 15×15 画板（`port-forward.svg` 无显式尺寸，
/// 按设计规范补齐为 15×15）。
static ICON_REGISTRY: LazyLock<HashMap<IconId, IconMeta>> = LazyLock::new(|| {
    let mut map = HashMap::new();

    // --- Fn: 功能图标 ---
    reg!(map, IconId::FnPlus, "assets/icon/plus.svg", 15, 15);
    reg!(map, IconId::FnGear, "assets/icon/gear.svg", 15, 15);
    reg!(map, IconId::FnClose, "assets/icon/cross-2.svg", 15, 15);
    reg!(map, IconId::FnDelete, "assets/icon/trash.svg", 15, 15);
    reg!(map, IconId::FnReload, "assets/icon/reload.svg", 15, 15);
    reg!(map, IconId::FnLayout, "assets/icon/layout.svg", 15, 15);
    reg!(
        map,
        IconId::FnQuickConnect,
        "assets/icon/link-1.svg",
        15,
        15
    );
    reg!(
        map,
        IconId::FnRestart,
        "assets/icon/restart-line.svg",
        24,
        24
    );
    reg!(
        map,
        IconId::FnFingerprint,
        "assets/icon/fingerprint-line.svg",
        24,
        24
    );

    // --- Nav: 导航图标 ---
    reg!(
        map,
        IconId::NavArrowLeft,
        "assets/icon/arrow-left.svg",
        15,
        15
    );
    reg!(
        map,
        IconId::NavArrowRight,
        "assets/icon/arrow-right.svg",
        15,
        15
    );
    reg!(
        map,
        IconId::NavArrowDown,
        "assets/icon/arrow-down.svg",
        15,
        15
    );

    // --- Mod: 模块专用 ---
    reg!(
        map,
        IconId::ModSftp,
        "assets/icon/folder-open-fill.svg",
        15,
        15
    );
    reg!(
        map,
        IconId::ModPortForward,
        "assets/icon/port-forward.svg",
        15,
        15
    );
    reg!(map, IconId::ModPin, "assets/icon/pin-top.svg", 15, 15);
    reg!(map, IconId::ModUnpin, "assets/icon/pin-bottom.svg", 15, 15);

    // --- Status ---
    reg!(
        map,
        IconId::StatusCircleDot,
        "assets/icon/circle-dot.svg",
        8,
        8
    );

    // --- File ---
    reg!(map, IconId::FileFolder, "assets/icon/folder.svg", 15, 15);
    reg!(
        map,
        IconId::FileFolderFill,
        "assets/icon/folder-2-fill.svg",
        24,
        24
    );
    reg!(
        map,
        IconId::FileFolderOpen,
        "assets/icon/folder-open-fill.svg",
        15,
        15
    );
    reg!(
        map,
        IconId::FileFolderPlus,
        "assets/icon/folder-plus.svg",
        15,
        15
    );
    reg!(
        map,
        IconId::FileDocumentFill,
        "assets/icon/file-text.svg",
        15,
        15
    );
    reg!(map, IconId::File, "assets/icon/file.svg", 15, 15);
    reg!(map, IconId::FileCode, "assets/icon/file-text.svg", 15, 15);
    reg!(map, IconId::FileImage, "assets/icon/file-text.svg", 15, 15);
    reg!(map, IconId::FileArchive, "assets/icon/file-text.svg", 15, 15);
    reg!(map, IconId::FileMedia, "assets/icon/file-text.svg", 15, 15);

    // --- Action ---
    reg!(
        map,
        IconId::ActionDownload,
        "assets/icon/download.svg",
        15,
        15
    );
    reg!(
        map,
        IconId::ActionDownload2Fill,
        "assets/icon/download-2-fill.svg",
        24,
        24
    );
    reg!(
        map,
        IconId::ActionDownload2Line,
        "assets/icon/download-2-line.svg",
        24,
        24
    );
    reg!(
        map,
        IconId::ActionUpload2Fill,
        "assets/icon/upload-2-fill.svg",
        24,
        24
    );
    reg!(
        map,
        IconId::ActionUpload2Line,
        "assets/icon/upload-2-line.svg",
        24,
        24
    );

    // --- Eye ---
    reg!(map, IconId::Eye, "assets/icon/eye-line.svg", 24, 24);
    reg!(map, IconId::EyeOff, "assets/icon/eye-off-line.svg", 24, 24);

    map
});

// ============================================================================
// IconState
// ============================================================================

/// 图标状态（决定颜色）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconState {
    Default,
    Hovered,
    Active,
    Disabled,
    Error,
    Warning,
    Success,
}

impl IconState {
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

// ============================================================================
// IconSize
// ============================================================================

/// 图标尺寸（SVG 渲染像素）。
///
/// 遵循 `doc/设计规范/图标规范.md` 的 9.1 节：
/// | 枚举值 | 像素 | 用途 |
/// | :--- | :---: | :--- |
/// | `Tiny` | 8×8 | 状态指示圆点 |
/// | `Small` | 12×12 | 紧凑按钮内、关闭图标 |
/// | `Medium` | 15×15 | 标准工具栏（默认） |
/// | `Large` | 24×24 | 设置面板、SFTP 顶栏 |
/// | `XLARGE` | 32×32 | 极少使用 |
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IconSize {
    pub width: u32,
    pub height: u32,
}

impl IconSize {
    pub const TINY: Self = Self { width: 8, height: 8 };
    pub const SMALL: Self = Self { width: 12, height: 12 };
    pub const MEDIUM: Self = Self { width: 15, height: 15 };
    pub const LARGE: Self = Self { width: 24, height: 24 };
    pub const XLARGE: Self = Self { width: 32, height: 32 };

    pub fn as_u32(self) -> u32 {
        self.width
    }
    pub fn as_f32(self) -> f32 {
        self.width as f32
    }

    pub fn icon_size(self) -> iced::Length {
        Length::Fixed(self.width as f32)
    }
}


// ============================================================================
// IconButtonSize
// ============================================================================

/// 图标按钮尺寸变体（容器尺寸 + 图标尺寸 + 内边距 捆绑在一起）。
///
/// 保证图标在按钮内视觉居中，填充比 50%~60% 为最佳。
///
/// ## 尺寸变体
///
/// | 变体 | 容器 | 图标 | Padding | 填充比 | 适用场景 |
/// | :--- | ---: | ---: | ---: | ---: | :--- |
/// | `TOP` | 36×36 | 15px | 10px | 42% | 顶栏操作按钮 |
/// | `STANDARD` | 28×28 | 15px | 6px | 54% | 标准工具栏 |
/// | `COMPACT` | 24×24 | 12px | 6px | 50% | 内联弹窗、紧凑工具栏 |
/// | `EXTRA_COMPACT` | 22×22 | 12px | 5px | 55% | 标签栏关闭按钮 |
#[derive(Debug, Clone, Copy)]
pub struct IconButtonSize {
    /// 按钮容器内图标尺寸
    pub icon_size: IconSize,
    /// 按钮内边距（各方向相同）
    pub padding: f32,
    /// 按钮宽度
    pub width: f32,
    /// 按钮高度
    pub height: f32,
}

impl IconButtonSize {
    /// 顶栏操作按钮：36×36，图标 15px，内边距 10px
    pub const TOP: Self = Self {
        icon_size: IconSize::MEDIUM,
        padding: 10.0,
        width: 36.0,
        height: 36.0,
    };

    /// 标准图标按钮：28×28，图标 15px，内边距 6px
    pub const STANDARD: Self = Self {
        icon_size: IconSize::MEDIUM,
        padding: 6.0,
        width: 28.0,
        height: 28.0,
    };

    /// 紧凑图标按钮：24×24，图标 12px，内边距 6px
    pub const COMPACT: Self = Self {
        icon_size: IconSize::SMALL,
        padding: 6.0,
        width: 24.0,
        height: 24.0,
    };

    /// 额外紧凑按钮：22×22，图标 12px，内边距 5px
    pub const EXTRA_COMPACT: Self = Self {
        icon_size: IconSize::SMALL,
        padding: 5.0,
        width: 22.0,
        height: 22.0,
    };

    /// 视觉填充比（图标面积 / 容器面积）
    pub fn visual_fill_ratio(self) -> f32 {
        let icon_area = self.icon_size.as_f32().powi(2);
        let container_area = self.width * self.height;
        icon_area / container_area
    }
}

// ============================================================================
// IconOptions
// ============================================================================

/// 图标渲染选项（Builder 模式）
#[derive(Debug, Clone)]
pub struct IconOptions {
    pub id: IconId,
    pub width: u32,
    pub height: u32,
    pub color: Color,
    pub rotation: Option<f32>,
}

impl Default for IconOptions {
    fn default() -> Self {
        Self {
            id: IconId::FnPlus,
            width: 15,
            height: 15,
            color: Color::from_rgb8(0xa0, 0xa0, 0xa5),
            rotation: None,
        }
    }
}

impl IconOptions {
    pub fn new(id: IconId) -> Self {
        Self {
            id,
            ..Default::default()
        }
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

    /// 从 DesignTokens 和 IconState 自动获取颜色
    pub fn with_state(mut self, tokens: &crate::theme::DesignTokens, state: IconState) -> Self {
        self.color = state.color(tokens);
        self
    }

    /// 快捷尺寸：12px（小图标、紧凑按钮内）
    pub fn small(self) -> Self {
        self.with_size(IconSize::SMALL.as_u32())
    }

    /// 快捷尺寸：15px（标准工具栏）
    pub fn medium(self) -> Self {
        self.with_size(IconSize::MEDIUM.as_u32())
    }

    /// 快捷尺寸：24px（大图标、设置面板）
    pub fn large(self) -> Self {
        self.with_size(IconSize::LARGE.as_u32())
    }

    /// 接受 IconSize 枚举的快捷方法
    pub fn size(self, s: IconSize) -> Self {
        self.with_size(s.as_u32())
    }

    /// 标记为旋转动画（初始角度 0，由动画层驱动）
    pub fn spinning(self) -> Self {
        self.with_rotation(0.0)
    }

    /// 从 DesignTokens + IconState 构造（替代已废弃的 `from_state`）
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
// SVG 内容处理（内部使用，公开以支持高级用法）
// ============================================================================

/// 加载 SVG 内容
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

/// 获取 SVG 元数据
pub fn get_icon_meta(id: IconId) -> Option<IconMeta> {
    ICON_REGISTRY.get(&id).copied()
}

// ============================================================================
// SVG 图标渲染
// ============================================================================

fn icon_path(id: IconId) -> Option<String> {
    let meta = ICON_REGISTRY.get(&id)?;
    let full_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(meta.path);
    Some(full_path.to_string_lossy().to_string())
}

/// 创建图标视图（无消息）
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

    svg = svg.style(move |_theme, _status| Style {
        color: Some(options.color),
    });

    if let Some(rotation) = options.rotation {
        svg = svg.rotation(iced::Radians(rotation * std::f32::consts::PI / 180.0));
    }

    svg.into()
}

/// 创建带消息的图标视图
pub fn icon_view_with<Message: 'static + Clone>(
    options: IconOptions,
    msg: Message,
) -> iced::Element<'static, Message> {
    let msg = std::sync::Arc::new(msg);
    icon_view(options).map(move |()| (*msg).clone())
}

// ============================================================================
// 便捷构造器（手动主题 + 自动主题）
// ============================================================================

/// 从配色方案 ID 获取 DesignTokens。
fn tokens_from_scheme_id(scheme_id: &str) -> crate::theme::DesignTokens {
    crate::theme::DesignTokens::for_color_scheme(scheme_id)
}

// --- 手动主题版（存量 API，保留） ---

/// 创建带状态的图标（需手动传入 DesignTokens）
pub fn icon(
    id: IconId,
    tokens: &crate::theme::DesignTokens,
    state: IconState,
) -> iced::Element<'static, ()> {
    icon_view(IconOptions::from_state(id, tokens, state))
}

/// 创建默认状态的图标（需手动传入 DesignTokens）
pub fn icon_default(id: IconId, tokens: &crate::theme::DesignTokens) -> iced::Element<'static, ()> {
    icon(id, tokens, IconState::Default)
}

/// 创建激活状态的图标（需手动传入 DesignTokens）
pub fn icon_active(id: IconId, tokens: &crate::theme::DesignTokens) -> iced::Element<'static, ()> {
    icon(id, tokens, IconState::Active)
}

// --- 自动主题版（从配色方案 ID 自动检测） ---

/// 创建图标（自动获取当前主题颜色）。
///
/// ## 参数
/// - `id`: 图标 ID
/// - `size`: 图标尺寸枚举
/// - `msg`: 消息（用于类型转换）
/// - `color_scheme_id`: 当前配色方案 ID（如 "terminal_dark"、"nord" 等）
///
/// ## 示例
/// ```ignore
/// icon_auto(IconId::FnGear, IconSize::MEDIUM, Message::Settings, "terminal_dark")
/// ```
pub fn icon_auto<Message: 'static + Clone>(
    id: IconId,
    size: IconSize,
    msg: Message,
    color_scheme_id: &str,
) -> iced::Element<'static, Message> {
    let tokens = tokens_from_scheme_id(color_scheme_id);
    let options = IconOptions::new(id)
        .with_size(size.as_u32())
        .with_color(tokens.text_secondary);
    let msg = std::sync::Arc::new(msg);
    icon_view(options).map(move |()| (*msg).clone())
}

/// 创建带消息的图标（自动获取当前主题颜色）。
///
/// ## 示例
/// ```ignore
/// icon_with(IconId::FnGear, IconSize::MEDIUM, Message::Settings, "nord")
/// ```
pub fn icon_with<Message: 'static + Clone>(
    id: IconId,
    size: IconSize,
    msg: Message,
    color_scheme_id: &str,
) -> iced::Element<'static, Message> {
    let tokens = tokens_from_scheme_id(color_scheme_id);
    icon_view_with(
        IconOptions::new(id)
            .with_size(size.as_u32())
            .with_color(tokens.text_secondary),
        msg,
    )
}

// ============================================================================
// icon_button helpers（手动主题 + 自动主题）
// ============================================================================

/// 创建图标按钮（需手动传入 DesignTokens）。
///
/// ## 参数
/// - `id`: 图标 ID
/// - `size`: 按钮尺寸变体（含图标尺寸 + padding + 容器尺寸）
/// - `msg`: 点击消息
/// - `tokens`: 主题 token
///
/// ## 示例
/// ```ignore
/// icon_button(IconId::FnGear, IconButtonSize::STANDARD, Message::Settings, &tokens)
/// ```
pub fn icon_button<Message: 'static + Clone>(
    id: IconId,
    size: IconButtonSize,
    msg: Message,
    tokens: &crate::theme::DesignTokens,
) -> iced::Element<'static, Message> {
    use iced::widget::button;

    let icon = icon_view_with(
        IconOptions::new(id).size(size.icon_size).with_color(tokens.text_secondary),
        msg.clone(),
    );

    button(icon)
        .on_press(msg)
        .width(Length::Fixed(size.width))
        .height(Length::Fixed(size.height))
        .padding(iced::Padding::new(size.padding))
        .style(glass_icon_button_style(tokens.clone()))
        .into()
}

/// 创建图标按钮（自动获取当前主题颜色）。
///
/// ## 示例
/// ```ignore
/// icon_button_auto(IconId::FnGear, IconButtonSize::STANDARD, Message::Settings, "nord")
/// icon_button_auto(IconId::FnClose, IconButtonSize::COMPACT, Message::Close, "terminal_dark")
/// ```
pub fn icon_button_auto<Message: 'static + Clone>(
    id: IconId,
    size: IconButtonSize,
    msg: Message,
    color_scheme_id: &str,
) -> iced::Element<'static, Message> {
    use iced::widget::button;

    let tokens = tokens_from_scheme_id(color_scheme_id);
    let icon = icon_view_with(
        IconOptions::new(id).size(size.icon_size).with_color(tokens.text_secondary),
        msg.clone(),
    );

    button(icon)
        .on_press(msg)
        .width(Length::Fixed(size.width))
        .height(Length::Fixed(size.height))
        .padding(iced::Padding::new(size.padding))
        .style(glass_icon_button_style(tokens))
        .into()
}

/// 创建带 Tooltip 的图标按钮（需手动传入 DesignTokens）。
///
/// ## 示例
/// ```ignore
/// icon_button_with_tooltip(IconId::FnGear, IconButtonSize::STANDARD, msg, "设置", &tokens)
/// ```
pub fn icon_button_with_tooltip<Message: 'static + Clone>(
    id: IconId,
    size: IconButtonSize,
    msg: Message,
    tooltip_text: impl std::fmt::Display,
    tokens: &crate::theme::DesignTokens,
) -> iced::Element<'static, Message> {
    use iced::widget::{text, tooltip};

    tooltip(
        icon_button(id, size, msg, tokens),
        text(tooltip_text.to_string()).size(12),
        tooltip::Position::Bottom,
    )
    .into()
}

/// 创建带 Tooltip 的图标按钮（自动获取当前主题颜色）。
///
/// ## 示例
/// ```ignore
/// icon_button_tooltip_auto(IconId::FnGear, IconButtonSize::STANDARD, msg, "设置", "nord")
/// ```
pub fn icon_button_tooltip_auto<Message: 'static + Clone>(
    id: IconId,
    size: IconButtonSize,
    msg: Message,
    tooltip_text: impl std::fmt::Display,
    color_scheme_id: &str,
) -> iced::Element<'static, Message> {
    use iced::widget::{text, tooltip};

    tooltip(
        icon_button_auto(id, size, msg.clone(), color_scheme_id),
        text(tooltip_text.to_string()).size(12),
        tooltip::Position::Bottom,
    )
    .into()
}

// ============================================================================
// 动画支持
// ============================================================================

/// 动画状态管理器
#[derive(Debug, Clone, Default)]
pub struct IconAnimationState {
    rotations: HashMap<IconId, f32>,
}

impl IconAnimationState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update_rotation(&mut self, id: IconId, degrees: f32) {
        self.rotations.insert(id, degrees);
    }

    pub fn get_rotation(&self, id: IconId) -> Option<f32> {
        self.rotations.get(&id).copied()
    }

    pub fn clear_rotation(&mut self, id: IconId) {
        self.rotations.remove(&id);
    }

    pub fn is_animating(&self, id: IconId) -> bool {
        self.rotations.contains_key(&id)
    }

    pub fn start_animation(&mut self, id: IconId) {
        if !self.rotations.contains_key(&id) {
            self.rotations.insert(id, 0.0);
        }
    }

    pub fn stop_animation(&mut self, id: IconId) {
        self.rotations.remove(&id);
    }
}

/// 旋转速度（度/毫秒），1 秒转一圈
pub const RELOAD_ROTATION_SPEED: f32 = 360.0 / 1000.0;

/// 计算动画旋转角度
pub fn compute_rotation(degrees_per_ms: f32, elapsed_ms: f32) -> f32 {
    (degrees_per_ms * elapsed_ms) % 360.0
}

/// 更新加载动画状态
pub fn tick_loading_animation(state: &mut IconAnimationState, delta_ms: f32) {
    if state.is_animating(IconId::FnReload) {
        let current = state.get_rotation(IconId::FnReload).unwrap_or(0.0);
        state.update_rotation(
            IconId::FnReload,
            (current + RELOAD_ROTATION_SPEED * delta_ms) % 360.0,
        );
    }
}
