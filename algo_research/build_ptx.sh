#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PTX_DIR="$ROOT/ptx"
mkdir -p "$PTX_DIR"

ARCH="compute_89"
CODE="sm_89,compute_89"
COMMON_FLAGS=(
  -std=c++17
  -O3
  --ptx
  --gpu-architecture=${ARCH}
  --gpu-code=${CODE}
  --generate-line-info
)

# Screening path: use Tensor-Core-friendly FP16/WMMA kernels.
nvcc "${COMMON_FLAGS[@]}" \
  "$ROOT/cuda/prism_screen_tc.cu" \
  -o "$PTX_DIR/prism_screen_tc.ptx"

# Exact modular family path: stage-stable cache kernels + metrics kernel.
nvcc "${COMMON_FLAGS[@]}" \
  "$ROOT/cuda/prism_modular_triad.cu" \
  -o "$PTX_DIR/prism_modular_triad.ptx"

printf 'PTX built in %s\n' "$PTX_DIR"
