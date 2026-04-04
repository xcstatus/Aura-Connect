//! 组件几何与动效 token（见 `doc/组件设计规范.md`）。

/// Button 圆角 6px（macOS 风格）
pub const RADIUS_BUTTON: f32 = 6.0;
/// Tab 圆角 8px
pub const RADIUS_TAB: f32 = 8.0;
/// Popup 圆角 10px
pub const RADIUS_POPUP: f32 = 10.0;
/// Panel 圆角 8px
pub const RADIUS_PANEL: f32 = 8.0;

/// 紧凑按钮高度
pub const BTN_HEIGHT_COMPACT: f32 = 28.0;
/// 标准按钮高度
pub const BTN_HEIGHT_STANDARD: f32 = 32.0;

/// 按钮水平 / 垂直内边距（规范 8×12）
pub const BTN_PADDING_H: f32 = 12.0;
pub const BTN_PADDING_V: f32 = 8.0;

/// 顶栏 / Tab 行高
pub const TOP_BAR_HEIGHT: f32 = 32.0;
/// 底栏（状态行等）固定高度
pub const BOTTOM_BAR_HEIGHT: f32 = 32.0;

/// 侧栏参考宽度
pub const SIDEBAR_WIDTH: f32 = 220.0;
pub const SIDEBAR_PADDING: f32 = 8.0;

/// 终端正文建议字号
pub const TERMINAL_FONT_SIZE_BODY: f32 = 13.0;
pub const TERMINAL_FONT_SIZE_BODY_ALT: f32 = 14.0;
pub const TERMINAL_LINE_HEIGHT: f32 = 1.4;

/// 侧栏分组标题
pub const SIDEBAR_GROUP_FONT_SIZE: f32 = 11.0;

/// 滚动条宽度（macOS 风格）
pub const SCROLLBAR_WIDTH: f32 = 6.0;

/// 终端叠层滚动条容器右侧 padding（与 `iced_app/terminal_rich.rs` `terminal_scrollbar_overlay` 一致）。
pub const TERMINAL_SCROLLBAR_OVERLAY_PAD_RIGHT: f32 = 10.0;

/// 终端网格命中测试：从 scroll rect 右边缘向内排除的像素（轨道 + 右侧 padding）。
/// 点击落在此条带内不作为终端格点映射，避免选区列偏移。
#[inline]
pub fn terminal_scroll_hit_exclude_right_px() -> f32 {
    SCROLLBAR_WIDTH + TERMINAL_SCROLLBAR_OVERLAY_PAD_RIGHT
}

/// 悬停过渡建议时长（ms），供后续动画层使用
pub const DURATION_HOVER_MS: u16 = 120;
/// 点击缩放时长（ms）
pub const DURATION_CLICK_MS: u16 = 80;
/// Tab 切换时长（ms）
pub const DURATION_TAB_MS: u16 = 150;

/// 点击缩放比例（规范 0.97）
pub const SCALE_CLICK: f32 = 0.97;
/// 悬停放大比例
pub const SCALE_HOVER: f32 = 1.02;

/// 模态弹窗打开时长（ms）
pub const DURATION_MODAL_MS: u16 = 200;
/// 模态弹窗关闭时长（ms）
pub const DURATION_MODAL_CLOSE_MS: u16 = 150;
/// 新建 Tab 展开时长（ms）
pub const DURATION_TAB_NEW_MS: u16 = 200;
/// 关闭 Tab 收缩时长（ms）
pub const DURATION_TAB_CLOSE_MS: u16 = 150;

/// 焦点环：2px、Accent 60% 透明（描边实现放在具体控件）
pub const FOCUS_OUTLINE_WIDTH: f32 = 2.0;
pub const FOCUS_OUTLINE_ALPHA: f32 = 0.6;

/// 滚动条透明度常量
pub const SCROLLBAR_HIDE_ALPHA: f32 = 0.08;
pub const SCROLLBAR_HOVER_ALPHA: f32 = 0.25;
