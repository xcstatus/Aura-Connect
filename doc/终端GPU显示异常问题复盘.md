# 终端 GPU 显示异常问题复盘

本文档沉淀 RustSsh 终端在 GPU 文本渲染路径上的异常问题，记录问题现象、根因收敛、排查过程与当前结论，供后续迭代和回归验证使用。

> 适用范围：`src/terminal/*`（`gpu_renderer` + `glyph_atlas` + `vt_widget`）以及相关诊断工具链。

---

## 1. 问题现象

### 1.1 用户可见表现

- 开启 GPU 终端文本渲染后，终端字符出现乱码、重叠、抽象化、错位。
- 同一场景切回 CPU 模式显示正常。
- 在 GPU 模式下选中并复制文本，复制内容正确。

### 1.2 关键判断

- “复制内容正确”说明 **VT/终端缓冲数据正确**。
- 异常集中在 **渲染链路**，不是 SSH 协议、会话数据、终端解析本身。

---

## 2. 影响范围

- GPU 模式可读性明显下降，影响 SSH 交互可用性。
- CPU 模式可用，但与目标（GPU 路径稳定可用）不符。
- 问题涉及文本主链路，优先级高。

---

## 3. 排查目标与方法

### 3.1 目标

- 定位问题在以下哪一层：
  1) atlas 生成阶段  
  2) 实例构建阶段  
  3) shader/采样/坐标阶段  
  4) 真实 GPU 执行阶段

### 3.2 方法

- 增加可复现静态测试内容（`RUST_SSH_TERM_DISPLAY_TEST=1`）。
- 增加同帧对比工具：CPU 参考图、GPU 模拟图、diff 图、拼图。
- 增加 atlas 可视化导出：`used_slots.png` + `used_slots.tsv`。
- 增加真实 offscreen 读回导出：`*_offscreen_real.png`。
- 通过环境变量做 A/B：
  - `RUST_SSH_GPU_GLYPH_NEAREST`
  - `RUST_SSH_GPU_GLYPH_UV_CROP`
  - `RUST_SSH_GPU_GLYPH_ALPHA_BOOST`
  - `RUST_SSH_TERM_DUMP_COMPARE`
  - `RUST_SSH_TERM_DUMP_OFFSCREEN`
  - `RUST_SSH_TERM_DUMP_DIR`

---

## 3.3 关键参数说明（查表）

| 参数 | 默认值 | 作用 | 典型用法 | 注意事项 |
|---|---|---|---|---|
| `RUST_SSH_TERMINAL_GPU` | 未设置（按运行条件自动） | 强制开启/关闭 GPU 文本路径的总开关（常用 `1` 开） | `RUST_SSH_TERMINAL_GPU=1` | 仅决定是否走 GPU 路径，不保证显示质量 |
| `RUST_SSH_TERM_DISPLAY_TEST` | `0` | 启动后注入固定测试文本，便于稳定复现 | `RUST_SSH_TERM_DISPLAY_TEST=1` | 仅用于诊断，不建议长期在正常流程开启 |
| `RUST_SSH_TERM_DUMP_COMPARE` | `0` | 导出同帧对比图（`cpu/gpu_sim/diff/compare`） | `RUST_SSH_TERM_DUMP_COMPARE=1` | 会产生图片文件，建议配合指定 dump 目录 |
| `RUST_SSH_TERM_DUMP_OFFSCREEN` | `0` | 导出真实 GPU offscreen 纹理（`*_offscreen_real.png`） | `RUST_SSH_TERM_DUMP_OFFSCREEN=1` | 已做非空实例保护，避免首帧空图误导 |
| `RUST_SSH_TERM_DUMP_DIR` | `./term-dumps` | 指定 dump 输出目录 | `RUST_SSH_TERM_DUMP_DIR=./term-dumps` | 建议每轮排查清理旧文件或按时间戳分组 |
| `RUST_SSH_GPU_GLYPH_NEAREST` | `0`（Linear） | atlas 采样过滤模式切换（Nearest/Linear） | `RUST_SSH_GPU_GLYPH_NEAREST=1` | 仅用于 A/B 诊断，Nearest 易出现锯齿 |
| `RUST_SSH_GPU_GLYPH_UV_CROP` | `0.0` | 在 shader 内收缩 slot UV 采样区（0.0~0.30） | `RUST_SSH_GPU_GLYPH_UV_CROP=0.05` | 过大可能裁掉有效字形边缘 |
| `RUST_SSH_GPU_GLYPH_ALPHA_BOOST` | `1.0` | 字形 alpha 增益（0.5~3.0） | `RUST_SSH_GPU_GLYPH_ALPHA_BOOST=1.35` | 仅影响“浓淡”，不解决几何映射错误 |
| `RUST_SSH_FONT_TTC` | 系统默认字体路径集 | 指定优先字体文件（TTC/TTF） | `RUST_SSH_FONT_TTC=/path/to/font.ttc` | 字体变化会显著影响指标，排查时应固定 |
| `RUST_SSH_FONT_FACE_INDEX` | 自动探测 | 指定 TTC face 索引 | `RUST_SSH_FONT_FACE_INDEX=0` | 仅在 TTC 多 face 文件下有效 |
| `RUST_SSH_VT_LOG_FRAME` | 未设置 | 控制逐帧诊断日志输出频率 | `RUST_SSH_VT_LOG_FRAME=60` | 值过小会产生日志噪声 |
| `RUST_LOG` | 默认日志级别 | 控制日志级别，常用于开启 `term-diag` | `RUST_LOG=term-diag=info` | 建议诊断结束后恢复较低日志级别 |

推荐排查组合：

```bash
RUST_SSH_TERM_DISPLAY_TEST=1 \
RUST_SSH_TERM_DUMP_COMPARE=1 \
RUST_SSH_TERM_DUMP_OFFSCREEN=1 \
RUST_LOG=term-diag=info \
cargo run
```

如需验证 alpha 敏感性，再附加：

```bash
RUST_SSH_GPU_GLYPH_ALPHA_BOOST=1.35
```

---

## 4. 排查过程（时间线归纳）

## 阶段 A：确认“数据正确，显示错误”

- 现象：GPU 模式显示异常，但复制结果正确。
- 结论：问题不在后端数据，聚焦渲染链路。

## 阶段 B：初步怀疑采样串扰/字体缩放

- 尝试 UV inset、采样器参数（linear/nearest）和缩放参数。
- 出现“字符像小点”/“大小感知异常”等副作用。
- 结论：单点参数调整无法根治，需引入量化诊断。

## 阶段 C：构建量化诊断工具链（关键转折）

- 新增 compare dump：生成 `cpu/gpu_sim/diff/compare`。
- 新增 atlas 证据导出：`used_slots.png` + `used_slots.tsv`。
- 结论：问题可被量化，不再依赖肉眼描述。

## 阶段 D：定位并修复第一主因（atlas 垂直放置异常）

- 证据：`used_slots.tsv` 中大量 `off_y` 过大（接近 slot 底部），导致字形大面积裁剪。
- 修复：`glyph_atlas` 增加 vertical placement fallback（基线异常时回退几何居中）。
- 结果：`off_y` 分布回归合理区间，`used_slots.png` 可辨识度明显提升。

## 阶段 E：区分“模拟问题”与“真实 GPU 执行问题”

- 新增真实 offscreen 导出，早期遇到 `Buffer ... still mapped`（map 时序错误）。
- 修复：改为提交后 `on_submitted_work_done -> map_async`，并避免首帧空实例导出。
- 核心证据：`offscreen_real` 与 `gpu_sim` 差异很小，说明真实 GPU 执行基本符合模拟。
- 结论：问题主要不在“驱动/提交执行”，而在“渲染模型构造本身”。

## 阶段 F：验证 alpha 是否主因

- 新增 `RUST_SSH_GPU_GLYPH_ALPHA_BOOST` 与覆盖率统计：
  - `gpu_cov_mean`
  - `gpu_cov_nonzero_ratio`
- 结果：增强 alpha 后核心 diff 改善不明显。
- 结论：不是单纯“太淡/太细”问题，核心仍在几何/映射语义。

## 阶段 G：UV/几何映射迭代

- 先尝试“仅 UV 裁剪”，导致覆盖率过高（字形拉伸过头）。
- 修正为“UV rect + dst rect 成对映射”：
  - `uv_rect`：字形在 slot 的真实区域
  - `dst_rect`：字形在 cell 的目标区域
- 目标：避免整槽硬拉伸，保留字形相对比例。

---

## 5. 根因归纳（当前阶段）

本问题不是单一 bug，而是 GPU 文本渲染从“整槽采样”到“字形真实几何映射”过程中的复合问题。当前已确认：

1. **已修复主因之一**：atlas 垂直偏移导致字形落到槽底、被裁剪。  
2. **已排除执行层**：`offscreen_real ≈ gpu_sim`，真实 GPU 执行与模拟一致。  
3. **主要剩余点**：glyph 的 UV/几何映射语义仍需继续收敛（CPU 参考与 GPU 模拟存在稳定差异）。

---

## 6. 关键证据（节选）

- 终端模式判断日志长期稳定：`gpu_terminal_text=true`，并有有效实例计数（`bg_instances`/`glyph_instances` 非 0）。
- compare dump 指标可稳定复现（示例）：
  - `mean_abs_diff`、`diff_ratio` 用于 CPU vs GPU 模拟差异量化。
  - `gpu_cov_mean`、`gpu_cov_nonzero_ratio` 用于覆盖率量化。
- offscreen 实拍验证：
  - `*_offscreen_real.png` 与同轮 `*_gpu_sim.png` 差异很小（执行层一致）。

---

## 7. 过程中新增的诊断与工程能力

- Display test 注入（启动即有可复现文本样本）。
- compare dump 全链路导出（CPU/GPU/diff/compare）。
- atlas 已用槽位可视化与清单导出（含 `off_x/off_y/bounds/fallback`）。
- 真实 offscreen 读回导出（规避首帧空帧、规避 mapped 时序错误）。
- GPU 采样与 alpha 调试开关（便于 A/B）。

---

## 8. 当前结论与后续计划

### 8.1 当前结论

- 问题已从“不可描述的乱码”收敛为“可量化的几何映射偏差”。
- 关键第一主因（vertical placement）已修复。
- 真实 GPU 执行链路已验证稳定。
- 最新迭代中采用“X 轴 monospace 全宽 + Y 轴紧致映射”后，指标回落并稳定在阶段最佳附近。

### 8.2 阶段性验收数据（最新一轮）

`RUST_SSH_TERM_DISPLAY_TEST=1` + compare/offscreen 开启后的最新结果：

- `mean_abs_diff = 12.726051330566406`
- `diff_ratio = 0.016061032190918922`
- `gpu_cov_mean = 0.13190850615501404`
- `gpu_cov_nonzero_ratio = 0.16732364892959595`

对比中间异常版本（仅 UV 裁剪导致拉伸）：

- `mean_abs_diff`: `15.95 -> 12.73`
- `diff_ratio`: `0.0318 -> 0.0161`
- `gpu_cov_nonzero_ratio`: `0.50 -> 0.167`

说明“覆盖过满/几何拉伸”主问题已被压制，剩余偏差主要为细节级（字体风格、边缘像素、特定字符类别）。

### 8.3 后续计划

- 继续迭代 `uv_rect + dst_rect` 的标定策略，与 CPU 参考在 baseline/advance/box drawing 上做更细对齐。
- 为关键映射规则补单元测试（glyph 实例生成与 rect 合法性）。
- 固化回归脚本：一键输出 compare/offscreen 指标，避免主观肉眼判断回归。

### 8.4 调试日志收敛（已执行）

- 将高频 `GPU glyph vertical placement fallback` 从 `INFO` 下调到 `DEBUG`，减少常规排查噪声。
- 保留关键 `INFO`：
  - 模式判定与实例计数
  - compare dump 结果
  - offscreen real dump 调度/保存结果

---

## 9. 建议的回归命令

```bash
RUST_SSH_TERM_DISPLAY_TEST=1 \
RUST_SSH_TERM_DUMP_COMPARE=1 \
RUST_SSH_TERM_DUMP_OFFSCREEN=1 \
RUST_LOG=term-diag=info \
cargo run
```

如需观察 alpha 敏感度可附加：

```bash
RUST_SSH_GPU_GLYPH_ALPHA_BOOST=1.35
```

---

## 10. Menlo 字体下“大小不一/基线不齐”专项（可交接开发）

### 10.1 现象定义

- 指定 `RUST_SSH_FONT_TTC=/System/Library/Fonts/Menlo.ttc` 后，GPU 路径与 CPU 路径差异明显。
- 同一行字符出现“看起来大小不一、上下不齐、不在一条水平线”。
- CPU 路径（`RUST_SSH_TERMINAL_GPU=0`）显示协调，说明问题集中在 GPU 字形映射与实例几何，而非 VT 数据。

### 10.2 根因分析（当前结论）

该问题属于“单字形策略过强导致的行级不一致”，重点在 `src/terminal/glyph_atlas.rs` 与 `src/terminal/gpu_renderer.rs`：

1) **逐字符缩放导致视觉字号漂移**  
历史实现对每个字符执行 `fit_scale_for_slot`，使不同字形得到不同缩放比例。  
结果：同一字号在一行内出现“字符忽大忽小”的视觉效果。

2) **逐字符垂直策略切换导致基线抖动**  
历史实现在 baseline 与 centered 之间按阈值回退，字符间可能走不同垂直放置策略。  
结果：同一行出现上下跳动，不再共用统一基线。

3) **tight rect（`uv_rect/dst_rect`）放大上述差异**  
`slot_uv_rect_u16`/`slot_dst_rect_u16` 按每个字符 bbox 紧映射，若上游缩放/基线已不一致，会被进一步放大。

4) **TTC face 差异是放大器，不是主因**  
Menlo.ttc 不同 face 的 metrics/hinting 存在差异，若 face 未固定，问题更明显。  
但根因仍是“逐字符策略不统一”。

### 10.3 方案（分阶段）

#### A. 先止血（P0，立即可做）

- 在 `src/terminal/glyph_atlas.rs` 中统一策略为：
  - 固定 scale（按字体与 slot 的全局比例，不做逐字符 fit）。
  - 固定 baseline（不做逐字符 baseline/center 切换）。
- 保留 zero-pixel 兜底逻辑，但只用于异常字符，不改变主路径策略。
- 增加 face 固定建议：`RUST_SSH_FONT_FACE_INDEX=0` 作为默认诊断基线。

#### B. 可切换 A/B（P0.5，定位 tight rect 影响）

- 新增开关（建议）：`RUST_SSH_GLYPH_TIGHT_RECT=0/1`。
  - `1`：沿用 tight rect（当前方式）。
  - `0`：使用 full-slot UV/DST（几何最稳定，用于验证）。
-（已落地）另一个开关：`RUST_SSH_GLYPH_FIT_SCALE=0/1`。
  - `0`（默认）：固定 scale（更稳定，便于对齐与回归）。
  - `1`：启用逐字符 fit scale（更不易裁切，但可能引入细微大小漂移）。
- 若 full-slot 下“行级一致性”显著改善，说明 tight rect 仍需重构为“按字体全局 metrics 映射”。

#### C. 中期重构（P1，质量收敛）

- 以“字体级 metrics”统一几何，不再按字符 bbox 决定行高/基线：
  - 基于 `ScaleFont` 的 `ascent/descent/height` 计算全局 baseline。
  - `dst_rect` 改为“字体级 Y 映射 + monospace X 映射”。
- 在 `src/terminal/gpu_renderer.rs` 增加诊断统计：
  - 每帧 glyph 的 `off_y` 分布区间；
  - `tight/full` 模式下 diff 指标对比。

#### D. 长期优化（P2）

- atlas key 升级为 grapheme 级（而非单 codepoint），减少组合字符下的异常映射。
- 将 CPU/GPU 对齐验收固定为自动化（compare/offscreen 指标 + 样例截图）。

### 10.4 任务拆分（供开发分派）

| 任务 | 责任文件 | 产出 | 风险 |
|---|---|---|---|
| 统一 scale/baseline 主路径 | `src/terminal/glyph_atlas.rs` | 消除行内大小/基线抖动 | 低 |
| tight/full rect A/B 开关 | `src/terminal/glyph_atlas.rs`、`src/terminal/gpu_renderer.rs` | 可快速定位几何映射问题 | 低 |
| 字体级 metrics 重构 | `src/terminal/glyph_atlas.rs` | 与 CPU 对齐度提升 | 中 |
| 指标与回归脚本固化 | `src/terminal/vt_widget.rs`、`doc/*` | 回归可量化、可追踪 | 低 |

### 10.5 验收标准

- 场景：英文命令、中文路径、混排、`vim/tmux/top`。
- 统一配置：`RUST_SSH_FONT_TTC=/System/Library/Fonts/Menlo.ttc` + `RUST_SSH_FONT_FACE_INDEX=0`。
- 通过标准：
  - GPU 路径行内字符不再出现明显大小漂移；
  - 同一行基线稳定，无肉眼可见上下跳动；
  - compare 指标不劣于当前阶段基线；
  - 回滚开关可随时切回旧策略（便于线上止损）。

### 10.6 建议调试命令（专项）

基础单次对比（推荐先跑）：

```bash
RUST_SSH_TERMINAL_GPU=1 \
RUST_SSH_FONT_TTC=/System/Library/Fonts/Menlo.ttc \
RUST_SSH_FONT_FACE_INDEX=0 \
RUST_SSH_TERM_DISPLAY_TEST=2 \
RUST_SSH_TERM_DUMP_COMPARE=1 \
RUST_SSH_TERM_DUMP_OFFSCREEN=1 \
RUST_LOG=term-diag=debug,vt.paint=debug \
cargo run
```

一键回归矩阵（tight/full + fit off/on，2x2）：

```bash
./scripts/terminal_gpu_regression.sh
```

如需手动单 case A/B，可覆盖以下开关：

```bash
RUST_SSH_GLYPH_TIGHT_RECT=0|1
RUST_SSH_GLYPH_FIT_SCALE=0|1
RUST_SSH_GPU_GLYPH_ALPHA_BOOST=1.25
RUST_SSH_GPU_GLYPH_NEAREST=1
```

