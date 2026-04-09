# Meta Frontend Service Plan

Last updated: 2026-04-09

## Purpose

Define the current Meta frontend-service contract so orchestration ownership, result rendering, replay glue, campaign recovery, ML actions, and naming compatibility boundaries remain explicit and regression-resistant.

## Scope Boundaries

In scope:

- `metaService.ts`
- orchestration of Stage 3 meta, batch campaigns, saved replay, ML status/actions, Stage 5, and policy eval
- service-layer rendering and status composition
- compatibility alias handling concentrated in the service

Out of scope:

- ML Signal frontend service
- backend command logic except where the service contract depends on it
- options workflows

## Current User-Facing Behavior

The Meta workspace feels like one integrated workflow largely because `metaService.ts` stitches many separate backend capabilities together. Current behavior means:

- request building, command invocation, preview rendering, saved-entry management, and status composition are all centralized in one service
- the service makes Stage 3 meta, Stage 5 ML finalization, saved replay, policy eval, and batch campaigns feel like one tool
- the same service also carries most of the Stage 4/Stage 5 naming compatibility burden

## Current Code Surface

- `web/src/services/metaService.ts`
  - the main frontend orchestration owner
- `web/src/components/MetaWorkspacePanel.vue`
  - view shell over service-driven state
- `web/src/stores/analysisStore.ts`
  - canonical state surface patched and invoked by the service
- `web/src/services/metaRequestBuilder.ts`
- `web/src/services/metaStage4RequestBuilder.ts`
- `web/src/services/metaBatchRequestBuilder.ts`
- `web/src/services/metaMlScopeBuilder.ts`
  - builder inputs composed by the service
- `src-tauri/src/commands_meta.rs`
- `src-tauri/src/commands_campaign.rs`
- `src-tauri/src/commands_artifacts.rs`
  - backend command surfaces consumed by the service

## Current Verification Surface

- `web/src/services/metaService.test.ts`
- `web/src/services/metaServiceCampaignResume.test.ts`
- `web/src/services/metaServiceCampaignStrategy.test.ts`
- `web/src/components/MetaWorkspacePanel.test.ts`
- `web/src/stores/analysisStore.actions.test.ts`

## Strengths Already Present

- the repo already has one clear frontend orchestration owner for the Meta workspace
- request shaping is delegated to dedicated builders rather than being hard-coded inline everywhere
- replay, recovery, status formatting, and result rendering are all already connected in one place
- compatibility aliasing is explicit in the service rather than being hidden in many components

## Weaknesses / Failure Modes

- the service owns a very large surface:
  - request wiring
  - execution
  - replay
  - rendering
  - batch recovery
  - ML status/actions
  - Stage 5
  - policy eval
- Stage 4/Stage 5 naming drift is amplified here because this is where user-facing and compatibility layers meet
- one global `statusLine` is reused across many unrelated operations, making status coupling high
- summary/replay/report asymmetry is easy to lose sight of when everything is routed through one service

## Regressions To Avoid

- treating `metaService.ts` as a thin wrapper when it is actually a major contract owner
- renaming Stage 4 compatibility paths inside the service without migrating store, builders, and replay behavior together
- changing service rendering/status behavior without the associated tests
- collapsing distinct contracts (meta, Stage 5, policy eval, campaigns) into one oversimplified service narrative
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because this service is the main orchestrator of many frontend/backend command boundaries
- [JSON Schema: Getting Started](https://json-schema.org/learn/getting-started-step-by-step)
  - relevant because the service coordinates several distinct structured request/result families
- [AWS Well-Architected: Make All Responses Idempotent](https://docs.aws.amazon.com/wellarchitected/latest/framework/rel_prevent_interaction_failure_idempotent.html)
  - relevant because campaign recovery and replay-safe behavior are part of this service’s current contract
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because this service is tightly coupled to store-backed workflow state and should keep those boundaries explicit
- [scikit-learn `TimeSeriesSplit`](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html)
  - relevant because chronological ML and meta-evaluation assumptions should stay explicit and must not drift toward shuffled-validation behavior

## Proposed Improvement Approach

1. Document the Meta service by responsibility slices rather than only by file name.
2. Make compatibility aliasing a first-class section of the service doc.
3. Record which builder families feed the service and which backend command families it orchestrates.
4. Preserve the distinction between:
   - execution orchestration
   - replay/recovery
   - render/status composition
5. Tie service changes to the tests that exercise each responsibility slice.
6. Treat any future decomposition of `metaService.ts` as a compatibility-sensitive refactor because the service currently binds many contracts together.
7. Treat ML/meta code quality as part of the implementation scope: keep evaluation assumptions, defaults, warnings, fixture payloads, and persisted shapes explicit enough for external reviewers to audit.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document the major responsibility slices owned by `metaService.ts`.
- [ ] Document which request builders it composes.
- [ ] Document the backend command families it orchestrates.
- [ ] Document current replay/recovery ownership in the service layer.
- [ ] Document Stage 4/Stage 5 compatibility aliasing concentrated in the service.
- [ ] Document status/rendering ownership and global `statusLine` coupling.
- [ ] Preserve tests for service orchestration, campaign recovery, and result rendering.
- [ ] Treat large service refactors as compatibility-sensitive across store, component, and artifact surfaces.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [ ] Name one stable fixture, saved artifact, or replay payload plus the exact frontend and Rust commands that validate this ML/meta contract.

## Verification Plan

- run `npm --prefix web run test -- src/services/metaService.test.ts`
- run `npm --prefix web run test -- src/services/metaServiceCampaignResume.test.ts`
- run `npm --prefix web run test -- src/services/metaServiceCampaignStrategy.test.ts`
- run `npm --prefix web run test -- src/components/MetaWorkspacePanel.test.ts`
- run `npm --prefix web run test -- src/stores/analysisStore.actions.test.ts`
- manually exercise Stage 3 meta, Stage 5, saved replay, and campaign resume from the Meta workspace
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched ML/meta surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the exact replay or saved-artifact checks named by the plan

## Rollout / Fallback / Compatibility Notes

- preserve current service behavior while compatibility aliases still exist
- prefer extracting smaller service helpers over in-place semantic rewrites if decomposition work begins
- keep replay and recovery behavior explicit if the service is split later
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/08_ml/41_meta_request_builders_2026-04-09.md`
- `REPO_Improvement/08_ml/42_meta_scope_builders_2026-04-09.md`
- `REPO_Improvement/08_ml/43_meta_service_campaigns_2026-04-09.md`
- `REPO_Improvement/08_ml/46_ml_meta_store_status_2026-04-09.md`
- `REPO_Improvement/08_ml/39_stage4_stage5_saved_replay_2026-04-09.md`
