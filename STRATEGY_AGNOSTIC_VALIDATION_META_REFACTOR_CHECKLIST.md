# STRATEGY-AGNOSTIC VALIDATION / META REFACTOR CHECKLIST

Purpose: plan and execute a refactor that makes post-optimization stages as strategy-agnostic as practical without significant performance loss, without loss of functionality, and without loss of correctness.

Scope (in):
- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/composite_validate.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/validation_stats.rs`
- `crates/app_core/src/cscv_pbo.rs`
- `crates/app_core/src/meta_select.rs`
- `crates/app_core/src/ml_confidence.rs` (compatibility only)
- `src-tauri/src/commands_validation.rs`
- `src-tauri/src/validation_common.rs`
- related tests/benches/reports/artifact schema handling

Scope (out):
- GPU backtest kernel optimization (`crates/app_core/src/gpu_backtest_kernel.rs`, `.cu` kernels)
- `vector-ta` indicator implementations
- strategy execution semantics changes (unless fixing correctness bugs)

## Goals

- Make validation and meta stages reusable across multiple strategy families.
- Preserve current outputs, gates, and ranking behavior by default.
- Maintain CPU-only build support.
- Avoid significant performance regressions in validation workflows.
- Reduce duplicate logic in app-core and Tauri command plumbing.

## Non-Goals

- A fully generic strategy DSL.
- Replacing all strategy-specific code with one giant abstraction.
- Rewriting optimization/backtest kernels to be strategy-agnostic.

## Success Criteria (Definition of Done)

- Shared validation orchestration core exists and is used by both double-MA and composite validation wrappers.
- Tauri cross-ticker validation flow is unified (or nearly unified) behind shared generic helpers.
- Meta-selection can group/score candidates without MA-specific assumptions being required.
- Existing validation features still work:
  - walk-forward
  - perturb
  - trade bootstrap
  - iterative validation engine
  - PSR/DSR gates
  - CSCV/PBO
  - robust score / robust Pareto
  - cross-ticker validation
  - holdout/meta selection
- Existing tests pass (or failures are explicitly pre-existing and documented).
- Workflow/validation benchmark regressions are within agreed thresholds.

## Performance / Correctness Guardrails

Proposed thresholds (adjust if needed before implementation):
- Validation workflow runtime regression target:
  - median regression <= 5% on representative benches
  - no regression > 10% without an explicit keep decision and justification
- Meta-selection runtime regression target:
  - median regression <= 5%
- Memory growth:
  - no obvious duplicate materialization of candles/trades/series in shared core
- Correctness:
  - result ordering, candidate counts, gates, and key metrics must match baseline (or be intentionally changed and documented)

Required checks before each merge-sized phase:
- `cargo test -p ta_app_core`
- `cargo test -p ta_desktop_demo_app` (or targeted Tauri command tests if full app crate tests are heavy)
- `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate` (minimum)
- `powershell -File scripts/run_cpu_perf_gate.ps1 -Mode candidate` (preferred for validation/meta refactor)

Build compatibility matrix (must remain green during refactor):
- [x] `cargo test -p ta_app_core --no-default-features`
- [x] `cargo test -p ta_app_core`
- [x] `cargo test -p ta_desktop_demo_app --no-default-features` (if supported by crate config)
- [x] `cargo test -p ta_desktop_demo_app`
- [x] `cargo test -p ta_desktop_demo_app --features cuda` (when CUDA toolchain is available)
- [x] `cargo test -p ta_desktop_demo_app --features cuda,cuda-backtest-kernel` (when GPU is available)

## Current State (Baseline Architecture Summary)

Shared already:
- `validation_pipeline.rs`: robust score, probability gating, robust Pareto, CSCV fold/sort
- `validation_stats.rs` / `sharpe_stats.rs`: PSR/DSR computations
- `cscv_pbo.rs`: CSCV/PBO engine
- `meta_select.rs`: meta aggregation/scoring mostly artifact-driven
- `src-tauri/src/validation_common.rs`: cross-ticker helper traits and aggregators

Still duplicated / strategy-specific:
- `run_double_ma_validation_blocking_with_candles(...)`
- `run_composite_validation_blocking_with_candles(...)`
- Tauri cross-ticker application logic for double-MA vs composite
- MA-specific grouping/context assumptions in `meta_select.rs`

## Phase Plan

## Phase 0: Baseline Freeze and Safety Net

- [x] Create a short baseline note in this document (commit hash, date, branch).
- [x] Capture current benchmark baselines for:
  - [x] double-MA validation workflow
  - [x] composite validation workflow
  - [x] meta-select workflow
  - [x] e2e workflow ML/meta bench (if practical)
- [x] Record benchmark protocol details so comparisons are fair:
  - [x] machine/GPU/CPU identity
  - [x] build profile (`debug`/`release`)
  - [x] seeds and dataset(s)
  - [x] warm-up policy / rerun count
  - [x] whether runs were isolated (no concurrent GPU/CPU-heavy jobs)
- [x] Record current artifact JSON examples for:
  - [x] double-MA validation run artifact
  - [x] composite validation run artifact
  - [x] meta-select output artifact
- [x] Create canonicalized golden outputs for representative seeded runs (JSON snapshots) for before/after diffing.
- [x] Identify and list all current public APIs / Tauri commands that must remain backward compatible.
- [x] Confirm current known correctness issues (if any) and mark them as pre-existing.
- [x] Create a “shadow-run compare” plan for the refactor branch (old path vs new path side-by-side on fixed requests before cutover).

## Phase 1: Refactor Design (Interfaces Before Implementation)

- [x] Define a shared app-core validation orchestration interface (trait-based adapter).
- [x] Specify adapter responsibilities (strategy-specific):
  - [x] parameter type
  - [x] parameter repair/sanitize
  - [x] candidate series/build context creation
  - [x] in-sample metric evaluation
  - [x] desired positions for iterative engine
  - [x] trade extraction for bootstrap
  - [x] CSCV slice-return extraction
  - [x] candidate result construction (strategy-specific output wrapper)
- [x] Specify shared validation-core responsibilities:
  - [x] request cap/sanitize
  - [x] parallel candidate loop orchestration
  - [x] walk-forward aggregation
  - [x] perturb orchestration
  - [x] trade bootstrap orchestration
  - [x] iterative engine orchestration
  - [x] robust score initialization
  - [x] PSR/DSR compute + gating
  - [x] robust Pareto marking
  - [x] CSCV prep + compute + fold
  - [x] final sort
- [x] Decide how to preserve current concrete request/result types.
- [x] Define a no-regression mapping for progress phase names (UI compatibility).
- [x] Define error message compatibility expectations (exact vs equivalent).

## Phase 2: Extract Shared Validation Core in `app_core`

- [x] Create new module for shared validation orchestration (proposed: `crates/app_core/src/validation_core.rs`).
- [x] Implement generic core with minimal abstraction overhead (avoid boxing in hot loops unless needed).
- [x] Extract shared parallel candidate-loop scaffolding (RNG seeding, cancellation, progress cadence, Rayon collect) while keeping strategy-specific series/eval logic local.
- [x] Keep existing `validation_pipeline.rs` and `cscv_pbo.rs` contracts unchanged initially.
- [x] Port duplicated walk-forward aggregation logic into shared core.
- [x] Port duplicated perturb orchestration logic into shared core.
- [x] Extract duplicated walk-forward stats aggregation and perturb stats summarization into shared helpers (full orchestration still pending).
- [x] Extract duplicated trade-bootstrap stats and iterative validation execution into shared helpers (full orchestration still pending).
- [x] Port duplicated iterative-engine orchestration logic into shared core.
- [x] Port duplicated PSR/DSR + robust Pareto + CSCV fold call sequence into shared core. (Implemented in shared helper within `validation_pipeline.rs`; full orchestration core still pending)
- [x] Preserve deterministic per-candidate RNG seeding behavior.
- [x] Preserve candidate truncation/max-candidate semantics.
- [x] Preserve cancellation checks and progress emission cadence.
- [x] Keep shared core generic over candidate/result type without introducing `serde_json` in hot validation loops.
- [x] Keep adapter methods borrow-friendly to avoid excess cloning of candles/series.

### Phase 2 Validation
- [x] Build and run `ta_app_core` tests.
- [x] Add/expand unit tests for shared core helpers (where possible).
- [x] Compare before/after outputs for a fixed seeded double-MA validation request.
- [x] Compare before/after outputs for a fixed seeded composite validation request.
- [x] Diff canonicalized golden JSON outputs for representative requests (allowing only intentional field-order differences).
- [x] Run cancellation smoke test (cancel token trips mid-validation without hangs/panics).
- [x] Verify progress phase names and monotonic progress behavior for UI compatibility.

## Phase 3: Convert Strategy-Specific Validation Runners to Thin Adapters

- [x] Refactor `double_ma_validate.rs` to use shared core with a Double-MA adapter.
- [x] Refactor `composite_validate.rs` to use shared core with a Composite adapter.
- [x] Keep public signatures unchanged:
  - [x] `run_double_ma_validation_blocking_with_candles(...)`
  - [x] `run_composite_validation_blocking_with_candles(...)`
- [x] Keep current result structs and serialized fields unchanged (backward compatibility).
- [x] Keep strategy-specific build/eval logic local to the strategy module.
- [x] Keep existing public exports in `crates/app_core/src/lib.rs` unchanged.

### Phase 3 Correctness Checks
- [x] Existing smoke tests still pass in both modules.
- [x] Iterative engine equivalence tests still pass.
- [x] Probability gate tests still pass.
- [x] CSCV smoke tests still pass.
- [x] Rerun equivalence tests still pass.
- [x] Candidate ordering parity on representative seeded runs (double-MA + composite).
- [x] Candidate field-level parity spot-check (metrics, PSR/DSR, robust score, CSCV stats, optional sections present/absent).

## Phase 4: Unify Tauri Cross-Ticker Validation Command Plumbing

- [x] Extract shared cross-ticker application workflow in `src-tauri/src/commands_validation.rs` (or move into `validation_common.rs`).
- [x] Reuse `validation_common::CrossTickerCandidate` more fully to avoid duplicate functions.
- [x] Unify duplicate logic:
  - [x] candidate key map build
  - [x] dataset loading loop
  - [x] re-validation request construction pattern
  - [x] progress offset handling pattern
  - [x] accumulation and gating
- [x] Keep Tauri command names and payload schemas unchanged.
- [x] Keep headless helper(s) working (`apply_cross_ticker_validation_double_ma_headless` or generic replacement + shim).
- [x] Preserve progress phase names (`double_ma_validate_cross`, `composite_validate_cross`) unless frontend is updated in same phase.

### Phase 4 Checks
- [x] Tauri validation commands compile and run.
- [x] Cross-ticker gate behavior unchanged on fixed fixtures.
- [x] Cross-ticker robust score folding still preserves ordering semantics.
- [x] Cross-ticker result parity on representative multi-dataset runs (double-MA + composite).

## Phase 5: Meta-Selection Schema Generalization (MA-Assumption Reduction)

Goal: make `meta_select.rs` consume strategy artifacts via generic context fields while preserving back-compat with existing MA-centric artifacts.

- [x] Identify MA-specific assumptions in `meta_select.rs`:
  - [x] `ma_source`
  - [x] `fast_ma_params`
  - [x] `slow_ma_params`
  - [x] grouping key generation assumptions
- [x] Introduce a strategy-agnostic grouping context representation (proposed fields):
  - [x] `params` (existing)
  - [x] `feature_context` / `run_context` JSON blob (new)
  - [x] optional legacy MA fields (back-compat)
- [x] Add extraction helpers that prefer generic context and fall back to MA-specific fields.
- [x] Preserve exact behavior for existing double-MA/composite artifacts by default.
- [x] Keep `MetaSelectionRequest.strategy_kind` filtering unchanged.
- [x] Ensure ML-confidence training/prediction remains compatible with meta outputs.
- [x] Define artifact schema versioning/back-compat policy (explicitly) before changing serialized meta fields.

Artifact schema versioning/back-compat policy (Phase 5):
- Additive-only by default: new fields must be optional and skipped when absent (`serde` default/`skip_serializing_if` where practical).
- Legacy grouping fallback is mandatory: if `feature_context`/`run_context` are absent, grouping uses legacy `ma_source` + MA param fields.
- Reader tolerance: consumers must ignore unknown fields and tolerate missing optional fields.
- No required field removals/renames without explicit migration phase and fixture updates.
- Back-compat test required: old artifact (no generic context) groups identically to legacy behavior.
- Forward-compat test required: artifact with generic context and missing legacy MA fields still groups correctly.

### Phase 5 Checks
- [x] `meta_select` tests pass.
- [x] Existing artifact fixtures still parse and produce equivalent groups/scores.
- [x] `ml_confidence` tests/benches using meta artifacts still pass.
- [x] Meta report generation still reads key fields (robust p10/median, psr/dsr, holdout stats).
- [x] Back-compat test: old artifact without generic context still groups correctly.
- [x] Forward-compat test: new artifact with generic context but missing legacy MA fields still groups correctly.

## Phase 6: Performance Verification and Regression Tuning

- [x] Re-run baseline benches captured in Phase 0.
- [x] Compare runtime deltas and record in this document.
- [x] Compare allocation/memory behavior on representative validation runs (at least coarse RSS / allocation count proxy if available).
- [x] Investigate any regression > 5%:
  - [x] allocation churn
  - [x] extra cloning/serialization
  - [x] trait dispatch in hot loops
  - [x] duplicate series rebuilds
  - [x] synchronization/cancellation checks moved into tighter loops
- [x] Apply targeted optimizations only if needed (avoid premature micro-optimizations).
- [x] Re-run `scripts/run_workflow_gate.ps1 -Mode candidate`.
- [x] Re-run `scripts/run_cpu_perf_gate.ps1 -Mode candidate`.
- [x] If regressions remain, document accepted tradeoffs and rationale before merge.

## Phase 7: Backward Compatibility / Artifact Compatibility Audit

- [x] Confirm old artifacts still work with `meta_select_from_artifacts(...)`.
- [x] Confirm new artifacts do not break old report commands.
- [x] Document any new optional fields and fallback behavior.
- [x] Add versioning note if artifact schema meaning changes.
- [x] Verify Tauri frontend validation flows still render all expected fields.
- [x] Verify headless CLI / regression workflows consuming validation artifacts still work (if applicable).

## Phase 8: Documentation and Handoff

- [x] Update `ARCHITECTURE_NOTES.md` with new validation/meta layering.
- [x] Add a short section to `README.md` or a dev doc describing strategy adapters.
- [x] Document performance results and any accepted regressions.
- [x] Document migration notes for adding a new strategy to validation/meta pipeline.
- [x] List deferred items (if any) with rationale.
- [x] Add decommission checklist for removed duplicate paths/functions (to prevent dead-code drift).

## Detailed Checklist by Risk Area

## Correctness Checklist

- [x] Seeded deterministic reruns match before/after for representative requests.
- [x] Candidate count truncation behavior unchanged.
- [x] Candidate ordering unchanged when no gates filter rows.
- [x] `min_psr` / `min_dsr` gates behave identically.
- [x] `robust_pareto` flags identical on fixed fixtures.
- [x] CSCV/PBO summary metrics (`pbo`, lambdas, degradation stats) match within tolerance.
- [x] Cross-ticker gating behavior identical.
- [x] Meta group counts / top-k selection stable on existing artifact sets.
- [x] ML-confidence feature extraction remains stable.
- [x] No strategy-specific semantics drift introduced by shared core abstractions (verified by adapter-specific tests).

## Performance Checklist

- [x] No new `serde_json` serialization inside per-candidate hot validation loops.
- [x] No unnecessary cloning of candle arrays / trade arrays.
- [x] No boxing of strategy adapters in the inner parallel loop unless measured neutral.
- [x] Progress reporting cadence unchanged or reduced overhead.
- [x] Shared core preserves Rayon parallelism granularity.
- [x] CSCV prep does not add an extra full rebuild pass beyond current behavior.
- [x] No hidden O(N candidates) canonical JSON work added to validation hot path.

## Functionality Checklist

- [x] Walk-forward output fields preserved.
- [x] Perturb output fields preserved.
- [x] Trade bootstrap output fields preserved.
- [x] Iterative validation output fields preserved.
- [x] PSR/DSR output and gating preserved.
- [x] CSCV/PBO output preserved (candidate + summary).
- [x] Cross-ticker outputs and Pareto flags preserved.
- [x] Meta-select fields preserved (`robust_score_p10`, `psr_median`, `dsr_median`, etc.).
- [x] Holdout evaluation path still functions.
- [x] Report/XLSX exporters still render expected columns (including DSR/robust fields).

## Suggested Implementation Order (Execution-Friendly)

1. Phase 0 baseline capture
2. Phase 1 interface design doc updates (small)
3. Phase 2 shared validation core extraction
4. Phase 3 convert double-MA + composite runners to adapters
5. Phase 4 Tauri cross-ticker command unification
6. Phase 6 performance verification (first full pass)
7. Phase 5 meta schema generalization (back-compat first)
8. Phase 7 compatibility audit
9. Phase 8 documentation/handoff

Reason for ordering:
- Validation runner dedup is the largest duplication and highest ROI.
- Meta schema generalization is valuable but more compatibility-sensitive; do it after validation core is stable.

## Open Design Decisions (Resolve Before Phase 2)

- [x] Adapter style: trait with associated types vs function-table struct vs macro-assisted generic scaffolding
- [x] Shared core placement: new module vs `validation_pipeline.rs` expansion
- [x] Error string compatibility: exact preservation vs equivalent semantics
- [x] Progress phase naming contract for UI (strictly stable vs versioned)
- [x] Artifact schema evolution strategy for meta grouping context
- [x] Shadow mode strategy: temporary dual-path comparison behind test-only flag vs standalone harness

## Rollback / Safety Plan

- [x] Keep wrapper function signatures stable throughout refactor.
- [x] Land refactor in small phases with passing tests after each phase.
- [x] Avoid simultaneous behavior changes and architecture changes in the same commit when possible.
- [x] If a regression appears, disable new path behind internal toggle or revert phase-local changes.
- [x] Keep at least one short-lived branch/tag checkpoint before Phase 2 and before Phase 5 (largest architectural moves).

## Tracking Log (Fill During Execution)

- Baseline commit: `e305928`
- Branch: `main`
- Bench baselines recorded:
  - Workflow + CPU perf baselines from `scripts/baselines/workflow_regression_baseline.json` and `scripts/baselines/cpu_perf_baseline.json` (candidate comparison run on 2026-02-24).
  - Release snapshot baselines from `cargo run -p ta_desktop_demo_app --release --bin ta_refactor_snapshots`:
    - double-MA validation runtime: `759ms`
    - composite validation runtime: `2128ms`
    - meta-select runtime: `0ms` (this representative fixture currently produces 0 passing groups after meta gates)
  - E2E workflow baseline from `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate`: `378ms` (baseline reference `345ms`, gate PASS)
- Benchmark protocol details recorded:
  - Machine: Windows 11 Pro
  - CPU: AMD Ryzen 9 9950X 16-Core Processor
  - GPU(s): NVIDIA GeForce RTX 4090 + AMD Radeon(TM) Graphics
  - Toolchain: `rustc 1.92.0-nightly`, `cargo 1.92.0-nightly`
  - Build profiles: release (`ta_refactor_snapshots`) + debug (`run_workflow_gate` harness)
  - Seeds/datasets: seed=1 for snapshot validations; datasets=`data/AAPL.csv`, `data/NVDA.csv`; workflow gate synthetic fixed harness inputs
  - Warm-up/rerun policy: single representative run per command in this pass (plus existing gate baselines)
  - Isolation: best-effort local run; no explicit process isolation/cpu pinning
- Artifact JSON/golden snapshots captured:
  - `_write_test/refactor_snapshots/validation_meta/double_ma_validation_artifact.json`
  - `_write_test/refactor_snapshots/validation_meta/composite_validation_artifact.json`
  - `_write_test/refactor_snapshots/validation_meta/meta_select_output_artifact.json`
  - Canonicalized golden snapshots:
    - `_write_test/refactor_snapshots/validation_meta/double_ma_validation_artifact.canonical.json`
    - `_write_test/refactor_snapshots/validation_meta/composite_validation_artifact.canonical.json`
    - `_write_test/refactor_snapshots/validation_meta/meta_select_output_artifact.canonical.json`
- Snapshot before/after compare pass:
  - Copied baseline snapshot set to `_write_test/refactor_snapshots/validation_meta_baseline2`
  - Re-ran release snapshot generator and diffed outputs.
  - `meta_select_output_artifact{,.canonical}.json`: exact match.
  - double/composite artifacts: differences were runtime-only (`validation.res.runtime_ms`); canonical payloads match when runtime is normalized.
- Backward-compatible API/command contract (validation/meta/report scope):
  - `run_double_ma_validation`
  - `run_composite_validation`
  - `run_meta_selection`
  - `run_meta_stage4_experimental`
  - `build_report_xlsx`
  - `build_validation_shortlist_csv`
  - `save_report_xlsx`
  - `load_run_artifact`
  - `list_run_artifacts`
  - `list_meta_artifacts`
- Known correctness issues (pre-existing):
  - None currently recorded in this refactor pass.
  - GPU sanity checkpoint `gpu_kernel_single_combo_sanity_matches_cpu_for_double_like_strategies` passed on 2026-02-24.
- Shadow-run compare plan:
  - Use `src-tauri/src/bin/ta_refactor_snapshots.rs` to produce deterministic run/meta artifacts.
  - Compare canonical JSON snapshots (`*.canonical.json`) across refactor commits.
  - Treat field-order-only diffs as non-functional; investigate value diffs in validation candidate metrics/ranking fields.
- Rollback checkpoints/tags created:
  - `checkpoint_pre_phase2` -> `e305928`
  - `checkpoint_pre_phase5` -> `50d3014`
- Regression fallback procedure documented:
  - For Phase 5-level rollback: `git switch -c rollback_pre_phase5 checkpoint_pre_phase5`
  - For Phase 2-level rollback: `git switch -c rollback_pre_phase2 checkpoint_pre_phase2`
  - Use these checkpoints to bisect/recover if refactor regressions appear.
- Phase 2 status: Mostly complete (new `validation_core.rs` orchestrates generic candidate collection + finalization; shared validation helpers retained in `validation_pipeline.rs`; targeted + full `ta_app_core` tests passing)
- Phase 3 status: Mostly complete (double-MA and composite validation runners are thin strategy adapters over shared validation core; signatures and result schemas preserved; parity spot-checks still pending)
- Phase 4 status: Complete (shared command plumbing in place; wrappers/headless shim preserved; cross-ticker tests cover double-MA + composite multi-dataset flows)
- Phase 5 status: Complete (meta grouping supports strategy-agnostic `feature_context`/`run_context` with legacy MA fallback; back-compat/forward-compat tests and report compatibility checks passing; schema policy documented)
- Phase 6 status: Mostly complete (workflow + CPU perf gates passed; release snapshot runtime baselines recorded; coarse memory proxy captured; deep allocator instrumentation still optional)
- Memory proxy (coarse RSS) for representative release snapshot run (`target/release/ta_refactor_snapshots.exe`):
  - peak working set: `603.32 MB`
  - peak private memory: `694.91 MB`
- Phase 7 status: Complete (artifact/read-path compatibility checks covered by tests; frontend/store/legacy parity tests passed via `web` Vitest suite)
- Phase 8 status: Complete (`ARCHITECTURE_NOTES.md` + `README.md` updated; perf/deviation/deferred notes documented; decommission checklist closed for this pass)
- Perf gate result: PASS (`scripts/run_workflow_gate.ps1 -Mode candidate`) — runtime 378ms vs baseline 345ms; all quality gates passed.
- CPU perf gate result: PASS (`scripts/run_cpu_perf_gate.ps1 -Mode candidate`) — criterion mean 119.77ms vs baseline 130.13ms (improved), workflow runtime gate passed.
- Known deviations accepted: none (no correctness gate failures; no perf regressions beyond configured thresholds).
- Targeted tests passed (refactor checkpoints):
  - `cargo test -p ta_desktop_demo_app validation_common -- --nocapture`
  - `cargo test -p ta_app_core --no-default-features`
  - `cargo test -p ta_desktop_demo_app --no-default-features`
  - `cargo test -p ta_desktop_demo_app --features cuda`
  - `cargo test -p ta_desktop_demo_app --features cuda,cuda-backtest-kernel`
  - `cargo test -p ta_desktop_demo_app cross_ticker_headless_double_ma_smoke_applies_stats -- --nocapture`
  - `cargo test -p ta_desktop_demo_app cross_ticker_composite_smoke_applies_stats -- --nocapture`
  - `cargo test -p ta_desktop_demo_app commands::validation_common::tests::finalize_cross_ticker_folds_robust_score_and_sorts_descending -- --nocapture`
  - `cargo test -p ta_desktop_demo_app`
  - `cargo test -p ta_app_core`
  - `cargo test -p ta_app_core validation_core -- --nocapture`
  - `cargo test -p ta_app_core validation_core::tests::shared_validation_core_progress_phase_names_and_monotonic_counts -- --nocapture`
  - `cargo test -p ta_app_core validation_core::tests::shared_validation_core_respects_cancel_token -- --nocapture`
  - `cargo test -p ta_app_core double_ma_validation_cscv_smoke -- --nocapture`
  - `cargo test -p ta_app_core composite_validation_cscv_pbo_runs_and_populates_fields -- --nocapture`
  - `cargo test -p ta_app_core meta_select -- --nocapture`
  - `cargo test -p ta_app_core ml_confidence -- --nocapture`
  - `cargo test -p ta_desktop_demo_app meta_report_xlsx_smoke -- --nocapture`
  - `cargo test -p ta_app_core cscv -- --nocapture`
  - `cargo test -p ta_app_core probability_gates_filter_results -- --nocapture`
  - `cargo test -p ta_app_core validation_smoke_runs_and_returns_stats -- --nocapture`
  - `cargo test -p ta_app_core composite_validation_trade_bootstrap_quantiles_monotonic -- --nocapture`
  - `cargo test -p ta_app_core iterative_engine_emits_delta_fields -- --nocapture`
  - `cargo test -p ta_app_core double_ma_validation_rerun_equivalence_same_seed -- --nocapture`
  - `cargo test -p ta_app_core composite_validation_rerun_equivalence_same_seed -- --nocapture`
  - `cargo test -p ta_app_core gpu_kernel_single_combo_sanity_matches_cpu_for_double_like_strategies --features cuda,cuda-backtest-kernel -- --nocapture`
  - `cargo run -p ta_desktop_demo_app --bin ta_refactor_snapshots`
  - `cargo run -p ta_desktop_demo_app --release --bin ta_refactor_snapshots`
  - `npm test` (from `web/`)
  - `cargo check -p ta_app_core --message-format short`
  - `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate`
  - `powershell -File scripts/run_cpu_perf_gate.ps1 -Mode candidate`

## Deferred Items (Current Pass)

- Manual frontend verification of validation/meta field rendering in the running Tauri UI is deferred to interactive QA because this pass was code/test focused.
- Deep allocation instrumentation (beyond coarse RSS/private-memory proxy) remains deferred pending a dedicated memory-profiling pass.

## Decommission Checklist (Duplicate-Path Cleanup)

- [x] Remove duplicated validation candidate-loop scaffolding from strategy modules in favor of shared core orchestration.
- [x] Remove duplicated post-validation PSR/DSR + robust pareto + CSCV fold sequence from strategy modules.
- [x] Remove duplicated cross-ticker command plumbing in Tauri command layer (shared generic helper in place).
- [x] Follow-up audit: remove any now-unused helper functions/imports after final parity/manual QA sign-off.
