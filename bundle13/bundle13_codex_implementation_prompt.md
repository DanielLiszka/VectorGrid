# Codex Implementation Prompt — Bundle 13 Analysis / Drilldown / Artifact-Hydration Responsiveness

## Usage
Use this document as the prompt for an OpenAI Codex agent that has access to this repository and the Bundle 13 guide markdown files.

When available, also provide these reference docs:
- `bundle13_analysis_drilldown_artifact_hydration_plan.md`
- `bundle13_analysis_drilldown_artifact_hydration_addendum.md`
- `gpt54_feature_review_map.md` with focus on `## 13. Analysis/drilldown and artifact-hydration responsiveness`

This tracker is repo-validated. Treat it as the implementation scope guard for Bundle 13.

---

# Prompt to give Codex

## Role and mission
You are implementing the repo-valid portions of Bundle 13: analysis/drilldown and artifact-hydration responsiveness.

Implement only the valid portions of:
- `bundle13_analysis_drilldown_artifact_hydration_plan.md`
- `bundle13_analysis_drilldown_artifact_hydration_addendum.md`
- `gpt54_feature_review_map.md` Bundle 13 scope guard

This is an implementation task, not a review task. Make the code changes, extend targeted tests, run the relevant checks, and preserve current product behavior.

## Authoritative scope
Stay inside Bundle 13:
- workflow-artifact hydration and sync
- compare / baseline loading
- analysis Top-table / overview refresh efficiency
- drilldown and local-stability request shaping, caching, and stale-render suppression
- heatmap and drill panel reveal responsiveness
- supporting benchmark / workflow-harness timing-boundary clarity

## Hard constraints
1. Preserve behavior and parity. Do not remove features.
2. Do not broaden scope into a chart rewrite, worker rewrite, or new UI surface.
3. Keep `MlSignal` outside the optimizer-native analysis surfaces.
4. Prefer small safe wins before deeper refactors.
5. Keep backend contract changes out of the core pass unless they are tiny and clearly required.
6. Be honest about timing boundaries. Criterion and workflow-regression evidence are not app-path hydration proof.

## Repo-state completion status

Treat these repo facts as already validated in the current worktree:
- [x] The main Bundle 13 implementation surfaces were correctly scoped to `web/src/stores/workflowArtifactsStore.ts`, `web/src/services/workflowArtifactsSync.ts`, `web/src/services/analysisService.ts`, `web/src/components/DrillWorkspacePanel.vue`, and `web/src/components/HeatmapWorkspacePanel.vue`.
- [x] `workflowArtifactsStore.ts` now returns a structured apply result, retains `ts` / `artifactPath` in the workflow-run snapshot, exposes `readLatestWorkflowRunIdentitySnapshot()`, and treats identical normalized payloads as a no-op without `updatedAtUnixMs` churn.
- [x] `workflowArtifactsSync.ts` now does a conservative pre-load skip, preserves the same-path / same-ts / validation-arrived-later edge case, and only refreshes analysis when `runChanged` or `validationChanged` is true.
- [x] `analysisService.ts` now uses named cache bounds, pending-promise dedupe, bounded baseline / price-bar caches, score-context-correct Top-table refresh, allowlisted local-stability request shaping, stale drill guards, drill restore, and heatmap visible-restore behavior.
- [x] `buildLocalStabilityInvocation(...)` now omits drill-only fields such as `bins`, uses `objectiveOf(runReq)`, propagates Double-MA `score_mode`, and keeps Composite CPU on the currently supported request shape.
- [x] `CompositeCpuLocalStabilityRequest` in `crates/app_core/src/composite_cpu.rs` still does not support `score_mode`, and this bundle correctly leaves that backend contract unchanged.
- [x] `DrillWorkspacePanel.vue` now uses visibility-aware restore behavior instead of a mount-time rerender nudge.
- [x] `HeatmapWorkspacePanel.vue` now prefers the cheap visible-restore path before falling back to a full refresh.
- [x] `src-tauri/src/commands_analysis.rs` remains a thin command facade. Bundle 13 did not require a backend contract change there.
- [x] `validationService.ts` and `executionService.ts` remain compatible with the richer workflow-run identity helpers.
- [x] `historyService.ts`, `optimizeService.ts`, `metaService.ts`, and `mlSignalService.ts` remain inspect-only compatibility surfaces and did not require direct changes.
- [x] `crates/app_core/benches/double_ma_followup_workflow.rs` now explicitly states that its timing is engine-side follow-up work only and excludes app-path artifact hydration / panel responsiveness.
- [x] `src-tauri/src/bin/ta_workflow_regression.rs` now exposes `timing_boundary_exclusions` with serde-default compatibility and preserves legacy payload deserialization.
- [x] Focused test coverage now exists for the core web surfaces: `web/src/services/analysisService.test.ts`, `web/src/services/workflowArtifactsSync.test.ts`, and `web/src/stores/workflowArtifactsStore.test.ts`.
- [x] Panel-level tests were not required because the visibility-aware behavior is covered through the service-level restore tests and the panel watchers stay thin.

## Required files

### Required code files
- `web/src/stores/workflowArtifactsStore.ts`
- `web/src/services/workflowArtifactsSync.ts`
- `web/src/services/analysisService.ts`
- `web/src/components/DrillWorkspacePanel.vue`
- `web/src/components/HeatmapWorkspacePanel.vue`

### Required benchmark / harness files
- `crates/app_core/benches/double_ma_followup_workflow.rs`
- `src-tauri/src/bin/ta_workflow_regression.rs`

### Inspect-only / touch only if clearly needed
- `src-tauri/src/commands_analysis.rs`
- `web/src/services/validationService.ts`
- `web/src/services/executionService.ts`
- `web/src/services/historyService.ts`
- `web/src/services/optimizeService.ts`
- `web/src/services/metaService.ts`
- `web/src/services/mlSignalService.ts`

## Repo-valid exclusions from the required set

These ideas were reviewed and should not be treated as required Bundle 13 implementation work:
- [x] Do not require a combined drilldown + local-stability backend command in `commands_analysis.rs`.
- [x] Do not require a backend latest-artifact head / digest command in this bundle.
- [x] Do not require dev-only timing instrumentation in `analysisService.ts`.
- [x] Do not require hidden Pareto render suppression. It remains optional backlog work.
- [x] Do not require speculative same-`data_id` dataset-mutation invalidation policy in this bundle. An exported cache invalidation hook is optional only if it falls out naturally from the drill-restore work.
- [x] Do not turn this into a chart-stack migration, worker rewrite, or broader analysis-surface redesign.

## Implementation order
Follow this order unless a safer adjacent variant is obviously better:
1. artifact sync / store identity and no-op behavior
2. `analysisService.ts` correctness fixes and hot-path dedupe
3. drill and heatmap reveal restore behavior
4. benchmark / harness timing-boundary clarity
5. targeted tests and verification

## Active implementation checklist

### Artifact sync / store
- [x] Add a structured workflow-artifact apply result in `workflowArtifactsStore.ts` with at least `applied`, `runChanged`, `validationChanged`, and `sourceChanged`.
- [x] Extend the latest workflow-run snapshot to retain `ts` and `artifactPath` in addition to `req` / `res`.
- [x] Add deterministic normalized signatures for run and validation payloads plus a `readLatestWorkflowRunIdentitySnapshot()` helper exposing at least `ts`, `artifactPath`, and `hasValidation`.
- [x] Make `applyNormalizedArtifacts(...)` a true no-op for identical normalized run / validation payloads. Identical payloads must not churn `updatedAtUnixMs`.
- [x] Preserve the later-validation edge case so same run content with later validation reports `runChanged = false` and `validationChanged = true`.
- [x] Update `applyWorkflowArtifactsPayload(...)` to return the structured apply result instead of a bare boolean.
- [x] In `workflowArtifactsSync.ts`, add a conservative pre-load skip before `load_run_artifact` based on latest `path` + `ts` versus the currently loaded artifact identity.
- [x] Preserve the same-path / same-ts / validation-arrived-later edge case by forcing a load when local validation is still absent.
- [x] Gate `refreshAnalysisFromWorkflowRun()` on meaningful apply results only: refresh when `runChanged` or `validationChanged` is true, not merely when parsing succeeded.

### Analysis service correctness and hot-path work
- [x] Make the cache helper generic and convert the cache-size policy into named constants.
- [x] Keep bounded behavior for existing drill and stability caches.
- [x] Add bounds to `baselineArtifactCache` and `drillPriceBarsCache`.
- [x] Add pending-promise dedupe maps plus a shared helper for drill curves, local stability, price bars, and baseline artifact loads.
- [x] Reuse the shared dedupe helper in `runVueDrillPipeline()`, `fetchDrillPriceBars()`, and `loadBaselineFromDisk()`.
- [x] Deduplicate Top-table refresh work so overview refresh happens once and the row pipeline (`scoreContext`, `collected`, `sorted`, `filtered`) is computed once per refresh path.
- [x] Ensure `sortRows(...)` receives `scoreObservationContextOf(res)` on the refresh path so score-context-sensitive modes stay consistent.
- [x] Refactor `buildTopRowModelsForCurrentFilters()` or an equivalent helper to accept the precomputed sorted rows instead of recollecting / resorting.
- [x] Reuse the single sorted / filtered result for caches, selection reconciliation, pinned-row reconciliation, render rows, and filter metadata.
- [x] Fix local-stability request shaping to use an explicit allowlist instead of spreading `...invocation.request`.
- [x] Remove drill-only fields such as `bins` from local-stability requests so drill-only UI fields do not pollute stability cache keys.
- [x] Use `objectiveOf(runReq)` for local stability.
- [x] Propagate `score_mode` for Double-MA local-stability requests.
- [x] Do not add `score_mode` to Composite CPU local-stability requests unless the backend type is extended in the same patch. Current repo state says it is unsupported.
- [x] Add a stale-sequence guard after `fetchDrillPriceBars()` and before any drill DOM / status / store mutation.
- [x] Add any second guard needed before drill-side status / meta writes so rapid selection changes cannot paint stale drill content.
- [x] Add a drill render-state cache keyed by run identity plus selected row identity.
- [x] Change `rerenderDrill()` to prefer restore over recompute for the same run identity and same selected row.
- [x] Add an exported `invalidateAnalysisExecutionCaches()` hook only if it materially helps the drill-restore implementation and stays local to the touched files. Do not treat speculative same-`data_id` dataset-mutation invalidation as a mandatory acceptance item.
- [x] Add a heatmap base-render signature and visible-restore path so repeated reveal can skip repainting the base grid when the underlying artifact / render identity is unchanged.
- [x] Preserve heatmap hover tooltip, crosshair, marker rendering, click-to-drill, and PNG export behavior while adding the visible-restore optimization.

### Panel reveal behavior
- [x] Update `DrillWorkspacePanel.vue` so repeated reveal prefers the cheaper drill restore path rather than re-entering the full selection / pipeline path.
- [x] Remove the mount-time `setTimeout(...rerenderDrill...)` nudge in favor of visibility-aware restore behavior.
- [x] Update `HeatmapWorkspacePanel.vue` so reveal uses the cheap visible-render helper when the underlying heatmap identity is unchanged.
- [x] Preserve current mount / visible semantics from the user's perspective while avoiding unnecessary full refresh work.

### Benchmark and harness evidence hygiene
- [x] Update `crates/app_core/benches/double_ma_followup_workflow.rs` metadata so it clearly states this is an engine-side Criterion follow-up benchmark and not proof of Tauri / Vue artifact hydration, Top-table rebuild, or panel reveal responsiveness.
- [x] Update `src-tauri/src/bin/ta_workflow_regression.rs` to add `timing_boundary_exclusions` with serde-default compatibility.
- [x] Make the workflow-regression exclusions explicitly mention that the harness excludes Bundle 13 app-path work such as Tauri analysis commands, workflow-artifact sync / apply, Top-table hydration, heatmap rendering, and drill / stability panel hydration.
- [x] Update the existing legacy-deserialization test so older payloads still deserialize safely.

## Verification checklist

### Web tests
- [x] Extend `web/src/services/workflowArtifactsSync.test.ts` for the sync pre-load skip and validation-arrived-later boundary.
- [x] Add focused coverage for workflow-artifact no-op apply behavior via `web/src/stores/workflowArtifactsStore.test.ts`.
- [x] Extend `web/src/services/analysisService.test.ts` for local-stability cache-key behavior, Double-MA `score_mode` propagation, Top-table score-context consistency, drill restore behavior, heatmap visible-render behavior, and in-flight dedupe where practical.
- [x] Panel-level tests were not required because the visibility-aware restore behavior is covered cleanly through the service tests and the panel components stay thin.

### Rust / harness tests
- [x] Update the legacy-deserialization test in `src-tauri/src/bin/ta_workflow_regression.rs` for the new exclusions field.
- [x] Compile the touched bench / harness files after the metadata changes.

### Commands to run
- [x] Run the narrowest relevant web tests for touched files.
- [x] Run web typecheck for touched frontend changes.
- [x] Run the narrowest relevant Rust test for `ta_workflow_regression.rs`.
- [x] Run `cargo bench -p ta_app_core --bench double_ma_followup_workflow --no-run`.
- [x] Run `powershell -File scripts/run_cpu_perf_gate.ps1 -Mode candidate` before handoff because the final patch changes performance-sensitive behavior.

## Optional deferred checklist

- [x] Hidden Pareto render suppression remains optional backlog work.
- [x] Combined follow-up backend commands remain optional backlog work.
- [x] Backend latest-artifact head / digest paths remain optional backlog work.
- [x] Dev-only app-path timing instrumentation remains optional backlog work.
- [x] Drill-bin bucketing and optional test-shim exports remain optional backlog work.

## Final response requirements
Report:
1. files changed
2. what changed, grouped by store/sync, analysis service, panels, and evidence-hygiene files
3. why parity is preserved
4. exact tests and commands run, with outcomes
5. deferred optional items, if any
6. genuine remaining risks only
