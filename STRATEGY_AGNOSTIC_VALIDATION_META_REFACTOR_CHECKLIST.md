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
- [ ] `cargo test -p ta_app_core --no-default-features`
- [ ] `cargo test -p ta_app_core`
- [ ] `cargo test -p ta_desktop_demo_app --no-default-features` (if supported by crate config)
- [ ] `cargo test -p ta_desktop_demo_app`
- [ ] `cargo test -p ta_desktop_demo_app --features cuda` (when CUDA toolchain is available)
- [ ] `cargo test -p ta_desktop_demo_app --features cuda,cuda-backtest-kernel` (when GPU is available)

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

- [ ] Create a short baseline note in this document (commit hash, date, branch).
- [ ] Capture current benchmark baselines for:
  - [ ] double-MA validation workflow
  - [ ] composite validation workflow
  - [ ] meta-select workflow
  - [ ] e2e workflow ML/meta bench (if practical)
- [ ] Record benchmark protocol details so comparisons are fair:
  - [ ] machine/GPU/CPU identity
  - [ ] build profile (`debug`/`release`)
  - [ ] seeds and dataset(s)
  - [ ] warm-up policy / rerun count
  - [ ] whether runs were isolated (no concurrent GPU/CPU-heavy jobs)
- [ ] Record current artifact JSON examples for:
  - [ ] double-MA validation run artifact
  - [ ] composite validation run artifact
  - [ ] meta-select output artifact
- [ ] Create canonicalized golden outputs for representative seeded runs (JSON snapshots) for before/after diffing.
- [ ] Identify and list all current public APIs / Tauri commands that must remain backward compatible.
- [ ] Confirm current known correctness issues (if any) and mark them as pre-existing.
- [ ] Create a “shadow-run compare” plan for the refactor branch (old path vs new path side-by-side on fixed requests before cutover).

## Phase 1: Refactor Design (Interfaces Before Implementation)

- [ ] Define a shared app-core validation orchestration interface (trait-based adapter).
- [ ] Specify adapter responsibilities (strategy-specific):
  - [ ] parameter type
  - [ ] parameter repair/sanitize
  - [ ] candidate series/build context creation
  - [ ] in-sample metric evaluation
  - [ ] desired positions for iterative engine
  - [ ] trade extraction for bootstrap
  - [ ] CSCV slice-return extraction
  - [ ] candidate result construction (strategy-specific output wrapper)
- [ ] Specify shared validation-core responsibilities:
  - [ ] request cap/sanitize
  - [ ] parallel candidate loop orchestration
  - [ ] walk-forward aggregation
  - [ ] perturb orchestration
  - [ ] trade bootstrap orchestration
  - [ ] iterative engine orchestration
  - [ ] robust score initialization
  - [ ] PSR/DSR compute + gating
  - [ ] robust Pareto marking
  - [ ] CSCV prep + compute + fold
  - [ ] final sort
- [ ] Decide how to preserve current concrete request/result types.
- [ ] Define a no-regression mapping for progress phase names (UI compatibility).
- [ ] Define error message compatibility expectations (exact vs equivalent).

## Phase 2: Extract Shared Validation Core in `app_core`

- [ ] Create new module for shared validation orchestration (proposed: `crates/app_core/src/validation_core.rs`).
- [ ] Implement generic core with minimal abstraction overhead (avoid boxing in hot loops unless needed).
- [ ] Keep existing `validation_pipeline.rs` and `cscv_pbo.rs` contracts unchanged initially.
- [ ] Port duplicated walk-forward aggregation logic into shared core.
- [ ] Port duplicated perturb orchestration logic into shared core.
- [ ] Port duplicated trade bootstrap orchestration logic into shared core.
- [ ] Port duplicated iterative-engine orchestration logic into shared core.
- [ ] Port duplicated PSR/DSR + robust Pareto + CSCV fold call sequence into shared core.
- [ ] Preserve deterministic per-candidate RNG seeding behavior.
- [ ] Preserve candidate truncation/max-candidate semantics.
- [ ] Preserve cancellation checks and progress emission cadence.
- [ ] Keep shared core generic over candidate/result type without introducing `serde_json` in hot validation loops.
- [ ] Keep adapter methods borrow-friendly to avoid excess cloning of candles/series.

### Phase 2 Validation
- [ ] Build and run `ta_app_core` tests.
- [ ] Add/expand unit tests for shared core helpers (where possible).
- [ ] Compare before/after outputs for a fixed seeded double-MA validation request.
- [ ] Compare before/after outputs for a fixed seeded composite validation request.
- [ ] Diff canonicalized golden JSON outputs for representative requests (allowing only intentional field-order differences).
- [ ] Run cancellation smoke test (cancel token trips mid-validation without hangs/panics).
- [ ] Verify progress phase names and monotonic progress behavior for UI compatibility.

## Phase 3: Convert Strategy-Specific Validation Runners to Thin Adapters

- [ ] Refactor `double_ma_validate.rs` to use shared core with a Double-MA adapter.
- [ ] Refactor `composite_validate.rs` to use shared core with a Composite adapter.
- [ ] Keep public signatures unchanged:
  - [ ] `run_double_ma_validation_blocking_with_candles(...)`
  - [ ] `run_composite_validation_blocking_with_candles(...)`
- [ ] Keep current result structs and serialized fields unchanged (backward compatibility).
- [ ] Keep strategy-specific build/eval logic local to the strategy module.
- [ ] Keep existing public exports in `crates/app_core/src/lib.rs` unchanged.

### Phase 3 Correctness Checks
- [ ] Existing smoke tests still pass in both modules.
- [ ] Iterative engine equivalence tests still pass.
- [ ] Probability gate tests still pass.
- [ ] CSCV smoke tests still pass.
- [ ] Rerun equivalence tests still pass.
- [ ] Candidate ordering parity on representative seeded runs (double-MA + composite).
- [ ] Candidate field-level parity spot-check (metrics, PSR/DSR, robust score, CSCV stats, optional sections present/absent).

## Phase 4: Unify Tauri Cross-Ticker Validation Command Plumbing

- [ ] Extract shared cross-ticker application workflow in `src-tauri/src/commands_validation.rs` (or move into `validation_common.rs`).
- [ ] Reuse `validation_common::CrossTickerCandidate` more fully to avoid duplicate functions.
- [ ] Unify duplicate logic:
  - [ ] candidate key map build
  - [ ] dataset loading loop
  - [ ] re-validation request construction pattern
  - [ ] progress offset handling pattern
  - [ ] accumulation and gating
- [ ] Keep Tauri command names and payload schemas unchanged.
- [ ] Keep headless helper(s) working (`apply_cross_ticker_validation_double_ma_headless` or generic replacement + shim).
- [ ] Preserve progress phase names (`double_ma_validate_cross`, `composite_validate_cross`) unless frontend is updated in same phase.

### Phase 4 Checks
- [ ] Tauri validation commands compile and run.
- [ ] Cross-ticker gate behavior unchanged on fixed fixtures.
- [ ] Cross-ticker robust score folding still preserves ordering semantics.
- [ ] Cross-ticker result parity on representative multi-dataset runs (double-MA + composite).

## Phase 5: Meta-Selection Schema Generalization (MA-Assumption Reduction)

Goal: make `meta_select.rs` consume strategy artifacts via generic context fields while preserving back-compat with existing MA-centric artifacts.

- [ ] Identify MA-specific assumptions in `meta_select.rs`:
  - [ ] `ma_source`
  - [ ] `fast_ma_params`
  - [ ] `slow_ma_params`
  - [ ] grouping key generation assumptions
- [ ] Introduce a strategy-agnostic grouping context representation (proposed fields):
  - [ ] `params` (existing)
  - [ ] `feature_context` / `run_context` JSON blob (new)
  - [ ] optional legacy MA fields (back-compat)
- [ ] Add extraction helpers that prefer generic context and fall back to MA-specific fields.
- [ ] Preserve exact behavior for existing double-MA/composite artifacts by default.
- [ ] Keep `MetaSelectionRequest.strategy_kind` filtering unchanged.
- [ ] Ensure ML-confidence training/prediction remains compatible with meta outputs.
- [ ] Define artifact schema versioning/back-compat policy (explicitly) before changing serialized meta fields.

### Phase 5 Checks
- [ ] `meta_select` tests pass.
- [ ] Existing artifact fixtures still parse and produce equivalent groups/scores.
- [ ] `ml_confidence` tests/benches using meta artifacts still pass.
- [ ] Meta report generation still reads key fields (robust p10/median, psr/dsr, holdout stats).
- [ ] Back-compat test: old artifact without generic context still groups correctly.
- [ ] Forward-compat test: new artifact with generic context but missing legacy MA fields still groups correctly.

## Phase 6: Performance Verification and Regression Tuning

- [ ] Re-run baseline benches captured in Phase 0.
- [ ] Compare runtime deltas and record in this document.
- [ ] Compare allocation/memory behavior on representative validation runs (at least coarse RSS / allocation count proxy if available).
- [ ] Investigate any regression > 5%:
  - [ ] allocation churn
  - [ ] extra cloning/serialization
  - [ ] trait dispatch in hot loops
  - [ ] duplicate series rebuilds
  - [ ] synchronization/cancellation checks moved into tighter loops
- [ ] Apply targeted optimizations only if needed (avoid premature micro-optimizations).
- [ ] Re-run `scripts/run_workflow_gate.ps1 -Mode candidate`.
- [ ] Re-run `scripts/run_cpu_perf_gate.ps1 -Mode candidate`.
- [ ] If regressions remain, document accepted tradeoffs and rationale before merge.

## Phase 7: Backward Compatibility / Artifact Compatibility Audit

- [ ] Confirm old artifacts still work with `meta_select_from_artifacts(...)`.
- [ ] Confirm new artifacts do not break old report commands.
- [ ] Document any new optional fields and fallback behavior.
- [ ] Add versioning note if artifact schema meaning changes.
- [ ] Verify Tauri frontend validation flows still render all expected fields.
- [ ] Verify headless CLI / regression workflows consuming validation artifacts still work (if applicable).

## Phase 8: Documentation and Handoff

- [ ] Update `ARCHITECTURE_NOTES.md` with new validation/meta layering.
- [ ] Add a short section to `README.md` or a dev doc describing strategy adapters.
- [ ] Document performance results and any accepted regressions.
- [ ] Document migration notes for adding a new strategy to validation/meta pipeline.
- [ ] List deferred items (if any) with rationale.
- [ ] Add decommission checklist for removed duplicate paths/functions (to prevent dead-code drift).

## Detailed Checklist by Risk Area

## Correctness Checklist

- [ ] Seeded deterministic reruns match before/after for representative requests.
- [ ] Candidate count truncation behavior unchanged.
- [ ] Candidate ordering unchanged when no gates filter rows.
- [ ] `min_psr` / `min_dsr` gates behave identically.
- [ ] `robust_pareto` flags identical on fixed fixtures.
- [ ] CSCV/PBO summary metrics (`pbo`, lambdas, degradation stats) match within tolerance.
- [ ] Cross-ticker gating behavior identical.
- [ ] Meta group counts / top-k selection stable on existing artifact sets.
- [ ] ML-confidence feature extraction remains stable.
- [ ] No strategy-specific semantics drift introduced by shared core abstractions (verified by adapter-specific tests).

## Performance Checklist

- [ ] No new `serde_json` serialization inside per-candidate hot validation loops.
- [ ] No unnecessary cloning of candle arrays / trade arrays.
- [ ] No boxing of strategy adapters in the inner parallel loop unless measured neutral.
- [ ] Progress reporting cadence unchanged or reduced overhead.
- [ ] Shared core preserves Rayon parallelism granularity.
- [ ] CSCV prep does not add an extra full rebuild pass beyond current behavior.
- [ ] No hidden O(N candidates) canonical JSON work added to validation hot path.

## Functionality Checklist

- [ ] Walk-forward output fields preserved.
- [ ] Perturb output fields preserved.
- [ ] Trade bootstrap output fields preserved.
- [ ] Iterative validation output fields preserved.
- [ ] PSR/DSR output and gating preserved.
- [ ] CSCV/PBO output preserved (candidate + summary).
- [ ] Cross-ticker outputs and Pareto flags preserved.
- [ ] Meta-select fields preserved (`robust_score_p10`, `psr_median`, `dsr_median`, etc.).
- [ ] Holdout evaluation path still functions.
- [ ] Report/XLSX exporters still render expected columns (including DSR/robust fields).

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

- [ ] Adapter style: trait with associated types vs function-table struct vs macro-assisted generic scaffolding
- [ ] Shared core placement: new module vs `validation_pipeline.rs` expansion
- [ ] Error string compatibility: exact preservation vs equivalent semantics
- [ ] Progress phase naming contract for UI (strictly stable vs versioned)
- [ ] Artifact schema evolution strategy for meta grouping context
- [ ] Shadow mode strategy: temporary dual-path comparison behind test-only flag vs standalone harness

## Rollback / Safety Plan

- [ ] Keep wrapper function signatures stable throughout refactor.
- [ ] Land refactor in small phases with passing tests after each phase.
- [ ] Avoid simultaneous behavior changes and architecture changes in the same commit when possible.
- [ ] If a regression appears, disable new path behind internal toggle or revert phase-local changes.
- [ ] Keep at least one short-lived branch/tag checkpoint before Phase 2 and before Phase 5 (largest architectural moves).

## Tracking Log (Fill During Execution)

- Baseline commit:
- Branch:
- Bench baselines recorded:
- Phase 2 status:
- Phase 3 status:
- Phase 4 status:
- Phase 5 status:
- Perf gate result:
- CPU perf gate result:
- Known deviations accepted:
