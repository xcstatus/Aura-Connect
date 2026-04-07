# RustSSH 亮色主题 (Light Theme)

**主题 ID**: `RustSshThemeId::Light`
**定位**: macOS 浅色模式、系统跟随
**Accent**: #007AFF (macOS 蓝)

---

## 一、色值总览

### 1.1 色卡展示

```
┌─────────────────────────────────────────────────────────────────────┐
│                        LIGHT THEME PALETTE                         │
├─────────────────────────────────────────────────────────────────────┤
│  BACKGROUND LAYERS                                                   │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐                   │
│  │ #FFFFFF │ │ #FAFAFA │ │ #F2F2F7 │ │ #F2F2F7 │  ← bg_primary    │
│  │ bg_pri  │ │ bg_sec  │ │ bg_head │ │ bg_pop  │    /secondary   │
│  └─────────┘ └─────────┘ └─────────┘ └─────────┘                   │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐                                │
│  │ #F2F2F7 │ │ #E5E5EA │ │ #D1D1D6 │  ← surface_1/2/3             │
│  │ surf_1  │ │ surf_2  │ │ surf_3  │                                │
│  └─────────┘ └─────────┘ └─────────┘                                │
├─────────────────────────────────────────────────────────────────────┤
│  TEXT                                                                │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐                                │
│  │ #1C1C1E │ │ #6E6E73 │ │ #A1A1A6 │  ← text_primary/secondary/dis │
│  │  text_1 │ │  text_2 │ │  text_3 │                                │
│  └─────────┘ └─────────┘ └─────────┘                                │
├─────────────────────────────────────────────────────────────────────┤
│  ACCENT                                                              │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐                    │
│  │ #007AFF │ │ #3395FF │ │ #005FCC │ │ #FFFFFF │  ← accent_base/    │
│  │ acc_b   │ │ acc_h   │ │ acc_a   │ │ on_lab  │    hover/active    │
│  └─────────┘ └─────────┘ └─────────┘ └─────────┘                    │
├─────────────────────────────────────────────────────────────────────┤
│  STATE                                                              │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐                    │
│  │ #00FF80 │ │ #FFCC00 │ │ #FF3B30 │ │ #007AFF │  ← success/warning/│
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
| `bg_primary` | #FFFFFF | rgb(255, 255, 255) | 主页面背景 |
| `bg_secondary` | #FAFAFA | rgb(250, 250, 250) | 侧边栏、面板背景 |
| `bg_header` | #F2F2F7 | rgb(242, 242, 247) | 顶栏、标签栏背景 |
| `bg_popup` | #F2F2F7 | rgb(242, 242, 247) | 弹出层、模态框背景 |
| `surface_1` | #F2F2F7 | rgb(242, 242, 247) | 卡片、输入框背景 |
| `surface_2` | #E5E5EA | rgb(229, 229, 234) | 悬停状态 |
| `surface_3` | #D1D1D6 | rgb(209, 209, 214) | 按下/选中状态 |
| `tab_active_bg` | #FFFFFF | rgb(255, 255, 255) | 当前标签页背景 |

### 2.2 文本色 (Text Colors)

| Token | Hex | 用途 | 对比度 (vs bg_primary) |
|-------|-----|------|------------------------|
| `text_primary` | #1C1C1E | 主要信息 | 16.8:1 ✓ AAA |
| `text_secondary` | #6E6E73 | 辅助说明 | 5.8:1 ✓ AA |
| `text_disabled` | #A1A1A6 | 禁用状态 | 3.2:1 ✓ AA Large |

### 2.3 边框色 (Border Colors)

| Token | RGBA | 用途 |
|-------|------|------|
| `border_default` | rgba(0, 0, 0, 0.08) | 默认边框、分隔线 |
| `border_subtle` | rgba(0, 0, 0, 0.04) | 更细的分隔 |

### 2.4 Accent 色 (Accent Colors)

| Token | Hex | 用途 |
|-------|-----|------|
| `accent_base` | #007AFF | 默认强调色、按钮、选中态 |
| `accent_hover` | #3395FF | 悬停状态 (+10% lightness) |
| `accent_active` | #005FCC | 按下/选中状态 (-10% lightness) |
| `on_accent_label` | #FFFFFF | 按钮标签（确保在蓝色背景上可读） |

### 2.5 状态色 (State Colors)

| Token | Hex | 语义 |
|-------|-----|------|
| `success` | #00FF80 | 连接成功、执行成功 |
| `warning` | #FFCC00 | 处理中、需要确认 |
| `error` | #FF3B30 | 连接失败、错误 |
| `info` | #007AFF | 提示信息、链接 |

### 2.6 终端色 (Terminal Colors)

| Token | Hex | 用途 |
|-------|-----|------|
| `terminal_bg` | #000000 | 终端背景（固定为纯黑） |
| `terminal_fg` | #FFFFFF | 终端前景 |
| `selection_bg` | rgba(0, 122, 255, 0.25) | 选中文本背景（蓝色调） |
| `line_highlight` | rgba(0, 0, 0, 0.03) | 当前行高亮 |

### 2.7 滚动条色 (Scrollbar Colors)

| Token | RGBA | 用途 |
|-------|------|------|
| `scrollbar_hover` | rgba(0, 0, 0, 0.10) | 滚动条悬停 |
| `scrollbar_active` | rgba(0, 0, 0, 0.20) | 滚动条拖动 |

---

## 三、与 Dark 主题的差异

### 3.1 Accent 色对比

| 维度 | Dark | Light | 原因 |
|------|------|-------|------|
| 基色 | #00FF80 (绿) | #007AFF (蓝) | macOS 标准；浅色配绿色对比度不足 |
| 语义 | 科技感、专业 | 系统一致 | 与 macOS 视觉语言保持一致 |

### 3.2 文本色对比

| Token | Dark | Light | 变化 |
|-------|------|-------|------|
| text_primary | #FFFFFF | #1C1C1E | 暗→亮，适应背景 |
| text_secondary | #A0A0A5 | #6E6E73 | 降低明度，避免刺眼 |
| text_disabled | #8E8E93 | #A1A1A6 | 亮色下禁用色需更明显 |

### 3.3 边框色对比

| Token | Dark | Light | 变化 |
|-------|------|-------|------|
| border_default | rgba(255,255,255,0.08) | rgba(0,0,0,0.08) | 从白底改为黑底 |
| border_subtle | rgba(255,255,255,0.04) | rgba(0,0,0,0.04) | 同上 |

---

## 四、UI 组件示例

### 4.1 按钮

| 类型 | 背景 | 文字 | 边框 | 说明 |
|------|------|------|------|------|
| Primary | accent_base (#007AFF) | on_accent_label (#FFF) | none | macOS 标准蓝 |
| Secondary | surface_1 (#F2F2F7) | text_primary (#1C1C1E) | border_default | 灰色系按钮 |
| Ghost | transparent | text_secondary (#6E6E73) | none | 文字按钮 |
| Danger | error (#FF3B30) | on_accent_label (#FFF) | none | 危险操作 |

### 4.2 输入框

| 状态 | 背景 | 边框 | 说明 |
|------|------|------|------|
| Default | #FFFFFF | border_subtle | 白色背景，区分卡片 |
| Focus | #FFFFFF | accent_base (#007AFF) | 2px solid macOS 蓝 |
| Error | #FFFFFF | error (#FF3B30) | 2px solid |
| Disabled | surface_2 (#E5E5EA) | none | 不可编辑 |

### 4.3 卡片

| 状态 | 背景 | 边框 | 阴影 |
|------|------|------|------|
| Default | #FFFFFF | border_subtle | 极轻阴影 (0 1px 3px rgba(0,0,0,0.04)) |
| Hover | surface_1 (#F2F2F7) | border_default | 无变化 |
| Selected | surface_1 + accent | border_default + accent | 无变化 |

> **注意**: Light 主题的卡片使用白色背景以与页面背景区分，这与 Dark 主题不同。

---

## 五、实现代码

### 5.1 Rust 常量定义

```rust
/// Light 主题色票 (src/theme/tokens.rs)
pub const fn light() -> DesignTokens {
    Self {
        bg_primary: Color::WHITE,
        bg_secondary: Color::from_rgb8(0xFA, 0xFA, 0xFA),
        bg_header: Color::from_rgb8(0xF2, 0xF2, 0xF7),
        bg_popup: Color::from_rgb8(0xF2, 0xF2, 0xF7),
        surface_1: Color::from_rgb8(0xF2, 0xF2, 0xF7),
        surface_2: Color::from_rgb8(0xE5, 0xE5, 0xEA),
        surface_3: Color::from_rgb8(0xD1, 0xD1, 0xD6),
        tab_active_bg: Color::WHITE,
        text_primary: Color::from_rgb8(0x1C, 0x1C, 0x1E),
        text_secondary: Color::from_rgb8(0x6E, 0x6E, 0x73),
        text_disabled: Color::from_rgb8(0xA1, 0xA1, 0xA6),
        border_default: Color::from_rgba(0.0, 0.0, 0.0, 0.08),
        border_subtle: Color::from_rgba(0.0, 0.0, 0.0, 0.04),
        accent_base: Color::from_rgb8(0x00, 0x7A, 0xFF),
        accent_hover: Color::from_rgb8(0x33, 0x95, 0xFF),
        accent_active: Color::from_rgb8(0x00, 0x5F, 0xCC),
        on_accent_label: Color::WHITE,
        success: Color::from_rgb8(0x00, 0xFF, 0x80),
        warning: Color::from_rgb8(0xFF, 0xCC, 0x00),
        error: Color::from_rgb8(0xFF, 0x3B, 0x30),
        info: Color::from_rgb8(0x5A, 0xC8, 0xFA),
        terminal_bg: Color::BLACK,
        terminal_fg: Color::WHITE,
        scrollbar_hover: Color::from_rgba(0.0, 0.0, 0.0, 0.10),
        scrollbar_active: Color::from_rgba(0.0, 0.0, 0.0, 0.20),
        line_highlight: Color::from_rgba(0.0, 0.0, 0.0, 0.03),
        selection_bg: Color::from_rgba(0.0, 0.478, 1.0, 0.25),
    }
}
```

---

## 六、设计决策

| 决策 | 原因 |
|------|------|
| Accent 改为 macOS 蓝 | 浅色背景上绿色对比度过高，蓝色更符合 macOS 设计语言 |
| 卡片使用白色背景 | 与页面背景区分，增加层次感 |
| text_secondary 降低明度 | 浅色背景需要更柔和的次要文本 |
| 边框使用 rgba(0,0,0,x) | 与浅色背景融合，避免突兀 |
| terminal_bg 仍为纯黑 | 终端标准不受主题影响 |
