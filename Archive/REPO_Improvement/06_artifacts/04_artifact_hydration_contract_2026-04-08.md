# Artifact Hydration Contract Plan

Last updated: 2026-04-09

## Purpose

Define the current workflow-artifact hydration contract so saved run artifacts can still repopulate current-run and current-validation state without replay drift or store-shape mismatch.

## Scope Boundaries

In scope:

- `load_run_artifact_hydration`
- hydration payload shape
- workflow artifact sync behavior
- current-run/current-validation restoration boundaries

Out of scope:

- full artifact detail payloads
- full history detail rendering
- options artifact hydration

## Current User-Facing Behavior

The app can restore the current workflow shell from the latest saved run artifact without forcing a full manual replay. That path uses a dedicated hydration payload rather than the raw artifact JSON. The hydration payload includes a current-run subtree and an optional current-validation subtree, and `workflowArtifactsSync` applies it into the workflow artifact store. Analysis refresh can then rebuild from the hydrated workflow state.

## Current Code Surface

- `src-tauri/src/commands_artifacts.rs`
  - exposes `load_run_artifact_hydration`
- `src-tauri/src/run_artifacts.rs`
  - builds `WorkflowArtifactsHydrationPayload`, deriving current-run and current-validation payloads from the saved artifact
- `web/src/services/workflowArtifactsSync.ts`
  - loads the latest artifact row, skips redundant hydration when identity matches, calls hydration, applies the payload, and refreshes analysis when needed
- `web/src/stores/workflowArtifactsStore.ts`
  - normalizes and stores hydrated current-run/current-validation payloads
- `web/src/services/historyService.ts`
  - consumes related artifact detail paths and replay flows that must stay compatible with hydration behavior

## Current Verification Surface

- `src-tauri/src/run_artifacts.rs` tests
- `web/src/services/workflowArtifactsSync.test.ts`
- `web/src/services/historyService.test.ts`
- `web/src/stores/workflowArtifactsStore` normalization coverage

## Strengths Already Present

- hydration is already a dedicated payload shape instead of an ad hoc partial artifact read
- hydration can skip work when the latest artifact identity is already applied
- analysis refresh is already tied to meaningful hydration changes
- hydrated current-run payload already carries `dataset_meta`, `ml_signal`, tags, and optimization-metric-surface data

## Weaknesses / Failure Modes

- hydration payload shape is smaller than detail payload shape, so field assumptions can drift
- current-run and current-validation restoration rules are code-only today
- identity-based skip logic is useful but easy to misunderstand if the store contract is not explicit
- hydration depends on raw artifact structure remaining compatible with the payload builder

## Regressions To Avoid

- breaking `load_run_artifact_hydration` while leaving list/detail APIs intact
- silently dropping `dataset_meta`, tags, or `ml_signal` from hydrated current-run state
- changing validation hydration semantics without updating workflow sync and analysis refresh
- making hydration depend on fields that are not guaranteed in currently valid saved artifacts
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [JSON Schema: Getting Started](https://json-schema.org/learn/getting-started-step-by-step)
  - relevant because hydration is an explicit cross-layer payload shape
- [RFC 8259](https://datatracker.ietf.org/doc/html/rfc8259)
  - relevant because hydration payloads derive from persisted JSON artifacts
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because hydration is a command-mediated boundary crossing
- [Vue: State Management](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because hydration is fundamentally shared workflow state restoration
- [JSON Schema: What is JSON Schema?](https://json-schema.org/overview/what-is-jsonschema)
  - relevant because artifacts, reports, manifests, and replay payloads behave like versioned serialized contracts that should stay explicit and reviewable

## Proposed Improvement Approach

1. Document hydration as a dedicated contract, not a side effect of detail loading.
2. Separate the payload into:
   - current run
   - current validation
   - identity/skip logic
3. Document which artifact fields are preserved into hydration and which stay detail-only.
4. Tie hydration contract changes to workflow sync and analysis refresh behavior in the same plan.
5. Treat artifact-surface code quality as part of the implementation scope: keep persisted shapes version-conscious, replayable, diagnosable, and obvious to future contributors.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the `load_run_artifact_hydration` command contract.
- [x] Document the current `current_run` payload fields.
- [x] Document when `current_validation` is present or absent.
- [x] Document identity-based skip behavior in `workflowArtifactsSync`.
- [x] Document how hydration interacts with workflow artifact store normalization.
- [x] Document which fields remain detail-only and should not be assumed in hydration.
- [x] Add or tighten tests if hydration shape or skip logic changes.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run targeted tests in `src-tauri/src/run_artifacts.rs`
- run `npm --prefix web run test -- src/services/workflowArtifactsSync.test.ts`
- run `npm --prefix web run test -- src/services/historyService.test.ts`
- run `npm --prefix web run test -- src/stores/workflowStore.test.ts`
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant quality and parity gates for touched artifact/report surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the applicable output-parity or artifact-shape checks

## Rollout / Fallback / Compatibility Notes

- preserve hydration payload compatibility even if artifact detail grows
- keep identity-based skip behavior conservative rather than over-eager
- if hydration shape changes, update both builder and frontend normalization together
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closed as a payload-contract hardening slice. The dedicated hydration command in `src-tauri/src/commands_artifacts.rs` and builder in `src-tauri/src/run_artifacts.rs` already expose a stable `current_run` / optional `current_validation` payload that `workflowArtifactsSync` and `historyService` consume directly.
- Added a direct Rust regression test for the no-validation case so hydration now has explicit coverage for preserving `report_xlsx_path`, normalized tags, `dataset_meta`, and `ml_signal` while omitting `current_validation` when the source artifact has no validation subtree.
- Re-verified the frontend consumers that make hydration meaningful: `workflowArtifactsSync.test.ts` still covers identity-based skip/load behavior, and `historyService.test.ts` still proves saved-run hydration repopulates the workflow artifact store and refreshes analysis when appropriate.
- Acceptance evidence for closure: `cargo test -p ta_desktop_demo_app workflow_hydration_payload_preserves_report_tags_and_ml_signal_without_validation --lib`, `npm.cmd --prefix web run test -- src/stores/workflowArtifactsStore.test.ts src/services/workflowArtifactsSync.test.ts src/services/historyService.test.ts`, `npm.cmd --prefix web run lint`, `npm.cmd --prefix web run typecheck`, and `cargo check -p ta_desktop_demo_app`.

## Related Docs

- `REPO_Improvement/06_artifacts/01_run_artifact_schema_2026-04-08.md`
- `REPO_Improvement/06_artifacts/29_artifact_identity_workflow_model_2026-04-08.md`
- `REPO_Improvement/02_data/22_dataset_provenance_2026-04-08.md`
- `REPO_Improvement/05_validation/19_validation_request_builder_2026-04-08.md`
