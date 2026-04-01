#!/usr/bin/env bash
set -euo pipefail

# Terminal key encoding regression matrix.
# Keeps basic invariants stable for vim/tmux/top style apps.

cargo test --test keyboard_matrix

