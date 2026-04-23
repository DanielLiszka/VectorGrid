# Dataset Context Surface Plan

Last updated: 2026-04-09

## Purpose

Define the improvement plan for the frontend dataset-context surface so the active dataset, timeframe, field availability, source path, and restore state are shown consistently across the shell and dependent workflows.

## Scope Boundaries

In scope:

- `web/src/stores/dataStore.ts`
- user-visible dataset context fields
- dataset readiness and shell-context display
- startup restore cues and base/current dataset surface semantics

Out of scope:

- low-level CSV parsing
- low-level resample execution internals
- options datasets

## Current User-Facing Behavior

The data store already exposes a rich dataset-context surface. Current user-facing fields include:

- `dataId`
- `datasetLabel`
- `datasetRange`
- `dataLen`
- `dataTimeframe`
- `dataTimeframeHint`
- `datasetContext`
- `dataFields`
- `dataFieldHint`
- `csvPath`
- `baseDataId`
- `baseDataMeta`
- `baseCsvPath`
- `baseDatasetLabel`
- `baseDatasetContext`

The shell and dependent panels already use those fields to determine whether data is loaded, what timeframe is active, and whether derived timeframe context or field limitations should be shown. The store also carries startup-restore posture and recent-path information.

## Current Code Surface

- `web/src/stores/dataStore.ts`
- `web/src/services/dataService.ts`
- `web/src/components/VueWorkspaceShell.vue`
- `web/src/components/ValidationWorkspacePanel.vue`
- `web/src/services/optimizeService.ts`
- `web/src/services/validationService.ts`
- `web/src/utils/core/datasetMeta.ts`

## Current Verification Surface

- data-store action tests
- `web/src/services/validationService.test.ts`
- `web/src/services/optimizeService.test.ts`
- shell/component tests that consume dataset readiness or context

## Strengths Already Present

- the store already separates current dataset state from base dataset state
- timeframe hints and field hints already exist
- dataset context already includes source/load-path flavor text instead of only raw paths
- startup restore and recent-path behavior are already represented in state
- optimization and validation services already consume dataset context for artifact and compatibility logic

## Weaknesses / Failure Modes

- dataset-context fields are rich enough to drift in meaning if not documented explicitly
- the difference between base and current/derived dataset state is easy to blur
- startup restore cues can become misleading if not kept aligned with actual load state
- field-hint and context text can diverge from actual capability gating if they are treated as cosmetic copy

## Regressions To Avoid

- showing a derived/current timeframe as if it were the base dataset
- losing field-availability hints that currently explain disabled capabilities
- breaking startup-restore or recent-path context cues
- letting shell context and service-level artifact context disagree
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because dataset context is shared across shell, optimize, and validation surfaces
- [Pinia State](https://pinia.vuejs.org/core-concepts/state.html)
  - relevant because dataset-context fields are an explicit shared contract, not incidental locals
- [PROV-Overview](https://www.w3.org/TR/prov-overview/)
  - relevant because dataset context is really a user-facing provenance summary
- [Window.localStorage](https://developer.mozilla.org/en-US/docs/Web/API/Window/localStorage)
  - relevant because startup snapshot and related dataset preferences are persisted client-side
- [RFC 4180](https://www.rfc-editor.org/rfc/rfc4180)
  - relevant because CSV-facing import, export, and repair behavior should remain interoperable and explicit instead of relying on provider-specific happy paths

## Proposed Improvement Approach

1. Define dataset context as a UI contract with explicit field families:
   - identity
   - timeframe
   - range/length
   - source/load path
   - field availability
   - startup restore
2. Document the difference between base dataset fields and current active dataset fields.
3. Treat `datasetContext`, `datasetRange`, and `dataFieldHint` as authoritative explanatory surfaces, not disposable copy.
4. Align shell-visible dataset context with artifact/meta generation so users are never shown conflicting dataset identity.
5. Add tests around startup restore, base/current timeframe divergence, and field-hint updates.
6. Treat data-surface code quality as part of the implementation scope: preserve explicit schemas, path safety, provenance clarity, diagnostics, and testable normalization rules rather than relying on permissive parsing.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the full dataset-context field inventory and meanings.
- [x] Document base versus current dataset surface semantics.
- [x] Document startup-restore and recent-path behavior.
- [x] Document field-availability and timeframe-hint behavior.
- [x] Add tests for shell-visible dataset context after load, clear, and timeframe change.
- [x] Add tests for startup restore and base/current context transitions.
- [x] Add tests proving artifact/meta builders stay aligned with visible dataset context.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run relevant data-store tests
- run `npm test -- src/services/optimizeService.test.ts`
- run `npm test -- src/services/validationService.test.ts`
- run shell/component tests that consume dataset readiness
- manually verify load, clear, startup restore, and timeframe change flows in the app shell
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched data surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and `cargo check --workspace` or a justified narrower target
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve current dataset-context field names unless a deliberate migration is written
- prefer additive clarifications over changing user-visible context semantics abruptly
- keep the base/current distinction explicit if the data surface becomes richer later
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Hardened dataset-context coverage across service, store, and panel layers: `web/src/services/dataService.test.ts`, `web/src/stores/dataStore.actions.test.ts`, and `web/src/components/DataWorkspacePanel.test.ts` now freeze context normalization, startup restore posture, and visible dataset-summary behavior in one verified slice.
- Added an explicit base/current cue in `web/src/components/DataWorkspacePanel.vue` by surfacing `Base timeframe: ...` whenever a derived timeframe is active, making base-versus-current context legible without changing existing field names or service contracts.
- Preserved downstream alignment by keeping optimize and validation service tests in the verification set, so visible dataset context and service-level consumers remain synchronized.
- Acceptance evidence captured during execution: `npm.cmd --prefix web run test -- src/components/DataWorkspacePanel.test.ts src/stores/dataStore.actions.test.ts src/services/dataService.test.ts src/services/optimizeService.test.ts src/services/validationService.test.ts`, `npm.cmd --prefix web run lint`, `npm.cmd --prefix web run typecheck`, and `cargo check -p ta_app_core`.

## Related Docs

- `REPO_Improvement/02_data/22_dataset_provenance_2026-04-08.md`
- `REPO_Improvement/02_data/26_derived_dataset_identity_2026-04-08.md`
- `REPO_Improvement/02_data/03_csv_load_contract_2026-04-08.md`

