# scripts/

本目录包含 RustSsh 的**可执行验证脚本**，用于：

- 功能回归（键盘编码等）
- 性能/基线数据采集（P2.1 Gate B）
- GPU 渲染回归（输出对比产物到 `term-dumps/`）

---

## 1) 键盘回归矩阵（P0/DoD）

### `keyboard_matrix.sh`

- **用途**：跑键盘编码不变量的自动化回归（用于 `vim/tmux/top` 类场景的基础保障）。
- **命令**：

```bash
./scripts/keyboard_matrix.sh
```

- **对应测试**：`tests/keyboard_matrix.rs`

---

## 2) 性能基线采集（P2.1 Gate B）

> 说明：本机环境若允许 `ps`，可采 `CPU%/RSS(KB)`。若环境限制 `ps`，可改用应用内 perf dump：
>
> ```bash
> RUST_SSH_PERF_DUMP=term-dumps/perf.csv cargo run
> ```

### `perf_baseline.sh`

- **用途**：对一个**已运行的** RustSsh 进程按秒采样，输出 CSV。
- **输出**：`ts_ms,cpu_pct,rss_kb`
- **命令**：

```bash
./scripts/perf_baseline.sh --pid <PID> --seconds 120 --out term-dumps/idle_120s.csv
```

### `p2_1_collect.sh`

- **用途**：P2.1 “一键采集”脚本，自动生成三段采样 CSV，并打印每段统计摘要（均值/峰值/方差）。
- **会生成**（默认输出目录 `term-dumps/`）：
  - `p2_1_meta.json`（自动写入版本/设置/采集参数）
  - `p2_1_idle_120s.csv`
  - `p2_1_throughput_180s.csv`
  - `p2_1_longrun_1800s.csv`
- **命令**：

```bash
./scripts/p2_1_collect.sh --pid <PID>
```

- **缩短试跑**（仅验证流程）：

```bash
./scripts/p2_1_collect.sh --pid <PID> --idle-seconds 30 --throughput-seconds 60 --longrun-seconds 120
```

- **注意事项**：
  - Throughput 阶段脚本会提示你在 app 内执行高输出命令（如 `seq 1 200000`）。
  - Longrun 阶段需要你保持 app 运行（可选间歇 burst）。

---

## 3) GPU 渲染回归

### `terminal_gpu_regression.sh`

- **用途**：跑 GPU 终端渲染回归矩阵，输出 compare/offscreen 产物到 `term-dumps/`。
- **命令**：

```bash
./scripts/terminal_gpu_regression.sh
```

- **产物**：默认写到 `term-dumps/`（可通过 `RUST_SSH_TERM_DUMP_DIR` 修改）。

---

## 4) 终端诊断开关（统一命名）

为减少噪声，终端诊断日志统一使用：

- **总开关**：`RUST_SSH_TERM_DIAG=1`
- **分项开关**：`RUST_SSH_TERM_DIAG_<SCOPE>=1`
  - `RUST_SSH_TERM_DIAG_HIT_TEST=1`：命中测试日志（`term_hit_test`）
  - `RUST_SSH_TERM_DIAG_VIEWPORT=1`：视口几何日志（`term_viewport`）

示例：

```bash
RUST_SSH_TERM_DIAG_HIT_TEST=1 RUST_LOG=term_hit_test=debug cargo run
```

> 说明：已有性能采集开关（如 `RUST_SSH_PERF_DUMP`）保持不变；诊断开关仅用于控制终端调试日志是否输出。

