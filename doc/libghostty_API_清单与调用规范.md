# libghostty / libghostty-vt API 清单与调用规范（结合 RustSsh + Ghostling demo）

本文档目的：

- 在 **RustSsh 项目语境**下，系统整理 vendored 的 `libghostty`（`resource/ghostty`）以及 demo `ghostling`（`resource/ghostling`）相关 API。
- 既包含 **项目当前已使用的 API**，也包含 **libghostty 已经提供、但项目尚未使用的 API**，方便后续能力对齐/扩展。

> 重要：`resource/ghostty/include/ghostty/vt.h` 明确标注该 API **尚不稳定**，未来可能 breaking change。请始终以你当前 vendored 的头文件为准。

---

## 1. 仓库内对应关系（你现在用的是哪一套）

- **RustSsh 当前使用（VT-only）**
  - 绑定来源：`resource/ghostty/include/ghostty/vt.h` → `build.rs` 里 bindgen 生成 `ghostty_vt_bindings.rs`
  - Rust 封装：`src/backend/ghostty_vt.rs` 的 `GhosttyVtTerminal`
  - 链接：`build.rs` 链接 `ghostty-vt` + `ghostty`（但 Rust 侧“surface/app”等 API 做了 feature gate，避免运行时崩）

- **Ghostling demo（VT-only，但展示了更多/更新的调用方式）**
  - 代码：`resource/ghostling/main.c`
  - 用途：演示“最小可用终端”如何基于 `libghostty-vt` 处理：PTY、VT 写入、render-state 渲染、key/mouse/focus 编码、以及响应终端查询（effects）。

- **libghostty embedding API（非 vt；更偏 GUI/Surface/App）**
  - 头文件：`resource/ghostty/include/ghostty.h`
  - 目前 RustSsh 未直接使用，但库已提供（并且 `build.rs` 也链接了 `ghostty` 静态库）。

---

## 2. 通用调用规范（所有模块都适用）

### 2.1 返回码 `GhosttyResult`（`ghostty/vt/types.h`）

- **`GHOSTTY_SUCCESS (0)`**：成功
- **`GHOSTTY_OUT_OF_MEMORY (-1)`**：内存不足（库会尽量“可恢复”）
- **`GHOSTTY_INVALID_VALUE (-2)`**：传参错误 / 空指针 / 越界等
- **`GHOSTTY_OUT_OF_SPACE (-3)`**：调用者提供的输出 buffer 太小（常见于 encode/format/graphemes）

### 2.2 Sized-struct ABI 规则（宏 `GHOSTTY_INIT_SIZED(T)`）

部分 struct 首字段是 `size`，用于 ABI 兼容；调用前必须初始化：

- 典型：`GhosttyRenderStateColors`、`GhosttyFormatterTerminalOptions`、formatter 的 extra structs、mouse encoder size 等（以头文件为准）。
- 推荐写法：`GhosttyRenderStateColors colors = GHOSTTY_INIT_SIZED(GhosttyRenderStateColors);`

### 2.3 allocator 规则（`ghostty/vt/allocator.h`）

- 大多数 `*_new(const GhosttyAllocator* allocator, ...)` 允许 `allocator = NULL`，表示使用默认 allocator。
- 自定义 allocator 需实现 `GhosttyAllocatorVtable { alloc/resize/remap/free }`，并保证：
  - `alignment`、`memory_len`、`new_len` 语义严格匹配注释
  - free/resize/remap 使用与 alloc 相同的参数契约

### 2.4 “句柄/数据有效期”与所有权

libghostty-vt 里大量 API 是“**opaque handle** + **查询/迭代**”模型：

- `GhosttyTerminal`、`GhosttyRenderState`、`GhosttyKeyEncoder`、`GhosttyKeyEvent`、`GhosttyMouseEncoder`、`GhosttyMouseEvent`、`GhosttyOscParser`、`GhosttySgrParser`、`GhosttyFormatter` 等都需要显式 `*_free`。
- **render-state 迭代出来的 row/cell 数据**（`RowIterator`/`RowCells`）通常只在 render-state 未再次 update 前有效（详见 `render.h` 文档）。
- **grid_ref** 也明确：只保证“非常短暂”的有效期；**一旦 terminal 有任何更新，就不应再使用旧 ref**。
- key event 的 `utf8` 文本指针：`ghostty_key_event_set_utf8` **不接管所有权**，调用方必须保证编码期间有效（见 `key/event.h`）。

### 2.5 线程/锁规范（关键）

从 `render.h` 的设计目标看：

- `ghostty_render_state_update(state, terminal)` 是 render-state 与 terminal **唯一必须同时访问**的时刻。
- 若你的 IO 线程在 `ghostty_terminal_vt_write`，渲染线程在 `ghostty_render_state_update`，**必须保证 update 时对 terminal 的独占访问**（外部加锁）。
- 其它读取 render-state 的操作应当与 update 串行或以快照/复制方式保证一致性。

### 2.6 Dirty 管理规范（关键）

`render.h` 强调 dirty 有两层：

- **全局 dirty**：`GHOSTTY_RENDER_STATE_DATA_DIRTY`（FALSE / PARTIAL / FULL）
- **行级 dirty**：`GHOSTTY_RENDER_STATE_ROW_DATA_DIRTY`

调用方责任：

- update 只会“更新 dirty”，**不会自动清除 dirty**。
- 渲染完成后需要：
  - 对已渲染的行：`ghostty_render_state_row_set(..., GHOSTTY_RENDER_STATE_ROW_OPTION_DIRTY, &false)`
  - 对全局：`ghostty_render_state_set(..., GHOSTTY_RENDER_STATE_OPTION_DIRTY, &GHOSTTY_RENDER_STATE_DIRTY_FALSE)`

Ghostling demo 在 render loop 里有完整示例（见 `resource/ghostling/main.c`）。

---

## 3. libghostty-vt API（`ghostty/vt.h` 及 `ghostty/vt/*.h`）

> 本节覆盖：库**已经提供**的 VT API（即使 RustSsh 当前未用到）。

### 3.1 Build Info（`ghostty/vt/build_info.h`）

- **`ghostty_build_info(GhosttyBuildInfo data, void* out) -> GhosttyResult`**
  - **功能**：查询编译期能力：SIMD、Kitty Graphics、tmux control mode、优化级别等
  - **规范**：`out` 类型随 `data` 变化（bool* / enum*），传错会导致未定义行为风险

### 3.2 Color（`ghostty/vt/color.h`）

- **`ghostty_color_rgb_get(GhosttyColorRgb color, uint8_t* r, uint8_t* g, uint8_t* b)`**
  - **功能**：拆 RGB（主要为 WASM/绑定不便场景）

### 3.3 Focus Encoding（`ghostty/vt/focus.h`）

- **`ghostty_focus_encode(GhosttyFocusEvent event, char* buf, size_t buf_len, size_t* out_written) -> GhosttyResult`**
  - **功能**：编码 focus gained/lost（CSI I / CSI O）
  - **规范**：buf 可为 NULL 用于探测所需长度；返回 `OUT_OF_SPACE` 时 `out_written`=required
  - **调用建议**：仅当 terminal mode `GHOSTTY_MODE_FOCUS_EVENT` 开启时发送（Ghostling demo 就是这样做的）

### 3.4 Paste Utilities（`ghostty/vt/paste.h`）

- **`ghostty_paste_is_safe(const char* data, size_t len) -> bool`**
  - **功能**：保守检查 paste 是否含危险序列（换行 / `ESC[201~` 等）
  - **建议**：用于“粘贴确认”或“危险粘贴提示”逻辑

### 3.5 Modes Utilities（`ghostty/vt/modes.h`）

- **内联 helper（header-only）**
  - `ghostty_mode_new(value, ansi)`
  - `ghostty_mode_value(mode)`
  - `ghostty_mode_ansi(mode)`
- **`ghostty_mode_report_encode(GhosttyMode mode, GhosttyModeReportState state, char* buf, size_t buf_len, size_t* out_written) -> GhosttyResult`**
  - **功能**：编码 DECRPM 报告
  - **规范**：与 focus encode 一样支持 NULL 探测 required size

### 3.6 Terminal（`ghostty/vt/terminal.h`）

- **生命周期**
  - `ghostty_terminal_new(allocator, &out_terminal, GhosttyTerminalOptions{cols,rows,max_scrollback}) -> GhosttyResult`
  - `ghostty_terminal_free(terminal)`
  - `ghostty_terminal_reset(terminal)`
- **写入 VT 字节流**
  - `ghostty_terminal_vt_write(terminal, data, len)`
  - **规范**：该 API 文档声明“永不失败”，适合喂入不可信外部输出
- **resize**
  - `ghostty_terminal_resize(terminal, cols, rows) -> GhosttyResult`
- **滚动 viewport**
  - `ghostty_terminal_scroll_viewport(terminal, behavior{TOP/BOTTOM/DELTA})`
- **模式读写**
  - `ghostty_terminal_mode_get(terminal, GhosttyMode, &out_bool) -> GhosttyResult`
  - `ghostty_terminal_mode_set(terminal, GhosttyMode, bool) -> GhosttyResult`
- **取数据（typed get）**
  - `ghostty_terminal_get(terminal, GhosttyTerminalData, out_ptr) -> GhosttyResult`
  - 已提供的数据包括：`COLS/ROWS/CURSOR_X/CURSOR_Y/CURSOR_VISIBLE/KITTY_KEYBOARD_FLAGS/SCROLLBAR/MOUSE_TRACKING/...`
- **随机访问（不用于高频渲染）**
  - `ghostty_terminal_grid_ref(terminal, GhosttyPoint point, GhosttyGridRef* out_ref) -> GhosttyResult`

> Ghostling demo 还展示了“effects/回调注册、write_pty、size report、title changed 等”，这部分在你当前 vendored `terminal.h` 里可能尚未暴露或接口不同；若要对齐，需要以你 vendored 版本为准做补齐或升级。

### 3.7 Render State（`ghostty/vt/render.h`）

- **生命周期**
  - `ghostty_render_state_new(allocator, &out_state) -> GhosttyResult`
  - `ghostty_render_state_free(state)`
- **更新快照**
  - `ghostty_render_state_update(state, terminal) -> GhosttyResult`
  - **规范**：update 时需要独占访问 terminal（多线程场景外部加锁）
- **查询 render-state 数据**
  - `ghostty_render_state_get(state, GhosttyRenderStateData, out) -> GhosttyResult`
  - 包含：cols/rows、dirty、row_iterator、背景/前景/光标/调色板、cursor 位置/可见/闪烁等
- **设置 render-state 选项**
  - `ghostty_render_state_set(state, GHOSTTY_RENDER_STATE_OPTION_DIRTY, &dirty) -> GhosttyResult`
- **颜色快照（sized struct）**
  - `ghostty_render_state_colors_get(state, GhosttyRenderStateColors* out_colors) -> GhosttyResult`
- **row iterator**
  - `ghostty_render_state_row_iterator_new(allocator, &out_iter) -> GhosttyResult`
  - `ghostty_render_state_row_iterator_free(iter)`
  - `ghostty_render_state_row_iterator_next(iter) -> bool`
  - `ghostty_render_state_row_get(iter, GhosttyRenderStateRowData, out) -> GhosttyResult`
  - `ghostty_render_state_row_set(iter, GHOSTTY_RENDER_STATE_ROW_OPTION_DIRTY, &false) -> GhosttyResult`
- **row cells**
  - `ghostty_render_state_row_cells_new(allocator, &out_cells) -> GhosttyResult`
  - `ghostty_render_state_row_cells_free(cells)`
  - `ghostty_render_state_row_cells_next(cells) -> bool`
  - `ghostty_render_state_row_cells_select(cells, x) -> GhosttyResult`
  - `ghostty_render_state_row_cells_get(cells, GhosttyRenderStateRowCellsData, out) -> GhosttyResult`
  - cells 可直接查询 `GRAPHEMES_LEN/BUF`、`STYLE`、以及“已解析”`FG_COLOR/BG_COLOR`

### 3.8 Grid Ref（`ghostty/vt/grid_ref.h`）

- **从 ref 取 cell/row**
  - `ghostty_grid_ref_cell(ref, &out_cell) -> GhosttyResult`
  - `ghostty_grid_ref_row(ref, &out_row) -> GhosttyResult`
- **取 grapheme codepoints（带 OUT_OF_SPACE 语义）**
  - `ghostty_grid_ref_graphemes(ref, buf_u32, buf_len, &out_len) -> GhosttyResult`
- **取 style**
  - `ghostty_grid_ref_style(ref, &out_style) -> GhosttyResult`
- **规范**：ref 的有效期极短；任何 terminal 更新都可能使其失效

### 3.9 Screen（`ghostty/vt/screen.h`）

- `ghostty_cell_get(GhosttyCell cell, GhosttyCellData data, void* out) -> GhosttyResult`
- `ghostty_row_get(GhosttyRow row, GhosttyRowData data, void* out) -> GhosttyResult`

用途：为 grid_ref / render_state 的 RAW cell/row 提供字段访问入口。

### 3.10 Style（`ghostty/vt/style.h`）

- `ghostty_style_default(GhosttyStyle* style)`
- `ghostty_style_is_default(const GhosttyStyle* style) -> bool`

### 3.11 Key Encoding（`ghostty/vt/key.h` + `ghostty/vt/key/*.h`）

- **encoder**
  - `ghostty_key_encoder_new(allocator, &out_encoder) -> GhosttyResult`
  - `ghostty_key_encoder_free(encoder)`
  - `ghostty_key_encoder_setopt(encoder, GhosttyKeyEncoderOption, const void* value)`
  - `ghostty_key_encoder_setopt_from_terminal(encoder, terminal)`
  - `ghostty_key_encoder_encode(encoder, event, out_buf, out_buf_size, &out_len) -> GhosttyResult`
    - **规范**：支持 `out_buf = NULL` 探测 required；也可能成功但 `out_len=0`（无输出）
- **event**
  - `ghostty_key_event_new(allocator, &out_event) -> GhosttyResult`
  - `ghostty_key_event_free(event)`
  - setters/getters：`*_set_action/get_action`、`*_set_key/get_key`、`*_set_mods/get_mods`、`*_set_consumed_mods/get_consumed_mods`、`*_set_composing/get_composing`、`*_set_utf8/get_utf8`、`*_set_unshifted_codepoint/get_unshifted_codepoint`
  - **规范**：`set_utf8` 不接管字符串内存；编码期间必须保持指针有效

### 3.12 Mouse Encoding（`ghostty/vt/mouse.h` + `ghostty/vt/mouse/*.h`）

- **encoder**
  - `ghostty_mouse_encoder_new(allocator, &out_encoder) -> GhosttyResult`
  - `ghostty_mouse_encoder_free(encoder)`
  - `ghostty_mouse_encoder_setopt(encoder, option, value)`
  - `ghostty_mouse_encoder_setopt_from_terminal(encoder, terminal)`
  - `ghostty_mouse_encoder_reset(encoder)`
  - `ghostty_mouse_encoder_encode(encoder, event, out_buf, out_buf_size, &out_len) -> GhosttyResult`
    - **规范**：可能成功但 `out_len=0`（例如 tracking 未开启）
    - `OUT_OF_SPACE` 语义同 key encoder
- **event**
  - `ghostty_mouse_event_new(allocator, &out_event) -> GhosttyResult`
  - `ghostty_mouse_event_free(event)`
  - setters/getters：action、button/clear_button/get_button、mods、position

### 3.13 OSC Parser（`ghostty/vt/osc.h`）

用于解析 OSC 序列（Streaming 逐字节），典型用途：标题设置、剪贴板、超链接、通知等。

- `ghostty_osc_new(allocator, &out_parser) -> GhosttyResult`
- `ghostty_osc_free(parser)`
- `ghostty_osc_reset(parser)`
- `ghostty_osc_next(parser, byte)`：逐字节喂入
- `ghostty_osc_end(parser, terminator_byte) -> GhosttyOscCommand`
- `ghostty_osc_command_type(command) -> GhosttyOscCommandType`
- `ghostty_osc_command_data(command, GhosttyOscCommandData, out) -> bool`

### 3.14 SGR Parser（`ghostty/vt/sgr.h`）

用于解析 SGR 参数列表为可枚举的 attribute（支持多种颜色格式等）。

- `ghostty_sgr_new(allocator, &out_parser) -> GhosttyResult`
- `ghostty_sgr_free(parser)`
- `ghostty_sgr_reset(parser)`
- `ghostty_sgr_set_params(parser, params_u16, separators, len) -> GhosttyResult`
- `ghostty_sgr_next(parser, &out_attr) -> bool`
- wasm 辅助/访问：`ghostty_sgr_unknown_full/partial`、`ghostty_sgr_attribute_tag/value`

### 3.15 Formatter（`ghostty/vt/formatter.h`）

把 terminal 当前屏幕内容格式化为 plain / VT / HTML。

- `ghostty_formatter_terminal_new(allocator, &out_formatter, terminal, GhosttyFormatterTerminalOptions) -> GhosttyResult`
  - **规范**：formatter 里持有对 terminal 的借用引用；**terminal 必须比 formatter 活得更久**
- `ghostty_formatter_format_buf(formatter, buf, buf_len, &out_written) -> GhosttyResult`
  - **规范**：buf=NULL 用于探测 required size（返回 `OUT_OF_SPACE`）
- `ghostty_formatter_format_alloc(formatter, allocator, &out_ptr, &out_len) -> GhosttyResult`
  - **规范**：释放策略必须与 allocator 匹配；allocator=NULL 时一般用 `free()`
- `ghostty_formatter_free(formatter)`

### 3.16 Size Report（`ghostty/vt/size_report.h`）

- `ghostty_size_report_encode(style, size, buf, buf_len, &out_written) -> GhosttyResult`
  - **规范**：支持 NULL 探测 required size

### 3.17 WASM Utilities（`ghostty/vt/wasm.h`）

为 WASM/JS 绑定提供低层内存分配工具（避免 JS/wasm 侧处理复杂 struct）。

已提供（节选）：

- `ghostty_wasm_alloc_opaque / ghostty_wasm_free_opaque`
- `ghostty_wasm_alloc_u8_array / free_u8_array`
- `ghostty_wasm_alloc_u16_array / free_u16_array`
- `ghostty_wasm_alloc_u8 / free_u8`
- `ghostty_wasm_alloc_usize / free_usize`
- `ghostty_wasm_free_sgr_attribute`（用于释放 wasm 分配的 attribute）

---

## 4. libghostty embedding API（`resource/ghostty/include/ghostty.h`）

> 这部分 API 不属于 vt.h（不是“纯 VT core”），而是更偏应用/Surface/配置/运行时回调的嵌入式 API。RustSsh 当前未直接使用，但库已提供，应纳入文档以便后续评估是否需要启用。

### 4.1 初始化/信息

- `int ghostty_init(uintptr_t, char**)`
- `void ghostty_cli_try_action(void)`
- `ghostty_info_s ghostty_info(void)`
- `const char* ghostty_translate(const char*)`
- `void ghostty_string_free(ghostty_string_s)`

### 4.2 Config

- `ghostty_config_t ghostty_config_new()`
- `void ghostty_config_free(ghostty_config_t)`
- `ghostty_config_t ghostty_config_clone(ghostty_config_t)`
- `void ghostty_config_load_cli_args(ghostty_config_t)`
- `void ghostty_config_load_file(ghostty_config_t, const char*)`
- `void ghostty_config_load_default_files(ghostty_config_t)`
- `void ghostty_config_load_recursive_files(ghostty_config_t)`
- `void ghostty_config_finalize(ghostty_config_t)`
- `bool ghostty_config_get(ghostty_config_t, void*, const char*, uintptr_t)`
- `ghostty_input_trigger_s ghostty_config_trigger(ghostty_config_t, const char*, uintptr_t)`
- `uint32_t ghostty_config_diagnostics_count(ghostty_config_t)`
- `ghostty_diagnostic_s ghostty_config_get_diagnostic(ghostty_config_t, uint32_t)`
- `ghostty_string_s ghostty_config_open_path(void)`

### 4.3 App（运行时 + 全局输入/动作回调）

- `ghostty_app_t ghostty_app_new(const ghostty_runtime_config_s*, ghostty_config_t)`
- `void ghostty_app_free(ghostty_app_t)`
- `void ghostty_app_tick(ghostty_app_t)`
- `void* ghostty_app_userdata(ghostty_app_t)`
- `void ghostty_app_set_focus(ghostty_app_t, bool)`
- `bool ghostty_app_key(ghostty_app_t, ghostty_input_key_s)`
- `bool ghostty_app_key_is_binding(ghostty_app_t, ghostty_input_key_s)`
- `void ghostty_app_keyboard_changed(ghostty_app_t)`
- `void ghostty_app_open_config(ghostty_app_t)`
- `void ghostty_app_update_config(ghostty_app_t, ghostty_config_t)`
- `bool ghostty_app_needs_confirm_quit(ghostty_app_t)`
- `bool ghostty_app_has_global_keybinds(ghostty_app_t)`
- `void ghostty_app_set_color_scheme(ghostty_app_t, ghostty_color_scheme_e)`

调用规范要点（从头文件类型推断的契约）：

- 需要提供 `ghostty_runtime_config_s`，其中包含 wakeup/action/clipboard/close_surface 等回调。
- App 往往需要周期性 `tick`（驱动定时器/内部事件）。

### 4.4 Surface（窗口/视图/渲染与输入）

- `ghostty_surface_config_s ghostty_surface_config_new()`
- `ghostty_surface_t ghostty_surface_new(ghostty_app_t, const ghostty_surface_config_s*)`
- `void ghostty_surface_free(ghostty_surface_t)`
- `void* ghostty_surface_userdata(ghostty_surface_t)`
- `ghostty_app_t ghostty_surface_app(ghostty_surface_t)`
- `ghostty_surface_config_s ghostty_surface_inherited_config(ghostty_surface_t, ghostty_surface_context_e)`
- `void ghostty_surface_update_config(ghostty_surface_t, ghostty_config_t)`
- `bool ghostty_surface_needs_confirm_quit(ghostty_surface_t)`
- `bool ghostty_surface_process_exited(ghostty_surface_t)`
- `void ghostty_surface_refresh(ghostty_surface_t)`
- `void ghostty_surface_draw(ghostty_surface_t)`
- `void ghostty_surface_set_content_scale(ghostty_surface_t, double, double)`
- `void ghostty_surface_set_focus(ghostty_surface_t, bool)`
- `void ghostty_surface_set_occlusion(ghostty_surface_t, bool)`
- `void ghostty_surface_set_size(ghostty_surface_t, uint32_t, uint32_t)`
- `ghostty_surface_size_s ghostty_surface_size(ghostty_surface_t)`
- `void ghostty_surface_set_color_scheme(ghostty_surface_t, ghostty_color_scheme_e)`

输入相关：

- `ghostty_input_mods_e ghostty_surface_key_translation_mods(ghostty_surface_t, ghostty_input_mods_e)`
- `bool ghostty_surface_key(ghostty_surface_t, ghostty_input_key_s)`
- `bool ghostty_surface_key_is_binding(ghostty_surface_t, ghostty_input_key_s, ghostty_binding_flags_e*)`
- `void ghostty_surface_text(ghostty_surface_t, const char*, uintptr_t)`
- `void ghostty_surface_preedit(ghostty_surface_t, const char*, uintptr_t)`
- `bool ghostty_surface_mouse_captured(ghostty_surface_t)`
- `bool ghostty_surface_mouse_button(ghostty_surface_t, ghostty_input_mouse_state_e, ghostty_input_mouse_button_e, ghostty_input_mods_e)`
- `void ghostty_surface_mouse_pos(ghostty_surface_t, double, double, ghostty_input_mods_e)`
- `void ghostty_surface_mouse_scroll(ghostty_surface_t, double, double, ghostty_input_scroll_mods_t)`
- `void ghostty_surface_mouse_pressure(ghostty_surface_t, uint32_t, double)`
- `void ghostty_surface_ime_point(ghostty_surface_t, double*, double*, double*, double*)`

剪贴板/选择区：

- `void ghostty_surface_complete_clipboard_request(ghostty_surface_t, const char*, void*, bool)`
- `bool ghostty_surface_has_selection(ghostty_surface_t)`
- `bool ghostty_surface_read_selection(ghostty_surface_t, ghostty_text_s*)`
- `bool ghostty_surface_read_text(ghostty_surface_t, ghostty_selection_s, ghostty_text_s*)`
- `void ghostty_surface_free_text(ghostty_surface_t, ghostty_text_s*)`

分屏相关：

- `void ghostty_surface_split(ghostty_surface_t, ghostty_action_split_direction_e)`
- `void ghostty_surface_split_focus(ghostty_surface_t, ghostty_action_goto_split_e)`
- `void ghostty_surface_split_resize(ghostty_surface_t, ghostty_action_resize_split_direction_e, uint16_t)`
- `void ghostty_surface_split_equalize(ghostty_surface_t)`

### 4.5 Inspector（调试/检查器）

- `void ghostty_inspector_free(ghostty_surface_t)`
- `void ghostty_inspector_set_focus(ghostty_inspector_t, bool)`
- `void ghostty_inspector_set_content_scale(ghostty_inspector_t, double, double)`
- `void ghostty_inspector_set_size(ghostty_inspector_t, uint32_t, uint32_t)`
- `void ghostty_inspector_mouse_button(ghostty_inspector_t, ...)`
- `void ghostty_inspector_mouse_pos(ghostty_inspector_t, ...)`
- `void ghostty_inspector_mouse_scroll(ghostty_inspector_t, ...)`
- `void ghostty_inspector_key(ghostty_inspector_t, ...)`
- `void ghostty_inspector_text(ghostty_inspector_t, ...)`
- Apple/Metal：`ghostty_inspector_metal_render`、`ghostty_inspector_metal_shutdown` 等（以 `#ifdef __APPLE__` 为准）

### 4.6 Benchmark

- `bool ghostty_benchmark_cli(const char*, const char*)`

---

## 5. RustSsh 当前“已用子集”速查（对应 `src/backend/ghostty_vt.rs`）

当前 Rust 封装主要覆盖：

- terminal：`ghostty_terminal_new/free/vt_write/resize/scroll_viewport/get/mode_get`（以实际封装为准）
- render：`ghostty_render_state_new/free/update/get/colors_get/row_iterator_* / row_cells_*`
- key：`ghostty_key_encoder_new/free/encode/setopt_from_terminal` + `ghostty_key_event_*`

未用但库已提供的（优先级常见）：

- mouse encoder + focus encode + paste safety + formatter + osc/sgr parser + grid_ref 深度遍历 + size report encode + wasm helpers

---

## 6. 迁移/对齐提示（当你想把 Ghostling 的能力搬进 RustSsh）

Ghostling demo 强烈依赖 “effects/回调” 来回答查询序列（否则 vim/tmux 可能卡住或降级）：

- write_pty：把终端需要回写的响应写回 PTY
- size report：回答 XTWINOPS 等
- device attributes / xtversion
- title_changed：OSC 0/2 改窗口标题

如果你后续要把这些能力引入 RustSsh，建议步骤是：

- **先确认你 vendored 的 `resource/ghostty` 版本**是否已经在 `vt/*.h` 暴露等价 API（而不是只在 Ghostling 的更高版本里存在）。
- 若没有，需要升级 `resource/ghostty` 或在 Zig/C 层补一层稳定 wrapper 再 bindgen。

