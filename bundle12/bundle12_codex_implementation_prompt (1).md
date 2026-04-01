# Codex Implementation Prompt — Bundle 12 Meta-selection and Stage 5 ML confidence throughput

## Usage
Use this document as the prompt for an OpenAI Codex agent that has access to the repository and the implementation-guide markdown files.

When possible, also make these guide files available to Codex as reference documents:
- `bundle12_meta_stage5_throughput_plan_v2.md` — primary implementation guide
- `bundle12_meta_stage5_throughput_addendum.md` — required safe additions and open-question guidance
- `bundle12_meta_stage5_throughput_plan.md` — older detailed guide; use only if it adds detail not already present in v2
- `gpt54_feature_review_map.md` — use the Bundle 12 scope guard and posture, especially `## 12. Meta-selection and Stage 5 ML confidence throughput`

The prompt below is intentionally self-contained, but the Codex agent should treat the listed markdown guides as authoritative design documents when they are available.

---

# Prompt to give Codex

## Role and mission
You are implementing a repo-grounded optimization/performance patch series for **Bundle 12: Meta-selection and Stage 5 ML confidence throughput**.

Your task is to **fully implement the required plan** from:
- `bundle12_meta_stage5_throughput_plan_v2.md` (primary)
- `bundle12_meta_stage5_throughput_addendum.md` (required safe additions)
- `gpt54_feature_review_map.md` → Bundle 12 scope guard

This is an **implementation task**, not a review task. Make the code changes, update tests/benchmarks, run the relevant validation commands, and leave the repo in a compileable, testable state.

## Authoritative scope
Stay inside the Bundle 12 scope:
- artifact scan/load
- grouping
- holdout evaluation
- Stage 4 / Stage 5 model-status, train, apply, and policy-evaluation throughput
- saved-artifact hydration around the meta pipeline
- benchmark and perf-gate coverage for this surface

## Hard constraints
1. Be **performance-first, optimization-first, benchmark-first, and parity-preserving**.
2. Do **not** add new product features, new workflow panels, or broad UX redesign.
3. Do **not** change ML semantics unless explicitly required by the plan.
4. Do **not** remove features or break compatibility aliases such as Stage 4 / Stage 5 naming that are already used in the codebase.
5. Prefer **small, safe, measurable** wins before deeper refactors.
6. Preserve correctness, current routing, current selection behavior, and existing saved-artifact workflows.
7. Keep the code changes tightly localized to the targeted files unless an adjacent test/helper must be updated.
8. Treat the **mixed-history `Auto` target behavior** in `ml_confidence.rs` as an **open correctness question**. For this pass, **do not change its runtime semantics**. Add tests/comments documenting the current behavior instead.
9. Optional items that go beyond the narrow prompt should only be implemented **after** the required work passes tests and remains parity-safe.

## Primary implementation guides and precedence
If there is any ambiguity, follow this precedence order:
1. `bundle12_meta_stage5_throughput_plan_v2.md`
2. `bundle12_meta_stage5_throughput_addendum.md`
3. `bundle12_meta_stage5_throughput_plan.md`
4. `gpt54_feature_review_map.md` Bundle 12 scope guard

The addendum does **not** replace v2. It adds safe wins that must be folded into the implementation.

---

## Files you must inspect and are expected to modify

### Required code files
- `crates/app_core/src/meta_select.rs`
- `crates/app_core/src/ml_confidence.rs`
- `src-tauri/src/commands_meta.rs`
- `web/src/services/metaService.ts`
- `web/src/services/metaRequestBuilder.ts`
- `web/src/services/metaStage4RequestBuilder.ts`

### Required benchmark / workflow files
- `crates/app_core/benches/meta_select_workflow.rs`
- `crates/app_core/benches/e2e_workflow_ml.rs`
- `src-tauri/src/bin/ta_workflow_regression.rs`

### Adjacent tests to update or add if they exist
- inline `#[cfg(test)]` sections in the Rust files above
- nearby service tests for `metaService.ts` and builder files, if present in the repo

If a dedicated test file does not exist, add focused tests in the touched file or the nearest existing test module.

---

## Required implementation outcome
Implement **Phases 1–4** from `bundle12_meta_stage5_throughput_plan_v2.md`, plus the **safe additions A–E** from the addendum.

That means the required outcome is:
1. safe app-path wins
2. single-pass row/scope builders
3. benchmark and regression-gate expansion
4. prepared-row / prefix policy-evaluation refactor
5. safe addendum items around early prefiltering, surfaced scan errors, duplicate full-model load removal, Stage 5 status synchronization, and optimistic saved-list upserts

Optional stretch items may be implemented **only after** the required work is complete and validated.

---

## Required changes by file and function

# 1. `src-tauri/src/commands_meta.rs`

This is the highest-priority file. Focus on these functions and payload types:
- `run_meta_selection_blocking`
- `load_meta_artifacts_with_paths_blocking`
- `load_meta_artifacts_blocking`
- `ml_confidence_model_status_blocking`
- `train_ml_confidence_model`
- `apply_ml_confidence_model`
- `run_meta_stage4_experimental_blocking`
- `run_ml_confidence_policy_evaluation_blocking`
- `summarize_meta_artifacts_for_scope`
- `apply_holdout_evaluation`
- `MlConfidenceModelStatus`
- `TrainMlConfidenceResponse`

## 1A. Make meta scanning cheaper and narrower
Implement all of the following:

### Filename prefilter
Add a stronger filename-level prefilter in `run_meta_selection_blocking` so it skips known non-run artifacts before parse. Include at least the current meta/stage5/policy-eval/batch-summary families.

Suggested helper shape:
- `is_known_non_run_artifact_filename(name: &str) -> bool`

### Buffered JSON parsing
Stop using `read_to_string + serde_json::from_str` on the hot path where feasible. Add a local helper using `BufReader` + `serde_json::from_reader`, and reuse it in:
- `run_meta_selection_blocking`
- `load_meta_artifacts_with_paths_blocking`
- `load_meta_artifacts_blocking`

Suggested helper shape:
- `read_json_value(path: &Path) -> Option<Value>`

### Early semantic prefilter and single-pass partition
Add the addendum’s stronger early semantic prefilter during `run_meta_selection_blocking`.

Requirements:
- reject anything that is not a run artifact kind
- reject wrong `strategy_kind`
- reject wrong `required_tag`
- reject artifacts without `validation/res/candidates`
- do **not** require timeframe in the relaxed prefilter if the request timeframe is still empty and needs inference

Suggested helper shape:
- `artifact_matches_meta_req_relaxed(artifact: &Value, req: &MetaSelectionRequest) -> bool`

Then partition directly into:
- `train_artifacts`
- `holdout_artifacts`

during the scan itself, instead of building `artifacts` first and splitting later.

### Timeframe inference after pruning
If `req.timeframe` is empty:
- infer timeframe from the already-pruned `train_artifacts`
- then do one final retain/filter on both `train_artifacts` and `holdout_artifacts`
- preserve the current “single timeframe only” safety behavior

### Preserve cancellation and progress behavior
Do not regress:
- cancellation checks
- `meta_read` progress emission semantics
- current error behavior when mixed/missing timeframes require explicit user input

## 1B. Make ML status summary-only and stop hiding scan failures
Implement all of the following:

### Replace full model payload in status
Change `MlConfidenceModelStatus` so the normal status path returns a **summary**, not the full `MlConfidenceModel`.

Required status payload direction:
- remove or stop populating heavy `model: Option<MlConfidenceModel>` in the normal path
- add `model_summary: Option<ta_app_core::MlConfidenceModelSummary>`
- add `meta_scan_error: Option<String>`
- preserve existing state/health/staleness/retrain fields

### Do not swallow meta scan failure
Replace the current `unwrap_or_default()` behavior on meta scan failure.

Required behavior:
- status should remain non-fatal if meta scan fails
- but it must surface `meta_scan_error`
- it must not silently pretend there are zero available meta artifacts and zero labeled rows without explanation

### Build status from summary + scope summary
Implement a summary-based loader and status builder. The status path should not deserialize the full model body unless the full model is explicitly needed elsewhere.

### Preserve stale/retrain semantics
Keep the same stale/retrain logic, but make it work from:
- model summary metadata
- available meta artifact summary
- available labeled row counts
- newest meta timestamp

## 1C. Slim train response
Change `TrainMlConfidenceResponse` so the hot path does **not** ship the full model weights/trees back to the frontend.

Required response shape:
- `model_path`
- `model_bytes`
- `model_summary`
- `metrics`
- `status`

If an external caller in the repo truly depends on the full model object, either update that caller or split it behind an explicit separate full-load path. Do **not** keep shipping the heavy model in the default train response unless a real dependency forces it.

## 1D. Minimal-feature ML apply payload
Change `apply_ml_confidence_model` so it accepts the minimal feature DTO instead of full meta group JSON.

Required direction:
- input should become `Vec<MlConfidenceGroupFeatures>` or the equivalent minimal typed payload from `ml_confidence.rs`
- keep the response shape broadly compatible: `confidences` and `warnings` remain fine
- preserve scope mismatch / unhealthy model warning behavior

## 1E. Remove duplicate full-model load and duplicate status work in Stage 5
Focus on `run_meta_stage4_experimental_blocking`.

Required changes:
- compute status first using the summary-only path
- do **not** eagerly parse the full model before status is computed
- load the full model only if scoring or fallback actually needs it
- avoid duplicate status computation after auto-train when the newly trained model already provides enough information to build the post-train status locally
- if auto-train fails and you need fallback-to-existing-model behavior, only load the full model in that fallback branch

At minimum, remove these waste patterns:
- full model load before status
- full model load inside status
- second status scan immediately after a successful auto-train if it can be derived from already-available data

## 1F. Single-pass scope summary on the command side
Refactor `summarize_meta_artifacts_for_scope` and its helpers so commands-side status calculation does not do multiple full passes just to:
- decide whether holdout rows exist
- then count labeled rows

Create a single scope summary helper that can return all of:
- `available_meta_artifacts`
- `newest_meta_ts_available`
- `available_labeled_rows`
- whether dataset-row labels exist
- any other metadata needed for stale/retrain logic

## 1G. Dense selected-group holdout evaluation
Refactor `apply_holdout_evaluation` so selective holdout evaluation allocates against the selected subset, not the full group count.

Required behavior:
- when `selected_groups` is present, allocate dense local arrays only for that subset
- remap back into global `result.groups` at the end
- preserve all existing scoring, pass-rate, final-score, evidence-flag, and optional `holdout_rows` semantics
- do not change the output for selected groups

## 1H. Prepared-row / prefix policy evaluation
Refactor `run_ml_confidence_policy_evaluation_blocking` to reduce repeated work across prefixes.

Required outcome:
- keep the same rolling-origin semantics
- keep no-leakage behavior
- prepare reusable per-meta row material once
- reuse those prepared rows across prefix training/evaluation
- keep policy outputs parity-safe

You may refactor this in stages internally, but the final delivered code should expose the prepared-row path and actually use it in policy evaluation.

## 1I. Optional but safe if small
If it is trivial and low-risk once you are already touching policy evaluation, cap warning growth so very long warning lists do not bloat artifacts. This is optional and must not delay the required work.

---

# 2. `crates/app_core/src/ml_confidence.rs`

Focus on these items:
- `MlConfidenceModel`
- `MlConfidenceModelMeta`
- `train_ml_confidence_model_from_meta_artifacts`
- `scope_has_holdout_rows`
- `build_dataset_from_meta_artifacts`
- `predict_ml_confidence_for_group`

## 2A. Add summary type
Add `MlConfidenceModelSummary`.

Required fields:
- `version`
- `scope`
- `model_kind`
- `label`
- `meta`

It should be cheap to derive from `MlConfidenceModel`, for example via `impl From<&MlConfidenceModel> for MlConfidenceModelSummary`.

## 2B. Add minimal feature DTO
Add `MlConfidenceGroupFeatures`.

Required fields:
- `samples`
- `pass_rate`
- `robust_score_p10`
- `robust_score_median`
- `max_dd_p90`
- `cross_score_median`
- `meta_score`

Add prediction support that does not require a full `Value` group.

Required direction:
- keep `predict_ml_confidence_for_group`
- add `predict_ml_confidence_for_features`
- the new path should reuse the same feature ordering and model semantics
- tests must show parity between the two prediction paths within tolerance

## 2C. Fuse `Auto` target detection and dataset build
The current `Auto` flow should stop doing one pass to decide if `holdout_rows` exist and a second pass to build rows.

Refactor to a single builder that can:
- scan meta artifacts once for the scope
- collect the candidate rows needed for `HoldoutGate` and/or `HoldoutDatasetPass`
- decide which target to use for `Auto`
- return the chosen row set plus build summary and warnings

Important constraint:
- keep the **current runtime semantics** of mixed-history `Auto`
- do **not** introduce hybrid backfill behavior in this pass
- add explicit tests that document the current behavior

## 2D. Split training into row preparation + row-based training
Refactor the trainer into a shape similar to:
- scope normalization
- row preparation/build
- row-based training implementation helper

A helper like this is expected:
- `train_ml_confidence_model_from_rows_impl(...)`

Then `train_ml_confidence_model_from_meta_artifacts(...)` becomes thinner and reusable.

## 2E. Expose prepared-row API needed by policy evaluation
Expose an internal/public-enough API that `commands_meta.rs` can use for prefix-based policy evaluation without rebuilding rows from raw JSON every iteration.

This can be:
- a prepared-row builder per meta artifact
- a scope row builder that returns reusable slices/buckets
- or equivalent

Do **not** make prepared rows part of a long-term stable artifact contract unless required. Keep them as an implementation optimization.

---

# 3. `crates/app_core/src/meta_select.rs`

Focus on the safe micro-optimization only.

Required changes:
- add a richer row type such as `RobustParamRow`
- change `GroupAcc.robust_params` so it stores the precomputed exact param signature once
- compute the signature once during candidate ingestion
- update `pick_representatives` to reuse the cached signature rather than recomputing canonical JSON for dedupe

Important constraints:
- keep `canonical_json` as the correctness baseline
- do **not** replace grouping keys with a new hash design in this pass
- do **not** redesign clustered grouping semantics

---

# 4. `web/src/services/metaService.ts`

This is where user-visible latency is felt. Focus on:
- `formatMlStatus`
- `maybeRunMetaMlAutopilot`
- `mlRefresh`
- `mlTrain`
- `mlApply`
- `runMeta`
- `runStage5`
- `runMlPolicyEvaluation`
- `refreshSavedInternal`
- `refreshStage4SavedInternal`
- `refreshPolicyEvalSavedInternal`

## 4A. Consume summary-only status
Update all status consumers so they rely on:
- top-level `status.model_healthy`
- `status.model_summary`
- `status.meta_scan_error`

Do **not** rely on `status.model` for routine status rendering.

A small helper like `applyMlStatusPayload(...)` is encouraged if it reduces duplication.

## 4B. Stop immediate post-train refresh when train already returned status
`mlTrain()` should use the returned `status` payload directly instead of immediately calling `mlRefresh()` again.

Required behavior:
- update `LAST_META_ML_STATUS` from the train response
- update the status line / button state from that payload
- if `metaUseMlConfidence` is on and there is a current meta result, still allow follow-up `mlApply()` as today

## 4C. Send minimal feature DTO in `mlApply`
Before invoking `apply_ml_confidence_model`, map the current meta groups to the minimal feature DTO.

Required behavior:
- preserve null/invalid-confidence handling
- preserve optional sort-by-ML behavior when the user has ML ranking enabled and the model is healthy enough
- do not send full group JSON over IPC anymore

## 4D. Defer saved-artifact list refreshes off the hot path
In all of these paths:
- `runMeta()`
- `runStage5()`
- `runMlPolicyEvaluation()`

do not block the hot path on awaited saved-list refreshes.

Required behavior:
- render the just-produced result immediately
- update visible state immediately
- perform the saved-list refresh asynchronously afterward
- any refresh failure must not wipe out the just-rendered result or local saved-list state

If a helper is useful, add a small fire-and-forget refresh wrapper with internal error handling.

## 4E. Add optimistic saved-list upserts before deferred refresh
Implement optimistic local upserts for:
- meta saved list
- Stage 5 saved list
- policy-eval saved list

Required behavior:
- construct a lightweight saved-entry row from the command response already in hand
- insert or replace it in local `SAVED_*`
- update the corresponding UI state immediately
- then reconcile with the real `list_*_artifacts` refresh in the background

Do not add a broad cache layer. This should stay local, lightweight, and response-driven.

## 4F. Refresh ML status after Stage 5 only when the model changed
After `runStage5()`:
- if the Stage 5 result says the model changed (`model_trained_now` or equivalent), update status
- at minimum, schedule a deferred `mlRefresh({ scope })`
- do not leave the status strip/buttons stale after an auto-train

The lighter safe option is sufficient. A larger response-shape extension is optional, not required.

---

# 5. `web/src/services/metaRequestBuilder.ts`

Keep this file very small and conservative.

Required change:
- ensure `clusterFloatBucket` normalizes to a strictly positive default (`0.25`) instead of allowing zero/negative values that effectively collapse clustered mode into an overly exact grouping shape

Do not add new knobs or broader behavior here.

---

# 6. `web/src/services/metaStage4RequestBuilder.ts`

Keep this file very small and conservative.

Required change:
- add client-side normalization for probability-ish fields that accepts either unit interval or percentage style input

Use it for:
- `min_ml_confidence`
- `accept_confidence`
- `review_confidence`
- `max_accept_uncertainty`

Keep backend normalization authoritative. This is just to reduce obvious frontend/backend drift.

---

# 7. Benchmarks and regression gates

## 7A. `crates/app_core/benches/meta_select_workflow.rs`
Add a second benchmark family with richer `feature_context` / `run_context` payloads so grouped key generation cost is more realistic.

Required additions:
- helper similar to `make_context_heavy_artifacts(...)`
- `exact_context_rich`
- `clustered_context_rich`

Keep the existing simple benchmark coverage.

## 7B. `crates/app_core/benches/e2e_workflow_ml.rs`
Add a benchmark family that isolates Stage 5 / policy-eval style history throughput.

Required additions:
- helper similar to `make_meta_history(meta_count, groups_per_meta)`
- benchmark rolling-prefix train/apply cost
- benchmark prepared-row prefix train/apply cost after the refactor

The goal is direct Criterion-boundary evidence for the prepared-row policy-eval refactor.

## 7C. `src-tauri/src/bin/ta_workflow_regression.rs`
Extend the regression breakdown to optionally include ML training/apply timing.

Required additions:
- `ml_train_ms`
- `ml_apply_ms`
- backward-compatible `#[serde(default)]` handling
- CLI flag similar to `--include-ml`
- regression thresholds for ML train/apply stage timings
- stage-budget checks wired through the same gate mechanism as existing stages

Preserve compatibility with older baseline JSON files.

---

# 8. Required tests

Add or update tests to cover all of the following.

## 8A. Summary-only status payload
Verify that status can be built from a summary-only model payload and that the frontend-visible metadata does not require the full model body.

## 8B. Minimal feature apply parity
Verify that:
- `predict_ml_confidence_for_group`
- `predict_ml_confidence_for_features`

produce the same answers within tolerance for the same logical group features.

## 8C. Fused `Auto` target behavior
Add tests covering:
- only aggregate holdout labels
- only `holdout_rows`
- mixed history with both present

Important: mixed-history tests should assert the **current** semantics, not invent new ones.

## 8D. Dense selected-group holdout equivalence
Verify that selective holdout evaluation with dense remapping preserves selected-group outputs.

## 8E. Deferred saved refresh does not block rendering
In frontend tests, verify that:
- result rendering happens before background refresh completion
- refresh failures do not wipe out the already rendered result or optimistic saved entry

## 8F. Policy-eval prepared-row parity
Verify that the prepared-row prefix path preserves policy outputs within tolerance and does not leak future data.

## 8G. `meta_scan_error` surfacing
Verify that status reports a meta scan failure explicitly instead of silently pretending the scan found zero artifacts.

---

# 9. Optional items after required work passes

These are optional. Do not let them delay the required work.

1. Cap policy-eval warning growth if it is trivial and safe.
2. If profiling shows XLSX export is a large share of `runMeta` latency, instrument and prepare a safe non-breaking follow-up path. Do not remove report generation by default in this pass unless there is clear evidence and no compatibility risk.
3. After the prepared-row refactor, only touch trainer scratch-buffer reuse if measurement shows the trainer is still a meaningful hotspot.
4. Do **not** implement artifact manifests, global caches, or hashed group-key redesigns unless the required work is complete and measurements clearly justify it.

---

# 10. Execution order

Implement in this order unless the codebase forces a nearby adjustment:

## Commit/phase sequence
1. `ml_confidence.rs`
   - `MlConfidenceModelSummary`
   - `MlConfidenceGroupFeatures`
   - `predict_ml_confidence_for_features`

2. `commands_meta.rs`
   - summary-only status
   - surfaced `meta_scan_error`
   - no swallowed meta scan failure

3. `metaService.ts`
   - consume summary-only status
   - centralize status updates if helpful

4. `commands_meta.rs` + `metaService.ts`
   - slim train response
   - no immediate post-train `mlRefresh`

5. `commands_meta.rs` + `metaService.ts`
   - minimal-feature apply DTO
   - frontend mapping

6. `commands_meta.rs`
   - stronger filename prefilter
   - buffered parse
   - early semantic prefilter
   - single-pass train/holdout partition

7. `commands_meta.rs`
   - dense selected-group holdout allocation remap

8. `ml_confidence.rs` + `commands_meta.rs`
   - single-pass dataset/scope builders

9. `metaService.ts`
   - optimistic upserts
   - deferred saved refreshes
   - Stage 5 post-train status synchronization

10. Benchmarks and tests

11. Prepared-row / prefix policy-eval refactor

This sequence minimizes risk by landing payload and orchestration wins before the deeper row-preparation refactor.

---

# 11. Validation commands you should run

Run the smallest relevant targets first, then broader verification.

## Rust
At minimum, run the relevant tests for the touched crates/modules. If the repo supports it, run broader crate/workspace verification after targeted tests pass.

Suggested minimum verification:
- targeted Rust tests for `meta_select.rs`, `ml_confidence.rs`, and `commands_meta.rs`
- broader `cargo test -p ta_app_core`
- broader Tauri/backend test/check target if the repo has one and it is practical

## TypeScript / frontend
Run at least:
- `npm run typecheck`
- relevant service tests if they exist
- broader `npm run test` if practical after targeted tests pass

## Bench / workflow surfaces
At minimum:
- make sure the new benches compile
- if practical, run the new/modified benchmark targets once
- make sure the workflow regression binary builds with the new ML fields/flags

If full benchmark execution is too slow in the agent environment, at least ensure the targets build cleanly and explain any unrun step.

---

# 12. Required reporting format back to the user

When you finish, report back in this structure:

1. **Files changed**
   - list every file modified

2. **What was implemented**
   - grouped by the same categories as this prompt

3. **What was intentionally not changed**
   - especially open-question or optional items you left alone

4. **Validation run**
   - tests/commands run
   - pass/fail
   - any commands skipped and why

5. **Behavior/parity notes**
   - confirm whether mixed-history `Auto` semantics were preserved
   - confirm whether any public response shapes changed and which frontend callers were updated

6. **Benchmark/gate changes**
   - list the new benchmark or stage fields added

7. **Risks / follow-ups**
   - only real ones that remain after implementation

---

# 13. Done checklist

The work is not done until all required items below are true:

- [ ] `run_meta_selection_blocking` uses stronger filename filtering
- [ ] `run_meta_selection_blocking` uses buffered JSON parsing
- [ ] `run_meta_selection_blocking` applies early semantic prefiltering
- [ ] `run_meta_selection_blocking` partitions train vs holdout during scan
- [ ] timeframe inference still works after early pruning
- [ ] ML status no longer ships full model weights/trees by default
- [ ] ML status surfaces `meta_scan_error`
- [ ] ML status does not silently swallow meta scan failure
- [ ] train response no longer ships the full model by default
- [ ] apply command uses minimal feature DTO
- [ ] frontend `mlApply` sends only minimal features
- [ ] Stage 5 no longer loads the full model before summary status is computed
- [ ] Stage 5 does not do redundant status work after successful auto-train
- [ ] commands-side scope summary is single-pass
- [ ] `ml_confidence.rs` target detection + row build is single-pass
- [ ] mixed-history `Auto` semantics are preserved and explicitly tested/documented
- [ ] selective holdout evaluation allocates densely against selected groups
- [ ] saved meta list updates immediately via optimistic upsert
- [ ] saved Stage 5 list updates immediately via optimistic upsert
- [ ] saved policy-eval list updates immediately via optimistic upsert
- [ ] deferred refreshes no longer block hot-path rendering
- [ ] Stage 5 refreshes or updates ML status only when model state changed
- [ ] `metaRequestBuilder.ts` clamps `clusterFloatBucket` to a positive default
- [ ] `metaStage4RequestBuilder.ts` normalizes unit-interval / percent inputs
- [ ] `meta_select_workflow.rs` has context-rich grouping benches
- [ ] `e2e_workflow_ml.rs` has Stage 5 / prefix-history benchmark coverage
- [ ] `ta_workflow_regression.rs` can optionally track ML train/apply stage timings
- [ ] required Rust tests were added/updated
- [ ] required frontend tests were added/updated if a nearby test surface exists
- [ ] relevant tests pass

## Final instruction
Do the implementation, not just the analysis. Keep the patch series tightly scoped, parity-safe, and benchmark-aware.
