# Live Execution Secure Credential Storage Validated Implementation Guide

Last updated: 2026-04-11

Companion to:
- `11_live_execution_secure_credential_storage_2026-04-11.md`

## Purpose

Convert the validated credential-storage findings into an explicit secret-owner, migration, and UI-truth contract for the current live surface.

## Secret Ownership Contract

After migration:
- backend secure storage is the only persisted owner for remembered Alpaca credentials
- frontend typed credentials are transient per-action overrides only
- no permanent dual-owner model remains

Affected Alpaca consumers:
- `test_alpaca_connection`
- `get_live_execution_snapshot`
- `execute_strategy_signal_once`
- `start_live_execution_runner`

## Command Resolution Rules

Default behavior:
- broker-facing commands resolve backend-stored credentials automatically

Transient override behavior:
- a fully typed frontend key/secret pair may override stored credentials for the current action only
- an incomplete typed pair is invalid and must not shadow stored backend credentials silently

## No-Readback Rule

Backend credential retrieval is presence/status only.

The frontend must never receive raw persisted Alpaca secrets after migration.

Allowed status surface:
- missing
- stored securely
- remember unavailable
- clear failed

Not allowed:
- raw secret readback for “edit existing key” convenience

## Degraded Storage Policy

For Alpaca remember-mode in v1:
- secure backend storage is required
- if secure storage is unavailable, remember-mode is unavailable
- no plaintext or local-file fallback is allowed for remembered Alpaca credentials

This is intentionally stricter than the current Polygon precedent.

## Precedence And Migration Matrix

Migration inputs to handle explicitly:
- complete typed frontend pair
- incomplete typed frontend pair
- backend-stored pair present
- current local-storage pair present
- legacy alias key present
- no credentials present

Migration sequence:
1. scan current and legacy frontend storage keys
2. attempt backend write
3. verify backend success
4. only then delete legacy frontend storage
5. surface explicit success or failure
6. if migration cannot be verified, require clear-and-reenter instead of guessing

## Frontend Truth Model After Migration

Startup behavior:
- credential input fields do not hydrate raw stored secrets
- stored credentials are represented by status, not by prefilled secret values

Visible states:
- `Missing`
- `Stored securely`
- `Session only`
- `Will store on next successful broker action`
- `Will update stored credentials`
- `Remember unavailable`
- `Clear failed`

Remember semantics:
- preserve deferred persistence
- credentials are committed only after a successful broker-facing action

## Owner Surface

- frontend:
  - `web/src/stores/executionStore.ts`
  - `web/src/services/executionService.ts`
  - `web/src/utils/core/storageKeys.ts`
  - `web/src/utils/storage.ts`
  - `web/src/components/ExecutionWorkspacePanel.vue`
- backend:
  - `src-tauri/src/commands_execution.rs`
  - `src-tauri/src/lib.rs`
  - secure-storage precedent and supporting code:
- `src-tauri/src/polygon.rs`
- `src-tauri/src/commands_data.rs`

## Preconditions

Before code changes start, freeze these choices in the parent bundle doc:

- backend secure storage is the only persisted owner
- no raw secret readback is allowed
- Alpaca remember-mode is keyring-only for v1
- deferred persistence semantics stay in place

## Ordered Implementation Slices

1. Backend command slice
   - add/set/clear/status Alpaca credential commands and registry wiring
2. Command-resolution slice
   - switch all four Alpaca consumers to backend-default credential resolution with transient override support
3. Migration slice
   - migrate current and legacy frontend storage keys into backend storage
4. Frontend truth-model slice
   - remove raw persisted secret hydration
   - replace it with presence/status-driven UI state
5. Hardening slice
   - redaction, migration failure, and clear-failure regression coverage

## File-Level Change Plan

- `src-tauri/src/commands_execution.rs`
  - add backend-owned Alpaca credential commands or supporting helpers
  - update command-resolution behavior for existing Alpaca consumers
- `src-tauri/src/lib.rs`
  - register any new credential commands
- `web/src/services/executionService.ts`
  - stop treating frontend storage as the persisted truth source
  - implement migration and transient override behavior
- `web/src/stores/executionStore.ts`
  - replace raw persisted secret mirrors with presence/status-driven UI state
- `web/src/utils/storage.ts`
  - retain only migration reads for the old frontend keys
- `web/src/components/ExecutionWorkspacePanel.vue`
  - render the post-migration truth model
- `web/src/utils/core/storageKeys.ts`
  - preserve current and legacy alias names for migration only

## Acceptance Evidence

Required evidence:

- migrated user from `vectorgrid_exec_keys_v1`
- migrated user from `vectorbt_exec_keys_v1`
- stored-but-no-readback startup state
- clear-failure state with truthful UI
- redaction-safe error path
- successful transient override path without persistent write until a broker-facing action succeeds

## Closure Conditions

Do not mark the bundle complete until:

- no broker-facing Alpaca command requires frontend-persisted secrets as its default source
- no raw persisted secret is read back into the frontend
- migration is explicit and reversible enough to debug
- frontend UI states remain truthful without raw persisted secret hydration
- secure-storage failure does not silently fall back to weaker remembered-secret persistence

## Verification Requirements

Required coverage:
- migration tests for current and legacy storage keys
- command-resolution tests for all four Alpaca consumers
- no-readback tests
- redaction tests proving secrets never leak into `statusLine`, `statusHint`, or `outputPreview`
- UI tests for:
  - stored securely
  - session only
  - missing
  - clear failed
  - remember unavailable

Run:
- `ExecutionWorkspacePanel.test.ts` when visible startup/remember behavior changes
- `executionStore.actions.test.ts`
- `executionService.test.ts`
- targeted Rust tests near new credential commands
- `npm.cmd --prefix web run test:browser:ux` and `npm.cmd --prefix web run test:desktop:smoke` when visible startup behavior changes

## Final Regression Validation Requirement

- Do not close this guide’s implementation work until the finished integrated state has been revalidated at the end of the work, not only during intermediate migration slices.
- Final validation must rerun the targeted backend/frontend tests against the completed secure-storage model, then run the relevant repo quality gates required by the final change set.
- Final validation must include any required browser, desktop, and accessibility checks for the credential UX, plus a paper/live-mode manual smoke path proving command resolution works without raw-secret readback or accidental disclosure.
- Final validation must include performance evaluation for touched secure-store lookup, hydration, and submit flows so the final state does not regress responsiveness.
- If the final integrated state introduces correctness, UX, safety, or performance regressions, narrow the slice, add a guard, or revert the offending portion before closure.

## Rollback Rule

If secure storage cannot be made less ambiguous than the current remembered-key UX, rollback to the prior local-only behavior and keep this bundle open rather than shipping a misleading half-migration.
