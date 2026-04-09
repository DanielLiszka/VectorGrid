# Optimization Metric Surface Artifact Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to optimization metric-surface artifact attachment so saved runs preserve enough metric-surface context for history/detail/hydration consumers without corrupting legacy compatibility.

## Scope Boundaries

In scope:

- `src-tauri/src/run_artifacts.rs`
- `src-tauri/src/metric_surface.rs`
- `web/src/services/historyService.ts`
- `web/src/stores/workflowArtifactsStore.ts`
- artifact persistence of `optimization_metric_surface`

Out of scope:

- analysis tab rendering details
- non-artifact live analysis recomputation
- options-related metric-surface artifacts

## Current User-Facing Behavior

Saved run artifacts can already carry `optimization_metric_surface`, and both hydration and detail payloads preserve it when present. History detail uses that data to build summary/detail strings, while workflow-artifact hydration threads it into the current run state so later analysis consumers can rely on it without reparsing raw artifact JSON.

Legacy artifacts can omit this field, and the current code explicitly preserves that absence instead of inventing synthetic surface data.

## Current Code Surface

- `src-tauri/src/run_artifacts.rs`
- `src-tauri/src/metric_surface.rs`
- `web/src/services/historyService.ts`
- `web/src/stores/workflowArtifactsStore.ts`

## Current Verification Surface

- `src-tauri/src/run_artifacts.rs` tests
- `web/src/services/historyService.test.ts`
- `web/src/stores/workflowArtifactsStore.test.ts`

## Strengths Already Present

- metric-surface attachment is already centralized in artifact save/hydration/detail code
- legacy artifacts can still load without the field
- history detail already knows how to summarize the surface compactly
- workflow hydration already threads the field into current-run state when present
- backend tests already cover present-versus-absent behavior

## Weaknesses / Failure Modes

- metric-surface attachment is adjacent to but distinct from main run/validation payloads, so it can be dropped accidentally
- history/detail and hydration consumers could drift if one path changes first
- the field is optional, making silent absence easy to overlook in new code
- field meaning is under-documented relative to its current product value

## Regressions To Avoid

- dropping `optimization_metric_surface` from saved artifacts when it is available
- inventing fake metric-surface data for legacy artifacts
- making hydration and detail disagree on the field shape
- changing summary/detail formatting assumptions without updating history consumers
- letting metric-surface attachment become options-contaminated in this non-options program

## External Research Summary

- [JSON Schema - What is JSON Schema?](https://json-schema.org/overview/what-is-jsonschema) is relevant because `optimization_metric_surface` is effectively a typed optional artifact subtree.
- [RFC 8259 - JSON](https://www.rfc-editor.org/rfc/rfc8259) is relevant because absent-versus-present optional fields matter for compatibility.
- [RFC 8785 - JSON Canonicalization Scheme](https://www.rfc-editor.org/rfc/rfc8785) is relevant because stable metric-surface structure supports deterministic detail and diff behavior.
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because hydrated metric-surface data becomes shared workflow state.
- [Pinia Testing Cookbook](https://pinia.vuejs.org/cookbook/testing.html) is relevant because current guarantees are exercised via stateful history/store tests.

## Proposed Improvement Approach

1. Freeze `optimization_metric_surface` as an optional but first-class artifact subtree.
2. Document when it should be attached, when it may be absent, and how legacy artifacts behave.
3. Keep hydration and detail consumers aligned to the same field shape.
4. Preserve the current product rule that absence is meaningful for older artifacts.
5. Expand tests around save/detail/hydration parity for this field.

## Checklist

- [ ] Document the `optimization_metric_surface` artifact subtree and when it is attached.
- [ ] Document legacy-artifact behavior when the field is absent.
- [ ] Document hydration/detail parity expectations for this field.
- [ ] Add or tighten tests for present-versus-absent behavior.
- [ ] Add or tighten tests for history summary/detail formatting from metric-surface data.
- [ ] Keep options-specific metric-surface work out of this non-options contract.

## Verification Plan

- run targeted `run_artifacts.rs` tests
- run `npm --prefix web test -- src/services/historyService.test.ts`
- run `npm --prefix web test -- src/stores/workflowArtifactsStore.test.ts`
- manually inspect one saved-run detail view with metric-surface metadata and one legacy artifact without it

## Rollout / Fallback / Compatibility Notes

- preserve optional-field semantics for legacy artifacts
- prefer additive hardening over making the field mandatory
- if the field shape changes, update builder, detail, and hydration consumers together
- keep this contract scoped to non-options run artifacts

## Related Docs

- `REPO_Improvement/06_artifacts/01_run_artifact_schema_2026-04-08.md`
- `REPO_Improvement/06_artifacts/12_art_hydration_detail_replay_2026-04-09.md`
- `REPO_Improvement/04_results/09_results_analysis_contract_2026-04-09.md`
- `REPO_Improvement/05_validation/23_metric_surface_contract_2026-04-08.md`
