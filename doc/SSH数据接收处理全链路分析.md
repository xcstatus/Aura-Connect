# SSH 数据接收处理全链路分析报告

**日期**: 2026-04-10
**作者**: Claude
**状态**: 分析完成，待优化

---

## 一、数据流完整链路

```
SSH Server
    ↓
russh 内部缓冲区
    ↓
pump task (mpsc:8192 包)
    ↓
data_rx → read_channel (overflow_buf)
    ↓
SshChannel::read_buffer
    ↓
TerminalSessionBridge::buf → drain_output (pending)
    ↓
GhosttyVtTerminal::write_vt
    ↓
styled_rows (dirty tracking)
    ↓
styled_fragments (render cache)
    ↓
终端显示
```

---

## 二、各环节详细分析

### 2.1 SSH 数据接收层 (`src/backend/ssh_session.rs`)

#### pump 循环 (line 617-686)

```rust
msg = channel.wait() => {
    match msg {
        Some(russh::ChannelMsg::Data { data }) => {
            if data_tx.send(data.to_vec()).await.is_err() {
                break;
            }
        }
        // ...
    }
}
```

| 评估项 | 状态 | 说明 |
|--------|------|------|
| 数据完整性保证 | ⚠️ | mpsc 满时 pump 阻塞，russh 内部缓冲区积压 |
| 潜在丢数据点 | ✅ | 无静默丢弃，send 阻塞保证数据不丢 |
| 性能风险 | 🔴 | 大数据包 + 高频率可能撑满 8192 包缓冲 |

**背压机制**：mpsc channel 容量 8192 包，`send().await` 在缓冲区满时会阻塞整个 pump 循环。

---

### 2.2 read_channel 溢出处理 (line 741-784)

**状态**：✅ 已修复

```rust
if data.len() > len {
    // 修复：将超出的数据存入溢出缓冲区
    let remaining = data[len..].to_vec();
    let mut overflow = entry.overflow_buf.lock().await;
    overflow.extend_from_slice(&remaining);
}
```

---

### 2.3 SshChannel::read_stream (line 922-940)

**状态**：✅ 已修复

```rust
if len < self.read_buffer.len() {
    let remaining = self.read_buffer[len..].to_vec();
    self.read_buffer = remaining;
} else {
    self.read_buffer.clear();
}
```

---

### 2.4 TerminalSessionBridge (`src/terminal/controller.rs`)

#### drain_output 帧预算控制 (line 100-150)

```rust
pub fn drain_output(
    &mut self,
    session: &mut dyn AsyncSession,
    mut on_bytes: impl FnMut(&[u8]),
) -> Result<usize> {
    // Phase 1: drain pending remainder
    // Phase 2: read from session (max_reads_per_frame 次)
    // Phase 3: truncate to limit
}
```

**关键参数**：

| 参数 | 默认值 | 说明 |
|------|--------|------|
| `max_bytes_per_frame` | 32 KB | 每帧最多处理字节数 |
| `max_reads_per_frame` | 64 | 每帧最多读取次数 |
| `buf` | 4 KB | 读取缓冲区大小 |
| `pending_capacity_limit` | 256 KB | pending 缓冲区硬上限 |

#### pending 缓冲区容量限制 (line 152-168)

**状态**：⚠️ 有静默丢弃风险

```rust
fn truncate_pending_to_limit(&mut self) {
    if self.pending.len() > self.pending_capacity_limit {
        let excess = self.pending.len() - self.pending_capacity_limit;
        self.pending.drain(..excess);  // ⚠️ 静默丢弃
    }
}
```

---

### 2.5 pump_output 调用层 (`src/app/update/tick.rs`)

#### pump 频率控制 (line 158-204)

```rust
fn compute_tick_ms(state: &IcedState) -> u32 {
    if !state.window_focused {
        250  // 窗口失焦时 250ms
    } else if idle_ms <= 1_000 {
        (1000 / target_fps).max(16)  // 活跃时 ~16-100ms
    } else if idle_ms <= 5_000 {
        (1000 / target_fps.min(30)).max(33)
    } else {
        (1000 / target_fps.min(10)).max(100)
    }
}
```

| 场景 | pump 频率 | 每帧 32KB 吞吐量 |
|------|-----------|-------------------|
| 60fps 活跃 | ~16ms | 2 MB/s |
| 窗口失焦 | 250ms | 128 KB/s |
| 低 fps/高延迟 | 100ms | 320 KB/s |

**问题**：高速数据流时（如 `cat 1MB`），吞吐量需求远超帧预算能力。

---

### 2.6 VT 解析层 (`src/backend/ghostty_vt.rs`)

#### write_vt (line 204-208)

```rust
pub fn write_vt(&mut self, bytes: &[u8]) {
    unsafe {
        ffi::ghostty_terminal_vt_write(self.terminal, bytes.as_ptr(), bytes.len());
    }
}
```

**评估**：无缓冲区限制，无静默丢弃，字节直接送入 C 库。

---

### 2.7 渲染层 (`src/terminal/controller.rs`)

#### 脏行追踪 (line 763-830)

```rust
pub fn update_dirty_styled_rows_and_clear_dirty_collect(
    &mut self,
    rows: &mut Vec<VtStyledRow>,
    only_dirty: bool,
) -> anyhow::Result<()> {
    // 依赖 VT 层的 dirty 标记，不直接暴露字节丢失
}
```

---

## 三、问题清单与优先级

### 🔴 P0 - 静默丢数据风险

| # | 问题 | 位置 | 影响 | 状态 |
|---|------|------|------|------|
| 1 | **pending 缓冲区静默丢弃** | `controller.rs:164-167` | 当 pending > 256KB 时丢弃最旧数据，用户无法感知 | ⚠️ 待优化 |
| 2 | **read_buffer 溢出bug** | `ssh_session.rs:929-934` | 大数据包截断导致字节丢失 | ✅ 已修复 |
| 3 | **read_channel 溢出bug** | `ssh_session.rs:770-775` | buffer 不够大时溢出数据丢弃 | ✅ 已修复 |
| 4 | **Phase 2→3 边界bug** | `controller.rs` | 未读 pending 前缀暴露于截断 | ✅ 已修复 |

### 🟡 P1 - 性能瓶颈

| # | 问题 | 位置 | 影响 | 状态 |
|---|------|------|------|------|
| 5 | **帧预算与数据速率不匹配** | `controller.rs:80-81` | 高速数据流时 pending 积压 | ⚠️ 待优化 |
| 6 | **非活跃标签页 pump 延迟** | `tick.rs:170` | 后台标签页 200-250ms 更新一次 | ⚠️ 待优化 |
| 7 | **mpsc send 阻塞 pump 循环** | `ssh_session.rs:644` | 高负载时阻塞数据接收 | ⚠️ 观察中 |

### 🟢 P2 - 监控/诊断增强

| # | 问题 | 位置 | 说明 | 状态 |
|---|------|------|------|------|
| 8 | **scrollback 接近限制无警告** | `ghostty_vt.rs` | 无明确预警 | ⚠️ 待优化 |
| 9 | **VT 层无字节级监控** | `ghostty_vt.rs` | 无法直接观察到 write_vt 字节数 | ⚠️ 待优化 |
| 10 | **缺少 pump 统计** | `tick.rs` | 无累计统计 | ✅ 已部分添加 |

---

## 四、已修复的问题记录

| 日期 | Bug | 修复内容 |
|------|-----|----------|
| 2026-04-10 | read_stream 缓冲区清空 bug | 保留未读取尾部字节 |
| 2026-04-10 | read_channel 溢出丢弃 bug | 添加 overflow_buf |
| 2026-04-10 | Phase 2→3 边界 bug | 未读 pending 前缀提前返回 |
| 2026-04-10 | truncate 泄漏检测 | 添加 BUG 日志 |
| 2026-04-10 | 诊断日志体系 | 分级日志已部署 |

**详见**: `doc/终端大文件显示丢失问题_复盘_v2.md`

---

## 五、待优化项清单

### 优化 1：pending 容量提升 (P0)

**问题**：pending 缓冲区超过 256KB 会静默丢弃数据。

**方案**：将容量提升到 1MB。

**改动位置**：`src/terminal/controller.rs:82`

```rust
pending_capacity_limit: 1024 * 1024,  // 从 256KB → 1MB
```

**优先级**：高
**改动范围**：1 行
**风险**：低

---

### 优化 2：pending 溢出预警日志 (P0)

**问题**：pending 接近容量时无警告，用户无法感知即将丢数据。

**方案**：添加高水位预警日志。

**改动位置**：`src/terminal/controller.rs`

```rust
if pending.len() > pending_capacity_limit * 3 / 4 {
    log::warn!(
        target: "term_pending",
        "pending buffer high watermark: {} B / {} B ({}%)",
        pending.len(),
        pending_capacity_limit,
        (pending.len() * 100) / pending_capacity_limit
    );
}
```

**优先级**：高
**改动范围**：5-10 行
**风险**：无

---

### 优化 3：drain_output 任务化 (P0)

**问题**：帧预算限制导致高速数据流时 pending 积压。

**方案**：将 drain_output 从帧循环中分离，放入独立 tokio task。

```
当前架构:
  UI tick (16-100ms) → pump_output → drain_output (32KB 帧预算)

优化架构:
  Background Task (无帧限制) → drain_output → VT 状态机
  UI tick (仅读取 dirty rows 并渲染)
```

**优先级**：高
**改动范围**：较大，涉及 IcedState 与 terminal 的交互重构
**风险**：较高，需要完整测试

---

### 优化 4：动态预算调整 (P1)

**问题**：固定帧预算无法适应不同数据速率场景。

**方案**：根据历史吞吐量动态调整 `max_bytes_per_frame`。

**改动位置**：`src/terminal/controller.rs`

```rust
let recent_avg = state.perf.bytes_in.avg_last_n(10);
let dynamic_budget = (recent_avg * 2).max(32 * 1024);
self.bridge.set_budgets(dynamic_budget, 64);
```

**优先级**：中
**改动范围**：中等
**风险**：中

---

### 优化 5：VT 层字节级诊断 (P2)

**问题**：VT 层无字节级监控，无法形成完整追踪。

**方案**：在 `write_vt` 前记录字节数。

**改动位置**：`src/terminal/controller.rs` pump_output

```rust
log::trace!(
    target: "vt_bytes",
    "vt_write: bytes={} | pending={} | dirty={}",
    bytes_before,
    pending_len,
    dirty_count
);
```

**优先级**：低
**改动范围**：少量
**风险**：无

---

### 优化 6：scrollback 限制预警 (P2)

**问题**：scrollback 接近限制无明确预警。

**方案**：添加 scrollback 使用率监控和预警。

**优先级**：低
**改动范围**：需要与 Ghostty FFI 交互

---

## 六、代码位置索引

| 文件 | 关键代码 | 说明 |
|------|---------|------|
| `src/backend/ssh_session.rs:617-686` | pump task | SSH 数据接收主循环 |
| `src/backend/ssh_session.rs:741-784` | read_channel | mpsc → 调用者缓冲区 |
| `src/backend/ssh_session.rs:922-940` | read_stream | SshChannel 读取接口 |
| `src/terminal/controller.rs:62-84` | TerminalSessionBridge | pending + 帧预算 |
| `src/terminal/controller.rs:100-150` | drain_output | 帧预算控制 |
| `src/terminal/controller.rs:152-168` | truncate_pending_to_limit | pending 溢出处理 |
| `src/terminal/controller.rs:480-511` | pump_output | VT 写入 + dirty 追踪 |
| `src/app/update/tick.rs:158-204` | pump_all_sessions | pump 调度 |
| `src/backend/ghostty_vt.rs:204-208` | write_vt | VT 解析入口 |

---

## 七、验证方法

```bash
# 开启诊断日志
RUST_LOG=term_pending=debug,term_cmd=debug cargo run

# 执行测试命令
ssh user@host "cat /path/to/large_file.txt"

# 观察日志
# - "pending NOT fully drained" → 帧预算不足
# - "BUG: truncate_pending_to_limit LEAKED!" → truncate 有 bug
# - "pending overflow: DROPPING" → pending 超出容量
# - "pending buffer high watermark" → pending 接近容量（优化后）
# - "scrollback buffer approaching limit" → scrollback 积压（优化后）
```

---

## 八、后续计划

| 优先级 | 优化项 | 预计工时 | 状态 |
|--------|--------|----------|------|
| P0 | pending 容量提升 | 5 分钟 | 待开始 |
| P0 | pending 溢出预警日志 | 15 分钟 | 待开始 |
| P0 | drain_output 任务化 | 2-3 天 | 待评审 |
| P1 | 动态预算调整 | 1 天 | 待评审 |
| P2 | VT 层字节级诊断 | 30 分钟 | 待开始 |
| P2 | scrollback 限制预警 | 1 天 | 待评审 |
