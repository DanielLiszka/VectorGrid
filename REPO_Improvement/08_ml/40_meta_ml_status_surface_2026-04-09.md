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
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Model Cards for Model Reporting](https://research.google/pubs/model-cards-for-model-reporting/)
  - relevant because user-facing model status should preserve interpretability around freshness and trustworthiness
- [scikit-learn Probability Calibration](https://scikit-learn.org/stable/modules/calibration.html)
  - relevant because the status surface describes confidence-model quality and should remain tied to meaningful calibration/quality signals
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because status data crosses a command boundary and should stay structurally clear before it is flattened into UI text
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because the status surface is store-backed and its ownership boundaries need to stay explicit
- [scikit-learn `TimeSeriesSplit`](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html)
  - relevant because chronological ML and meta-evaluation assumptions should stay explicit and must not drift toward shuffled-validation behavior

## Proposed Improvement Approach

1. Document the status surface as a presentation layer over a richer backend payload.
2. Record which fields in the backend status payload materially affect button gating and autopilot.
3. Keep `metaMlStatus` and global `statusLine` distinct in documentation because they have different roles.
4. Preserve explicit wording around stale, blocked, and legacy states instead of simplifying toward a single “needs attention” label.
5. Tie formatting changes to tests so presentation edits do not silently change action semantics.
6. Keep the current surface honest about its compactness: it is useful, but not a full ML model report.
7. Treat ML/meta code quality as part of the implementation scope: keep evaluation assumptions, defaults, warnings, fixture payloads, and persisted shapes explicit enough for external reviewers to audit.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the current backend status payload fields that feed the ML status surface.
- [x] Document `metaMlStatus` as a formatted presentation surface, not the logic source of truth.
- [x] Document which buttons/actions depend on the cached ML status.
- [x] Document how autopilot uses status state to decide refresh/train/apply behavior.
- [x] Document the distinction between `metaMlStatus` and global `statusLine`.
- [x] Preserve tests for status formatting, button gating, and autopilot interactions.
- [x] Preserve backend status-classification tests.
- [x] Treat major formatting simplifications as behavior-sensitive changes, not just copy edits.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Name one stable fixture, saved artifact, or replay payload plus the exact frontend and Rust commands that validate this ML/meta contract.

## Verification Plan

- run `npm --prefix web run test -- src/services/metaService.test.ts`
- run `npm --prefix web run test -- src/components/MetaWorkspacePanel.test.ts`
- run `npm --prefix web run test -- src/stores/domainStores.test.ts`
- run targeted status tests in `src-tauri/src/commands_meta.rs`
- manually verify button gating for:
  - missing model
  - stale model
  - blocked/unhealthy model
  - legacy model
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched ML/meta surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the exact replay or saved-artifact checks named by the plan

## Rollout / Fallback / Compatibility Notes

- preserve current distinctions in the status payload even if the rendered text is later redesigned
- prefer additive structured status fields over cramming more meaning into one string
- keep autopilot tied to backend status data, not fragile string parsing
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closed by making the ML status surface describe its own ownership boundaries instead of leaving them implicit in `formatMlStatus()` and button gating code.
- `MetaWorkspacePanel.vue` now states that:
  - `metaMlStatus` is a formatted model-status surface
  - `statusLine` is action feedback
  - button gating depends on cached backend status and scope completeness
  - autopilot reacts after meta runs rather than parsing UI text
- `MetaWorkspacePanel.test.ts` and `metaService.test.ts` now pin the richer ML/status contract text so later copy cleanup cannot silently collapse stale, blocked, legacy, or replay distinctions.
- `REPO_MAP.md` was updated to route contributors to the Stage 5 / policy-eval / ML-status contract notes quickly.
- Stable fixture / replay payload:
  - `Model: ml_confidence_CompositeMo_1m.json · state=warning · model_kind=logistic`
  - `target=holdout_dataset_pass`
  - `available: meta=10, rows=300`
- Acceptance evidence:
  - `npm.cmd --prefix web run test -- src/components/MetaWorkspacePanel.test.ts src/services/metaService.test.ts src/services/metaStage4RequestBuilder.test.ts`
  - `npm.cmd --prefix web run lint`
  - `npm.cmd --prefix web run typecheck`
  - `cargo test -p ta_desktop_demo_app commands_meta --lib`
  - `cargo test -p ta_app_core ml_confidence --lib`

## Related Docs

- `REPO_Improvement/08_ml/33_ml_conf_health_staleness_2026-04-09.md`
- `REPO_Improvement/08_ml/45_meta_frontend_service_2026-04-09.md`
- `REPO_Improvement/08_ml/46_ml_meta_store_status_2026-04-09.md`
- `REPO_Improvement/01_shell/35_runtime_banner_surface_2026-04-08.md`
- `REPO_Improvement/01_shell/28_runtime_store_status_2026-04-08.md`
