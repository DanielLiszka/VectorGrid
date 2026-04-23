# Startup Dataset Restore Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to startup dataset restoration so persisted dataset state, deferred affordance loading, and degraded restore behavior remain explicit and safe.

## Scope Boundaries

In scope:

- startup snapshot persistence and restore semantics
- `autoRestoreDataset` behavior
- core versus deferred startup hydration in `dataStore.ts`
- status-line behavior during restore success or failure

Out of scope:

- new persistence mechanisms beyond current storage
- options datasets
- full workflow bootstrap redesign outside data restore

## Current User-Facing Behavior

`web/src/stores/dataStore.ts` already owns a real startup-restore path. The store reads `DATA_STARTUP_SNAPSHOT_KEY`, restores `autoRestoreDataset`, gets market-data directory info, optionally reloads the base CSV, optionally reapplies a derived timeframe, and then marks startup hydration complete. Deferred startup affordances such as sample CSV listing happen later through `ensureStartupAffordancesLoaded()`. During restore, the store exposes `Startup restore pending.`, `Restoring ...`, `Restored ...`, or `Startup restore failed: ...` status text rather than hiding the transition.

Current behavior also includes:

- startup restore depends on persisted base CSV path and selected timeframe
- restore only runs once per startup path unless state is explicitly reset
- degraded restore failures fall back to empty-dataset state instead of crashing the shell
- startup persistence is rewritten when dataset roots move for Polygon-backed base paths

## Current Code Surface

- `web/src/stores/dataStore.ts`
- `web/src/utils/core/storageKeys.ts`
- `web/src/utils/storage.ts`
- `web/src/services/dataService.ts`
- `web/src/services/settingsBootstrap.ts`
- `web/src/main.ts`
- `web/src/stores/dataStore.actions.test.ts`

## Current Verification Surface

- `web/src/stores/dataStore.actions.test.ts`
- `web/src/main.test.ts`
- `targeted data-store hydration behavior in unit tests`

## Strengths Already Present

- startup restore is already centralized in the data store
- the restore path already distinguishes core hydration from deferred affordance loading
- restore failure is already degraded safely into a visible empty-state posture
- the snapshot already tracks source kind, timeframe, base path, and source identity hints

## Weaknesses / Failure Modes

- startup restore is one of the densest paths in `dataStore.ts` and currently depends on many helper assumptions
- core and deferred startup behavior are documented only in code
- status-line semantics for restore phases are useful but not yet frozen in one implementation doc
- dataset-move rewriting and Polygon-root changes intersect with startup restore in subtle ways

## Regressions To Avoid

- attempting repeated startup restores on every mount
- hiding restore failure behind a blank ready state
- re-downloading or re-discovering data when the stored base CSV path should be reused
- breaking persisted restore behavior when market-data roots or Polygon cache roots move
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because startup restore correctness depends on one authoritative store-owned restore path.
- [MDN localStorage](https://developer.mozilla.org/en-US/docs/Web/API/Window/localStorage) is relevant because startup snapshot persistence relies on browser storage semantics and compatibility behavior.
- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because the startup snapshot behaves like a versioned persisted contract that should not drift casually.
- [RFC 4180](https://www.rfc-editor.org/rfc/rfc4180)
  - relevant because CSV-facing import, export, and repair behavior should remain interoperable and explicit instead of relying on provider-specific happy paths

## Proposed Improvement Approach

1. Document the startup snapshot schema and the exact restore sequence from persisted state to hydrated dataset.
2. Freeze the difference between core startup hydration and deferred affordance loading.
3. Document the current status-line transitions during restore success, pending, and failure.
4. Clarify how restore interacts with moved Polygon cache roots and source-kind rewriting.
5. Keep restore additive and safe rather than silently dropping persisted datasets.
6. Treat data-surface code quality as part of the implementation scope: preserve explicit schemas, path safety, provenance clarity, diagnostics, and testable normalization rules rather than relying on permissive parsing.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document every field in the startup snapshot and where it is written.
- [x] Document the startup restore sequence and stop conditions.
- [x] Document the difference between `startupHydratedOnce` and `startupDeferredHydratedOnce`.
- [x] Document status-line behavior for restore pending, success, and failure.
- [x] Add or tighten tests for restore success, derived-timeframe restore, and restore failure.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/stores/dataStore.actions.test.ts`
- run `npm --prefix web run test -- src/main.test.ts`
- manually verify one cold start with auto-restore on and one with a broken restore path after any restore refactor
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched data surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and `cargo check --workspace` or a justified narrower target
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve current safe degradation to empty-dataset state on restore failure
- avoid changing persisted storage keys or snapshot fields without a compatibility plan
- prefer making restore phases more explicit before making them more aggressive
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Added explicit store-level coverage in `web/src/stores/dataStore.actions.test.ts` for startup restore pending and restore failure semantics so the app keeps surfacing `Startup restore pending.` and `Startup restore failed: ...` states instead of collapsing into a blank ready posture.
- Kept the existing restore model intact: startup hydration still owns the single restore attempt, deferred affordances still load later, and broken restore paths still degrade safely to an empty-dataset shell rather than re-downloading or crashing startup.
- Preserved storage compatibility by hardening visible behavior and tests only; no snapshot key, restore ordering, or persisted schema change was introduced in this slice.
- Acceptance evidence captured during execution: `npm.cmd --prefix web run test -- src/stores/dataStore.actions.test.ts src/components/DataWorkspacePanel.test.ts src/services/dataService.test.ts src/services/optimizeService.test.ts src/services/validationService.test.ts`, `npm.cmd --prefix web run lint`, `npm.cmd --prefix web run typecheck`, and `cargo check -p ta_app_core`.

## Related Docs

- `REPO_Improvement/02_data/08_repo_data_browser_ux_2026-04-09.md`
- `REPO_Improvement/02_data/11_polygon_root_settings_2026-04-09.md`
- `REPO_Improvement/01_shell/07_shell_runtime_boot_2026-04-08.md`
