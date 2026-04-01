#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
perf_baseline.sh --pid <PID> --seconds <N> --out <csv_path>

Samples CPU% and RSS(KB) for an existing RustSsh process.
Outputs CSV: ts_ms,cpu_pct,rss_kb

Examples:
  ./scripts/perf_baseline.sh --pid 12345 --seconds 120 --out term-dumps/idle_120s.csv
  ./scripts/perf_baseline.sh --pid 12345 --seconds 1800 --out term-dumps/mem_30min.csv

Notes:
  - This script does not start/stop the app; it only samples an existing PID.
  - For throughput tests, run a high-output command inside the app while sampling.
  - If `ps` is not permitted (sandbox), use app-internal perf dump:
      RUST_SSH_PERF_DUMP=term-dumps/perf.csv cargo run
EOF
}

PID=""
SECONDS="60"
OUT=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --pid) PID="${2:-}"; shift 2 ;;
    --seconds) SECONDS="${2:-}"; shift 2 ;;
    --out) OUT="${2:-}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown arg: $1" >&2; usage; exit 2 ;;
  esac
done

if [[ -z "${PID}" || -z "${OUT}" ]]; then
  usage
  exit 2
fi

mkdir -p "$(dirname "${OUT}")"

echo "ts_ms,cpu_pct,rss_kb" > "${OUT}"

end=$(( $(date +%s) + SECONDS ))
while [[ "$(date +%s)" -lt "${end}" ]]; do
  if ! kill -0 "${PID}" 2>/dev/null; then
    echo "PID ${PID} exited; stopping." >&2
    exit 1
  fi
  ts_ms=$(python3 - <<'PY'
import time
print(int(time.time() * 1000))
PY
)
  # ps output: %cpu rss(kb)
  if ! line="$(ps -p "${PID}" -o %cpu= -o rss= 2>/dev/null | awk '{$1=$1; print $0}')" ; then
    echo "ps is not permitted in this environment." >&2
    echo "Fallback: use app-internal perf dump instead:" >&2
    echo "  RUST_SSH_PERF_DUMP=${OUT} cargo run" >&2
    exit 3
  fi
  cpu="$(echo "${line}" | awk '{print $1}')"
  rss="$(echo "${line}" | awk '{print $2}')"
  echo "${ts_ms},${cpu},${rss}" >> "${OUT}"
  sleep 1
done

echo "Wrote ${OUT}"

