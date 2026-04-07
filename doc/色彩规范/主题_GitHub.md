# RustSSH GitHub 风格主题 (GitHub Theme)

**主题 ID**: `RustSshThemeId::GitHub`
**定位**: 开发者熟悉的界面、GitHub 生态集成
**Accent**: #238636 (GitHub 绿)

---

## 一、色值总览

### 1.1 色卡展示

```
┌─────────────────────────────────────────────────────────────────────┐
│                    GITHUB THEME PALETTE                            │
├─────────────────────────────────────────────────────────────────────┤
│  BACKGROUND LAYERS                                                   │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐                   │
│  │ #161B22│ │ #0D1117│ │ #21262D │ │ #21262D │  ← bg_primary    │
│  │ bg_pri  │ │ bg_sec  │ │ bg_head │ │ bg_pop  │    (GitHub Dark) │
│  └─────────┘ └─────────┘ └─────────┘ └─────────┘                   │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐                                │
│  │ #21262D│ │ #30363D │ │ #484F58 │  ← surface_1/2/3             │
│  │ surf_1  │ │ surf_2  │ │ surf_3  │                                │
│  └─────────┘ └─────────┘ └─────────┘                                │
├─────────────────────────────────────────────────────────────────────┤
│  TEXT                                                                │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐                                │
│  │ #E6EDF3│ │ #8B949E │ │ #6E7681 │  ← text_primary/secondary/dis │
│  │  text_1 │ │  text_2 │ │  text_3 │                                │
│  └─────────┘ └─────────┘ └─────────┘                                │
├─────────────────────────────────────────────────────────────────────┤
│  ACCENT                                                              │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐                    │
│  │ #238636│ │ #2EA043 │ │ #196C2E │ │ #FFFFFF │  ← accent_base/    │
│  │ acc_b   │ │ acc_h   │ │ acc_a   │ │ on_lab  │    hover/active    │
│  └─────────┘ └─────────┘ └─────────┘ └─────────┘                    │
├─────────────────────────────────────────────────────────────────────┤
│  STATE                                                              │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐                    │
│  │ #3FB950│ │ #D29922 │ │ #F85149 │ │ #58A6FF │  ← success/warning/│
│  │ success │ │ warning │ │  error  │ │  info   │    error/info      │
│  └─────────┘ └─────────┘ └─────────┘ └─────────┘                    │
├─────────────────────────────────────────────────────────────────────┤
│  TERMINAL                                                            │
│  ┌─────────┐ ┌─────────┐                                            │
│  │ #0D1117│ │ #E6EDF3 │  ← terminal_bg / terminal_fg               │
│  └─────────┘ └─────────┘                                            │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 二、完整 Token 列表

### 2.1 背景色 (Background Colors)

| Token | Hex | RGB | 用途 |
|-------|-----|-----|------|
| `bg_primary` | #161B22 | 22, 27, 34 | 主页面背景 |
| `bg_secondary` | #0D1117 | 13, 17, 23 | 侧边栏、面板背景（更深） |
| `bg_header` | #21262D | 33, 38, 45 | 顶栏、标签栏背景 |
| `bg_popup` | #21262D | 33, 38, 45 | 弹出层、模态框背景 |
| `surface_1` | #21262D | 33, 38, 45 | 卡片、输入框背景 |
| `surface_2` | #30363D | 48, 54, 61 | 悬停状态 |
| `surface_3` | #484F58 | 72, 79, 88 | 按下/选中状态 |
| `tab_active_bg` | #0D1117 | 13, 17, 23 | 当前标签页背景 |

### 2.2 文本色 (Text Colors)

| Token | Hex | RGB | 用途 | 对比度 (vs bg_primary) |
|-------|-----|-----|------|------------------------|
| `text_primary` | #E6EDF3 | 230, 237, 243 | 主要信息 | 12.3:1 ✓ AAA |
| `text_secondary` | #8B949E | 139, 148, 158 | 辅助说明 | 5.5:1 ✓ AA |
| `text_disabled` | #6E7681 | 110, 118, 129 | 禁用状态 | 3.8:1 ✓ AA Large |

### 2.3 边框色 (Border Colors)

| Token | RGBA | Hex | 用途 |
|-------|------|-----|------|
| `border_default` | rgba(139, 148, 158, 0.13) | #484F58 @ 13% | 默认边框、分隔线 |
| `border_subtle` | rgba(139, 148, 158, 0.08) | #484F58 @ 8% | 更细的分隔 |

### 2.4 Accent 色 (Accent Colors)

| Token | Hex | RGB | 用途 |
|-------|-----|-----|------|
| `accent_base` | #238636 | 35, 134, 54 | 默认强调色（GitHub 绿） |
| `accent_hover` | #2EA043 | 46, 160, 67 | 悬停状态 |
| `accent_active` | #196C2E | 25, 108, 46 | 按下/选中状态 |
| `on_accent_label` | #FFFFFF | 255, 255, 255 | 按钮标签 |

### 2.5 状态色 (State Colors)

| Token | Hex | RGB | 语义 | GitHub 原文名 |
|-------|-----|-----|------|---------------|
| `success` | #3FB950 | 63, 185, 80 | 成功 | `color-success-fg` |
| `warning` | #D29922 | 210, 153, 34 | 警告 | `color-attention-fg` |
| `error` | #F85149 | 248, 81, 73 | 错误 | `color-danger-fg` |
| `info` | #58A6FF | 88, 166, 255 | 信息 | `color-accent-fg` |

### 2.6 终端色 (Terminal Colors)

| Token | Hex | RGB | 用途 |
|-------|-----|-----|------|
| `terminal_bg` | #0D1117 | 13, 17, 23 | 终端背景（GitHub 深色） |
| `terminal_fg` | #E6EDF3 | 230, 237, 243 | 终端前景 |
| `selection_bg` | rgba(56, 139, 253, 0.25) | rgba(56, 139, 253, 0.25) | 选中文本背景 |
| `line_highlight` | rgba(139, 148, 158, 0.08) | - | 当前行高亮 |

### 2.7 滚动条色 (Scrollbar Colors)

| Token | RGBA | 用途 |
|-------|------|------|
| `scrollbar_hover` | rgba(139, 148, 158, 0.15) | 滚动条悬停 |
| `scrollbar_active` | rgba(139, 148, 158, 0.25) | 滚动条拖动 |

---

## 三、与 Dark 主题的对比

### 3.1 核心差异

| 维度 | Dark 主题 | GitHub 主题 | 说明 |
|------|-----------|-------------|------|
| 背景深度 | #1C1C1E (28,28,30) | #161B22 (22,27,34) | GitHub 更深 |
| 强调色 | #00FF80 (科技绿) | #238636 (GitHub 绿) | 品牌一致性 |
| 边框色 | rgba(255,255,255,0.08) | rgba(139,148,158,0.13) | GitHub 可见性更好 |
| 终端背景 | #000000 (纯黑) | #0D1117 (GitHub 深) | 融入主题 |

### 3.2 GitHub 官方色彩参考

```
GitHub Dark Theme Colors (Figma 源码):

canvas-default:     #0D1117
canvas-subtle:      #161B22
canvas-inset:       #010409
border-default:     #30363D
border-muted:       #21262D
fg-default:         #E6EDF3
fg-muted:           #8B949E
fg-subtle:          #6E7681
accent-fg:          #58A6FF
success-fg:         #3FB950
attention-fg:       #D29922
danger-fg:          #F85149
```

---

## 四、UI 组件示例

### 4.1 按钮 (GitHub 风格)

| 类型 | 背景 | 文字 | 边框 | 说明 |
|------|------|------|------|------|
| Primary | accent_base (#238636) | #FFFFFF | none | GitHub 绿色按钮 |
| Secondary | surface_2 (#30363D) | text_primary | border_default | GitHub 灰色按钮 |
| Outline | transparent | text_primary | border_default | 边框按钮 |
| Danger | transparent | error (#F85149) | border_default | 危险操作 |

> **GitHub 按钮特征**: 圆角 6px，字体 12px/14px，无阴影，高度 32px/36px

### 4.2 输入框

| 状态 | 背景 | 边框 | 说明 |
|------|------|------|------|
| Default | surface_1 (#21262D) | border_default (#30363D) | GitHub 风格 |
| Focus | surface_1 | accent (#58A6FF) | 蓝色焦点环 |
| Error | surface_1 | error (#F85149) | 红色边框 |
| Disabled | surface_2 (#30363D) | none | 降低透明度 |

### 4.3 卡片

| 状态 | 背景 | 边框 | 说明 |
|------|------|------|------|
| Default | surface_1 (#21262D) | border_default (#30363D) | 明显的边框 |
| Hover | surface_2 (#30363D) | border_default | 背景变亮 |
| Selected | surface_1 + accent border | border + accent (#58A6FF) | 蓝色边框 |

---

## 五、实现代码

### 5.1 Rust 常量定义

```rust
/// GitHub 主题色票 (src/theme/tokens.rs)
pub const fn github() -> DesignTokens {
    Self {
        // 背景色 - GitHub Dark
        bg_primary: Color::from_rgb8(0x16, 0x1B, 0x22),     // #161B22
        bg_secondary: Color::from_rgb8(0x0D, 0x11, 0x17),    // #0D1117 (更深的侧边栏)
        bg_header: Color::from_rgb8(0x21, 0x26, 0x2D),       // #21262D
        bg_popup: Color::from_rgb8(0x21, 0x26, 0x2D),        // #21262D
        surface_1: Color::from_rgb8(0x21, 0x26, 0x2D),       // #21262D
        surface_2: Color::from_rgb8(0x30, 0x36, 0x3D),       // #30363D
        surface_3: Color::from_rgb8(0x48, 0x4F, 0x58),        // #484F58
        tab_active_bg: Color::from_rgb8(0x0D, 0x11, 0x17),   // #0D1117

        // 文本色
        text_primary: Color::from_rgb8(0xE6, 0xED, 0xF3),     // #E6EDF3
        text_secondary: Color::from_rgb8(0x8B, 0x94, 0x9E),   // #8B949E
        text_disabled: Color::from_rgb8(0x6E, 0x76, 0x81),   // #6E7681

        // 边框色
        border_default: Color::from_rgba(0.545, 0.580, 0.620, 0.13),  // #484F58 @ 13%
        border_subtle: Color::from_rgba(0.545, 0.580, 0.620, 0.08),   // #484F58 @ 8%

        // Accent 色 - GitHub 绿
        accent_base: Color::from_rgb8(0x23, 0x86, 0x36),       // #238636
        accent_hover: Color::from_rgb8(0x2E, 0xA0, 0x43),      // #2EA043
        accent_active: Color::from_rgb8(0x19, 0x6C, 0x2E),    // #196C2E
        on_accent_label: Color::WHITE,

        // 状态色 - GitHub 风格
        success: Color::from_rgb8(0x3F, 0xB9, 0x50),          // #3FB950
        warning: Color::from_rgb8(0xD2, 0x99, 0x22),          // #D29922
        error: Color::from_rgb8(0xF8, 0x51, 0x49),            // #F85149
        info: Color::from_rgb8(0x58, 0xA6, 0xFF),             // #58A6FF

        // 终端色
        terminal_bg: Color::from_rgb8(0x0D, 0x11, 0x17),       // #0D1117
        terminal_fg: Color::from_rgb8(0xE6, 0xED, 0xF3),       // #E6EDF3
        scrollbar_hover: Color::from_rgba(0.545, 0.580, 0.620, 0.15),
        scrollbar_active: Color::from_rgba(0.545, 0.580, 0.620, 0.25),
        line_highlight: Color::from_rgba(0.545, 0.580, 0.620, 0.08),
        selection_bg: Color::from_rgba(0.220, 0.545, 0.992, 0.25),  // #58A6FF @ 25%
    }
}
```

### 5.2 主题枚举更新

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RustSshThemeId {
    #[default]
    Dark,
    Light,
    Warm,
    GitHub,  // 新增
}

impl DesignTokens {
    pub fn for_id(id: RustSshThemeId) -> Self {
        match id {
            RustSshThemeId::Dark => Self::dark(),
            RustSshThemeId::Light => Self::light(),
            RustSshThemeId::Warm => Self::warm(),
            RustSshThemeId::GitHub => Self::github(),  // 新增
        }
    }
}
```

---

## 六、设计决策

| 决策 | 原因 |
|------|------|
| 背景比 Dark 更深 | 与 GitHub 界面一致，减少眼睛疲劳 |
| Accent 使用 GitHub 官方绿 | 品牌一致性，开发者熟悉 |
| 边框使用 rgba(139,148,158) | 比白色边框更柔和、更具 GitHub 风格 |
| terminal_bg 使用 #0D1117 | 与主题融合，不是纯黑 |
| terminal_fg 使用 #E6EDF3 | GitHub 标准前景色 |
| 状态色使用 GitHub 官方值 | 完全匹配 GitHub 界面风格 |

---

## 七、GitHub 设计参考

### 7.1 GitHub UI 特征

- **圆角**: 6px（按钮、输入框、卡片）
- **边框**: 1px solid，默认颜色 #30363D
- **间距**: 8px 基准网格
- **字体**: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif
- **按钮高度**: 32px (小) / 36px (默认) / 40px (大)

### 7.2 GitHub 色彩系统

```
Priority: functional > brand > neutral

Functional:
  - success (green)
  - danger (red)
  - attention (yellow/orange)
  - fg (foreground text)
  - bg (background)

Brand:
  - accent (blue links)

Neutral:
  - gray (borders, muted text)
  - scale (temperature scale from 0-10)
```
