# Codex implementation prompt — Bundle 13 analysis / drilldown / artifact-hydration responsiveness

Use the following prompt with your OpenAI Codex agent.

---

You are implementing a repo-scoped performance/correctness plan in a Rust/Tauri + Vue/TypeScript codebase.

Your job is to **fully implement the Bundle 13 guide** from the attached/source documents, with emphasis on **performance, optimization, and correctness without removing features or breaking functionality**.

## Source-of-truth documents to read first

Read these documents before you change any code. Treat them as the primary implementation brief.

1. `/mnt/data/gpt54_feature_review_map.md`
2. `/mnt/data/bundle13_analysis_drilldown_artifact_hydration_plan.md`
3. `/mnt/data/bundle13_analysis_drilldown_artifact_hydration_addendum.md`

If those absolute paths are mounted differently in your environment, locate them by filename and use those copies.


## Path mapping note

If your Codex environment is working from the uploaded standalone files rather than the full repo tree, use this mapping:

- `/mnt/data/commands_analysis.rs` -> `src-tauri/src/commands_analysis.rs`
- `/mnt/data/analysisService.ts` -> `web/src/services/analysisService.ts`
- `/mnt/data/workflowArtifactsSync.ts` -> `web/src/services/workflowArtifactsSync.ts`
- `/mnt/data/workflowArtifactsStore.ts` -> `web/src/stores/workflowArtifactsStore.ts`
- `/mnt/data/DrillWorkspacePanel.vue` -> `web/src/components/DrillWorkspacePanel.vue`
- `/mnt/data/HeatmapWorkspacePanel.vue` -> `web/src/components/HeatmapWorkspacePanel.vue`
- `/mnt/data/double_ma_followup_workflow.rs` -> `crates/app_core/benches/double_ma_followup_workflow.rs`
- `/mnt/data/ta_workflow_regression.rs` -> `src-tauri/src/bin/ta_workflow_regression.rs`

## Bundle / scope you are implementing

You are implementing **Bundle 13: Analysis/drilldown and artifact-hydration responsiveness**.

Stay within that bundle’s scope:
- artifact hydration
- compare/baseline loading
- drilldown/local-stability cache behavior
- heatmap/drill responsiveness
- legacy-runtime to Vue artifact sync and refresh behavior
- benchmark timing-boundary clarity for the bundle’s companion perf artifacts

## Primary repo files to inspect and modify

These are the main files you should work in.

- `src-tauri/src/commands_analysis.rs`
- `web/src/services/analysisService.ts`
- `web/src/services/workflowArtifactsSync.ts`
- `web/src/stores/workflowArtifactsStore.ts`
- `web/src/components/DrillWorkspacePanel.vue`
- `web/src/components/HeatmapWorkspacePanel.vue`

## Supporting benchmark / workflow files to inspect and modify

- `crates/app_core/benches/double_ma_followup_workflow.rs`
- `src-tauri/src/bin/ta_workflow_regression.rs`

## Hard constraints

You must follow these constraints exactly.

1. **Preserve product behavior and parity.**
   - No feature removals.
   - No intentional behavior regressions.
   - No breaking contract changes unless explicitly called out below as optional, backward-compatible stretch work.

2. **Do not broaden scope into a redesign.**
   - No chart-stack migration.
   - No table-stack migration.
   - No broad architecture rewrite.
   - No new workflow panels or new product surfaces.
   - No speculative GPU expansion claims.

3. **Do not drift outside Bundle 13.**
   - Do not reopen already-landed frontend-only cleanup around shell-mount helpers, accessibility copy, or generic UI polish.
   - `MlSignal` remains intentionally outside the standard optimizer-native analysis surfaces. Do not change that.
   - Do not turn this into a Bundle 14 data-loading project or a live-execution project.

4. **Prefer smaller parity-safe changes before larger refactors.**
   - Implement the low-risk/high-value changes first.
   - Only do optional stretch work after core items are complete and validated.

5. **Be honest about timing boundaries.**
   - Engine Criterion benchmarks are not app-path proof.
   - Workflow-regression timing is not analysis-hydration timing.
   - Preserve existing benchmarks, but make their scope clearer.

6. **Do not ask for clarification unless absolutely blocked.**
   - Use the provided docs and code as the implementation source of truth.
   - If a type/detail is ambiguous, inspect adjacent code and make the smallest safe decision.

## High-level implementation goals

Your implementation must achieve all of the following:

1. Make unchanged workflow-artifact sync a true no-op whenever it is safe.
2. Stop replaying heavy analysis refresh work when the normalized artifact payload did not materially change.
3. Eliminate duplicate Top-table/overview recompute work.
4. Fix local-stability request shaping so cache keys are not polluted by drill-only UI fields and local-stability uses the correct scoring semantics.
5. Prevent stale drill renders/status updates during rapid selection changes.
6. Deduplicate identical in-flight async work on the frontend.
7. Bound currently unbounded caches.
8. Make repeated drill/heatmap panel reveal cheap when the underlying artifact/view state has not changed.
9. Improve benchmark / perf-harness timing-boundary clarity for the bundle’s supporting evidence.
10. Add or update targeted tests for the new behavior.

---

# Required implementation work

## 1) `web/src/stores/workflowArtifactsStore.ts` — mandatory

Implement the artifact-change detection and no-op apply behavior described in the plan.

### Required behavior

1. Introduce a richer apply result type, e.g. `WorkflowArtifactsApplyResult`, with at least:
   - `applied`
   - `runChanged`
   - `validationChanged`
   - `sourceChanged`

2. Extend the latest run snapshot so it is useful for identity checks.
   - The current snapshot only stores `req`/`res`.
   - Extend it to also retain `ts` and `artifactPath`.

3. Add stable signatures for normalized run and validation payloads.
   - Add a deterministic `stableStringify()` or equivalent signature helper.
   - Add helpers equivalent to:
     - `workflowRunSignature(run)`
     - `workflowValidationSignature(validation)`

4. Add a read helper for the current artifact identity.
   - Add `readLatestWorkflowRunIdentitySnapshot()` or equivalent.
   - It should expose at minimum:
     - `ts`
     - `artifactPath`
     - `hasValidation`

5. Make normalized apply truly no-op when appropriate.
   - `applyNormalizedArtifacts(...)` should only churn store state when the normalized run or validation actually changed.
   - Identical payloads must not bump `updatedAtUnixMs`.
   - Source-only changes may update `store.source`, but should not be treated as run/validation changes.

6. Change `applyWorkflowArtifactsPayload(...)` to return the richer apply result instead of a bare boolean.

### Important edge-case requirement

Support the “validation arrived later” case.

If the run content is the same but validation appears later, that should count as:
- `runChanged = false`
- `validationChanged = true`

### Acceptance details for this file

- No-op identical payloads must not cause timestamp churn.
- The result object must let callers distinguish unchanged data from true run/validation changes.
- The helper names can vary, but the behavior must match the plan/addendum.

---

## 2) `web/src/services/workflowArtifactsSync.ts` — mandatory

Implement the conservative sync-boundary optimization described in the plan.

### Required behavior

1. Import and use the new artifact identity reader from `workflowArtifactsStore.ts`.
   - e.g. `readLatestWorkflowRunIdentitySnapshot()`

2. Add a conservative pre-load skip before `load_run_artifact`.
   - Compare the latest row from `list_run_artifacts` against the currently loaded artifact identity.
   - Use at least:
     - latest path
     - latest ts
   - Only skip loading when it is safe.

3. Preserve the “validation may have arrived later in-place” edge case.
   - If the latest artifact path/ts matches but local validation is still absent, **do not** skip the artifact load.
   - This is specifically to avoid missing a same-path/same-ts artifact rewrite that added validation.

4. After loading and normalizing the artifact, call `applyWorkflowArtifactsPayload(...)` and only trigger `refreshAnalysisFromWorkflowRun()` when:
   - `runChanged === true`, or
   - `validationChanged === true`

5. The function should return whether anything was meaningfully applied.

### Do not do

- Do not always call `refreshAnalysisFromWorkflowRun()` just because the payload parsed.
- Do not turn same-artifact refreshes into heavy no-op churn.

### Optional but acceptable if easy

If you find repeated overlapping sync calls, you may serialize or dedupe them, but that is secondary to the mandatory identity/no-op behavior.

---

## 3) `web/src/services/analysisService.ts` — mandatory and highest-effort file

This is the main implementation surface. Follow the plan and addendum carefully.

### 3A) Cache helper cleanup and cache bounds — mandatory

Implement the cache cleanup described in the plan.

#### Required behavior

1. Make `cacheSet(...)` generic.
2. Add explicit cache-size constants or equivalent for consistency.
3. Keep bounded behavior for existing drill/stability caches.
4. Add bounds to currently unbounded caches:
   - `baselineArtifactCache`
   - `drillPriceBarsCache`

#### Suggested policy

Use the plan’s spirit and sizes unless existing local conventions clearly differ:
- drill curves: ~96
- local stability: ~96
- baseline artifacts: ~24
- drill price bars: ~24

The exact helper/constant names may vary, but the caches must become bounded.

---

### 3B) In-flight async dedupe — mandatory

Implement the addendum’s in-flight dedupe layer.

#### Required behavior

1. Add pending-promise maps for at least:
   - drill curves
   - local stability
   - drill price bars
   - baseline artifact loads

2. Add a shared helper like `cacheGetOrLoad(...)` that:
   - checks settled-result cache first
   - then checks pending promise cache
   - otherwise starts the load, stores the promise, populates the settled cache on success, and clears pending state in `finally`

3. Reuse this helper in:
   - `runVueDrillPipeline()` for drilldown
   - `runVueDrillPipeline()` for local stability
   - `fetchDrillPriceBars()`
   - `loadBaselineFromDisk()`

4. Preserve existing user-visible semantics as much as possible.
   - If a result was shared from an in-flight request, treat that as effectively warm/shared rather than issuing duplicate backend work.

#### Why this is mandatory

This was identified in the addendum as one of the biggest remaining warm-path issues.

---

### 3C) Top-table dedupe and score-context correctness — mandatory

Implement the top-table refresh refactor from the plan.

#### Required behavior

1. `refreshAnalysisFromWorkflowRun()` should own overview refresh exactly once.
   - Remove the nested/duplicate overview refresh inside `refreshTopTableFromWorkflowRun()`.

2. Compute the row pipeline exactly once per refresh path.
   - `scoreContext`
   - `collected`
   - `sorted`
   - `filtered`

3. Refactor `buildTopRowModelsForCurrentFilters()` so it can accept `preSorted` rows or equivalent.
   - This is to avoid recollecting and resorting inside the same refresh.

4. Ensure `sortRows(...)` consistently receives `scoreObservationContextOf(res)` where needed.
   - This is important for score modes whose ordering depends on observation context.

5. Reuse the single computed `sorted`/`filtered` result everywhere in the refresh path:
   - `topRowsModelCache`
   - `topRowsAllModelCache`
   - selected-row reconciliation
   - pinned candidate reconciliation
   - row rendering
   - filter metadata

6. Preserve current behavior for:
   - selection guardrails
   - pinned candidate sync
   - empty/filter-empty states
   - `MlSignal` special-casing

#### Do not do

- Do not change ranking semantics.
- Do not drop selection restoration behavior.
- Do not change visible table columns beyond what is required for correctness/performance.

---

### 3D) Local-stability request correction — mandatory

Implement **both** parts of the fix:
1. remove drill-only fields from the local-stability request
2. propagate the correct scoring semantics

#### Required behavior

1. Stop building the local-stability request by spreading `...invocation.request`.
2. Build the local-stability request from an explicit allowlist of fields.
3. Do **not** include `bins` in the local-stability request.
4. Use `objectiveOf(runReq)` instead of `text(runReq.objective) || "Sharpe"`.
5. Propagate `score_mode` for the Double-MA local-stability request.
6. For Composite CPU local-stability:
   - inspect the actual request type definition
   - if `score_mode` is supported, propagate it
   - if it is not supported, do not force an unsupported field into the request type

7. Keep any required existing fields, such as strategy/halftrend/signal-mode data, but only from an explicit allowed set.

#### Acceptance details

- Two drill invocations that differ only by `bins` must still generate the same local-stability cache key.
- Score-mode-ranked runs must not silently compute local stability under the wrong scoring semantics.

---

### 3E) Stale drill race fix — mandatory

Implement the addendum’s stale-sequence guards in `runVueDrillPipeline()`.

#### Required behavior

1. Keep existing stale-sequence guards after drilldown compute and after local stability compute.
2. Add a stale-sequence guard **after** `fetchDrillPriceBars()` and **before** any drill-side DOM/status/state mutation.
3. Add another guard before drill-side status/meta writes if needed.

#### The problem you are fixing

Rapid row changes can currently allow the earlier request to paint stale drill DOM or status text after the later request became active.

#### Acceptance details

- Quick row changes must not flash stale equity/drawdown/trade-table/status output from the old selection.

---

### 3F) Drill restore path and cache invalidation hook — mandatory

Implement the drill restore optimization from the plan, plus the cache invalidation hook from the addendum.

#### Required behavior

1. Add a stable run-identity helper for restore decisions.
   - e.g. `workflowRunIdentity()` or equivalent

2. Add a `DrillRenderState` structure (name can vary) that stores enough information to restore the current rendered drill view cheaply.
   - at minimum tie restore validity to:
     - current run identity
     - current selected row id

3. Capture drill render state after successful drill pipeline completion.
4. Add `restoreDrillViewFromCache()` or equivalent.
5. Change `rerenderDrill()` so it prefers restore over re-entering the full selection/pipeline path.
6. Fall back to recompute only when:
   - run identity changed
   - selected row changed
   - necessary cached render state is missing/invalid

7. Add an exported invalidation hook such as `invalidateAnalysisExecutionCaches()` that at least clears:
   - drill curves cache
   - local stability cache
   - drill price-bar cache
   - drill render state
   - hover/selection state as appropriate
   - and bumps any sequence/version needed to cancel stale work

#### Important note

You do **not** need to wire every future dataset-change call site if they live outside the current bundle files, but you should add the invalidation hook now so the next integration point can use it.

---

### 3G) Heatmap hidden-render suppression and visible restore — mandatory

Implement the heatmap optimization from the plan.

#### Required behavior

1. Add heatmap render state / signature tracking for the base canvas.
   - The signature should reflect the underlying artifact/view identity, not just hover state.
   - Include enough data to avoid false reuse when the base visual should change.
   - Typical inputs include some combination of:
     - run identity or artifact identity
     - heatmap bins dimensions
     - objective / score mode if relevant to rendered values
     - canvas size / DPR-scaled size

2. Make repeated reveal cheap.
   - If the base signature is unchanged, skip the full base canvas repaint.
   - Continue to repaint overlay/crosshair/hover affordances as needed.

3. Preserve existing behavior for:
   - hover tooltip
   - crosshair
   - marker rendering
   - click-to-drill
   - export PNG behavior

4. Add a helper like `ensureHeatmapVisibleRender()` or equivalent so the panel can request a cheap reveal-time restore rather than a full analysis refresh every time it becomes visible.

#### Do not do

- Do not remove the overlay interactivity.
- Do not break click-to-drill.
- Do not reduce correctness in exchange for repaint suppression.

---

### 3H) Optional in-scope Pareto hidden-render suppression — optional, only after core

This is not required for acceptance, but if it is trivial and clearly safe, you may mirror the heatmap hidden-render suppression for Pareto.

Conditions:
- only do it after core items are stable
- do not expand scope into a broader chart rewrite
- preserve click behavior and current semantics

If not implemented, note it as deferred in your final summary.

---

### 3I) Preserve current special cases — mandatory

Do not break or broaden these existing behaviors:

1. `MlSignal` remains outside the optimizer-native analysis surfaces.
2. Existing compare/baseline semantics must still work.
3. Existing drill/heatmap export and interaction behaviors must still work.
4. Existing status messages may be improved only insofar as they remain correct and non-breaking.

---

## 4) `web/src/components/HeatmapWorkspacePanel.vue` — mandatory

Update the panel so repeated reveal uses the cheaper heatmap visible-render path instead of replaying more work than necessary.

### Required behavior

1. Preserve current visible/mount semantics from the user’s perspective.
2. Stop using repeated panel reveal as a reason to force full heatmap hydration when the base artifact/render signature is unchanged.
3. Call the new heatmap visible-render helper from `analysisService.ts` on reveal when appropriate.
4. Only trigger the heavier refresh path when the artifact/render state actually requires it.

### Constraints

- Keep the panel behavior parity-safe.
- Do not redesign the component structure unless needed for the restore path.
- Do not introduce a new UI surface.

---

## 5) `web/src/components/DrillWorkspacePanel.vue` — mandatory

Update the panel so repeated reveal prefers drill restore rather than replaying full pipeline work.

### Required behavior

1. Preserve current visible/mount semantics from the user’s perspective.
2. Replace any mount-time “rerender” nudge that blindly re-enters expensive selection/pipeline work with a more deliberate reveal-time behavior.
3. Make repeated reveal call the cheaper drill restore path where possible.
4. Fall back to the expensive path only when restore is invalid/unavailable.

### Constraints

- Keep selection semantics intact.
- Do not break drill status or interaction.
- Do not redesign the panel/UI.

---

## 6) `src-tauri/src/commands_analysis.rs` — inspect, but core changes optional

This file is part of the bundle and must be inspected.

### Core-phase instruction

Do **not** change the product contract in the core implementation unless a tiny local fix is obviously needed.

### Optional stretch only after core passes

If, and only if, all core items are complete and validated, you may consider a backward-compatible optional enhancement such as:
- a combined follow-up command for drilldown + local stability, or
- a metadata envelope / sidecar response that improves backend-used / followup-cache-used visibility

These are **not required** for acceptance.

If you do not implement them, note them as deferred.

---

## 7) `crates/app_core/benches/double_ma_followup_workflow.rs` — mandatory evidence-hygiene change

Keep this benchmark, but make its timing boundary harder to misread.

### Required behavior

Add metadata/output clarification that this benchmark is:
- an engine-side Criterion benchmark
- useful for backend follow-up compute reuse
- **not** app-path proof for Tauri/web artifact hydration, chart hydration, or panel reveal responsiveness

### Acceptance details

The benchmark should clearly advertise its timing-boundary limits in printed metadata or equivalent.

Do not change the core benchmark purpose unless you have a very strong reason.

---

## 8) `src-tauri/src/bin/ta_workflow_regression.rs` — mandatory evidence-hygiene change

Keep this harness, but make its timing boundary harder to misread.

### Required behavior

1. Add a serialized/exposed exclusions field such as `timing_boundary_exclusions`.
2. Make it explicit that this workflow regression harness excludes bundle-13 app-path work such as:
   - Tauri analysis command latency
   - workflow artifact sync/apply
   - Top-table hydration
   - heatmap rendering
   - drill/local-stability panel hydration

3. Preserve serde/default compatibility.
4. Update the existing legacy-deserialization test accordingly.

### Acceptance details

The summary struct and tests should still deserialize older payloads safely.

---

# Tests to add or update

Add targeted tests for the new behavior. Prefer narrow, high-signal tests.

## Highest-priority tests

1. **workflow artifact apply no-op test**
   - identical normalized payloads do not register as run/validation changes
   - `updatedAtUnixMs` does not churn on no-op apply

2. **validation-arrives-later test**
   - same run + later validation produces `validationChanged=true` without falsely treating the run as changed

3. **sync pre-load skip test**
   - when latest path/ts matches and local validation already exists, `load_run_artifact` is skipped
   - when local validation is absent, the artifact load still occurs

4. **local-stability cache-key test**
   - different drill `bins` values do not affect the local-stability cache key

5. **local-stability score-mode propagation test**
   - Double-MA local-stability requests inherit `score_mode`
   - if Composite CPU supports it, test that too

6. **top-table score-context consistency test**
   - the explicit sort path and the filtered-row builder do not diverge on score-context-sensitive score modes

7. **drill restore test**
   - repeated reveal with same run identity + selected row uses restore instead of full pipeline recompute

8. **heatmap visible-render test**
   - unchanged base signature skips base repaint while preserving overlay behavior

9. **in-flight dedupe test or equivalent manual proof**
   - identical overlapping drill/stability/price-bar/baseline requests are shared rather than duplicated

## If a test-only shim is needed

If the repo’s current test harness needs explicit access to private helpers, add a minimal test-only export/shim rather than making helpers broadly public in production.

---

# Optional stretch work — only after core implementation is complete and validated

These are valid ideas from the addendum, but they are **not required** for acceptance.

## Stretch A — `invalidateAnalysisExecutionCaches()` call-site wiring

If you can safely locate obvious dataset-change/resample call sites inside the current touched files, wire the invalidation hook there.

If the real call sites live in another bundle/surface, just add the exported hook now and note integration points in the final summary.

## Stretch B — hidden Pareto render suppression

Only implement if it is small, safe, and clearly adjacent.

## Stretch C — combined follow-up backend command

Backward-compatible only. Do not replace existing commands. Defer if it expands scope.

## Stretch D — backend “latest artifact head / digest” path

Probably outside the current file set. Defer unless it is already adjacent and trivial.

## Stretch E — dev-only timing instrumentation in `analysisService.ts`

Acceptable if implemented as a clearly gated dev-only diagnostic. Not required.

## Explicitly do **not** do these in this task

- chart library migration
- off-main-thread canvas/worker rewrite
- broad panel architecture rewrite
- new UI surfaces
- new feature ideation beyond the performance/correctness scope

---

# Implementation order you should follow

Use this order unless the code strongly suggests a safer nearby variant.

## P0 — correctness / parity

1. fix local-stability request shaping
2. propagate score semantics (`objectiveOf(runReq)` and `score_mode` where supported)
3. add stale-sequence guards after price-bar fetch and before drill-side DOM/status writes

## P1 — warm-path performance

4. add artifact apply signatures + structured apply result
5. add conservative sync pre-load skip and change-gated analysis refresh
6. dedupe Top-table/overview recompute
7. add in-flight async dedupe
8. bound currently unbounded caches

## P2 — repeated reveal responsiveness

9. add heatmap render signature / visible restore path
10. add drill render-state restore path
11. update `HeatmapWorkspacePanel.vue`
12. update `DrillWorkspacePanel.vue`

## P3 — perf-evidence hygiene

13. update `double_ma_followup_workflow.rs` timing-boundary metadata
14. update `ta_workflow_regression.rs` exclusions metadata + test

## P4 — optional follow-ons

15. cache invalidation wiring if adjacent and safe
16. optional Pareto suppression
17. optional dev-only diagnostics
18. optional backward-compatible command enhancements

---

# Validation / commands to run

Run the narrowest relevant checks that exist in the repo.

At minimum, do as many of these as are available and relevant:

- targeted web/unit tests for modified TS/Vue files
- targeted Rust tests for modified Rust files
- TypeScript build/typecheck for touched web files
- formatting/linting used by the repo
- targeted bench/test compile for the two Rust benchmark/workflow files if practical

If some checks are unavailable in the environment, say exactly which ones you could not run.

Do **not** claim tests passed if you did not actually run them.

---

# Final response format

When you finish, respond with a concise implementation report using this structure:

1. **Files changed**
   - list every modified file

2. **What changed**
   - short bullets grouped by:
     - artifact sync/store
     - analysis service
     - drill/heatmap panels
     - benchmark/harness clarity
     - tests

3. **Why the changes are safe**
   - explain how behavior/parity was preserved

4. **Tests/checks run**
   - exact commands run
   - pass/fail outcome

5. **Deferred optional items**
   - list any stretch items not implemented and why

6. **Remaining risk / follow-up**
   - only genuine remaining risks, not generic commentary

---

# Additional implementation notes

- Use the implementation plan and addendum as the authoritative detail source.
- If the plan and current code diverge slightly, follow the **behavioral intent** of the plan while adapting to the actual types/functions present in the repo.
- When checking `CompositeCpu` local-stability request support for `score_mode`, inspect the request type before adding the field.
- Preserve current `MlSignal` special-casing in `refreshAnalysisFromWorkflowRun()`.
- Preserve click-to-drill, hover, tooltip, crosshair, and PNG export behavior on Heatmap.
- Preserve drill status, chart render, trade-table behavior, and current selection semantics on Drill.
- Prefer the smallest safe diff that fully implements the behavior.

