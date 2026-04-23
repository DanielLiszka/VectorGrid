# Live Execution Secure Credential Storage Plan

Last updated: 2026-04-12

## Purpose

Define a measured plan to move live-execution broker credential persistence out of frontend local storage and into a backend-owned secure storage path that matches the seriousness of discrete live mode.

## Scope Boundaries

In scope:

- Alpaca credential persistence for the current live-execution surface
- backend-owned secure storage, retrieval, clear, and status commands
- migration away from frontend local-storage persistence for remembered live-execution credentials
- redaction, diagnostics, and UX rules for broker credential handling

Out of scope:

- new broker integrations
- cloud secret sync
- options live trading
- general-purpose credential vault work outside the current live-execution surface

## Current User-Facing Behavior

The execution surface already supports remembering broker credentials, but the current implementation stores remembered Alpaca credentials in frontend persistence rather than a backend-owned secure store.

That is acceptable for a bounded MVP, but it is not a strong posture for a discrete live mode.

## Current Code Surface

- `web/src/stores/executionStore.ts`
- `web/src/services/executionService.ts`
- `web/src/utils/core/storageKeys.ts`
- `src-tauri/src/commands_execution.rs`
- `src-tauri/src/lib.rs`
- existing secure-storage precedent:
  - `src-tauri/src/polygon.rs`
  - `src-tauri/src/commands_data.rs`

## Current Verification Surface

- `web/src/stores/executionStore.actions.test.ts`
- `web/src/services/executionService.test.ts`
- targeted Rust tests near new or extended execution credential commands
- current Polygon secure-storage tests for precedent and behavior shape

## Strengths Already Present

- the execution UX already has a `Remember keys` concept
- the repo already has a backend secure-storage precedent for Polygon credentials
- the current live scope is narrow enough that the credential contract can stay explicit
- the Tauri command surface already owns the real broker actions

## Weaknesses / Failure Modes

- frontend local storage is too weak for remembered live broker credentials
- secret lifecycle is split between frontend convenience and backend broker usage
- a discrete live mode should not rely on the web layer as the primary secret owner
- migration mistakes could strand users with stale remembered credentials or ambiguous storage state

## Regressions To Avoid

- breaking current dry-run or paper workflows while tightening credential storage
- persisting secrets in logs, artifacts, status text, or migration diagnostics
- creating silent credential precedence ambiguity between frontend and backend storage
- making credential failures harder to diagnose while trying to hide implementation detail
- allowing this surface to stay behaviorally working while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Tauri state management](https://v2.tauri.app/develop/state-management/)
- [Alpaca Trading API](https://docs.alpaca.markets/docs/trading-api)
- [Keyring crate docs](https://docs.rs/keyring/latest/keyring/)

## Proposed Improvement Approach

1. Reuse the existing backend secure-storage pattern already present for Polygon credentials rather than inventing a second approach.
2. Add explicit execution credential commands for:
   - set
   - get status / presence
   - clear
3. Keep the frontend `Remember keys` UX, but make it a frontend toggle over backend-owned secret storage.
4. Add an explicit migration rule for existing remembered local credentials.
5. Ensure that backend commands and frontend state expose presence and error status without ever returning raw secrets unless explicitly required for a justified UX flow.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Record the current credential persistence path and define the target secure-storage owner.
- [x] Define the backend command contract for setting, clearing, and checking stored Alpaca credentials.
- [x] Define whether raw credential retrieval is ever necessary for the frontend after migration.
- [x] Define migration behavior for already persisted frontend credentials.
- [x] Define the precedence rules if both legacy frontend and new backend credential stores exist during migration.
- [x] Define user-visible status text for stored, missing, invalid, and cleared credential states.
- [x] Add or strengthen frontend tests for remember/clear flows and status behavior.
- [x] Add or strengthen Rust tests for secure storage, clear semantics, and redaction-safe error handling.
- [x] Define rollback conditions if the secure-storage path introduces more ambiguity than the current remembered-key UX.

## Verification Plan

- rerun targeted execution-store and execution-service tests near touched files
- rerun targeted Rust tests near new or extended execution credential commands
- rerun any existing Polygon secure-storage tests that share infrastructure
- rerun `npm.cmd --prefix web run lint`
- rerun `npm.cmd --prefix web run typecheck`
- rerun `cargo check --workspace`
- run `powershell -ExecutionPolicy Bypass -File scripts/run_repo_quality_gate.ps1 -IncludeDesktopTypecheck -IncludeWorkspaceTests`

## Final Regression Validation Requirement

- Do not mark this item complete until the finished implementation state has been revalidated at the end of the work, not only during storage migration slices.
- Final validation must include targeted unit/integration tests for secure-store resolution, migration, degraded-storage handling, redaction, and masked frontend behavior, plus the relevant repo quality gates required by the final change set.
- Final validation must include browser, desktop, and accessibility checks for the credential UX, plus a paper/live-mode manual smoke path proving command resolution works without raw-secret readback or accidental secret disclosure.
- Final validation must include performance evaluation for any touched secure-store lookup, hydration, or submit path so the final state does not introduce avoidable latency or retry regressions.
- If the final integrated state introduces correctness, UX, safety, or performance regressions, narrow the slice, add a guard, or revert the offending portion before closure.

## Execution Notes

- Alpaca credential ownership now matches the seriousness of the live surface:
  - `src-tauri/src/commands_execution.rs` exposes secure credential status/set/clear commands and backend-default credential resolution for broker-facing actions
  - `web/src/services/executionService.ts` wraps secure-storage status, migration, and clear/set flows
  - `web/src/stores/executionStore.ts` and `web/src/components/ExecutionWorkspacePanel.vue` now show credential presence/state without hydrating raw remembered secrets back into the UI
- The migration posture is explicit and additive:
  - legacy frontend-local remembered keys can migrate forward into secure storage
  - backend presence/status becomes the remembered-credential truth source
  - raw-secret readback is not part of the frontend contract after migration
- This epic closed for the current single-broker execution surface:
  - existing paper/live workflows remain intact
  - secure-storage failures still surface diagnosably without leaking credentials
- Acceptance evidence:
  - `cargo test -p ta_desktop_demo_app alpaca_keyring_service_name_scopes_to_app_identifier --lib`
  - `cargo test -p ta_desktop_demo_app alpaca_credential_storage_status_tracks_presence_bits --lib`
  - `cargo test -p ta_desktop_demo_app store_alpaca_api_credentials_rejects_empty_values_before_secure_storage --lib`
  - `cargo test -p ta_desktop_demo_app normalize_alpaca_typed_override_rejects_partial_pairs --lib`
  - `npm.cmd --prefix web run test -- src/services/executionService.test.ts src/stores/executionStore.actions.test.ts src/components/ExecutionWorkspacePanel.test.ts`
  - `npm.cmd --prefix web run lint`
  - `npm.cmd --prefix web run typecheck`
  - `npm.cmd --prefix web run test:browser:ux`
  - `npm.cmd --prefix web run a11y:smoke`
  - `npm.cmd --prefix web run test:desktop:smoke`
  - `powershell -ExecutionPolicy Bypass -File scripts/run_repo_quality_gate.ps1 -IncludeDesktopTypecheck -IncludeWorkspaceTests`

## Rollout / Fallback / Compatibility Notes

- keep the current UI contract stable while the credential owner moves behind the scenes
- prefer one clear backend-owned truth source after migration, not a permanent dual-store model
- if migration cannot be made explicit and reversible, require a one-time clear-and-reenter flow instead of silently guessing
- do not widen this plan into a general account-management or broker-abstraction feature

## Related Docs

- `05_live_execution_productization_2026-04-10.md`
- `06_live_execution_operator_safety_controls_2026-04-11.md`
- `REPO_Improvement/09_live/11_settings_secret_storage_2026-04-09.md`
- `REPO_Improvement/09_live/06_broker_connectivity_2026-04-09.md`
