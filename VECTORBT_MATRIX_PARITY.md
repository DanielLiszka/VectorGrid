# VectorBT “Matrix Optimization” Parity (VectorTA Edition)

This document focuses on **vectorbt-style, matrix-like backtest optimization** (maximum throughput over large parameter grids), but implemented in this repo’s Rust/Tauri architecture and using **VectorTA (`vector-ta`)** for *all* indicator computation (CPU + CUDA kernels).

It is **not** a goal to replicate the entire vectorbt portfolio feature surface area (order models, multi-asset cash sharing, etc.) if it materially reduces matrix throughput or complicates GPU kernel paths.

---

## What “matrix-like optimization” means here

When we say “matrix-like”, we mean:

- **Many backtests at once**, where each “column” is a distinct parameter combination.
- Prefer **dense/contiguous arrays** and **streaming reductions** (best / top-K / heatmap) over per-run object graphs.
- Prefer **batch indicator evaluation** with reuse/caching, rather than recomputing the same indicator repeatedly.
- Keep GPU as a first-class backend (VRAM-resident kernel path stays a differentiator).

---

## vectorbt primitives worth mirroring (research notes)

The following vectorbt components are especially relevant to matrix speed and ergonomics:

### 1) IndicatorFactory (indicator/signal “factory” pipeline)

vectorbt’s `IndicatorFactory` provides a way to define an indicator once, then **run it over parameter grids** (including combinations), returning outputs plus metadata/indexing.

- Docs: https://vectorbt.dev/api/indicators/factory/

Key ideas we care about:

- A **standardized indicator spec** (inputs + params → outputs).
- `run` / `run_combs` style APIs that operate on **broadcasted parameter sets**.
- Built-in handling of parameter combinations and output wrappers.

### 2) Parameter product + broadcasting helpers

vectorbt has utilities to build parameter products and broadcast them into shapes needed for matrix evaluation.

- Docs: https://vectorbt.dev/api/utils/params/

Key ideas we care about:

- Constructing a deterministic **Cartesian product** over discrete domains.
- Supporting “comb” runs (selective combinations vs full product).
- Broadcasting scalars / vectors without materializing huge intermediate matrices.

### 3) ArrayWrapper / reshape utilities (indexing + broadcasting model)

vectorbt’s `ArrayWrapper` and reshape helpers formalize:

- How 1D/2D arrays map to “columns” (parameter instances) and “rows” (time).
- How to reshape/unstack/stack while preserving labeling.

- Reshape docs: https://vectorbt.dev/api/base/reshape_fns/

Key idea we care about:

- A lightweight, explicit **layout/indexing model** to map `(time, column)` ↔ linear storage.

### 4) Record arrays (events as “records”, not dense matrices)

vectorbt uses “records” (orders, trades, positions…) that scale better than storing dense per-column event lists, and can be grouped by “column”.

- Docs: https://vectorbt.dev/api/records/base/

Key ideas we care about:

- A **column id** (e.g., `col`) on each record to map events back to a parameter instance.
- Prefer **struct-of-arrays (SoA)** for cache locality and filtering.

---

## What this repo already does well (matrix-speed aligned)

This repo already has several “vectorbt-like” ideas implemented (often faster than Python when the GPU kernel path is used):

- **Tiled grid search** with streaming best/top‑K reduction and optional heatmap (`ta_app_core::double_ma`).
- **GPU backtest kernel** (VRAM-resident per-tile) for extreme throughput.
- **Search-space abstractions + samplers** (Random / QMC Halton+Sobol / TPE / NSGA‑II) in `ta_optimizer::mo`.
- **Capability-aware MA registry + dynamic MA params plumbing** via `vector-ta`.

---

## Parity gaps that matter for matrix optimization (VectorTA-compatible)

These are the main missing pieces if the goal is “vectorbt ergonomics + matrix speed”, while keeping VectorTA as the indicator engine:

### A) A generalized “IndicatorFactory-like” layer for VectorTA indicators

Goal: strategies compose indicators and signals via a stable API, without hand-wiring each sweep.

What we want:

- A small set of **typed batch outputs** (e.g., `IndicatorMatrixF64`) that:
  - are **row-major contiguous** (`period`/`param` rows × `time` cols),
  - carry enough metadata for indexing/slicing,
  - can be produced by CPU batch, CUDA MA-sweep, or CUDA kernel paths.

### B) Deterministic Cartesian products over discrete param domains (with indexing)

Sampling exists (`ta_optimizer::mo`), but matrix workflows often want:

- “Run the whole grid (or a bounded grid) deterministically”
- “Map combo index ↔ param values”

This enables:

- stable caching keys,
- repeatable reproduction,
- consistent column mapping for records.

### C) Record arrays with column mapping (trade/order/event records)

For matrix mode, we need a compact way to store *events* for selected columns (e.g., best + top-K), without large allocations:

- `TradeRecords` (SoA) with a `col` field
- optional: later, `OrderRecords`, `PositionRecords`

### D) A reusable “signals → portfolio” engine for CPU (optional but valuable)

The repo currently has strategy-specific backtest loops (which are great for speed). A reusable engine can:

- reduce duplicated logic,
- make it easier to add new strategies that still run in matrix mode.

This should not replace specialized GPU kernels; it should be a CPU building block.

---

## What’s implemented in this change (foundation work)

This change adds *foundational* primitives to close the gaps above without refactoring existing fast paths:

1) **Deterministic grid enumeration** for `ta_optimizer::mo::SearchSpace` (Cartesian products over discrete domains with safety guards).
2) **Trade record arrays** (`TradeRecords`, SoA) that carry a **`col` id**, enabling vectorbt-like “records grouped by column”.
3) A minimal, **IndicatorFactory-like** API for VectorTA moving averages (CPU), returning a typed `IndicatorMatrixF64`.

These are intentionally small, dependency-free building blocks that can be adopted incrementally by `ta_app_core` without jeopardizing GPU paths.

---

## Next incremental steps (recommended order)

1) Add an app-core level **VectorTA indicator batch API** that returns `IndicatorMatrix*` outputs consistently across CPU/GPU backends.
2) Add a “column index contract” across optimizers:
   - deterministic column ids for grid runs,
   - stable ids for sampled runs (seeded sampler + ordered insertion).
3) Use `TradeRecords` for storing **best/top‑K drilldowns** (optional), and later for exporting richer per-run artifacts.
4) Optional: introduce a reusable CPU “signals → portfolio” engine, and migrate one strategy at a time.
