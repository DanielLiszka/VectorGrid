# Tensor Core ALMA Patch Bundle

This bundle is a **best-effort concrete integration** for a Tensor Core path that actually fits the VectorGrid / ta-desktop-demo style architecture described in the research reports:

- keep prices in VRAM
- precompute indicator slabs on-device
- feed compact indicator slabs into the existing exact evaluator
- avoid touching the already-validated row-first backtest kernel unless absolutely necessary

## What this implements

A **Tensor Core ALMA family precompute** path using **cuBLASLt**.

Why ALMA?
- Your public technical notes describe ALMA as a weighted dot-product over sliding windows.
- They also describe large parameter sweeps and VRAM-resident indicator execution as core to the app.
- Tensor Cores are strongest on matrix multiply / GEMM style workloads.

This patch converts a batched ALMA family into:

- `A = packed sliding price windows` with shape `[rows, max_period]`
- `B = transposed ALMA weight matrix` with shape `[max_period, params]`
- `C = A x B` with shape `[rows, params]`

That GEMM is launched through **cuBLASLt**, which should use Tensor Core kernels when the GPU and shapes are favorable.

## Files

- `crates/app_core/src/tensorcore_alma.rs`
  - Rust wrapper
  - ALMA weight builder
  - small self-contained CUDA buffer helpers for the benchmark binary
- `crates/app_core/native/tensorcore_alma/tca_alma.h`
- `crates/app_core/native/tensorcore_alma/tca_alma.cu`
  - cuBLASLt Tensor Core GEMM path
  - TF32-fast and FP16 modes
- `src-tauri/src/bin/ta_tensorcore_alma_bench.rs`
  - standalone benchmark + correctness spot-check
- `patches/*.snippet`
  - merge hints for `Cargo.toml`, `build.rs`, `lib.rs`, and a likely call site

## Intended place in the pipeline

This path belongs in **indicator preparation**, not in the backtest body.

Use it when:
- family is ALMA (or another fixed-weight window family you adapt later)
- bars are large
- parameter count is large enough to amortize packing + GEMM overhead
- you are already doing VRAM-resident exact search

Do **not** use it for:
- tiny sweeps
- recursive indicators like EMA-style recurrences
- the main lockstep trade-state kernel

## Suggested integration strategy

1. Deduplicate the unique **fast ALMA** specs for the sweep.
2. Deduplicate the unique **slow ALMA** specs for the sweep.
3. Build one `TensorCoreAlmaPlan` per family.
4. Produce two device slabs:
   - fast slab `[rows_fast, fast_params]`
   - slow slab `[rows_slow, slow_params]`
5. In the exact evaluator, use `max(fast_valid_from, slow_valid_from)` as the warmup offset.
6. Keep the existing exact `row_first_global_topk` path unchanged.

## Build

You need a CUDA toolkit with cuBLASLt available.

Typical environment:

```bash
export CUDA_HOME=/usr/local/cuda
export VECTORGRID_ENABLE_TENSORCORE_ALMA=1
cargo build --features tensorcore-alma
```

## Benchmark

Run the included binary:

```bash
cargo run --features tensorcore-alma --bin ta_tensorcore_alma_bench
```

It prints:
- median runtime over repeated runs
- sampled absolute error against a CPU ALMA reference

## Practical defaults

Start with:
- precision: `TcPrecision::Tf32Fast`
- chunk rows: `8192`
- enable only when `params >= 128` and `bars >= 65_536`

Those thresholds are conservative heuristics, not hard rules.

## Why TF32-fast first

The patch defaults to **TF32-fast** instead of FP16 because it is usually the safer numeric starting point for price-series work while still targeting Tensor Cores on supported GPUs.

If you later decide the error budget is acceptable, the plan also supports an FP16 storage path for higher throughput.

## What I would measure first

Compare this path against your current ALMA CUDA indicator path on:
- 200k bars
- 250 ALMA configs
- 1 family at a time
- VRAM-resident exact sweep

Report at least:
- indicator-prep time only
- full exact sweep time
- max / mean numeric drift on sampled outputs
- end-to-end effect on ranked results

## Important honesty note

This bundle was created without compiling against your live repo tree in this environment.
It is designed to be **close to directly usable**, but you should still expect minor wiring work:
- module path adjustments
- crate name adjustments
- build script merging
- aligning the final call site with your existing CUDA allocation / stream management

