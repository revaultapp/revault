#!/usr/bin/env bash
# Run Rust test coverage from src-tauri/
# Usage:
#   ./coverage.sh           → HTML report, opens in browser
#   ./coverage.sh --summary → table in terminal

set -e

# Locate llvm tools (macOS Homebrew or system PATH)
if command -v llvm-cov &>/dev/null; then
  export LLVM_COV=$(command -v llvm-cov)
  export LLVM_PROFDATA=$(command -v llvm-profdata)
elif [ -d "/opt/homebrew/opt/llvm/bin" ]; then
  export LLVM_COV=/opt/homebrew/opt/llvm/bin/llvm-cov
  export LLVM_PROFDATA=/opt/homebrew/opt/llvm/bin/llvm-profdata
elif [ -d "/usr/local/opt/llvm/bin" ]; then
  export LLVM_COV=/usr/local/opt/llvm/bin/llvm-cov
  export LLVM_PROFDATA=/usr/local/opt/llvm/bin/llvm-profdata
else
  echo "llvm tools not found. Install with: brew install llvm" >&2
  exit 1
fi

if [ "${1}" = "--summary" ]; then
  cargo cov-summary
else
  cargo cov
fi
