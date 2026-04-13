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

/// 图标按钮尺寸
pub const ICON_BUTTON_SIZE: f32 = 28.0;           // 标准图标按钮 28×28
pub const ICON_BUTTON_SIZE_COMPACT: f32 = 24.0;   // 紧凑图标按钮 24×24
pub const ICON_SIZE_COMPACT: f32 = 12.0;          // 紧凑按钮内图标视觉尺寸

/// 按钮水平 / 垂直内边距（规范 8×12）
pub const BTN_PADDING_H: f32 = 12.0;
pub const BTN_PADDING_V: f32 = 8.0;

/// 按钮组之间的间距
pub const BUTTON_GROUP_SPACING: f32 = 4.0;

/// 顶栏 / Tab 行高
pub const TOP_BAR_HEIGHT: f32 = 36.0;
/// Tab 标签默认宽度
pub const TAB_CHIP_WIDTH: f32 = 140.0;
/// 底栏（状态行等）固定高度
pub const BOTTOM_BAR_HEIGHT: f32 = 36.0;

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

// ============ 设置中心布局常量 ============

/// 设置中心模态框宽度比例（主窗口的 85%）
pub const SETTINGS_MODAL_WIDTH_RATIO: f32 = 0.85;
/// 设置中心模态框最小宽度
pub const SETTINGS_MODAL_MIN_WIDTH: f32 = 800.0;
/// 设置中心模态框最大宽度
pub const SETTINGS_MODAL_MAX_WIDTH: f32 = 1200.0;
/// 设置中心模态框高度比例（主窗口的 80%）
pub const SETTINGS_MODAL_HEIGHT_RATIO: f32 = 0.80;
/// 设置中心模态框最小高度
pub const SETTINGS_MODAL_MIN_HEIGHT: f32 = 600.0;

/// 设置中心侧边栏宽度
pub const SETTINGS_SIDEBAR_WIDTH: f32 = 220.0;
/// 设置中心侧边栏内边距
pub const SETTINGS_SIDEBAR_PADDING: f32 = 12.0;
/// 设置中心侧边栏项间距
pub const SETTINGS_SIDEBAR_ITEM_SPACING: f32 = 4.0;

/// 设置中心标题栏高度
pub const SETTINGS_HEADER_HEIGHT: f32 = 32.0;

/// 设置中心 Tab 栏高度
pub const SETTINGS_TAB_HEIGHT: f32 = 40.0;
/// 设置中心 Tab 项间距
pub const SETTINGS_TAB_SPACING: f32 = 12.0;
/// 设置中心 Tab 选中指示条高度
pub const SETTINGS_TAB_INDICATOR_HEIGHT: f32 = 2.0;

/// 设置中心内容区内边距
pub const SETTINGS_CONTENT_PADDING: f32 = 24.0;
/// 设置中心配置项间距
pub const SETTINGS_ITEM_SPACING: f32 = 16.0;
/// 设置中心标题与描述间距
pub const SETTINGS_LABEL_DESC_SPACING: f32 = 4.0;

// ============ 弹窗样式常量 ============

/// 弹窗圆角半径
pub const MODAL_RADIUS: f32 = 12.0;
/// 弹窗阴影偏移 Y
pub const MODAL_SHADOW_OFFSET_Y: f32 = 4.0;
/// 弹窗阴影模糊半径
pub const MODAL_SHADOW_BLUR: f32 = 16.0;
/// 弹窗阴影透明度
pub const MODAL_SHADOW_ALPHA: f32 = 0.25;
/// 弹窗边框宽度
pub const MODAL_BORDER_WIDTH: f32 = 1.0;

// ============ 标签栏布局常量 ============

/// 标签页关闭按钮图标视觉宽度
pub const TAB_CLOSE_ICON_W: f32 = 22.0;
/// 标签页关闭按钮可交互区域宽度
pub const TAB_CLOSE_HIT_W: f32 = 24.0;
/// 标签文字与关闭按钮间距
pub const TAB_LABEL_CLOSE_SPACING: f32 = 3.0;
/// 标签左右 padding（chip 内层）
pub const TAB_CHIP_PAD_H: f32 = 4.0;
/// 标签页最小宽度（含关闭按钮 + padding + 间距）
pub const TAB_CHIP_MIN_WIDTH: f32 = TAB_CLOSE_HIT_W + TAB_CHIP_PAD_H * 2.0 + TAB_LABEL_CLOSE_SPACING; // ≈ 35px

// ============ Breadcrumb 布局常量 ============

/// Breadcrumb 区域高度
pub const BREADCRUMB_HEIGHT: f32 = 28.0;
/// Breadcrumb 水平内边距
pub const BREADCRUMB_PADDING_H: f32 = 12.0;
/// Breadcrumb 垂直内边距
pub const BREADCRUMB_PADDING_V: f32 = 6.0;
/// 左侧会话标识与右侧按钮间距
pub const BREADCRUMB_SECTIONS_GAP: f32 = 16.0;
/// 右侧按钮间距
pub const BREADCRUMB_BTN_GAP: f32 = 4.0;
/// Breadcrumb 浮动图标大小
pub const BREADCRUMB_FLOAT_ICON_SIZE: f32 = 22.0;
/// Breadcrumb 浮动图标与边缘距离
pub const BREADCRUMB_FLOAT_ICON_MARGIN: f32 = 8.0;

// ============ SFTP 布局常量 ============

/// SFTP 面板顶栏高度
pub const SFTP_TOP_BAR_HEIGHT: f32 = 36.0;
/// SFTP 文件行高度
pub const SFTP_FILE_ROW_HEIGHT: f32 = 28.0;
/// SFTP 面板最小高度
pub const SFTP_PANEL_MIN_HEIGHT: f32 = 150.0;
/// SFTP 面板默认比例（上下布局）
pub const SFTP_PANEL_DEFAULT_RATIO: f32 = 0.4;
/// SFTP 左右布局比例
pub const SFTP_PANEL_SIDE_RATIO: f32 = 0.4;

/// SFTP 文件列表列宽
pub const SFTP_COL_PERMISSIONS: f32 = 80.0;
pub const SFTP_COL_SIZE: f32 = 60.0;
pub const SFTP_COL_MODIFIED: f32 = 120.0;
pub const SFTP_COL_SPACING: f32 = 8.0;
pub const SFTP_COL_NAME_ICON_SPACING: f32 = 6.0;
