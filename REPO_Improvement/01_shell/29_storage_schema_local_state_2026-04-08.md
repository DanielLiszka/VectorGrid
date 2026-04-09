# Storage Schema Local State Plan

Last updated: 2026-04-09

## Purpose

Define the current non-options local-state storage schema so persistence, key ownership, alias migration, and bootstrap behavior can be improved without breaking already-shipped shell state.

## Scope Boundaries

In scope:

- browser `localStorage` usage for shell/workflow/settings persistence
- key naming and alias migration in `web/src/utils/core/storageKeys.ts` and `web/src/utils/storage.ts`
- settings/workflow bootstrap from persisted state
- persisted table-state and dock/open-state keys already in use

Out of scope:

- Tauri-side durable artifact storage
- options-specific keys or state
- new cloud sync or profile sync features

## Current User-Facing Behavior

The app already persists a meaningful amount of non-options local state across reloads:

- workflow mode, active workspace, workflow step, tabs, and split fraction
- controls collapsed/advanced defaults
- theme, native effect, help mode, density mode, and desktop-notification mode
- workstation perspectives
- task-center open state
- result-table state keys for top results, validation results, and meta results
- some history/settings/recent-file compatibility keys and batch-summary pointers

The persistence model is already additive and user-visible: the shell tries to reopen in a familiar state rather than resetting to defaults every time.

## Current Code Surface

- `web/src/utils/core/storageKeys.ts`
  - declares the primary storage-key inventory and alias map
- `web/src/utils/storage.ts`
  - central storage wrappers, alias reads, alias cleanup, and schema migrations
- `web/src/services/settingsBootstrap.ts`
  - reads persisted settings values and applies them through typed store actions
- `web/src/stores/settingsStore.ts`
  - persists theme, effect, help mode, density, notification mode, and perspectives
- `web/src/stores/workflowStore.ts`
  - persists workflow/runtime shell state and split fraction
- `web/src/composables/useTaskCenterVisibility.ts`
  - persists task-center open state directly
- `web/src/services/settingsBootstrap.test.ts`
  - verifies settings bootstrap reads, normalizes, and dispatches correctly
- `web/src/stores/settingsStore.actions.test.ts`
  - verifies persistence-related settings behavior

## Current Verification Surface

- `web/src/services/settingsBootstrap.test.ts`
- `web/src/stores/settingsStore.actions.test.ts`
- `web/src/services/settingsService.test.ts`
- `web/src/stores/workflowStore.test.ts`
- `web/src/utils/storage.test.ts`
- `web/src/composables/useTaskCenterVisibility.test.ts`

## Strengths Already Present

- the repo already has a centralized key inventory
- schema versioning and alias migration already exist in `storage.ts`
- storage access is mostly wrapped instead of raw ad hoc calls
- settings bootstrap uses typed store actions instead of mutating stores indirectly
- legacy `vectorbt_*` aliases are already recognized and cleaned up

## Weaknesses / Failure Modes

- the full local-state inventory is not documented in one place
- some persistence still bypasses the shared wrapper path, especially task-center state
- schema versioning exists, but ownership boundaries per key family are still implicit
- bootstrap rules for invalid or partial persisted values are only partly documented through tests
- future contributors could add new persistence keys without updating a central contract

## Regressions To Avoid

- silently renaming persisted keys without an alias migration
- breaking legacy alias cleanup for shipped users
- losing workflow or settings continuity on reload
- writing invalid values that bootstrap cannot normalize safely
- bypassing the shared storage helpers in ways that skip migration or cleanup
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [MDN `localStorage`](https://developer.mozilla.org/en-US/docs/Web/API/Window/localStorage)
  - relevant because current shell state is persisted through browser storage semantics and failure handling
- [JSON Schema overview](https://json-schema.org/overview/what-is-jsonschema)
  - relevant because the storage-key program behaves like a lightweight schema that should be versioned and documented
- [RFC 8259 JSON](https://www.rfc-editor.org/rfc/rfc8259)
  - relevant because several persisted values are serialized JSON blobs and should remain parseable/stable
- [Pinia testing cookbook](https://pinia.vuejs.org/cookbook/testing.html)
  - relevant because persistence behavior is already tested through store-level actions and should stay that way
- [WAI-ARIA Authoring Practices Guide](https://www.w3.org/WAI/ARIA/apg/)
  - relevant because dialogs, menus, overlays, keyboard flows, and dense shell surfaces in a public-facing app need explicit accessibility and interaction contracts

## Proposed Improvement Approach

1. Treat local state as a documented schema, not just a collection of keys.
2. Catalog the current key families:
   - workflow shell keys
   - settings keys
   - table-state keys
   - legacy alias keys
   - compatibility pointer keys
3. Define ownership for each family:
   - who writes it
   - who reads it
   - who migrates it
4. Reduce raw `localStorage` usage where practical by routing more surfaces through the shared helpers.
5. Keep migrations additive:
   - preserve old keys through aliases when required
   - clean up aliases only after primary-key writes succeed
6. Treat shell-level code quality as part of the implementation scope: preserve keyboard flow clarity, accessible semantics, concise component boundaries, and explicit overlay/state contracts.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Execution Notes

- kept the shared storage wrappers authoritative by routing `settingsBootstrap.ts` and `useTaskCenterVisibility.ts` through `readStorageString(...)` / `writeStorageString(...)` rather than ad hoc raw `localStorage` access
- verified the storage schema and alias-migration contract directly in `web/src/utils/storage.test.ts`, while `useTaskCenterVisibility.test.ts` covers the task-center open-state key through the shared helpers
- preserved existing key names and compatibility behavior; this pass hardened ownership and access patterns instead of renaming or deleting keys

## Checklist

- [x] Enumerate the full non-options local-state key inventory.
- [x] Document which store/service/composable owns each key.
- [x] Document the current schema-version and alias-migration behavior.
- [x] Document which keys are scalar versus serialized JSON payloads.
- [x] Document normalization behavior for invalid persisted values.
- [x] Identify any remaining direct `localStorage` access that should be aligned with shared helpers.
- [x] Tighten persistence tests when new keys or migrations are added.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/services/settingsBootstrap.test.ts`
- run `npm --prefix web run test -- src/stores/settingsStore.actions.test.ts`
- run `npm --prefix web run test -- src/stores/workflowStore.test.ts`
- manually verify that a reload preserves workflow/settings state and that bad persisted values fall back safely
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend quality gates for touched shell surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and any required `npm --prefix web run a11y:smoke` or shell-level smoke checks
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve existing primary keys unless a compatibility migration is added
- prefer alias-based migrations over hard key deletion
- keep the schema version monotonic and tied to real migration logic
- if task-center persistence is moved into shared storage helpers later, preserve the current key first
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/01_shell/11_settings_storage_shortcuts_2026-04-08.md`
- `REPO_Improvement/01_shell/27_settings_service_bootstrap_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/22_tauri_settings_persistence_2026-04-08.md`
- `REPO_Improvement/11_docs_ops/11_impl_doc_quality_bar_2026-04-08.md`
