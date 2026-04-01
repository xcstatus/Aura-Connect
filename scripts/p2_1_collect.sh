#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
p2_1_collect.sh --pid <PID> [--out-dir term-dumps] [--idle-seconds 120] [--throughput-seconds 180] [--longrun-seconds 1800]

One-click data collection for P2.1 (Gate B baseline):
  - idle (2min)
  - throughput (3min)  <-- you run a high-output command inside the app during this window
  - longrun (30min)    <-- you keep the app running; optional but recommended

Outputs:
  <out-dir>/p2_1_idle_<Ns>.csv
  <out-dir>/p2_1_throughput_<Ns>.csv
  <out-dir>/p2_1_longrun_<Ns>.csv

Each stage prints summary stats for cpu_pct and rss_kb:
  mean / max / variance

Example:
  ./scripts/p2_1_collect.sh --pid 47725
EOF
}

PID=""
OUT_DIR="term-dumps"
IDLE_SECS="120"
THR_SECS="180"
LONG_SECS="1800"
META_JSON=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --pid) PID="${2:-}"; shift 2 ;;
    --out-dir) OUT_DIR="${2:-}"; shift 2 ;;
    --idle-seconds) IDLE_SECS="${2:-}"; shift 2 ;;
    --throughput-seconds) THR_SECS="${2:-}"; shift 2 ;;
    --longrun-seconds) LONG_SECS="${2:-}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown arg: $1" >&2; usage; exit 2 ;;
  esac
done

if [[ -z "${PID}" ]]; then
  usage
  exit 2
fi

mkdir -p "${OUT_DIR}"
META_JSON="${OUT_DIR}/p2_1_meta.json"

write_meta() {
  # Capture repo + toolchain + OS + persisted app settings (settings.json).
  python3 - "$META_JSON" "$PID" "$OUT_DIR" "$IDLE_SECS" "$THR_SECS" "$LONG_SECS" <<'PY'
import json, os, platform, subprocess, sys, time

out = sys.argv[1]
pid = sys.argv[2]
out_dir = sys.argv[3]
idle_secs = int(sys.argv[4])
thr_secs = int(sys.argv[5])
long_secs = int(sys.argv[6])

def sh(cmd):
    try:
        return subprocess.check_output(cmd, stderr=subprocess.STDOUT, text=True).strip()
    except Exception as e:
        return None

def read_cargo_toml():
    try:
        import tomllib  # py3.11+
        with open("Cargo.toml", "rb") as f:
            return tomllib.load(f)
    except Exception:
        return None

toml = read_cargo_toml() or {}

meta = {
    "collected_at_ms": int(time.time() * 1000),
    "pid": int(pid),
    "collection": {
        "out_dir": out_dir,
        "stages": {
            "idle": {"seconds": idle_secs, "out": f"{out_dir}/p2_1_idle_{idle_secs}s.csv"},
            "throughput": {"seconds": thr_secs, "out": f"{out_dir}/p2_1_throughput_{thr_secs}s.csv"},
            "longrun": {"seconds": long_secs, "out": f"{out_dir}/p2_1_longrun_{long_secs}s.csv"},
        },
        "sampler": {"type": "ps", "fields": ["cpu_pct", "rss_kb"]},
    },
    "platform": {
        "system": platform.system(),
        "release": platform.release(),
        "version": platform.version(),
        "machine": platform.machine(),
    },
    "repo": {
        "git_head": sh(["git", "rev-parse", "HEAD"]),
        "git_status_porcelain": sh(["git", "status", "--porcelain"]),
    },
    "toolchain": {
        "rustc": sh(["rustc", "-V"]),
        "cargo": sh(["cargo", "-V"]),
    },
    "package": {
        "crate_name": (toml.get("package") or {}).get("name"),
        "version": (toml.get("package") or {}).get("version"),
        "lib_name": (toml.get("lib") or {}).get("name"),
    },
    "settings": {
        "paths_tried": [],
        "path": None,
        "loaded": None,
    },
}

# Match Settings::get_path(): ProjectDirs("com","rustssh","rust-ssh") + "settings.json"
home = os.path.expanduser("~")
candidates = [
    # Common ProjectDirs layout on macOS:
    os.path.join(home, "Library", "Application Support", "rust-ssh", "settings.json"),
    os.path.join(home, "Library", "Application Support", "com.rustssh.rust-ssh", "settings.json"),
    os.path.join(home, "Library", "Preferences", "com.rustssh.rust-ssh", "settings.json"),
]
meta["settings"]["paths_tried"] = candidates
chosen = None
for p in candidates:
    if os.path.exists(p):
        chosen = p
        break
if chosen:
    meta["settings"]["path"] = chosen
    try:
        with open(chosen, "r", encoding="utf-8") as f:
            meta["settings"]["loaded"] = json.load(f)
    except Exception as e:
        meta["settings"]["loaded_error"] = str(e)
else:
    meta["settings"]["loaded_error"] = "settings.json not found in candidates"

os.makedirs(os.path.dirname(out), exist_ok=True)
with open(out, "w", encoding="utf-8") as f:
    json.dump(meta, f, ensure_ascii=False, indent=2)
print(f"Wrote meta: {out}")
PY
}

summarize_csv() {
  local csv="$1"
  python3 - "$csv" <<'PY'
import csv, math, sys
path = sys.argv[1]
cpu = []
rss = []
with open(path, newline="") as f:
    r = csv.DictReader(f)
    for row in r:
        try:
            cpu.append(float(row["cpu_pct"]))
            rss.append(float(row["rss_kb"]))
        except Exception:
            pass

def stats(xs):
    if not xs:
        return None
    mean = sum(xs) / len(xs)
    var = sum((x - mean) ** 2 for x in xs) / len(xs)
    mx = max(xs)
    return mean, mx, var

scpu = stats(cpu)
srss = stats(rss)
print(f"file: {path}")
if scpu:
    print(f"  cpu_pct: mean={scpu[0]:.3f} max={scpu[1]:.3f} var={scpu[2]:.3f} n={len(cpu)}")
else:
    print("  cpu_pct: (no samples)")
if srss:
    print(f"  rss_kb:  mean={srss[0]:.1f} max={srss[1]:.1f} var={srss[2]:.1f} n={len(rss)}")
else:
    print("  rss_kb:  (no samples)")
PY
}

countdown() {
  local n="$1"
  while [[ "$n" -gt 0 ]]; do
    echo "Starting in ${n}s..."
    sleep 1
    n=$((n-1))
  done
}

run_stage() {
  local name="$1"
  local secs="$2"
  local out="${OUT_DIR}/p2_1_${name}_${secs}s.csv"
  echo ""
  echo "== Stage: ${name} (${secs}s) =="
  echo "Output: ${out}"
  ./scripts/perf_baseline.sh --pid "${PID}" --seconds "${secs}" --out "${out}"
  summarize_csv "${out}"
}

echo "P2.1 collection (PID=${PID})"
echo "out-dir=${OUT_DIR} idle=${IDLE_SECS}s throughput=${THR_SECS}s longrun=${LONG_SECS}s"
write_meta

echo ""
echo "== Idle =="
echo "Keep the app focused and mostly idle during this window."
countdown 3
run_stage "idle" "${IDLE_SECS}"

echo ""
echo "== Throughput =="
cat <<'EOF'
During the next window, run ONE of these inside the app terminal:
  - seq 1 200000
  - yes | head -n 200000
EOF
countdown 5
run_stage "throughput" "${THR_SECS}"

echo ""
echo "== Longrun =="
cat <<'EOF'
Keep the app running and connected. Optional bursts every 1-2 minutes:
  - seq 1 50000
EOF
countdown 5
run_stage "longrun" "${LONG_SECS}"

echo ""
echo "Done."

