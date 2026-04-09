# Dataset Invalidation Plan

Last updated: 2026-04-08

## Purpose

Define the current dataset invalidation and reuse contract so data reloads, derived resamples, live updates, and workflow restoration can be improved without leaving stale dataset state alive.

## Scope Boundaries

In scope:

- base-dataset replacement and derived-dataset invalidation in Tauri state
- derived resample reuse rules
- frontend clear/reload handling and stale-state cleanup
- dataset invalidation behavior that affects validation, artifact hydration, and startup restore

Out of scope:

- new cache backends
- options-specific data flows
- changing the current dataset-retention budget policy itself

## Current User-Facing Behavior

The app already has real invalidation behavior, not just best-effort refreshes:

- clearing a base dataset removes the descendant tree
- derived resamples are reused while the parent base dataset is unchanged
- live-bar updates invalidate active derived datasets
- identical base reloads can reuse the in-memory base id when source identity matches
- frontend clear operations reset UI state and workflow artifacts without turning unknown or already-cleared ids into hard user-facing failures

So the current product posture is already conservative about stale derived data. If the base meaning changes, the app prefers invalidation over pretending the old derived state is still valid.

## Current Code Surface

- `src-tauri/src/state.rs`
  - owns dataset tree identity, source-version tracking, derived reuse, and invalidation on base changes
- `src-tauri/src/commands_data.rs`
  - exposes load/clear/resample command behavior to the frontend
- `web/src/services/dataService.ts`
  - interprets load source and load-status outcomes
- `web/src/stores/dataStore.ts`
  - clears UI-facing state, tracks current/base dataset context, and surfaces reuse vs fresh-resample outcomes
- `web/src/services/validationService.ts`
  - depends on accurate dataset context during artifact persistence and validation requests

## Current Verification Surface

- `src-tauri/src/state.rs` tests
  - `load_replaces_prior_dataset_and_discards_prior_derived_cache`
  - `base_live_update_replaces_arc_and_invalidates_active_derived_dataset`
  - `resample_reuses_active_derived_dataset_until_base_changes`
  - `identical_base_load_reuses_existing_id_and_preserves_derived_cache`
- `src-tauri/src/commands_data.rs` tests
- `web/src/stores/dataStore.actions.test.ts`

## Strengths Already Present

- Rust owns the actual invalidation authority instead of the frontend guessing
- derived reuse is version-aware rather than path-only
- live updates and base reloads already invalidate or preserve descendants intentionally
- frontend clear paths are already designed to be safe and non-fragile
- load-source messaging already distinguishes binary cache, fresh resample, and in-memory derived reuse

## Weaknesses / Failure Modes

- invalidation reasons are mostly implicit rather than surfaced as first-class UX detail
- workflow/artifact replay can feel abrupt when stale dataset state is cleared without enough explanation
- dataset-tree removal and loaded-context cleanup are correct but not documented in one place
- same-source reuse versus real invalidation can be easy to misunderstand when reviewing only frontend code
- invalidation behavior is richer than current UI messaging

## Regressions To Avoid

- leaving stale derived datasets alive after base changes
- making clear/reset paths fail on already-removed or unknown ids
- breaking same-source base reuse for identical source identity
- invalidating derived state too aggressively on harmless no-op reloads
- letting validation or artifact restore proceed with stale dataset assumptions

## External Research Summary

- [Rust `std::fs::metadata`](https://doc.rust-lang.org/std/fs/fn.metadata.html)
  - relevant because base-source identity and freshness decisions are tied to file-origin metadata and cache safety
- [RFC 8259 JSON](https://www.rfc-editor.org/rfc/rfc8259)
  - relevant because dataset context and invalidation-adjacent state are persisted and replayed through JSON-shaped artifacts
- [Vue state management guidance](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because the frontend should reflect backend invalidation truth instead of inventing a second authority
- [Pinia actions](https://pinia.vuejs.org/core-concepts/actions.html)
  - relevant because invalidation-related UI cleanup should stay inside store-driven actions instead of ad hoc component mutation

## Proposed Improvement Approach

1. Document the invalidation authority split explicitly:
   - Rust owns real dataset invalidation
   - frontend owns presentation and safe reset behavior
2. Record the current invalidation triggers:
   - base replacement
   - live-bar update
   - derived-parent version mismatch
   - identical-source reuse
3. Add a user-facing vocabulary for stale-state explanation:
   - reused
   - freshly resampled
   - invalidated because base changed
4. Preserve the current safe clear behavior:
   - unknown ids stay harmless
   - stale descendants are removed eagerly
5. Tighten tests where invalidation affects:
   - validation context
   - startup restore
   - artifact replay

## Checklist

- [ ] Document the current invalidation triggers and reuse triggers.
- [ ] Document base/derived version relationships and descendant-tree removal.
- [ ] Document frontend clear/reset behavior and why unknown clears stay non-fatal.
- [ ] Document same-source base reuse separately from true invalidation.
- [ ] Add or tighten tests if invalidation messaging or command behavior changes.
- [ ] Identify where stale-dataset cues should surface in the UI without creating a second invalidation authority.

## Verification Plan

- run `cargo test -p ta_desktop_demo_app state`
- run `cargo test -p ta_desktop_demo_app commands_data`
- run `npm --prefix web test -- src/stores/dataStore.actions.test.ts`
- manually verify base reload, resample reuse, and clear flows still produce the expected state transitions

## Rollout / Fallback / Compatibility Notes

- keep Rust as the source of truth for invalidation
- prefer additive invalidation reasons or status text over silent behavior changes
- preserve current reuse semantics for identical source identity
- if stale-dataset UI cues are added later, ensure they reflect backend decisions rather than speculative frontend heuristics

## Related Docs

- `REPO_Improvement/02_data/23_resample_contract_2026-04-08.md`
- `REPO_Improvement/02_data/26_derived_dataset_identity_2026-04-08.md`
- `REPO_Improvement/02_data/27_dataset_meta_schema_2026-04-08.md`
- `REPO_Improvement/05_validation/25_validation_context_compat_2026-04-08.md`
