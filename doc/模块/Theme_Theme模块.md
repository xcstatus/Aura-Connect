# Theme 模块 - 技术规格文档

> 模板版本：v1.0
> 适用阶段：架构设计 / 开发实现 / 代码审查 / 维护交接
> 维护人：待填写
> 最后更新：2026-04-10

---

## 一、架构概览

### 1.1 模块职责

Theme 模块是 RustSsh 的设计系统核心，负责：

- **配色方案管理**：定义和管理所有 UI 颜色、终端颜色、ANSI 调色板
- **用户配色支持**：用户自定义配色创建、编辑、导入导出
- **布局常量**：提供组件几何尺寸、圆角、间距等常量
- **动画引擎**：提供缓动函数、插值计算等动画工具
- **Iced 桥接**：将设计 Token 映射为 Iced 框架的 Theme
- **图标系统**：SVG 图标资源加载、状态驱动着色、旋转动画

### 1.2 目录结构

```
src/theme/
├── mod.rs           # 模块入口，公共 API 统一导出
├── tokens.rs        # 配色方案 + DesignTokens + 颜色工具函数
├── user_scheme.rs   # 用户自定义配色数据结构和管理
├── import_export.rs # 配色方案导入导出
├── layout.rs        # 布局常量（尺寸、圆角、动画时长）
├── animation.rs     # 缓动函数 + 插值引擎
├── iced_bridge.rs   # DesignTokens → Iced Theme 桥接
└── icons.rs         # 图标系统（注册表、状态、渲染）
```

### 1.3 核心数据结构

| 结构 | 文件 | 说明 |
| :--- | :--- | :--- |
| `ColorScheme` | `tokens.rs` | 完整配色方案（UI + 终端，可独立覆盖） |
| `DesignTokens` | `tokens.rs` | UI 专用色票（从 ColorScheme 派生） |
| `DebugTokens` | `tokens.rs` | DebugOverlay 专用色票 |
| `UserColorScheme` | `user_scheme.rs` | 用户自定义配色 |
| `UserColorSchemes` | `user_scheme.rs` | 用户配色列表 |
| `ExportedScheme` | `user_scheme.rs` | 导入导出格式 |
| `IconId` | `icons.rs` | 图标唯一标识枚举 |
| `IconState` | `icons.rs` | 图标状态枚举（决定颜色） |
| `IconOptions` | `icons.rs` | 图标渲染选项 |
| `IconAnimationState` | `icons.rs` | 图标动画状态管理器 |

### 1.4 设计原则

```
Foundation Colors (基础色)
    ↓
Surface Colors (表面色)
    ↓
Semantic Tokens (语义色)
```

1. **Foundation 不可直接使用**：最底层的 RGB 值，仅通过 Surface/Semantic 派生
2. **语义化命名**：所有 Token 使用语义化名称（如 `success` 而非 `green`）
3. **主题无关**：所有组件通过 `DesignTokens` 获取颜色，自动适配主题切换
4. **禁止硬编码**：严禁在业务代码中直接写 `#RRGGBB` 色值

---

## 二、公开 API

### 2.1 模块导出（mod.rs）

```rust
// 动画
pub use animation::{
    anim_done,     // 动画是否完成
    anim_t,        // 根据 tick 计算进度 t ∈ [0,1]
    anim_t_bidir,  // 带反向的进度计算
    ease_in,       // ease-in 缓动
    ease_in_out,   // ease-in-out 缓动
    ease_out,      // ease-out 缓动
    lerp,          // 线性插值
    lerp_color,    // 颜色插值
};

// 图标
pub use icons::{
    all_icon_ids,           // 获取所有注册的图标 ID
    colorize_svg,           // SVG currentColor 替换
    compute_rotation,       // 计算旋转角度
    get_icon_meta,          // 获取图标元数据
    icon,                   // 带状态的图标视图
    icon_active,            // 激活状态图标
    icon_default,           // 默认状态图标
    icon_view,              // 渲染图标视图
    load_svg_content,       // 加载 SVG 内容
    rotate_svg_content,     // 旋转 SVG 内容
    tick_loading_animation, // 更新加载动画
    IconAnimationState,      // 动画状态管理器
    IconId,                 // 图标 ID 枚举
    IconMeta,               // 图标元数据
    IconOptions,            // 图标渲染选项
    IconSizes,              // 图标尺寸
    IconState,              // 图标状态枚举
    IconStyleConfig,        // 图标样式配置
    IconThemeAdapter,       // 主题适配器
    RELOAD_ROTATION_SPEED,  // 旋转速度常量
};

// 主题桥接
pub use iced_bridge::{
    default_rustssh_iced_theme,  // 默认 Iced Theme
    iced_extended,                // 扩展色板
    iced_palette,                 // 基础调色板
    rustssh_iced_theme,           // 根据 scheme_id 生成 Theme
};

// 色票
pub use tokens::{
    color,              // 颜色工具函数（to_hex/from_hex）
    COLOR_SCHEMES,       // 所有预设配色方案数组
    ColorScheme,         // 完整配色方案
    DebugTokens,         // DebugOverlay 色票
    DesignTokens,        // UI 色票
};

// 用户配色
pub use user_scheme::{
    ColorFieldCategory,  // 颜色字段分类（Base/Terminal/Ansi）
    COLOR_FIELDS,        // 所有可编辑颜色字段元数据
    ColorFieldMeta,      // 单个颜色字段元数据
    ExportedScheme,      // 导出格式
    get_fields_by_category, // 按分类获取字段
    UserColorScheme,     // 用户配色方案
    UserColorSchemes,    // 用户配色列表
};

// 导入导出
pub use import_export::{
    export_user_scheme_to_json,  // 导出用户配色为 JSON
    import_from_json,            // 从 JSON 导入
};
```

### 2.2 ColorScheme 关键方法

```rust
// 根据 ID 获取配色方案
pub fn from_id(id: &str) -> Option<Self>

// 预设方案（const 函数，可在编译时求值）
pub const fn terminal_dark()  -> Self  // 默认主题
pub const fn terminal_light() -> Self
pub const fn nord()           -> Self
pub const fn solarized()      -> Self
pub const fn monokai()        -> Self
pub const fn gruvbox()        -> Self
pub const fn dracula()        -> Self

// === 终端颜色衍生方法 ===
// 获取生效的终端颜色（None 则使用衍生值）

pub fn effective_term_bg(&self) -> Color           // 衍生自 bg_primary
pub fn effective_term_fg(&self) -> Color           // 衍生自 text_primary
pub fn effective_term_cursor(&self) -> Color       // 衍生自 accent_base
pub fn effective_term_cursor_bg(&self) -> Color    // 衍生自 accent_base
pub fn effective_term_selection_bg(&self) -> Color // 衍生自 selection_bg
pub fn effective_term_selection_fg(&self) -> Color // 衍生自 text_primary
pub fn effective_term_scrollbar_bg(&self) -> Color // 衍生自 surface_1
pub fn effective_term_scrollbar_fg(&self) -> Color // 衍生自 surface_3
```

### 2.3 DesignTokens 关键方法

```rust
// 从配色方案 ID 获取色票
pub fn for_color_scheme(scheme_id: &str) -> Self

// 从 ColorScheme 派生
pub fn from_scheme(scheme: &ColorScheme) -> Self

// 判断是否为暗色主题（亮度 < 0.45）
pub fn is_dark(&self) -> bool

// 获取遮罩层颜色
pub fn scrim(&self) -> Color
pub fn scrim_with_alpha(&self, alpha: f32) -> Color

// 层级遮罩（用于嵌套弹窗）
pub fn layered_scrim(&self, level: usize) -> Color
pub fn layered_scrim_with_alpha(&self, alpha: f32, level: usize) -> Color
```

### 2.4 图标系统关键函数

```rust
// 创建带状态的图标（推荐方式）
pub fn icon(id: IconId, tokens: &DesignTokens, state: IconState) -> iced::Element<'static, ()>

// 创建选项并渲染
IconOptions::new(id)
    .with_size(15)
    .with_color(tokens.text_secondary)
    .with_rotation(90.0)

// 从状态创建（自动从 DesignTokens 获取颜色）
IconOptions::from_state(id, tokens, state)
```

---

## 三、设计决策

### 3.1 为什么分为 ColorScheme 和 DesignTokens？

**ColorScheme** 包含完整信息（UI + 终端 + ANSI 调色板），用于序列化/持久化和终端颜色发送。

**DesignTokens** 仅包含 UI 使用的颜色，用于组件渲染。更轻量，避免将终端细节泄露到 UI 层。

### 3.2 为什么使用 const fn 定义主题？

所有主题定义使用 `const fn`，允许在编译时求值，减少运行时开销。适合 RustSsh 少量固定主题的场景。

### 3.3 为什么图标系统自管理 SVG？

RustSsh 使用独立的 SVG 资源文件，通过 `LazyLock` 延迟加载，而非将 SVG 内联为字符串。这样：
- 便于设计师更新图标
- 减少编译产物大小
- 支持运行时切换图标主题

### 3.4 为什么动画基于 tick_count 而非时间？

RustSsh 使用动态帧率（`iced::time::every(tick_ms)`），tick_ms 在 16-100ms 间浮动。动画基于 `tick_count * tick_ms` 计算进度，确保动画在低帧率下仍然平滑。

---

## 四、关键实现细节

### 4.1 配色方案结构

```
ColorScheme
├── UI 颜色
│   ├── 背景层级: bg_primary, bg_secondary, bg_header, bg_popup
│   ├── 表面层级: surface_1, surface_2, surface_3, tab_active_bg
│   ├── 文本层级: text_primary, text_secondary, text_disabled
│   ├── 边框层级: border_default, border_subtle
│   ├── Accent 色: accent_base, accent_hover, accent_active, on_accent_label
│   └── 状态色: success, warning, error, info
│
├── 终端颜色
│   ├── 基础色: term_bg, term_fg, term_cursor, term_cursor_bg
│   ├── 选区: term_selection_bg, term_selection_fg
│   ├── 滚动条: term_scrollbar_bg, term_scrollbar_fg
│   └── 行高亮: line_highlight
│
└── ANSI 调色板
    └── ansi: [Color; 16]  // ANSI 0-15
```

### 4.2 Iced 桥接映射关系

```
DesignTokens
    │
    ├── iced_palette() ──→ Palette
    │   (background, text, primary, success, warning, danger)
    │
    ├── iced_extended() ──→ Extended
    │   (background.weakest ~ strongest,
    │    primary.base/weak/strong,
    │    secondary, success, warning, danger,
    │    is_dark)
    │
    └── rustssh_iced_theme() ──→ Theme::Custom
        (组合 Palette + Extended)
```

### 4.3 图标状态机

```
IconState::Default  ──→ text_secondary (#A0A0A5)
IconState::Hovered ──→ text_primary (#FFFFFF)
IconState::Active   ──→ accent_base (#00FF80)
IconState::Disabled ──→ text_disabled (#6C6C70)
IconState::Error    ──→ error (#FF3B30)
IconState::Warning  ──→ warning (#FFCC00)
IconState::Success  ──→ success (#00FF80)
```

### 4.4 动画时长常量（layout.rs）

| 常量 | 值 | 用途 |
| :--- | :--- | :--- |
| `DURATION_HOVER_MS` | 120ms | 悬停过渡 |
| `DURATION_CLICK_MS` | 80ms | 点击缩放 |
| `DURATION_TAB_MS` | 150ms | Tab 切换 |
| `DURATION_MODAL_MS` | 200ms | 模态打开 |
| `DURATION_MODAL_CLOSE_MS` | 150ms | 模态关闭 |
| `DURATION_TAB_NEW_MS` | 200ms | 新建 Tab |
| `DURATION_TAB_CLOSE_MS` | 150ms | 关闭 Tab |

**禁止在组件代码中硬编码这些数字，必须引用 layout.rs 常量。**

### 4.5 缓动函数选用规则

| 动画场景 | 缓动函数 | 原因 |
| :--- | :--- | :--- |
| 按钮悬停进入 | `ease_out` | 快速到达悬停状态，有力 |
| 按钮按下 | `ease_in` | 减速下沉，真实感 |
| 按钮悬停退出 | `ease_in` | 逆向自然 |
| Tab 切换 | `ease_in_out` | 平滑过渡 |
| 模态打开 | `ease_out` | 进入快 |
| 模态关闭 | `ease_in` | 退出从容 |
| 颜色过渡 | `ease_out` | 变化不应突兀 |
| 加载/进度 | `linear` | 匀速，时间指示准确 |

---

## 五、与其他模块的边界

| 边界接口 | 协作模块 | 交互方式 |
| :--- | :--- | :--- |
| `DesignTokens` | `src/app/` | 应用初始化时创建并持有 |
| `rustssh_iced_theme()` | `iced` 框架 | 传入 Iced Runtime |
| `icon_view()` | UI 组件 | 返回 `iced::Element` |
| 主题切换 | `src/settings/` | 设置变更触发重建 |

**Theme 模块不直接操作 UI**，只提供数据结构和渲染参数。UI 组件负责根据状态选择合适的 Token。

---

## 六、性能特征

### 6.1 启动性能

- `ICON_REGISTRY`：使用 `LazyLock`，首次访问时一次性加载
- `DesignTokens::for_color_scheme()`：O(1) 哈希表查找
- 主题切换：仅涉及内存中的 `DesignTokens` 替换，无 IO

### 6.2 运行时性能

- 颜色获取：`DesignTokens` 字段访问为 O(1)
- 缓动函数：`#[inline]` 标记，编译时内联
- 图标渲染：SVG 文件路径查找 O(1)

### 6.3 内存特征

- `ColorScheme`/`DesignTokens`：~1KB（纯数据，无堆分配）
- `ICON_REGISTRY`：~2KB（HashMap + 元数据）
- 图标动画状态：`HashMap<IconId, f32>`，按需增长

---

## 七、测试策略

| 测试类型 | 覆盖内容 | 测试文件位置 |
| :--- | :--- | :--- |
| 单元测试 | `animation.rs` 缓动函数边界 | 内联在 `animation.rs` |
| 单元测试 | `tokens.rs` 亮度计算 | 内联在 `tokens.rs` |
| 单元测试 | `icons.rs` 状态映射 | 内联在 `icons.rs` |
| 集成测试 | 主题切换后 UI 渲染正确性 | `src/app/` 相关测试 |

---

## 八、新增主题指南

### 8.1 步骤 1：在 ColorScheme 实现新主题

在 `tokens.rs` 中添加 `const fn`：

```rust
/// My Custom Theme
pub const fn my_custom() -> Self {
    let ansi = [/* ANSI 0-15 */];
    Self {
        id: "my_custom",
        name: "My Custom",
        // UI 颜色
        bg_primary: Color::from_rgb8(0x??, 0x??, 0x??),
        // ... 其他字段
        ansi,
    }
}
```

### 8.2 步骤 2：在 from_id 添加匹配

```rust
pub fn from_id(id: &str) -> Option<Self> {
    match id {
        "my_custom" => Some(Self::my_custom()),
        // ...
    }
}
```

### 8.3 步骤 3：在 COLOR_SCHEMES 数组添加

```rust
pub const COLOR_SCHEMES: &[ColorScheme] = &[
    // ... 现有主题
    ColorScheme::my_custom(),
];
```

### 8.4 步骤 4：在设置中心添加选项（可选）

在 `src/settings/` 相关文件中添加下拉选项。

### 8.5 步骤 5：创建主题文档（推荐）

在 `doc/色彩规范/` 下创建 `主题_MyCustom.md`，包含色卡展示和使用指南。

---

## 九、新增图标指南

### 9.1 步骤 1：添加 SVG 文件

将 SVG 文件放入 `assets/icon/`，命名规范：`kebab-case.svg`

### 9.2 步骤 2：在 IconId 枚举添加变体

```rust
pub enum IconId {
    // ... 现有变体
    MyNewIcon,  // kebab-case.svg
}
```

### 9.3 步骤 3：在 ICON_REGISTRY 注册

```rust
map.insert(IconId::MyNewIcon, IconMeta {
    path: "assets/icon/kebab-case.svg",
    width: 15,
    height: 15,
});
```

### 9.4 步骤 4：在代码中使用

```rust
use crate::theme::{icon, IconId, IconState};

// 带状态使用
icon(IconId::MyNewIcon, tokens, IconState::Default)

// 带选项使用
icon_view(IconOptions::new(IconId::MyNewIcon)
    .with_size(24)
    .with_color(tokens.accent_base))
```

---

## 十、常见问题

### Q: 如何在组件中获取当前主题的 Token？

```rust
// 通过 App 持有的 tokens
let tokens = state.tokens();

// 或通过 Iced State 获取
let tokens = iced_state.theme_tokens();
```

### Q: 为什么我的图标颜色不对？

检查：
1. 是否通过 `IconOptions::from_state()` 或 `IconState::color(tokens)` 获取颜色
2. 是否在组件重建时传入了最新的 `tokens`
3. 是否在 `IconState` 枚举中添加了新状态

### Q: 动画看起来不流畅怎么办？

1. 确认使用的是 `anim_t()` 计算进度，而非直接用时间
2. 确认 `tick_ms` 是动态获取的（来自 `iced::time::every(tick_ms)`）
3. 确认没有在动画中创建新对象（禁止在 `view()` 中分配）

### Q: 主题切换后某些颜色没变？

1. 确认组件在主题切换时重建了
2. 检查是否有硬编码的颜色值没有替换为 Token
3. 确认 `app.set_theme()` 后调用了 `ctx.request_repaint()`

---

## 十一、变更历史

| 日期 | 版本 | 变更内容 | 作者 |
| :--- | :--- | :--- | :--- |
| 2026-04-10 | 1.1 | 重构 ColorScheme：终端颜色改为 Option + 衍生方法；新增用户配色模块（user_scheme.rs）；新增导入导出功能（import_export.rs） | - |
| 2026-04-10 | 1.0 | 初始版本，整理模块架构和 API 文档 | - |
