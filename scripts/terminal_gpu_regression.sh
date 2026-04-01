#!/usr/bin/env bash
set -euo pipefail

# GPU terminal regression matrix:
# - tight/full UV mapping
# - fit-scale off/on
# Dumps compare/offscreen artifacts for each run into term-dumps/

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

FONT_TTC_DEFAULT="/System/Library/Fonts/Menlo.ttc"
FONT_FACE_DEFAULT="0"

export RUST_SSH_TERMINAL_GPU=1
export RUST_SSH_TERM_DISPLAY_TEST="${RUST_SSH_TERM_DISPLAY_TEST:-2}"
export RUST_SSH_TERM_DUMP_COMPARE=1
export RUST_SSH_TERM_DUMP_OFFSCREEN=1
export RUST_SSH_TERM_DUMP_DIR="${RUST_SSH_TERM_DUMP_DIR:-term-dumps}"
export RUST_SSH_FONT_TTC="${RUST_SSH_FONT_TTC:-$FONT_TTC_DEFAULT}"
export RUST_SSH_FONT_FACE_INDEX="${RUST_SSH_FONT_FACE_INDEX:-$FONT_FACE_DEFAULT}"
export RUST_LOG="${RUST_LOG:-term-diag=debug,vt.paint=debug}"

echo "== RustSsh terminal GPU regression =="
echo "dump dir: $RUST_SSH_TERM_DUMP_DIR"
echo "font: $RUST_SSH_FONT_TTC#$RUST_SSH_FONT_FACE_INDEX"
echo

run_case() {
  local tight="$1"
  local fit="$2"
  echo "---- case: tight_rect=$tight fit_scale=$fit ----"
  RUST_SSH_GLYPH_TIGHT_RECT="$tight" \
  RUST_SSH_GLYPH_FIT_SCALE="$fit" \
  cargo run
}

run_case 0 0
run_case 1 0
run_case 0 1
run_case 1 1

echo
echo "Done. Check artifacts under: $RUST_SSH_TERM_DUMP_DIR"
