# Architecture Notes

Last updated: 2026-02-12

## Product invariants

- Keep the app’s core value proposition intact:
  - fast `vector-ta` showcase,
  - especially CUDA VRAM-resident Double MA optimization path.
- No removal of existing workflows:
  - optimize, validate, meta selection, holdout, reports, run artifacts.
- CPU-only builds must continue to work without CUDA.

## Current module boundaries (incremental refactor)

- `src-tauri/src/commands.rs`
  - Tauri command surface and workflow orchestration.
  - Should gradually move toward high-level orchestration only.
- `src-tauri/src/commands_meta.rs`
  - ML confidence model lifecycle and meta-selection/holdout orchestration.
  - Extracted from `commands.rs` to keep command APIs stable while reducing central file growth.
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
  - `commands_data.rs` (load/resample/polygon adapters),
  - `commands_validation.rs` (validation orchestration),
  - `commands_meta.rs` (meta/holdout orchestration) [done],
  - `commands_reports.rs` (xlsx/csv output orchestration).
- Shared helpers:
  - artifact schema adapters,
  - dataset scope normalization,
  - reusable holdout/evaluation selectors.

## Measurement before/after each extraction

- Build/test status (`cargo test --workspace` when environment permits).
- Workflow runtime parity on benchmark pack.
- File size + function complexity trend (commands orchestration should shrink over time).
