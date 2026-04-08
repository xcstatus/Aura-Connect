#!/bin/bash
# performance_gate.sh — P2.1 性能门禁（单次执行）

PID=$1
OUT="term-dumps/gate_$(date +%s)"

echo "=== RustSSH Performance Gate ==="
echo "PID: $PID"
echo "Output: $OUT/"

mkdir -p "$OUT"

# Idle 60s
echo "--- Phase 1: Idle (60s) ---"
./scripts/perf_baseline.sh --pid $PID --seconds 60 --out "$OUT/idle.csv"

# 提示用户执行高输出命令
echo "请在终端中执行: seq 1 200000"
read -p "按回车继续..."

# Throughput 120s
echo "--- Phase 2: Throughput (120s) ---"
./scripts/perf_baseline.sh --pid $PID --seconds 120 --out "$OUT/throughput.csv"

echo "--- Phase 3: Longrun (300s, 自动) ---"
./scripts/perf_baseline.sh --pid $PID --seconds 300 --out "$OUT/longrun.csv"

echo "=== Done. Check $OUT/ for CSV results. ==="