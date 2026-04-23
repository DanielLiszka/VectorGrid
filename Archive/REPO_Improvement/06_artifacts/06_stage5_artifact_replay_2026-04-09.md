# Stage5 Artifact Replay Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to saved Stage 5 artifact replay so typed replay loading, alias debt, selection retention, and action-surface constraints remain explicit and stable.

## Scope Boundaries

In scope:

- `src-tauri/src/commands_artifacts.rs`
- `src-tauri/src/run_artifacts.rs`
- `web/src/services/metaService.ts`
- `web/src/components/MetaWorkspacePanel.vue`
- saved Stage 5 replay/load/reveal/delete flows

Out of scope:

- Saved Meta report-open behavior
- policy-eval replay
- options-related artifact replay

## Current User-Facing Behavior

Saved Stage 5 supports refresh, load, reveal, and delete, but intentionally does not expose `Open report`. Loading a saved Stage 5 artifact re-renders the Stage 5 result surface inside the Meta workspace and refreshes the saved family list while preserving selection intent.

There is also a naming/alias debt: the frontend still uses some `stage4Saved*` state names even though the visible family label is `Saved Stage 5`. That debt is survivable today, but it needs to be documented rather than left implicit.

## Current Code Surface

- `src-tauri/src/commands_artifacts.rs`
- `src-tauri/src/run_artifacts.rs`
- `web/src/services/metaService.ts`
- `web/src/components/MetaWorkspacePanel.vue`
- `web/src/services/metaService.test.ts`

## Current Verification Surface

- `web/src/services/metaService.test.ts`
- `web/src/components/MetaWorkspacePanel.test.ts`
- `src-tauri/src/run_artifacts.rs` replay/list tests

## Strengths Already Present

- Stage 5 replay already uses a typed backend response
- the current action surface is explicit and intentionally narrower than Saved Meta
- saved-list refresh and selection logic already exist
- family-specific labels and empty states are already explicit in the Meta workspace
- tests already cover typed replay loading and the absence of report-open behavior

## Weaknesses / Failure Modes

- Stage 4/Stage 5 alias debt is easy to worsen if not documented
- the replay response currently mirrors the same file path into both Stage 4 and Stage 5 fields, which is subtle
- future UI work could accidentally add report expectations to a family that does not currently have a real report artifact
- refresh and selection retention can drift if identity rules are not frozen

## Regressions To Avoid

- breaking the current stage4/stage5 alias compatibility without a coordinated cleanup
- assuming separate Stage 4 and Stage 5 artifact files when the current replay contract does not promise that
- accidentally adding or implying report-open support where no real Stage 5 report artifact exists
- clearing the selected Stage 5 artifact on refresh
- making Stage 5 replay depend on raw artifact structure instead of typed replay output
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [JSON Schema - What is JSON Schema?](https://json-schema.org/overview/what-is-jsonschema) is relevant because the replay response is acting as a typed contract between backend and Meta UI.
- [RFC 8259 - JSON](https://www.rfc-editor.org/rfc/rfc8259) is relevant because Stage 5 replay state depends on stable JSON result semantics.
- [Calling the Frontend from Rust | Tauri](https://v2.tauri.app/es/develop/calling-frontend/) is relevant because saved Stage 5 replay is a command-mediated state restoration path.
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because Stage 5 replay restores shared Meta state and family selection state.
- [Pinia Testing Cookbook](https://pinia.vuejs.org/cookbook/testing.html) is relevant because current guarantees around replay and selection retention are stateful and test-driven.

## Proposed Improvement Approach

1. Freeze Saved Stage 5 replay as a typed restoration contract with a deliberately narrower action surface than Saved Meta.
2. Document the Stage 4/Stage 5 alias debt and how it is tolerated today.
3. Keep `no report-open action` as an explicit product rule until a real Stage 5 report artifact exists.
4. Preserve typed replay ownership and selected-item retention through refresh.
5. Expand tests around alias compatibility, replay loading, and action-surface constraints.
6. Treat artifact-surface code quality as part of the implementation scope: keep persisted shapes version-conscious, replayable, diagnosable, and obvious to future contributors.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the typed Stage 5 replay response and how the frontend consumes it.
- [x] Document the current Stage 4/Stage 5 naming alias debt.
- [x] Document why Saved Stage 5 does not currently expose `Open report`.
- [x] Document selection-retention behavior on refresh.
- [x] Add or tighten tests for typed replay loading and alias compatibility.
- [x] Add or tighten tests proving report-open remains intentionally unavailable for this family.
- [x] Keep replay semantics separate from artifact-library indexing concerns.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/services/metaService.test.ts`
- run `npm --prefix web run test -- src/components/MetaWorkspacePanel.test.ts`
- run targeted `run_artifacts.rs` Stage 5 replay/list tests
- manually verify refresh, load, reveal, and delete flows inside Saved Stage 5
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant quality and parity gates for touched artifact/report surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the applicable output-parity or artifact-shape checks

## Rollout / Fallback / Compatibility Notes

- preserve current alias compatibility until there is a deliberate cleanup plan
- do not invent report-open semantics for Stage 5 without a real backend artifact
- prefer additive documentation and tests over renaming state casually
- if alias cleanup happens later, migrate tests and UI labels in one coordinated change
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/06_artifacts/03_artifact_library_index_2026-04-09.md`
- `REPO_Improvement/08_ml/31_stage5_result_surface_2026-04-08.md`
- `REPO_Improvement/08_ml/35_stage4_request_contract_2026-04-08.md`
- `REPO_Improvement/06_artifacts/07_policy_eval_artifact_replay_2026-04-09.md`

## Execution Notes

- Closed as a typed replay contract audit with no new code required. Existing Meta-service replay tests already prove typed Stage 5 loading, selection retention, and the narrower Saved Stage 5 action surface.
- The current Stage 4/Stage 5 alias debt remains documented but intentionally tolerated; this closure does not rename store fields or widen the product surface.
- `Open report` remains intentionally unavailable for this family, and the current UI tests already preserve that constraint.
- Acceptance evidence for closure: `npm.cmd --prefix web run test -- src/services/metaService.test.ts src/components/MetaWorkspacePanel.test.ts`, `npm.cmd --prefix web run lint`, and `npm.cmd --prefix web run typecheck`.
