# Bundle 10 Validated Tracking Checklist

Date: 2026-04-01
Scope: `MlSignal` dataset export, score-cache validation, replay, artifact fidelity, bounded warm-path improvements, benchmark coverage, and research-harness contract truthfulness
Authority: Derived from the current repo state plus [codex_prompt_mlsignal_bundle10_implementation_2026-04-01.md](/C:/Rust%20Projects/ta_desktop_demo/bundle10/codex_prompt_mlsignal_bundle10_implementation_2026-04-01.md)

Use this file as the authoritative progress tracker for Bundle 10. It tracks only the repo-valid portions of the older plan and addendum. If the older docs ask for work that is not represented here, treat that work as stale, preserve-only, or optional follow-up rather than required implementation.

## Validation Completed

- [x] Audited `bundle10/mlsignal_bundle10_full_plan_2026-04-01.md` against the current repo.
- [x] Audited `bundle10/mlsignal_bundle10_plan_addendum_2026-04-01.md` against the current repo.
- [x] Cross-checked scope against Bundle 10 in [gpt54_feature_review_map.md](/C:/Rust%20Projects/ta_desktop_demo/gpt54_feature_review_map.md).
- [x] Cross-checked current frontend `MlSignal` orchestration and replay paths.
- [x] Cross-checked current backend/runtime `MlSignal` dataset export, score-cache, validation, and sweep paths.
- [x] Cross-checked `research/ml_signal/*` as a contract/truthfulness surface rather than a mode-split or app-training surface.
- [x] Narrowed required scope to only repo-valid missing work.

## Preserve-Only / Already Landed

- [x] Preserve the current `strategyKind === "MlSignal"` bridge for this pass.
- [x] Preserve the existing `MlSignal` Tauri command surface in `src-tauri/src/commands_ml_signal.rs`.
- [x] Preserve existing `workflowArtifactsStore.ts` normalization for `dataset_meta` / `datasetMeta` and `ml_signal` / `mlSignal`.
- [x] Preserve existing `validation_context` handling for ML validation and sweep.
- [x] Preserve already-shipped dataset export outputs including `feature_schema_path`, `export_manifest_path`, and `split_manifest_path`.
- [x] Preserve already-shipped split-membership precompute in dataset export.
- [x] Preserve already-shipped candidate-specific `track_record_bars` unless a directly related correctness bug requires a tiny fix.
- [x] Preserve advisory model-bundle status behavior for this pass.
- [x] Keep offline CatBoost/calibration/walk-forward work in `research/ml_signal/`.
- [x] Keep the top-level `MlSignal` mode split deferred.
- [x] Treat `crates/app_core/benches/e2e_workflow_ml.rs` as background context only, not as direct `MlSignal` benchmark coverage.
- [x] Do not treat `research/ml_signal/common.py` as required parity work.
- [x] Do not create a brand-new ML replay test surface when the existing `mlSignalService.test.ts` and `historyService.test.ts` files can be extended.

## Required Implementation

### Backend Correctness

- [x] Prefer calibrated score columns first: `p_long_cal` / `p_short_cal`, then `p_long` / `p_short`.
- [x] Persist explicit `score_space`.
- [x] Reject raw-only score caches for probability-threshold validation.
- [x] Support long-only validation with one-sided long scores.
- [x] Fail explicitly when two-sided validation is missing a side.
- [x] Implement backend semantics for frontend-serialized `allow_flip`.
- [x] Implement backend semantics for frontend-serialized `cooldown_bars`.
- [x] Implement backend semantics for frontend-serialized `signal_persistence_bars`.
- [x] Count and persist `duplicate_timestamps`.
- [x] Persist `coverage_ratio`.
- [x] Persist `score_scope`.

### Artifact / Replay Determinism

- [x] Persist dataset export as an artifact-backed ML stage instead of leaving it as UI-only status text.
- [x] Extend the `ml_signal` artifact block with explicit ML-stage identity.
- [x] Carry dataset/export/split/schema metadata forward into later ML artifacts.
- [x] Make `loadSavedRun()` preserve the `ml_signal` block alongside the already-preserved `dataset_meta`.
- [x] Restore saved ML validation thresholds from saved `validation.req`.
- [x] Restore saved ML sweep axes and related request fields from saved `validation.req`.
- [x] Make `rerunSavedRun()` rerun the full ML flow instead of only regenerating the score cache.
- [x] Keep `strategyKind = "MlSignal"` restoration correct during ML apply/rerun without broad shell refactors.

### Warm-Path Improvements

- [x] Replace repeated rolling-window rescans in dataset export with prefix/rolling helpers.
- [x] Reuse export row buffers where the change stays local and safe.
- [x] Reduce validation/sweep allocation churn where the change stays local and safe.
- [x] Add parsed score-cache reuse or equivalent deterministic warm-path improvement.
- [x] Keep loaded-dataset warm caches bounded; no extra app-global cache layer was added because the local warm-path wins were sufficient for this pass.

### Frontend Status / UX Truthfulness

- [x] Surface truthful score coverage, score scope, and duplicate information in ML status text once backend returns it.
- [x] Keep the current optimize-shell bridge honest; do not frame `MlSignal` as a standard optimizer-native Top/Pareto/heatmap workflow.

### Benchmarks / Research Contract / Docs

- [x] Add direct `MlSignal` benchmark coverage in a dedicated Criterion surface.
- [x] Update research-harness manifest/output metadata so calibrated-column expectations are explicit.
- [x] Update research docs so `validation_scores.csv` vs `walkforward_test_scores.csv` scope differences are explicit.
- [x] Add explicit walk-forward proxy / desktop-parity disclaimers to research outputs/docs that currently imply more parity than they provide.
- [x] Update `research/ML_SIGNAL_STRATEGY_IMPLEMENTATION_PLAN_2026-03-18.md` only as needed for truthfulness.
- [x] Leave `Frontend Review/Unfinished/mlsignal_mode_separation_plan_2026-03-31.md` untouched; no deferred entry-criteria update was required for this pass.

### Tests / Verification

- [x] Add or update focused Rust tests for calibrated-column preference, raw-only rejection, missing-side handling, metadata persistence, and ML policy semantics.
- [x] Add or update focused TypeScript tests for dataset-export artifact persistence, enriched ML artifact metadata, and ML replay/rerun restoration.
- [x] Extend existing ML history/replay tests if replay behavior changes.
- [x] Run lightweight Python syntax/compile verification if research scripts change.

## Optional Follow-Up Only

- [ ] Sweep-size preflight or button disablement for oversized ML policy sweeps.
- [ ] Debounced or explicit-trigger model-bundle status refresh in `MlSignalWorkspacePanel.vue`.
- [ ] Larger lazy candidate-generation or top-k refinements, but only if they stay small and benchmark-backed.
- [ ] Compact score-cache JSON write-path work, but only if benchmarked and safe.

## Final Acceptance

- [x] Required backend correctness items are complete.
- [x] Required replay/artifact determinism items are complete.
- [x] Warm-path improvements stayed bounded and did not introduce ad-hoc globals.
- [x] Direct `MlSignal` benchmark coverage exists.
- [x] Research-harness docs and outputs are explicit about score contract and proxy-only parity.
- [x] Preserve-only guardrails were respected.
- [x] Focused Rust / TypeScript verification was run, or any blocker is explicitly recorded.
- [x] Python syntax/compile verification was run if research files changed, or any blocker is explicitly recorded.

## Verification Log

- [x] `npm --prefix web test -- src/services/mlSignalService.test.ts src/services/historyService.test.ts`
- [x] `npm --prefix web run typecheck`
- [x] `$env:RUSTC_WRAPPER=''; cargo test -p ta_app_core ml_signal --lib -- --nocapture`
- [x] `$env:RUSTC_WRAPPER=''; cargo check -q -p ta_desktop_demo_app`
- [x] `$env:RUSTC_WRAPPER=''; cargo bench -p ta_app_core --bench ml_signal_workflow --no-run`
- [x] `python -m py_compile research/ml_signal/train_catboost.py research/ml_signal/calibrate.py research/ml_signal/evaluate_walkforward.py research/ml_signal/gate_report.py`
