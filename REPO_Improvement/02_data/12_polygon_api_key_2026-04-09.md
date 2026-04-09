# Polygon API Key Plan

Last updated: 2026-04-09

## Purpose

Document and improve the current Polygon API-key contract so storage order, UI exposure, and fallback behavior remain secure and compatible without breaking existing download workflows.

## Scope Boundaries

In scope:

- backend Polygon API-key lookup and storage order
- keyring, legacy settings file, and env fallback behavior
- frontend `hasApiKey` exposure and save/clear flows
- data-panel copy and diagnostics around API-key state

Out of scope:

- rotating credentials through external secret managers
- options workflows
- exposing the raw API key back to the frontend

## Current User-Facing Behavior

The Polygon API-key path is already layered. In `src-tauri/src/polygon.rs`, lookup prefers the OS keyring, then falls back to legacy `polygon_settings.json` with best-effort migration to keyring, then falls back to environment values from `POLYGON_KEY`, `POLYGON_API_KEY`, or a local `.env`. `src-tauri/src/commands_data.rs` never sends the raw key to the frontend; it only reports `has_api_key` through `polygon_get_cache_info`, while `polygon_set_api_key` and `polygon_clear_api_key` mutate backend storage. The frontend stores only the current draft string, then refreshes cache info and selection state through `dataService.ts` and `dataStore.ts`.

Current shipped behavior includes:

- the UI can save and clear the key without ever reading it back from cache info
- the data panel treats the key as a capability gate for download actions
- legacy settings-file fallback still works for older local setups
- the current UI copy still suggests “saved in local Polygon cache settings” even though keyring is the preferred storage path

## Current Code Surface

- `src-tauri/src/polygon.rs`
- `src-tauri/src/commands_data.rs`
- `web/src/services/dataService.ts`
- `web/src/services/dataService.test.ts`
- `web/src/stores/dataStore.ts`
- `web/src/stores/dataStore.actions.test.ts`
- `web/src/components/DataWorkspacePanel.vue`

## Current Verification Surface

- `src-tauri/src/polygon.rs`
- `web/src/services/dataService.test.ts`
- `web/src/stores/dataStore.actions.test.ts`
- `web/src/components/DataWorkspacePanel.test.ts`

## Strengths Already Present

- the raw API key is already kept backend-side and not exposed through cache-info responses
- the storage chain already preserves backward compatibility for legacy local setups
- frontend save/clear behavior is already centralized through one service and one store path
- download readiness already reflects `hasApiKey` rather than trusting UI drafts alone

## Weaknesses / Failure Modes

- the storage-order contract is important but currently documented only in code
- current UI wording does not perfectly match the preferred keyring-first backend behavior
- there is no dedicated implementation doc for save/clear/error behavior and fallback order
- env and `.env` fallback behavior is helpful but easy to remove accidentally during cleanup

## Regressions To Avoid

- leaking the actual API key to the frontend or to cache-info payloads
- misdocumenting the storage order so future changes remove real fallback behavior
- breaking save/clear compatibility for users still relying on legacy local settings
- gating download UI off the draft field instead of backend-confirmed `hasApiKey` state
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Polygon Aggregates / Custom Bars](https://polygon.io/docs/rest/stocks/aggregates/custom-bars) is relevant because the API-key contract exists only to unlock current Polygon download workflows.
- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because cache-info and API-key availability behave like stable command contracts.
- [Tauri Calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because the frontend/backend split is the main security boundary for this key flow.
- [RFC 4180](https://www.rfc-editor.org/rfc/rfc4180)
  - relevant because CSV-facing import, export, and repair behavior should remain interoperable and explicit instead of relying on provider-specific happy paths

## Proposed Improvement Approach

1. Document the exact API-key lookup order and keep the frontend blind to raw key material.
2. Freeze the current `hasApiKey`-only frontend exposure and the save/clear refresh loop.
3. Call out the UI-copy mismatch around key storage and correct wording before changing logic.
4. Keep legacy settings-file and env fallback behavior explicit until a deliberate migration plan exists.
5. Add narrow tests if save/clear semantics or fallback order change.
6. Treat data-surface code quality as part of the implementation scope: preserve explicit schemas, path safety, provenance clarity, diagnostics, and testable normalization rules rather than relying on permissive parsing.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document keyring, legacy JSON, env, and `.env` fallback order.
- [ ] Document `polygon_get_cache_info`, `polygon_set_api_key`, and `polygon_clear_api_key` frontend/backend behavior.
- [ ] Document current UI copy and any mismatches with backend storage behavior.
- [ ] Keep raw-key non-exposure as an explicit security invariant.
- [ ] Add or tighten tests if fallback order or save/clear behavior changes.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run targeted Polygon backend tests covering env and legacy fallback behavior
- run `npm --prefix web run test -- src/services/dataService.test.ts`
- run `npm --prefix web run test -- src/stores/dataStore.actions.test.ts`
- manually verify save/clear flows in the Data panel after any wording or backend change
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched data surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and `cargo check --workspace` or a justified narrower target
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve backward-compatible fallback behavior unless a replacement migration is already implemented
- prefer tightening UI wording before reworking backend key storage
- do not widen frontend exposure beyond current `hasApiKey` state
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/02_data/11_polygon_root_settings_2026-04-09.md`
- `REPO_Improvement/02_data/13_polygon_request_contract_2026-04-09.md`
- `REPO_Improvement/02_data/16_polygon_dataset_listing_2026-04-09.md`
