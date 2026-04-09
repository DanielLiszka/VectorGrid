# ML/Meta Store Status Plan

Last updated: 2026-04-09

## Purpose

Define the current ML/meta store-status contract so `analysisStore` ownership, action delegation, saved-family state, status strings, and Stage 4 compatibility naming remain explicit and stable.

## Scope Boundaries

In scope:

- `analysisStore.ts` state for the Meta workspace
- saved meta / Stage 5 / policy-eval state
- ML/Stage 5 status strings and action flags
- store-to-service delegation boundaries

Out of scope:

- deep service orchestration details except where store delegation depends on them
- generic non-meta results-store surfaces
- options workflows

## Current User-Facing Behavior

The Meta workspace is store-backed through `analysisStore`. Current behavior means:

- the store holds the canonical UI state for Meta inputs, saved families, ML controls, previews, and status strings
- the store queues most execution work into `metaService`
- the store still uses Stage 4 compatibility naming for many Stage 5-related fields
- status strings and enablement flags live side by side with durable-ish workflow fields

## Current Code Surface

- `web/src/stores/analysisStore.ts`
  - canonical Meta workspace state and action entrypoints
- `web/src/services/metaService.ts`
  - the service the store delegates to for most real work
- `web/src/components/MetaWorkspacePanel.vue`
  - the main consumer of store state and actions
- `web/src/stores/domainStores.test.ts`
- `web/src/stores/analysisStore.actions.test.ts`
  - main store verification surfaces

## Current Verification Surface

- `web/src/stores/analysisStore.actions.test.ts`
- `web/src/stores/domainStores.test.ts`
- `web/src/components/MetaWorkspacePanel.test.ts`
- `web/src/services/metaService.test.ts`

## Strengths Already Present

- the Meta workspace has one clear canonical store
- the store already distinguishes many separate sub-surfaces:
  - Stage 3 meta
  - batch campaigns
  - saved meta
  - saved Stage 5
  - saved policy eval
  - ML controls/status
- action methods already delegate to the service rather than duplicating orchestration logic inside the store
- legacy snapshot compatibility is already considered in store state

## Weaknesses / Failure Modes

- Stage 4 compatibility naming remains embedded in the store even though the UI says Stage 5
- durable-ish workflow fields and ephemeral status strings are mixed together, which makes the store heavier and easier to misinterpret
- store actions can look like the behavior owner when they are often only queueing service calls
- field renames here have very wide blast radius because the panel, service, tests, and persisted snapshots all depend on them

## Regressions To Avoid

- renaming `stage4Saved*` or `metaStage4*` fields without a coordinated migration
- treating service-delegating actions as pure no-risk refactors
- collapsing status strings and operational flags into fewer fields without verifying UI behavior
- breaking persisted/default snapshots while cleaning up naming
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because this store is the canonical shared state surface for a large workspace and its boundaries should remain explicit
- [Pinia Testing](https://pinia.vuejs.org/api/%40pinia/testing/)
  - relevant because store behavior and action delegation need strong test coverage when so much UI state depends on them
- [JSON Schema: Getting Started](https://json-schema.org/learn/getting-started-step-by-step)
  - relevant because persisted/default state shapes and compatibility fields should be treated as structured contracts
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because many store-backed actions ultimately drive command-boundary workflows through the service layer
- [scikit-learn `TimeSeriesSplit`](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html)
  - relevant because chronological ML and meta-evaluation assumptions should stay explicit and must not drift toward shuffled-validation behavior

## Proposed Improvement Approach

1. Document the store by state families rather than as one flat bag of fields.
2. Make service delegation explicit so future contributors do not mistake store actions for orchestration ownership.
3. Preserve and document the Stage 4 compatibility naming still embedded in Stage 5-related store state.
4. Separate in documentation between:
   - canonical workflow state
   - saved-family state
   - status/preview strings
   - action flags
5. Tie any rename or cleanup work to store tests, panel tests, service tests, and persisted snapshot expectations.
6. Keep legacy snapshot compatibility visible until the repo intentionally migrates those fields.
7. Treat ML/meta code quality as part of the implementation scope: keep evaluation assumptions, defaults, warnings, fixture payloads, and persisted shapes explicit enough for external reviewers to audit.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document the major Meta workspace state families in `analysisStore`.
- [ ] Document Stage 4 compatibility naming that still backs Stage 5 state.
- [ ] Document which store actions only queue `metaService` calls.
- [ ] Document the distinction between persistent-ish workflow state and ephemeral status strings.
- [ ] Preserve tests for store actions, defaults, and domain-store wiring.
- [ ] Preserve panel tests that depend on store field naming and availability flags.
- [ ] Treat store field renames as compatibility-sensitive across UI, service, and persisted snapshots.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [ ] Name one stable fixture, saved artifact, or replay payload plus the exact frontend and Rust commands that validate this ML/meta contract.

## Verification Plan

- run `npm --prefix web run test -- src/stores/analysisStore.actions.test.ts`
- run `npm --prefix web run test -- src/stores/domainStores.test.ts`
- run `npm --prefix web run test -- src/components/MetaWorkspacePanel.test.ts`
- run `npm --prefix web run test -- src/services/metaService.test.ts`
- manually inspect saved Stage 5 and policy-eval flows after any store-shape change
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched ML/meta surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the exact replay or saved-artifact checks named by the plan

## Rollout / Fallback / Compatibility Notes

- preserve current field names while Stage 4 compatibility still exists across service and artifact surfaces
- if status fields are later separated from more durable workflow state, do so additively first
- prefer explicit migration notes over silent state-key renames
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/08_ml/40_meta_ml_status_surface_2026-04-09.md`
- `REPO_Improvement/08_ml/41_meta_request_builders_2026-04-09.md`
- `REPO_Improvement/08_ml/45_meta_frontend_service_2026-04-09.md`
- `REPO_Improvement/01_shell/28_runtime_store_status_2026-04-08.md`
- `REPO_Improvement/01_shell/29_storage_schema_local_state_2026-04-08.md`
