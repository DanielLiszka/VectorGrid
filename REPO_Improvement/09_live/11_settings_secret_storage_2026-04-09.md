# Settings Secret Storage Plan

Last updated: 2026-04-09

## Purpose

Define the current non-options settings and secret-storage posture so improvements can harden credential handling without breaking existing market-data settings, Polygon key behavior, or live execution workflows.

## Scope Boundaries

In scope:

- persisted non-secret desktop/settings state
- Polygon API key storage and fallback behavior
- execution credential persistence in frontend storage
- current browser-versus-Tauri secret boundaries
- migration considerations for safer secret handling

Out of scope:

- options credentials
- broker/order semantics beyond their storage posture
- general UI redesign of the Settings workspace

## Current User-Facing Behavior

The repo currently has a split persistence model:

- normal settings such as market-data root override live in Tauri-side app-local JSON
- the Polygon API key is Tauri-owned and prefers OS keyring storage, with fallback to a local settings file and finally environment-based discovery
- live execution API key/secret persistence is frontend-owned and stored in browser local storage when the user enables Remember keys
- many non-secret UI preferences also live in browser storage

This means the current system already distinguishes secrets from general settings in some places, but not uniformly. Polygon is backend/keyring-aware; broker execution credentials are still browser-storage-backed.

## Current Code Surface

- `src-tauri/src/settings.rs`
  - owns `app_settings.json` and market-data-root persistence
- `src-tauri/src/polygon.rs`
  - owns Polygon API key read/write/clear behavior, keyring usage, legacy file fallback, and env fallback
- `src-tauri/src/commands_data.rs`
  - exposes Polygon key and market-data settings commands
- `web/src/services/executionService.ts`
  - persists broker execution keys in browser storage under `EXEC_KEYS_STORAGE_KEY`
- `web/src/stores/executionStore.ts`
  - mirrors persisted execution credentials into the live execution UI state
- `web/src/utils/core/storageKeys.ts`
  - defines execution-key and general storage-key names
- `web/src/utils/storage.ts`
  - centralizes browser storage helpers and alias migration behavior
- `web/src/stores/settingsStore.ts`
  - owns the broader browser-side settings and preference model

## Current Verification Surface

- `src-tauri/tests/settings_tests.rs`
- unit tests in `src-tauri/src/polygon.rs`
- `web/src/stores/executionStore.actions.test.ts`
- `web/src/services/executionService.test.ts`
- `web/src/stores/settingsStore.actions.test.ts`

## Strengths Already Present

- normal settings and secret-like settings are not fully collapsed into one storage system
- Polygon key storage already prefers a safer Tauri-owned mechanism
- Polygon key storage already includes compatibility fallbacks instead of brittle all-or-nothing behavior
- browser storage is already centralized through helper functions and alias migration rules
- execution UI already exposes whether keys are missing, incomplete, session-only, or persisted

## Weaknesses / Failure Modes

- broker execution credentials are still stored in browser local storage, which is materially weaker than Tauri-side secret storage
- there is no single storage-authority doc explaining why Polygon and execution credentials differ
- migration from browser-stored execution keys to safer storage would affect existing Remember keys behavior
- mixed persistence layers can be misdescribed as “all settings are local storage” or “all secrets are safe,” neither of which is true today
- fallback behavior can hide where a secret actually came from unless explicitly documented

## Regressions To Avoid

- breaking existing Polygon key lookup order without a migration story
- silently dropping stored execution keys or changing Remember keys semantics without clear compatibility handling
- broadening plaintext secret persistence when trying to simplify storage logic
- mixing secret and non-secret settings into one undocumented storage bucket
- weakening current key-state UX that tells the operator what persistence posture is active

## External Research Summary

- [Tauri Store plugin](https://v2.tauri.app/es/plugin/store/)
  - relevant because non-secret app settings can remain in conventional persisted storage without being treated like secrets
- [Tauri Stronghold plugin](https://v2.tauri.app/zh-cn/plugin/stronghold/)
  - relevant because it represents the current official Tauri direction for encrypting secret material rather than leaving it in browser storage
- [OWASP Secrets Management Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Secrets_Management_Cheat_Sheet.html)
  - relevant because secret-storage improvements should reduce plaintext exposure and clarify authority boundaries
- [Alpaca orders documentation](https://docs.alpaca.markets/docs/orders-at-alpaca)
  - relevant because broker-facing execution credentials protect real account actions and should be treated accordingly

## Proposed Improvement Approach

1. Document the current storage model explicitly:
   - app-local JSON for normal Tauri-owned settings
   - keyring-first Polygon API key storage
   - browser local storage for execution credentials
2. Treat the current broker credential posture as a known hardening gap, not as an accidental detail.
3. Preserve current compatibility for remembered execution keys until a deliberate migration path exists.
4. Keep secret-storage planning separate from general settings cleanup.
5. Use future secret-storage changes to converge toward Tauri-owned secrets without breaking the current non-secret settings contract.

## Checklist

- [ ] Document current non-secret settings persistence on the Tauri side.
- [ ] Document Polygon keyring/file/env fallback order.
- [ ] Document current execution-key browser storage behavior and its risk posture.
- [ ] Document current key-state UX exposed in the execution workspace.
- [ ] Identify which stored values are secret, quasi-secret, or non-secret.
- [ ] Define compatibility expectations for any future migration away from browser-stored execution keys.
- [ ] Add or tighten tests if secret lookup order or persistence behavior changes.
- [ ] Keep options-related credentials out of scope for this non-options doc set.

## Verification Plan

- run `cargo test -p ta_desktop_demo_app --test settings_tests`
- run targeted unit tests in `src-tauri/src/polygon.rs`
- run `npm --prefix web test -- src/services/executionService.test.ts`
- run `npm --prefix web test -- src/stores/executionStore.actions.test.ts`
- run `npm --prefix web test -- src/stores/settingsStore.actions.test.ts`

## Rollout / Fallback / Compatibility Notes

- keep current Polygon keyring/file/env compatibility intact unless a migration is explicitly implemented
- preserve existing Remember keys behavior until a safer replacement exists and is migrated deliberately
- separate general settings cleanup from secret-storage hardening to avoid accidental regressions
- if execution secrets move into Tauri-owned storage later, provide import/cleanup behavior for existing browser-stored keys

## Related Docs

- `REPO_Improvement/02_data/11_polygon_root_settings_2026-04-08.md`
- `REPO_Improvement/02_data/12_polygon_api_key_2026-04-08.md`
- `REPO_Improvement/01_shell/11_settings_storage_shortcuts_2026-04-08.md`
- `REPO_Improvement/09_live/06_broker_connectivity_2026-04-09.md`
- `REPO_Improvement/11_docs_ops/07_local_vs_ci_authority_DATE.md`
