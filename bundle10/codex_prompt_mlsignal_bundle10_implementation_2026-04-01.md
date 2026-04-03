# Codex implementation prompt — MlSignal bundle 10 hardening, replay correctness, and warm-path performance

## Role and mission

You are an OpenAI Codex coding agent working inside this repo.

Your job is to **implement** the `MlSignal` bundle-10 hardening plan end-to-end, not just restate it. The goal is to improve:

- **correctness**,
- **replay determinism**,
- **warm-path performance**,
- **artifact fidelity**, and
- **research-harness contract clarity**,

while **preserving current features and behavior wherever possible**.

This pass is specifically about the current `MlSignal` workflow:

- dataset export,
- score-cache build/alignment,
- score-cache-backed validation,
- policy sweep,
- artifact persistence,
- saved-run load/apply/rerun,
- research-bundle contract (`train_catboost.py`, `calibrate.py`, `evaluate_walkforward.py`, `gate_report.py`),
- and direct `MlSignal` benchmark coverage.

Do **not** stop at high-level analysis. Make the code changes, add/update tests, and update docs.

---

## Read these docs first

Read these documents before editing code. For this prompt, the older plan/addendum are background context, not the final authority.

### Primary repo docs
1. `bundle10/mlsignal_bundle10_full_plan_2026-04-01.md`
2. `bundle10/mlsignal_bundle10_plan_addendum_2026-04-01.md`
3. `gpt54_feature_review_map.md` — especially **section `## 10. ML signal dataset export, score-cache validation, and replay flows`**
4. `research/ML_SIGNAL_STRATEGY_IMPLEMENTATION_PLAN_2026-03-18.md`
5. `Frontend Review/Unfinished/mlsignal_mode_separation_plan_2026-03-31.md`

### Precedence order when docs conflict
Use this precedence order:

1. the repo-state validation findings and authoritative checklist in this prompt
2. current code realities in the repo
3. `gpt54_feature_review_map.md` bundle-10 scope/guardrails
4. `bundle10/mlsignal_bundle10_plan_addendum_2026-04-01.md`
5. `bundle10/mlsignal_bundle10_full_plan_2026-04-01.md`
6. `research/ML_SIGNAL_STRATEGY_IMPLEMENTATION_PLAN_2026-03-18.md`
7. `Frontend Review/Unfinished/mlsignal_mode_separation_plan_2026-03-31.md` (**deferred context only**)

### Progress tracker
Use `bundle10/bundle10_mlsignal_workflow_validated_tracking_checklist.md` as the authoritative progress tracker for this bundle.

---

## Repo-state validation findings (2026-04-01)

The Bundle 10 plan and addendum are only partially valid for the current repo. Use the checklist below as the implementation authority.

### Already present; preserve and do not re-implement
- [x] `MlSignal` already has dedicated dataset export, score-cache build, validation, and policy-sweep backends in `crates/app_core/src/ml_signal.rs` and `src-tauri/src/commands_ml_signal.rs`.
- [x] `workflowArtifactsStore.ts` already normalizes and preserves both `dataset_meta` / `datasetMeta` and `ml_signal` / `mlSignal` for newly persisted artifacts.
- [x] ML validation and sweep already send `validation_context`, and `commands_ml_signal.rs` already validates that context.
- [x] `web/src/services/mlSignalService.test.ts` and `web/src/services/historyService.test.ts` already exist. Extend them instead of creating brand-new test surfaces.
- [x] The research harness already copies `feature_schema.json`, `export_manifest.json`, and `split_manifest.json` into bundle outputs, and `calibrate.py` already writes `p_long_cal` / `p_short_cal`.
- [x] Dataset export already writes `export_manifest_path` and `split_manifest_path`, and dataset export already precomputes split membership.
- [x] Candidate-specific `track_record_bars` already exists in `crates/app_core/src/ml_signal.rs`. Preserve it unless a directly related bug requires a small correction.
- [x] Keep the current `strategyKind === "MlSignal"` bridge and advisory model-status behavior for this pass.
- [x] Keep training offline in `research/ml_signal/`, and keep the top-level mode split deferred.

### Stale or demoted items from the older plan
- [x] Do not treat `loadSavedRun()` as missing all ML artifact preservation. The remaining gap is the dropped `ml_signal` block; `dataset_meta` is already carried through.
- [x] Do not require a brand-new `historyService.test.ts`; focused ML replay coverage belongs in the existing file.
- [x] Do not require build-request `validation_context` or other score-cache request-shape churn unless a concrete implementation proves it necessary. The current build request struct does not carry that field.
- [x] Do not treat `crates/app_core/benches/e2e_workflow_ml.rs` as direct `MlSignal` coverage. Keep it as background context and add a real MlSignal benchmark surface separately if benchmark work is implemented.
- [x] Do not turn shell-drift notes into a full mode-separation refactor.
- [x] Do not treat `research/ml_signal/common.py` as required parity work. Its current role is to consume exported feature columns, not to reimplement indicator math.

## Authoritative validated checklist

If anything below conflicts with later detailed sections or with the older plan/addendum, follow this checklist and the current code.

### Required Bundle 10 implementation scope
- [ ] Support calibrated score-column ingestion by preferring `p_long_cal` / `p_short_cal`, then `p_long` / `p_short`, and persist explicit `score_space`.
- [ ] Reject raw-only score caches for probability-threshold validation. Keep raw columns diagnostic-only unless a future raw-margin mode is intentionally added.
- [ ] Support long-only validation with one-sided long scores, and fail explicitly when two-sided validation is missing a side.
- [ ] Implement backend semantics for frontend-serialized `allow_flip`, `cooldown_bars`, and `signal_persistence_bars`.
- [ ] Count and persist `duplicate_timestamps`, `coverage_ratio`, and `score_scope` across the score-cache file, manifest, artifact payload, and user-visible ML metadata.
- [ ] Persist dataset export as an artifact-backed ML stage instead of leaving it as UI-only status text.
- [ ] Carry dataset/export/split/schema metadata forward into later ML artifacts, and extend the `ml_signal` artifact block with explicit ML-stage identity so replay does not rely only on request-shape heuristics.
- [ ] Make `loadSavedRun()` preserve the `ml_signal` block alongside the already-preserved `dataset_meta`.
- [ ] Restore saved ML validation/sweep requests into store state and make `rerunSavedRun()` rerun the full ML flow, not only score-cache generation.
- [ ] Improve warm-path performance with repo-local, low-risk wins only: prefix-sum/rolling export math, reusable export row buffers, candidate-buffer reuse, and parsed score-cache reuse or equivalent memoization.
- [ ] Only add loaded-dataset warm caches such as timestamp-index maps or log returns if the implementation stays small and reuses existing app-state / loaded-data patterns.
- [ ] Add direct `MlSignal` benchmark coverage. The current `e2e_workflow_ml.rs` file alone is not sufficient evidence for this workflow.
- [ ] Update research harness docs and outputs so calibrated-column expectations, score scope/coverage where practical, and proxy-only parity limitations are explicit.
- [ ] Add or update focused Rust, TypeScript, and lightweight Python verification around the touched behavior.

### Optional follow-up only
- [ ] Sweep-size preflight or button disablement for oversized ML policy sweeps.
- [ ] Debounced or explicit-trigger model-status refresh in `MlSignalWorkspacePanel.vue`.
- [ ] Larger lazy candidate-generation or top-k refinements only if they stay small and benchmark-backed.

---

## Hard constraints / non-goals

These are non-negotiable unless the source-of-truth docs explicitly require otherwise.

### Keep these constraints
- Keep `MlSignal` as the current special `strategyKind` bridge **for this pass**.
- Preserve current user-visible functionality unless you are fixing a correctness bug.
- Prefer explicit errors over silent fallbacks.
- Preserve CPU-only build health.
- Keep training offline in `research/ml_signal/`.
- Keep native Rust model scoring out of scope unless there is already a small, clearly required change for contract fidelity.
- Preserve backward compatibility for old saved artifacts where practical:
  - normalizers should accept both old and new shapes,
  - support both `snake_case` and `camelCase` artifact fields where this repo already follows that pattern.
- If you need additional derived-data caching, use repo-appropriate app state / loaded-data context patterns instead of introducing ad-hoc globals.
- Extend existing tests and artifact/state paths before inventing new ones.

### Do not implement in this pass
- full top-level `MlSignal` shell mode split
- moving offline Python training into Tauri
- replacing score-cache validation with a new trade engine
- large UI redesign beyond clarity/preflight/status improvements
- binary/columnar score-cache format as the primary persistence format
- speculative ML-platform expansion unrelated to this workflow
- raw-margin policy mode unless the docs explicitly require it (they currently do not)

### If you touch the deferred mode-separation doc
Only update its **entry criteria / context notes**. Do **not** implement the top-level mode split itself in code during this pass.

---

## Primary repo files to inspect and likely modify

These are the main files that should be considered part of the implementation scope.

### Backend / Tauri
- `crates/app_core/src/ml_signal.rs`
- `src-tauri/src/commands_ml_signal.rs`

### Frontend workflow / replay / UI
- `web/src/services/mlSignalService.ts`
- `web/src/services/mlSignalService.test.ts`
- `web/src/services/historyService.ts`
- `web/src/services/historyService.test.ts`
- `web/src/stores/workflowArtifactsStore.ts`
- `web/src/components/MlSignalWorkspacePanel.vue`

### Read-only compatibility surfaces unless the validated checklist requires a small change
- `web/src/stores/optimizeStore.ts`
- `web/src/components/OptimizeWorkspacePanel.vue`
- `Frontend Review/Unfinished/mlsignal_mode_separation_plan_2026-03-31.md`

### Benchmarking
- `crates/app_core/benches/e2e_workflow_ml.rs` (background only; not direct MlSignal coverage)
- a new dedicated `MlSignal` bench surface if benchmark work is implemented

### Research harness
- `research/ml_signal/train_catboost.py`
- `research/ml_signal/calibrate.py`
- `research/ml_signal/evaluate_walkforward.py`
- `research/ml_signal/gate_report.py`
- `research/ml_signal/README.md`

### Repo docs to update for truthfulness / future readers
- `research/ML_SIGNAL_STRATEGY_IMPLEMENTATION_PLAN_2026-03-18.md`
- `Frontend Review/Unfinished/mlsignal_mode_separation_plan_2026-03-31.md` (deferred/context notes only if touched)

---

## Adjacent repo files you may inspect and modify only if necessary

Touch these only when the listed files clearly require it:

- `src-tauri/src/state.rs`
- `src-tauri/src/data_cache.rs`
- any loaded-data-context / app-state files needed to cache:
  - `timestamp -> candle index`
  - `log_returns`
  - parsed score-cache files
- `research/ml_signal/common.py` only if a concrete touched-file contract issue requires a small follow-up; do not use it as a generic parity-work surface
- `web/src/services/historyService.test.ts` if it exists, or create a focused test file if needed
- any generated Tauri command bindings **only via the repo’s generation flow**, never manual edits, if the repo requires regeneration after request/response type changes

If the repo uses slightly different paths than above, locate equivalent files by basename and function names.

---

## Required implementation order

Follow this order unless the repo structure forces a small deviation.

### Phase P0 — correctness and contract fidelity
1. Fix calibrated-column ingestion, explicit score-space handling, and raw-only rejection.
2. Fix one-sided long-only behavior vs explicit two-sided requirements.
3. Implement missing backend strategy semantics (`allow_flip`, `cooldown_bars`, `signal_persistence_bars`).
4. Add truthful score coverage / score scope / duplicate metadata.

### Phase P1 — replay and artifact determinism
5. Persist dataset export as an artifact-backed ML stage.
6. Extend `ml_signal` artifact metadata so ML stage identity and dataset/export/split/schema provenance survive later stages.
7. Make `loadSavedRun()` preserve `ml_signal` and make ML artifact restoration stop relying only on request-shape heuristics.
8. Restore saved validation/sweep requests into store state and rerun the full ML flow from saved artifacts.

### Phase P2 — measured warm-path wins
9. Optimize exporter rolling math and row-buffer reuse.
10. Reduce validation/sweep allocation churn and parsed score-cache reload overhead.
11. Only add loaded-dataset warm caches if they stay small and reuse existing app-state / loaded-data patterns.

### Phase P3 — benchmark and research-harness truthfulness
12. Add direct `MlSignal` benchmark coverage.
13. Update research harness outputs/docs for score contract clarity and proxy-only parity honesty.
14. Update the repo docs that describe the shipped MlSignal workflow.

### Optional follow-up only
15. Sweep-size preflight and other UI refinements are optional. Do not let them block the required scope.

---

## Scope lock

Later file-by-file sections in this prompt are background guidance only.

If a later section asks for work that is not in the authoritative validated checklist above, treat it as:
- preserve-only context,
- optional follow-up,
- or stale plan text to ignore.

## Detailed implementation requirements by file

# 1. `crates/app_core/src/ml_signal.rs`

This file is the center of gravity for correctness and hot-path work.

## 1.1 Score-space and score-scope contract
Implement explicit score-space metadata.

### Required additions
Add explicit enums and carry them through the score-cache path:

- `MlSignalScoreSpace`
  - `Probability`
  - `RawMargin`
- `MlSignalScoreScope`
  - `FullDataset`
  - `CalibrationSlice`
  - `WalkforwardTest`
  - `CustomPartial`

Add or refactor supporting structs as needed, for example:
- parsed score-CSV summary struct
- loaded score-cache struct carrying both manifest metadata and aligned points

### Required behavior
- Prefer calibrated probability columns first:
  - `p_long_cal`
  - `p_short_cal`
- Then fallback to probability columns:
  - `p_long`
  - `p_short`
- Raw margins (`raw_long`, `raw_short`) may still be parsed and persisted for diagnostics, but **do not** silently drive probability-threshold validation.
- If validation is still probability-threshold based, raw-only score caches must fail explicitly.
- If score scope cannot be inferred reliably from current data, store `CustomPartial` instead of guessing.

### Implement this in or around
- `find_header_index` (replace with deterministic preferred-order behavior)
- `read_score_rows_csv`
- score-cache file structs
- score-cache manifest structs
- `load_score_cache`

## 1.2 One-sided long-only correctness
Fix the current mismatch between accepted input shape and actual policy behavior.

### Required behavior
- `long_only = true` should allow long-only score data (`p_long` present, `p_short` absent).
- `long_only = false` must require both long and short score streams.
- Missing required sides should surface as explicit errors, not silent flat output.

### Implement this in or around
- `score_target`
- any validation prechecks or cache-load validation that are better places for explicit failures

## 1.3 Duplicate timestamp accounting and score coverage
The current “last row wins” behavior is too implicit.

### Required behavior
- Count duplicate timestamps explicitly.
- Persist duplicate counts into the result and manifest.
- Persist and surface:
  - `matched_rows`
  - `skipped_rows`
  - `bars_scored`
  - `coverage_ratio`
  - `score_scope`
  - `duplicate_timestamps`
  - `first_timestamp`
  - `last_timestamp`
- If alignment input is unsorted or duplicated, make that observable.

### Acceptable implementation choices
- You may keep a map-based fallback path.
- If straightforward, add a sorted-CSV fast path using two-pointer alignment against candle timestamps.

## 1.4 Remove unsafe raw fallback
Do **not** keep the current `point.p_long.or(point.raw_long)` / `point.p_short.or(point.raw_short)` style policy mapping.

### Required behavior
- Probability validation mode should use probability columns only.
- Raw margins may still be kept in cache/artifact payloads for diagnostics.
- If raw-only data is encountered, fail explicitly in a tested, user-visible way.

## 1.5 Policy semantics parity with serialized frontend config
Implement the missing strategy semantics already serialized by the frontend.

### Required semantics
In addition to current behavior, the desired-position state machine must honor:
- `allow_flip`
- `cooldown_bars`
- `signal_persistence_bars`

### Required state-machine behavior
Preserve existing semantics for:
- `trade_on_next_bar`
- `long_only`
- `max_hold_bars`
- `exit_on_flat`
- `exit_on_opposite`

### Expected interpretation
Use the plan’s parity-safe approach:
- `signal_persistence_bars` should require a target to persist for N consecutive bars before it is actionable.
- `cooldown_bars` should suppress re-entry for a number of bars after flattening due to exit/hold expiration/non-flip opposite handling.
- `allow_flip` should determine whether opposite persisted signals immediately reverse or flatten first.

### Implement this in or around
- `build_desired_positions`
- preferably refactor to a `build_desired_positions_into(...)` buffer-reuse form

## 1.6 Track-record length and candidate activity
Do not use “number of rows with any score” as candidate track-record length.

### Required behavior
- `track_record_bars` should be candidate-specific.
- Base it on actual active non-flat bars (or a stricter candidate-specific proxy), not just raw score availability.

### Implement this in or around
- `validate_ml_signal_candidates`

## 1.7 Export hot-path optimization
This is a required performance pass.

### Required optimization work
Replace repeated rescans with rolling/prefix helpers for:
- `log_ret_2`
- `log_ret_3`
- `log_ret_5`
- `log_ret_h`
- `realized_vol_h`
- `realized_vol_3h`

Use precomputed:
- return prefix sums
- squared-return prefix sums or equivalent rolling-stat helpers

### Also reduce exporter overhead
- precompute `split_id_by_row` instead of `split_windows.iter().find(...)` per row
- reuse the per-row CSV buffer instead of allocating a fresh `Vec<String>` each loop
- avoid unnecessary string churn where practical

### Keep behavior stable
- no schema reordering
- no feature removals
- no label alignment changes unless a bug is discovered and fixed with tests

## 1.8 Validation/sweep allocation reuse
This is a required performance pass.

### Required optimization work
Refactor per-candidate requested/desired vector allocation so they are reused across candidates.

Suggested shape:
- `build_desired_positions_into(score_points, params, requested, desired)`

### Keep behavior stable
Do not change the validation meaning; reduce allocation churn only.

## 1.9 Score-cache persistence cost
The large score-cache JSON should be cheaper to write/read without changing the format family.

### Required behavior
- It is acceptable to write the big score-cache JSON in compact form.
- The separate manifest may remain pretty-printed.
- Do not introduce a new primary binary format in this pass.

## 1.10 Backend tests to add/update
Add or extend Rust tests in the existing test module or repo-appropriate location.

### Required backend tests
- calibrated columns are preferred over uncalibrated ones
- raw-only score input fails explicitly
- long-only validation accepts one-sided long score input
- two-sided validation rejects missing short side
- duplicate timestamps are counted and surfaced
- `track_record_bars` differs across candidates with different active exposure
- policy logic honors `allow_flip`, `cooldown_bars`, `signal_persistence_bars`
- export prefix-math path is parity-safe vs previous logic or known-good helpers
- coverage/scope fields are populated sensibly

---

# 2. `src-tauri/src/commands_ml_signal.rs`

## 2.1 Strengthen score-cache build context validation
Validation and sweep already carry validation context; build should too.

### Required changes
Extend the build request to carry optional validation context (or equivalent loaded-dataset identity context).

Use that context to validate the currently loaded dataset/timeframe before building the score cache, similar to validation/sweep.

### Required behavior
- score-cache build should validate that the cache is being built against the intended loaded dataset context
- model-status command stays advisory/metadata-only for this pass
- do not add new training commands

## 2.2 If warm dataset caching needs Tauri/state support
If implementing warm parsed-cache / timestamp-index / log-return caching requires changes in `state.rs` or related app-state files, make the smallest clean change that fits the repo’s existing state model.

---

# 3. `web/src/services/mlSignalService.ts`

This file must become the canonical frontend orchestrator for the ML workflow stages.

## 3.1 Add ML artifact action kinds
Add explicit action kinds in persisted artifact metadata such as:
- `dataset_export`
- `score_cache_build`
- `validation`
- `policy_sweep`

### Required behavior
- action kind belongs in artifact metadata / persisted run request context
- do not add spurious backend request fields unless backend structs actually accept them

## 3.2 Persist the dataset export stage
Dataset export currently only updates UI text. It must persist a run artifact.

### Required behavior
- `exportMlSignalDataset()` should persist a run artifact
- `currentRun.mlSignal` should carry dataset/export manifest/schema metadata immediately after export
- later stages must merge forward prior ML metadata instead of overwriting it

## 3.3 Merge forward prior ML artifact metadata
When the user moves from dataset export -> score cache build -> validation -> sweep, the ML artifact block must retain earlier stage data.

### Required fields to carry forward where available
- `action_kind`
- `dataset_path`
- `export_manifest_path`
- `split_manifest_path`
- `bundle_id`
- `feature_schema_path`
- `feature_schema_version`
- `score_cache_path`
- `score_manifest_path`
- `score_space`
- `score_scope`
- `coverage_ratio`
- `duplicate_timestamps`
- `policy`
- `policy_sweep`

## 3.4 Score-cache build request cleanup
### Required behavior
- remove extra unknown fields from backend invoke payloads if current structs do not use them
- pass validation/build context consistently where backend now expects it
- keep artifact metadata richer than backend payload if needed

## 3.5 Sweep preflight
Add frontend sweep-size estimation and block obviously oversized sweeps before invoking the backend.

### Required behavior
- estimate Cartesian-product sweep size from current axes
- if estimated size exceeds the backend cap (20,000 in the current plan/code), block the action and show a clear status/meta message
- surface the estimated size in the panel

## 3.6 UI metadata quality
Update store-facing status/meta text so the user can see:
- score coverage ratio
- score scope
- duplicate timestamps (if any)
- dataset export schema/manifest identity
- more honest sweep status

## 3.7 Tests to add/update
Extend `mlSignalService.test.ts` to cover:
- dataset export artifact persistence
- richer artifact payload merging across stages
- score-cache build request shape after cleanup
- sweep preflight blocking when estimate exceeds cap
- metadata/status strings for coverage/scope/duplicates where applicable

If existing service tests are insufficient, add narrowly scoped new tests rather than weakening coverage.

---

# 4. `web/src/stores/workflowArtifactsStore.ts`

## 4.1 Extend the ML artifact interface
Extend the normalized `WorkflowMlSignalArtifact` shape to support the richer workflow metadata.

### Required normalized fields
At minimum include:
- `actionKind`
- `datasetPath`
- `exportManifestPath`
- `splitManifestPath`
- `bundleId`
- `scoreCachePath`
- `scoreManifestPath`
- `scoreSpace`
- `scoreScope`
- `coverageRatio`
- `duplicateTimestamps`
- `featureSchemaPath`
- `featureSchemaVersion`
- `policy`
- `policySweep`

### Required behavior
- normalizer must accept both snake_case and camelCase
- old artifacts without these fields must still load
- keep `markRaw` / current store conventions consistent with existing code style

---

# 5. `web/src/services/historyService.ts`

This file is critical for replay correctness.

## 5.1 Preserve ML artifact blocks when loading saved runs
### Required behavior
When loading saved run artifacts into runtime state, do not drop:
- `dataset_meta`
- `ml_signal`

The current runtime payload should preserve these blocks if they were stored.

## 5.2 Restore ML validation and sweep request state
Add helpers that restore saved ML validation requests into `optimizeStore`.

### Required behavior
- for single validation requests, restore thresholds, hold settings, exit flags, PSR/DSR gates, and score-cache path
- for sweep requests, restore sweep axes, top-k, PSR/DSR gates, and score-cache path
- set `strategyKind = "MlSignal"` when applying/loading/rerunning ML artifacts so the current bridge panel is active

## 5.3 Use explicit ML action kind where available
Stop relying only on shape heuristics to infer whether a saved ML run was dataset export vs score-cache build vs validation/sweep. Use `ml_signal.action_kind` first when present.

## 5.4 Rerun the full saved ML artifact pipeline
This is required.

### Required behavior
For `MlSignal` artifacts:
- load dataset from saved artifact
- resample/apply timeframe if needed
- restore run request state
- restore validation/sweep request state if present
- rebuild score cache by default for deterministic reruns
- then rerun validation or policy sweep if the saved artifact included them

Do **not** stop after regenerating the score cache.

## 5.5 Apply/load behavior
When applying or loading an ML artifact:
- preserve current ML artifact metadata
- keep analysis/status behavior honest (this is not optimizer-native analysis)
- do not implement the separate top-level mode yet

## 5.6 Tests to add/update
If there is an existing `historyService` test file, extend it. Otherwise add focused tests.

### Required coverage
- `loadSavedRun()` preserves `dataset_meta` and `ml_signal`
- ML saved-run apply/load sets `strategyKind = "MlSignal"`
- rerun of a saved validation artifact rebuilds the cache and reruns validation
- rerun of a saved sweep artifact rebuilds the cache and reruns the sweep
- request restoration uses `action_kind` when present rather than only shape heuristics

---

# 6. `web/src/stores/optimizeStore.ts`

Keep the current bridge, but make it explicit and less brittle.

## Required changes
- centralize `isMlSignalStrategyKind(...)` helper instead of repeating string-literal checks
- keep `runOptimization()` routing to score-cache generation for `MlSignal`
- keep `runValidation()` routing to ML validation for `MlSignal`
- improve status-line wording so the bridge is honest
- do **not** grow this file into a pseudo-dedicated ML store during this pass

---

# 7. `web/src/components/OptimizeWorkspacePanel.vue`

Keep this as a bridge, not a redesign surface.

## Required or recommended minimal change
- keep the current special-case `MlSignal` mount functional
- add a small bridge note / copy improvement so the panel no longer implies `MlSignal` is a normal optimizer route
- do not attempt the full mode split here

If the repo already has equivalent copy/structure, keep the change minimal.

---

# 8. `web/src/components/MlSignalWorkspacePanel.vue`

## 8.1 Add sweep-size visibility and gating
### Required behavior
- compute estimated sweep size from current axes
- show it in the UI
- disable/block sweep launch when estimate exceeds backend cap

## 8.2 Surface richer artifact metadata
Once the service/store layers expose it, show:
- dataset export manifest
- split manifest
- score space
- score scope
- coverage ratio
- duplicate timestamp count
- score cache / feature schema identity

## 8.3 Copy cleanup
Slightly revise copy so it is clear that:
- this is an `MlSignal` workflow
- it is mounted inside `Optimize` as a compatibility bridge
- it is not a conventional optimizer search path

## 8.4 Optional low-risk refinement
If model-status refresh currently fires on every keystroke when editing bundle id, debounce it or move it to blur/button-trigger unless selection comes from a discrete chooser.

---

# 9. `crates/app_core/benches/e2e_workflow_ml.rs`

The current filename is misleading if it only benchmarks Stage 5 / meta ML-confidence.

## Required benchmark work
Add direct `MlSignal` Criterion coverage for at least these surfaces:
- `ml_signal/export_dataset`
- `ml_signal/build_score_cache`
- `ml_signal/validate_cold_cache`
- `ml_signal/validate_warm_cache`
- `ml_signal/policy_sweep_small`
- `ml_signal/policy_sweep_large`

## Acceptable implementation choices
- keep the current file and add a second benchmark group for `MlSignal`, or
- split into a clearer dedicated bench file if that fits the repo better

## Required outcome
There must be a direct benchmark surface for the actual `MlSignal` workflow, not just meta/Stage-5 ML-confidence.

---

# 10. Research harness files

The Python flow is still part of the shipped workflow contract. Update it for clarity and compatibility.

## 10.1 `research/ml_signal/train_catboost.py`
### Required changes
Add clearer bundle manifest metadata describing the score contract, for example:
- preferred probability columns
- raw margin columns
- whether desktop validation expects a probability pair
- whether emitted `validation_scores.csv` is calibration-slice scoped

If you can safely add explicit metadata indicating that `validation_scores.csv` is calibration-only / not full-dataset, do so.

## 10.2 `research/ml_signal/calibrate.py`
### Required changes
Update the bundle manifest more fully so calibration is explicit, including:
- `calibration_path`
- `calibration_applied`
- clearer score-contract metadata

## 10.3 `research/ml_signal/evaluate_walkforward.py`
### Required changes
Do not oversell app-policy parity.

Add explicit summary fields indicating that this path is a proxy if it does not reproduce the desktop policy state machine / iterative simulator semantics.

At minimum, make it clear that current gaps include things like:
- no `max_hold_bars` state machine parity
- no `exit_on_flat` / `exit_on_opposite` policy parity
- no cooldown/persistence state handling parity
- no iterative stop/take-profit parity

If there is a clean, low-risk way to expose explicit score-scope metadata here, do it.

### Optional low-risk extension
If it is simple and does not create churn, you may add an explicit full-dataset scoring output or flag for the app-side workflow. This is optional, not required.

## 10.4 `research/ml_signal/gate_report.py`
### Required changes
Make the research gate explicitly state that it is a research gate, not a replacement for desktop score-cache validation.

Carry through proxy/parity notes from walk-forward summary data when available.

## 10.5 `research/ml_signal/README.md`
### Required documentation additions
Add or update sections covering:
- score-column contract
- preferred probability columns order
- raw-only caches unsupported for desktop probability validation unless a future explicit raw-margin mode is added
- walk-forward proxy / non-parity note

## 10.6 `research/ml_signal/common.py` if present
Inspect it for feature/parity concerns if the repo includes it.

### Required behavior if touched
- do not casually change feature math
- only update if needed to keep parity/documentation truthful
- especially verify realized-vol / feature-frame assumptions if they differ from current Rust exporter behavior

---

# 11. Repo docs to update

## 11.1 `ML_SIGNAL_STRATEGY_IMPLEMENTATION_PLAN_2026-03-18.md`
Update the doc so it matches the newly implemented reality.

### Required doc truthfulness updates
Reflect that:
- score-cache-backed validation is the primary in-app path today
- probability-pair validation is the supported v1 contract
- replay/artifact identity requirements are now stronger
- direct `MlSignal` benchmarks exist (after you add them)
- offline training remains out of the app

## 11.2 `mlsignal_mode_separation_plan_2026-03-31.md`
Do **not** implement the mode split.

Only update this doc if needed to clarify:
- the split remains deferred
- replay/artifact correctness is now a prerequisite / entry criterion
- benchmark and artifact fidelity work was completed first

Keep the doc aligned with current implementation reality.

---

## Additional implementation guidance

### Prefer parity-safe small changes over broad rewrites
If there are two ways to get the win:
- choose the smaller change that preserves behavior and is easy to test
- do not redesign the architecture for elegance if a contained fix exists

### Prefer explicit failure over silent fallback
This especially applies to:
- score-space mismatch
- missing required score sides
- stale dataset/context mismatch
- unsupported raw-only validation inputs

### Preserve backward compatibility for saved artifacts
- keep old artifacts loadable where practical
- add richer fields without breaking old loaders
- normalize both old and new field naming styles where the store already uses that pattern

### Keep offline Python adjacent but out of the hot path
Update its contract and docs, but do not fold training into the desktop runtime.

---

## Verification and test requirements

Run the strongest realistic verification available in the repo.

### Rust / backend
At minimum:
- format modified Rust files
- run targeted Rust tests for `ml_signal`
- run or compile the updated benchmark target(s)

Examples if supported by repo tooling:
- `cargo fmt`
- `cargo test -p ta_app_core ml_signal -- --nocapture`
- `cargo bench -p ta_app_core --bench e2e_workflow_ml --no-run`

If the repo uses different crate names/bench commands, adapt to the actual workspace.

### Frontend
At minimum:
- run targeted Vitest coverage for modified service/store/history logic
- ensure TypeScript/Vue code typechecks/builds if repo tooling permits

Examples if supported:
- targeted `vitest` for `mlSignalService.test.ts`
- targeted/new tests for history-service ML replay behavior
- repo typecheck/build commands

### Python research harness
At minimum:
- ensure modified scripts parse/compile
- if full environment is unavailable, run lightweight compile checks or equivalent

Examples:
- `python -m py_compile research/ml_signal/*.py`
- or repo-equivalent Python verification

### If some verification cannot run
Do not hide it. Report exactly what you could not run and why.

---

## Completion checklist

This final acceptance checklist must stay aligned with the authoritative validated checklist above.

### Required scope
- [ ] calibrated columns, explicit score space, and raw-only rejection landed
- [ ] long-only one-sided support and explicit two-sided failure landed
- [ ] `allow_flip`, `cooldown_bars`, and `signal_persistence_bars` are honored by the backend
- [ ] duplicate timestamps, coverage ratio, and score scope landed
- [ ] dataset export now persists as an ML artifact stage
- [ ] later ML artifacts carry forward dataset/export/split/schema metadata
- [ ] `loadSavedRun()` preserves `ml_signal` alongside the already-preserved `dataset_meta`
- [ ] saved validation/sweep requests restore into store state and `rerunSavedRun()` reruns the full ML flow
- [ ] warm-path wins are limited to small, repo-local improvements and do not introduce ad-hoc globals
- [ ] direct `MlSignal` benchmark coverage exists
- [ ] research harness docs and outputs are honest about calibrated-column expectations and proxy-only parity

### Preserve-only guardrails respected
- [ ] the current `strategyKind === "MlSignal"` bridge remains intact
- [ ] offline Python training remains outside Tauri
- [ ] no top-level mode split was implemented
- [ ] no optimizer-native `MlSignal` heatmap / Top / Pareto redesign was introduced

### Verification
- [ ] focused Rust tests added or updated
- [ ] focused TypeScript tests added or updated
- [ ] existing ML history/replay tests extended instead of replaced
- [ ] lightweight Python syntax or compile verification performed, or the blocker is reported explicitly

---

## Final response format

When you finish, respond with:

1. a concise summary of what changed, grouped into:
   - backend
   - frontend
   - benchmarks
   - research harness/docs
2. a list of the exact files you modified
3. a list of tests/verification commands you ran and the outcomes
4. any intentionally deferred items that remain out of scope for this pass
5. any assumptions you had to make because a file or dependency was missing

Do **not** respond with only a plan. Implement the code.
