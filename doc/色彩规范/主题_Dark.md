# RustSSH 暗色主题 (Dark Theme)

**主题 ID**: `RustSshThemeId::Dark`
**定位**: 默认主力主题，面向专业开发者
**Accent**: #00FF80 (科技绿)

---

## 一、色值总览

### 1.1 色卡展示

```
┌─────────────────────────────────────────────────────────────────────┐
│                        DARK THEME PALETTE                          │
├─────────────────────────────────────────────────────────────────────┤
│  BACKGROUND LAYERS                                                   │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐                   │
│  │ #1C1C1E │ │ #232326 │ │ #28282D │ │ #2C2C2E │  ← bg_secondary   │
│  │ bg_pri  │ │ bg_sec  │ │ bg_head │ │ bg_pop  │                   │
│  └─────────┘ └─────────┘ └─────────┘ └─────────┘                   │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐                                │
│  │ #2C2C2E │ │ #3A3A3C │ │ #48484A │  ← surface_1/2/3              │
│  │ surf_1  │ │ surf_2  │ │ surf_3  │                                │
│  └─────────┘ └─────────┘ └─────────┘                                │
├─────────────────────────────────────────────────────────────────────┤
│  TEXT                                                                │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐                                │
│  │ #FFFFFF │ │ #A0A0A5 │ │ #8E8E93 │  ← text_primary/secondary/dis  │
│  │  text_1 │ │  text_2 │ │  text_3 │                                │
│  └─────────┘ └─────────┘ └─────────┘                                │
├─────────────────────────────────────────────────────────────────────┤
│  ACCENT                                                              │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐                    │
│  │ #00FF80 │ │ #33FF99 │ │ #00CC66 │ │ #000000 │  ← accent_base/    │
│  │ acc_b   │ │ acc_h   │ │ acc_a   │ │ on_lab  │    hover/active    │
│  └─────────┘ └─────────┘ └─────────┘ └─────────┘                    │
├─────────────────────────────────────────────────────────────────────┤
│  STATE                                                              │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐                    │
│  │ #00FF80 │ │ #FFCC00 │ │ #FF3B30 │ │ #5AC8FA │  ← success/warning/ │
│  │ success │ │ warning │ │  error  │ │  info   │    error/info      │
│  └─────────┘ └─────────┘ └─────────┘ └─────────┘                    │
├─────────────────────────────────────────────────────────────────────┤
│  TERMINAL                                                            │
│  ┌─────────┐ ┌─────────┐                                            │
│  │ #000000 │ │ #FFFFFF │  ← terminal_bg / terminal_fg               │
│  └─────────┘ └─────────┘                                            │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 二、完整 Token 列表

### 2.1 背景色 (Background Colors)

| Token | Hex | RGBA | 用途 |
|-------|-----|------|------|
| `bg_primary` | #1C1C1E | rgb(28, 28, 30) | 主页面背景 |
| `bg_secondary` | #232326 | rgb(35, 35, 38) | 侧边栏、面板背景 |
| `bg_header` | #28282D | rgb(40, 40, 45) | 顶栏、标签栏背景 |
| `bg_popup` | #2C2C2E | rgb(44, 44, 46) | 弹出层、模态框背景 |
| `surface_1` | #2C2C2E | rgb(44, 44, 46) | 卡片、输入框背景 |
| `surface_2` | #3A3A3C | rgb(58, 58, 60) | 悬停状态 |
| `surface_3` | #48484A | rgb(72, 72, 74) | 按下/选中状态 |
| `tab_active_bg` | #000000 | rgb(0, 0, 0) | 当前标签页背景 |

### 2.2 文本色 (Text Colors)

| Token | Hex | 用途 | 对比度 (vs bg_primary) |
|-------|-----|------|------------------------|
| `text_primary` | #FFFFFF | 主要信息 | 13.5:1 ✓ AAA |
| `text_secondary` | #A0A0A5 | 辅助说明 | 5.8:1 ✓ AA |
| `text_disabled` | #8E8E93 | 禁用状态 | 4.2:1 ✓ AA |

### 2.3 边框色 (Border Colors)

| Token | RGBA | 用途 |
|-------|------|------|
| `border_default` | rgba(255, 255, 255, 0.08) | 默认边框、分隔线 |
| `border_subtle` | rgba(255, 255, 255, 0.04) | 更细的分隔 |

### 2.4 Accent 色 (Accent Colors)

| Token | Hex | 用途 |
|-------|-----|------|
| `accent_base` | #00FF80 | 默认强调色、按钮、选中态 |
| `accent_hover` | #33FF99 | 悬停状态 (+10% lightness) |
| `accent_active` | #00CC66 | 按下/选中状态 (-10% lightness) |
| `on_accent_label` | #000000 | 按钮标签（确保在绿色背景上可读） |

### 2.5 状态色 (State Colors)

| Token | Hex | 语义 |
|-------|-----|------|
| `success` | #00FF80 | 连接成功、执行成功 |
| `warning` | #FFCC00 | 处理中、需要确认 |
| `error` | #FF3B30 | 连接失败、错误 |
| `info` | #5AC8FA | 提示信息、链接 |

### 2.6 终端色 (Terminal Colors)

| Token | Hex | 用途 |
|-------|-----|------|
| `terminal_bg` | #000000 | 终端背景（固定为纯黑） |
| `terminal_fg` | #FFFFFF | 终端前景 |
| `selection_bg` | rgba(0, 255, 128, 0.25) | 选中文本背景 |
| `line_highlight` | rgba(255, 255, 255, 0.03) | 当前行高亮 |

### 2.7 滚动条色 (Scrollbar Colors)

| Token | RGBA | 用途 |
|-------|------|------|
| `scrollbar_hover` | rgba(255, 255, 255, 0.10) | 滚动条悬停 |
| `scrollbar_active` | rgba(255, 255, 255, 0.20) | 滚动条拖动 |

---

## 三、颜色语义使用指南

### 3.1 何时使用 accent_base (#00FF80)

| 场景 | 示例 |
|------|------|
| ✓ 主按钮背景 | 「连接」「保存」按钮 |
| ✓ 选中的标签/会话 | 左侧栏当前会话 |
| ✓ 活动指示器 | 正在输入的光标 |
| ✓ 成功状态 | 连接已建立 |
| ✓ 图标强调 | 工具栏选中的图标 |
| ✗ 大面积使用 | 背景色、卡片背景 |

### 3.2 何时使用 text_secondary (#A0A0A5)

| 场景 | 示例 |
|------|------|
| ✓ 辅助说明文字 | 「上次连接: 2小时前」 |
| ✓ 标签描述 | 「主机名」标签 |
| ✓ 次要信息 | 会话列表的连接地址 |
| ✗ 主要操作引导 | 按钮文字、标题 |

### 3.3 状态色使用规范

```
连接状态 → 颜色映射

  ┌──────────┐
  │ ● 已连接  │  → success (#00FF80)
  └──────────┘

  ┌──────────┐
  │ ◐ 连接中  │  → warning (#FFCC00)
  └──────────┘

  ┌──────────┐
  │ ● 断开   │  → text_disabled (#8E8E93)
  └──────────┘

  ┌──────────┐
  │ ✕ 错误   │  → error (#FF3B30)
  └──────────┘
```

---

## 四、UI 组件示例

### 4.1 按钮

| 类型 | 背景 | 文字 | 边框 |
|------|------|------|------|
| Primary | accent_base (#00FF80) | on_accent_label (#000) | none |
| Secondary | surface_1 (#2C2C2E) | text_primary (#FFF) | border_default |
| Ghost | transparent | text_secondary (#A0A0A5) | none |
| Danger | error (#FF3B30) | on_accent_label (#FFF) | none |

### 4.2 输入框

| 状态 | 背景 | 边框 | 说明 |
|------|------|------|------|
| Default | surface_1 (#2C2C2E) | border_subtle | rgba(255,255,255,0.04) |
| Focus | surface_1 (#2C2C2E) | accent_base (#00FF80) | 2px solid |
| Error | surface_1 (#2C2C2E) | error (#FF3B30) | 2px solid |
| Disabled | surface_2 (#3A3A3C) | none | 不可编辑 |

### 4.3 卡片

| 状态 | 背景 | 边框 | 阴影 |
|------|------|------|------|
| Default | surface_1 (#2C2C2E) | border_subtle | 无 |
| Hover | surface_2 (#3A3A3C) | border_default | 无（纯色风格） |
| Selected | surface_1 + accent | border_default + accent | 无 |

---

## 五、实现代码

### 5.1 Rust 常量定义

```rust
/// Dark 主题色票 (src/theme/tokens.rs)
pub const fn dark() -> DesignTokens {
    Self {
        bg_primary: Color::from_rgb8(0x1C, 0x1C, 0x1E),
        bg_secondary: Color::from_rgb8(0x23, 0x23, 0x26),
        bg_header: Color::from_rgb8(0x28, 0x28, 0x2D),
        bg_popup: Color::from_rgb8(0x2C, 0x2C, 0x2E),
        surface_1: Color::from_rgb8(0x2C, 0x2C, 0x2E),
        surface_2: Color::from_rgb8(0x3A, 0x3A, 0x3C),
        surface_3: Color::from_rgb8(0x48, 0x48, 0x4A),
        tab_active_bg: Color::BLACK,
        text_primary: Color::WHITE,
        text_secondary: Color::from_rgb8(0xA0, 0xA0, 0xA5),
        text_disabled: Color::from_rgb8(0x8E, 0x8E, 0x93),
        border_default: Color::from_rgba(1.0, 1.0, 1.0, 0.08),
        border_subtle: Color::from_rgba(1.0, 1.0, 1.0, 0.04),
        accent_base: Color::from_rgb8(0x00, 0xFF, 0x80),
        accent_hover: Color::from_rgb8(0x33, 0xFF, 0x99),
        accent_active: Color::from_rgb8(0x00, 0xCC, 0x66),
        on_accent_label: Color::BLACK,
        success: Color::from_rgb8(0x00, 0xFF, 0x80),
        warning: Color::from_rgb8(0xFF, 0xCC, 0x00),
        error: Color::from_rgb8(0xFF, 0x3B, 0x30),
        info: Color::from_rgb8(0x5A, 0xC8, 0xFA),
        terminal_bg: Color::BLACK,
        terminal_fg: Color::WHITE,
        scrollbar_hover: Color::from_rgba(1.0, 1.0, 1.0, 0.10),
        scrollbar_active: Color::from_rgba(1.0, 1.0, 1.0, 0.20),
        line_highlight: Color::from_rgba(1.0, 1.0, 1.0, 0.03),
        selection_bg: Color::from_rgba(0.0, 1.0, 0.5, 0.25),
    }
}
```

---

## 六、设计决策

| 决策 | 原因 |
|------|------|
| 终端背景固定为纯黑 | 终端标准，确保色彩准确性 |
| Accent 使用高饱和绿 | 科技感强，与传统终端提示符呼应 |
| text_secondary 使用 #A0A0A5 | 确保对比度 ≥ 4.5:1 |
| surface_3 不设置边框 | 避免层次过于复杂 |
| tab_active_bg 使用纯黑 | 与终端融合，突出内容区 |
