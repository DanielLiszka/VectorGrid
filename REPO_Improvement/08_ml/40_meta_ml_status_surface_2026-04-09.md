# Meta ML Status Surface Plan

Last updated: 2026-04-09

## Purpose

Define the current Meta ML status surface so backend health/staleness classification, frontend formatting, button gating, and autopilot coupling remain explicit and do not get mistaken for one simple string field.

## Scope Boundaries

In scope:

- `metaMlStatus`
- ML status formatting and store patching
- ML action button gating
- autopilot interactions with ML status
- the distinction between backend status data and frontend presentation

Out of scope:

- deeper health/staleness math already covered in the dedicated health/staleness doc
- Stage 5 ranking logic
- ML Signal status surfaces
- options workflows

## Current User-Facing Behavior

The Meta workspace exposes one compact ML status area with buttons for refresh, train/update, apply, and delete. Current behavior means:

- the user sees a formatted summary string rather than a structured dashboard
- action availability depends on the latest cached status payload and current scope completeness
- autopilot can react to this status automatically after meta runs
- one surface is carrying:
  - model existence
  - stale/retrain state
  - health state
  - label thresholds
  - row counts
  - warnings/errors

## Current Code Surface

- `web/src/components/MetaWorkspacePanel.vue`
  - ML status block and action buttons
- `web/src/stores/analysisStore.ts`
  - `metaMlStatus`, action flags, and button-trigger entrypoints
- `web/src/services/metaService.ts`
  - `formatMlStatus`
  - `applyMlStatusPayload`
  - `updateMetaMlButtons`
  - autopilot behavior
- `src-tauri/src/commands_meta.rs`
  - backend status classification and status payload generation
- `crates/app_core/src/ml_confidence.rs`
  - model metrics and metadata that feed backend status classification

## Current Verification Surface

- `web/src/services/metaService.test.ts`
- `web/src/components/MetaWorkspacePanel.test.ts`
- `web/src/stores/domainStores.test.ts`
- tests embedded in `src-tauri/src/commands_meta.rs`

## Strengths Already Present

- backend status classification is already centralized before the frontend formats it
- the UI exposes actionable status rather than forcing users to inspect raw model files
- button gating already reuses the same status payload instead of ad hoc heuristics
- autopilot is tied to explicit status signals instead of implicit assumptions

## Weaknesses / Failure Modes

- `metaMlStatus` looks like a simple string but actually encodes many backend decisions
- a single text surface can hide important distinctions between legacy, stale, unhealthy, and blocked states
- the same global `statusLine` channel is shared with many unrelated meta operations, so messaging can overwrite itself
- formatting changes in the service can unintentionally change the perceived contract without any backend change

## Regressions To Avoid

- treating `metaMlStatus` as the canonical logic source rather than a formatted presentation layer
- flattening distinct backend states into one generic message
- breaking action gating during status-copy or formatting refactors
- hiding autopilot-trigger-relevant distinctions in the rendered status

## External Research Summary

- [Model Cards for Model Reporting](https://research.google/pubs/model-cards-for-model-reporting/)
  - relevant because user-facing model status should preserve interpretability around freshness and trustworthiness
- [scikit-learn Probability Calibration](https://scikit-learn.org/stable/modules/calibration.html)
  - relevant because the status surface describes confidence-model quality and should remain tied to meaningful calibration/quality signals
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because status data crosses a command boundary and should stay structurally clear before it is flattened into UI text
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because the status surface is store-backed and its ownership boundaries need to stay explicit

## Proposed Improvement Approach

1. Document the status surface as a presentation layer over a richer backend payload.
2. Record which fields in the backend status payload materially affect button gating and autopilot.
3. Keep `metaMlStatus` and global `statusLine` distinct in documentation because they have different roles.
4. Preserve explicit wording around stale, blocked, and legacy states instead of simplifying toward a single “needs attention” label.
5. Tie formatting changes to tests so presentation edits do not silently change action semantics.
6. Keep the current surface honest about its compactness: it is useful, but not a full ML model report.

## Checklist

- [ ] Document the current backend status payload fields that feed the ML status surface.
- [ ] Document `metaMlStatus` as a formatted presentation surface, not the logic source of truth.
- [ ] Document which buttons/actions depend on the cached ML status.
- [ ] Document how autopilot uses status state to decide refresh/train/apply behavior.
- [ ] Document the distinction between `metaMlStatus` and global `statusLine`.
- [ ] Preserve tests for status formatting, button gating, and autopilot interactions.
- [ ] Preserve backend status-classification tests.
- [ ] Treat major formatting simplifications as behavior-sensitive changes, not just copy edits.

## Verification Plan

- run `npm --prefix web test -- src/services/metaService.test.ts`
- run `npm --prefix web test -- src/components/MetaWorkspacePanel.test.ts`
- run `npm --prefix web test -- src/stores/domainStores.test.ts`
- run targeted status tests in `src-tauri/src/commands_meta.rs`
- manually verify button gating for:
  - missing model
  - stale model
  - blocked/unhealthy model
  - legacy model

## Rollout / Fallback / Compatibility Notes

- preserve current distinctions in the status payload even if the rendered text is later redesigned
- prefer additive structured status fields over cramming more meaning into one string
- keep autopilot tied to backend status data, not fragile string parsing

## Related Docs

- `REPO_Improvement/08_ml/33_ml_conf_health_staleness_2026-04-09.md`
- `REPO_Improvement/08_ml/45_meta_frontend_service_2026-04-09.md`
- `REPO_Improvement/08_ml/46_ml_meta_store_status_2026-04-09.md`
- `REPO_Improvement/01_shell/35_runtime_banner_surface_2026-04-08.md`
- `REPO_Improvement/01_shell/28_runtime_store_status_2026-04-08.md`
