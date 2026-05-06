# Architecture Notes

Last updated: 2026-02-26

## Product invariants

- Keep the app’s core value proposition intact:
  - fast `vector-ta` showcase,
  - especially CUDA VRAM-resident Double MA optimization path.
- No removal of existing workflows:
  - optimize, validate, meta selection, holdout, reports, run artifacts.
- CPU-only builds must continue to work without CUDA.

## Frontend runtime ownership (Vue + Vite)

- Frontend runtime ownership is now fully in `web/src` Vue + Pinia services/stores.
- Legacy frontend runtime path (`web/src/legacy/*`, legacy bridge mounts, and template host bootstrapping) has been retired.
- Tauri command contracts remain stable and are enforced by:
  - generated command registry checks (`web/src/tauri/commands.ts`),
  - lint command-name validation (`web/scripts/check-known-tauri-commands.mjs`),
  - parity check (`web/scripts/check-tauri-parity.mjs`).
- Large table surfaces (Top/Validation/Meta) are virtualized at the component layer to keep UI render cost bounded.
- Task/progress updates are throttled/debounced in `web/src/stores/taskStore.ts` to prevent high-frequency UI churn.

## Double MA backend semantics (current)

- `CpuOnly`: indicators + backtest on CPU (f64 path).
- `GpuOnly`: indicators on GPU, copied to host, backtest on CPU.
- `GpuKernel`: full GPU/VRAM path for supported MA/strategy constraints.
- `GpuHybridKernel`: phase-1 backend contract; currently wired to the same runtime path/constraints as `GpuKernel` while hybrid mixed-indicator orchestration is implemented incrementally.

## Current module boundaries (incremental refactor)

- `src-tauri/src/commands.rs`
  - Tauri command surface and module wiring/re-exports.
  - Keeps command names stable while implementation is decomposed across focused modules.
- `src-tauri/src/commands_data.rs`
  - Data and I/O-adjacent commands:
  - load/resample, repo CSV discovery, market-data directory config, Polygon cache/download helpers, native file pickers.
- `src-tauri/src/commands_validation.rs`
  - Validation orchestration and cross-ticker application for Composite/Double MA.
  - Includes shared chart-price-bar projection used by validation UI.
- `src-tauri/src/commands_optimize.rs`
  - Run-cancellation commands and optimization orchestration (Double MA, Composite CPU, Composite MO).
  - Owns Tauri progress sink adapters used by optimization/validation/meta flows.
- `src-tauri/src/commands_analysis.rs`
  - Candidate-detail and local-stability wrappers over app-core compute paths.
- `src-tauri/src/commands_system.rs`
  - GPU status/device commands and window-effect control.
  - Isolates platform/runtime environment concerns from strategy workflows.
- `src-tauri/src/commands_reports_api.rs`
  - Thin Tauri command wrappers for report build/save/open/reveal flows.
  - Delegates heavy export logic to `commands_reports.rs`.
- `src-tauri/src/commands_artifacts.rs`
  - Thin Tauri command wrappers for run/meta artifact CRUD/list/open operations.
  - Delegates persistence/retention internals to `run_artifacts.rs`.
- `src-tauri/src/commands_meta.rs`
  - ML confidence model lifecycle and meta-selection/holdout orchestration.
  - Extracted from `commands.rs` to keep command APIs stable while reducing central file growth.
- `src-tauri/src/commands_reports.rs`
  - Report/XLSX generation core (analysis/legacy workbook builders).
  - Delegates shared helpers and specialty builders to focused report submodules.
- `src-tauri/src/commands_reports_shared.rs`
  - Shared report helper utilities (path/open/sanitize/quantile helpers).
- `src-tauri/src/commands_reports_csv.rs`
  - Validation shortlist CSV assembly and escaping/sorting/filtering logic.
- `src-tauri/src/commands_reports_meta_xlsx.rs`
  - Meta report workbook builder extracted from main report module.
- `src-tauri/src/run_artifacts.rs`
  - Run/meta artifact persistence, listing, tag updates, retention, and config fingerprints.
  - Extracted to reduce `commands.rs` complexity while preserving behavior.
- `src-tauri/src/validation_common.rs`
  - Shared cross-ticker validation abstractions.
  - Uses trait-based candidate adapters and shared aggregation to avoid per-strategy duplication.
- `src-tauri/src/data_cache.rs`
  - Dataset identity and cache I/O.
- `crates/app_core/`
  - Compute and validation engines (optimization/validation/meta logic).
  - Includes shared validation statistics helpers in `validation_stats.rs` for PSR/DSR and probability gates.
  - Includes shared statistical primitives in `stats_common.rs` to avoid repeated helper logic across validators.
  - Includes trait-based validation post-processing in `validation_pipeline.rs` (probability scoring/gates, pareto tagging, CSCV fold, final sort).
  - Includes shared validation orchestration in `validation_core.rs`; strategy modules (`double_ma_validate.rs`, `composite_validate.rs`) act as thin adapters that keep strategy-specific build/eval logic local.
  - Meta grouping now supports strategy-agnostic context (`feature_context`/`run_context`) with legacy MA-based fallback for backward compatibility.
- `crates/strategies/`
  - Strategy execution logic and metrics.

## Refactor policy

- Prefer extraction over rewrite:
  - move cohesive blocks into focused modules,
  - keep public command signatures stable.
- Require behavior parity for refactors:
  - same request should produce equivalent outputs within tolerance.
- Avoid routing performance-critical loops through new abstraction layers in hot paths.

## Near-term decomposition targets

- `commands.rs` split candidates:
  - `commands_data.rs` (load/resample/polygon adapters) [done],
  - `commands_validation.rs` (validation orchestration) [done],
  - `commands_meta.rs` (meta/holdout orchestration) [done],
  - `commands_optimize.rs` (run/cancel/progress + optimization orchestration) [done],
  - `commands_analysis.rs` (candidate-detail/local-stability wrappers; legacy API names remain) [done],
  - `commands_reports.rs` (xlsx/csv output orchestration internals) [done],
  - `commands_reports_shared.rs` (report helper primitives) [done],
  - `commands_reports_csv.rs` (validation shortlist csv internals) [done],
  - `commands_reports_meta_xlsx.rs` (meta xlsx internals) [done],
  - `commands_reports_api.rs` (report command wrappers) [done],
  - `commands_artifacts.rs` (artifact command wrappers) [done],
  - `commands_system.rs` (gpu/window command surface) [done].
- Shared helpers:
  - artifact schema adapters,
  - dataset scope normalization,
  - reusable holdout/evaluation selectors.

## Measurement before/after each extraction

- Build/test status (`cargo test --workspace` when environment permits).
- Workflow runtime parity on benchmark pack.
- File size + function complexity trend (commands orchestration should shrink over time).
