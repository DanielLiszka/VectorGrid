# PRISM-SIG-HVCOST CUDA/PTX Reference Artifact

This directory contains a concrete Rust + CUDA/PTX reference artifact for the
PRISM family of optimizers. The intent is not to replace the host/runtime
architecture already in the app, but to give you a paper-shaped implementation
that already matches the constraints discussed in the proposal and maintainer
feedback:

- search unit = `(strategy signature, numeric cell, fidelity stage)`
- signature-conditioned conservative conformal screen
- expected Pareto hypervolume gain **per millisecond** scheduling
- graph-stable batching for CUDA graph replay
- first-class partial reevaluation keyed by `changed_group_signature`
- quantile-normalized thresholds in latent coordinates where appropriate
- VRAM-resident exact reevaluation path with modular cache tensors
- PTX-first CUDA compilation target for Rust integration

## Layout

- `src/` — Rust host-side optimizer, scheduler, conformal model, cost model,
  archive logic, and a mock backend that compiles and runs without CUDA.
- `cuda/prism_screen_tc.cu` — Tensor-Core-friendly screening kernels.
- `cuda/prism_modular_triad.cu` — exact modular strategy kernels and cache-based
  reevaluation pipeline.
- `cuda/prism_common.cuh` — shared kernel-side structs/enums.
- `scripts/build_ptx.sh` — PTX build script for RTX 4090 / `sm_89`.
- `examples/synthetic.rs` — host-side synthetic smoke run.

## Implemented strategy family

The paper implementation focuses on one **invented but practical** modular
strategy family called `modular_triad`.

It factorizes the exact evaluation graph into three module groups:

1. **Regime gate**
   - `none`
   - `set_gate` = `linearreg_angle + er + chop`
   - `vtrs_gate` = `var + kurtosis + buff_averages`

2. **Entry trigger**
   - `vcc` = squeeze-release continuation
   - `vemr` = volatility-expansion mean reversion
   - `abmt` = adaptive baseline + MACZ trend
   - `tdb` = Donchian + DI/ADXR breakout

3. **Risk / exit overlay**
   - ATR multiple
   - DevStop
   - Chandelier

That keeps the artifact narrow enough to be benchmarkable while still supporting
mixed discrete signatures and partial reevaluation.

## Exact evaluation model

The CUDA exact path is **module staged** rather than monolithic:

1. compute regime cache if regime params changed
2. compute entry cache if entry params changed
3. compute risk cache if risk params changed
4. compose signal from regime + entry caches
5. compute exact metrics from cached signal/risk arrays

This is the key to making partial reevaluation meaningful when indicators are
already VRAM resident. A `changed_group_signature` of `risk` should only trigger
risk recomputation + compose + metrics; `entry` should reuse regime cache; and
`full` recomputes all modules.

The cache tensors are intentionally graph-stable:

- `regime_cache   : [batch_shape, candles] u8`
- `entry_cache    : [batch_shape, candles] i8`
- `risk_cache     : [batch_shape, candles] f32`
- `composed_signal: [batch_shape, candles] i8`

This lets the host capture a distinct CUDA graph per:

`(signature.graph_shape, stage.graph_stage_id, changed_group_signature, batch_shape)`

## Screening model

The screening path is split into:

1. `prism_encode_latent_features_kernel`
2. `prism_tc_linear_predict_kernel`
3. `prism_conservative_bounds_kernel`

The intended production contract is:

- host refits the signature-conditioned conformal linear screen
- host uploads padded `beta` weights and base half-widths
- device encodes latent features, does WMMA GEMM inference, then expands
  conservative bounds
- host computes conservative hypervolume gain per predicted millisecond and
  schedules exact batches

In other words, the Tensor Cores are **not** used for exact backtesting.
They are used only where they fit the workload: dense screening.

## Rust host-side status

The Rust crate compiles without CUDA and includes:

- registry / signature model
- conformal screen model
- runtime cost model
- graph-stable batch planner
- Pareto archive
- progressive cell search + promotion logic
- synthetic mock backend for smoke runs

Smoke test / example:

```bash
cargo run --example synthetic
```

Expected output shape:

```text
final_hv=... stage=... archive=... evals=... partial_reuses=... graph_hit_rate=... bytes_avoided=...
```

## PTX build

This environment did **not** include `nvcc`, so PTX could not be built here.
On a machine with the CUDA toolkit installed:

```bash
./scripts/build_ptx.sh
```

This emits:

- `ptx/prism_screen_tc.ptx`
- `ptx/prism_modular_triad.ptx`

## Integration notes for the Rust app backend

The artifact intentionally does **not** hard-code a Rust CUDA binding crate,
because your app already has a PTX-based backend. The intended integration is:

1. compile `.cu` -> `.ptx`
2. load PTX modules using the app's existing CUDA/PTX path
3. map `PtxLaunchManifest::for_4090()` kernel names to launch sites
4. build one graph per graph-stable key
5. keep `PrismModuleCacheView` allocations resident across launches
6. run optimizer scheduling in Rust and exact batches through the app backend

## What is still left for true paper-grade completion

This artifact is much farther than a sketch, but it is still not a published
result until the following are done in the real app:

- wire exact indicator-bank pointers from VectorTA to `PrismIndicatorBankView`
- replace the mock backend with the real PTX launcher / graph capture logic
- calibrate the runtime cost model from measured GPU timings
- benchmark against brute-force GPU, current progressive search, NSGA-II, TPE,
  and QMC under real wall-clock budgets
- measure false-prune rate, joint interval coverage, exact eval savings,
  partial reuse rate, graph replay hit rate, and bytes avoided

The goal of this artifact is to make that remaining work mostly *integration and
benchmarking*, not invention from scratch.
