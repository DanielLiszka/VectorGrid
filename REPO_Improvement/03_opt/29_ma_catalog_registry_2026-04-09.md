# Moving Average Catalog Registry Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to the moving-average catalog registry so the authoritative indicator inventory, kernel-support truth, parameter schemas, and frontend normalization remain stable, explicit, and regression-resistant.

## Scope Boundaries

In scope:

- backend moving-average registry export through Tauri
- frontend moving-average catalog normalization and caching
- kernel-support and kernel-preference truth used for UI hinting and route planning
- parameter-schema, notes, and eligibility data consumed by optimize surfaces

Out of scope:

- dynamic-indicator catalog behavior outside moving averages
- backend route resolution itself
- options-related catalog surfaces

## Current User-Facing Behavior

The repo already exposes a normalized moving-average catalog to the frontend. The backend command returns VectorTA registry entries enriched with app-local kernel support and kernel-preference policy, and the frontend converts the raw snake_case response into a stable camelCase catalog used by optimize forms, MA editors, and route messaging.

That means the catalog is not just a convenience list. It is the user-visible authority for what MA ids exist, which backends they support, whether they are sweep-capable, whether they require candle-structured input, whether they are period-based, and what parameter schema each MA exposes.

Because this data is consumed early in the optimize flow, drift here can silently distort the rest of the user experience before any optimization request is even built.

## Current Code Surface

- `src-tauri/src/ma_registry.rs`
- `src-tauri/src/commands_data.rs`
- `crates/app_core/src/ma_policy.rs`
- `web/src/services/movingAverageCatalogService.ts`
- `web/src/components/OptimizeWorkspacePanel.vue`
- `web/src/services/optimizeRequestBuilder.ts`

## Current Verification Surface

- `web/src/services/movingAverageCatalogService.test.ts`
- `crates/app_core/src/ma_policy.rs`
- `web/src/services/optimizeRequestBuilder.test.ts`
- `web/src/components/OptimizeWorkspacePanel.test.ts`

## Strengths Already Present

- the backend already treats the VectorTA MA registry as the authoritative source of ids, labels, notes, and parameter schemas
- Tauri already enriches the raw registry with app-local kernel-support and kernel-preference fields
- the frontend already normalizes the raw payload into a cached typed catalog
- duplicate parameter keys are already deduplicated during normalization
- the catalog already carries enough metadata to drive both UI messaging and request-building decisions

## Weaknesses / Failure Modes

- kernel support, CUDA sweep support, and kernel preference are close enough semantically that contributors can conflate them
- the frontend normalization layer can drift from the backend shape if field names change silently
- parameter-schema ordering and deduplication rules are not yet frozen in a dedicated implementation plan
- UI consumers can start treating convenience flags as hard legality instead of advisory policy
- catalog caching can hide contract regressions if tests do not force refresh or reset state

## Regressions To Avoid

- changing field names or semantics without synchronized frontend normalization updates
- conflating `supports_cuda_ma_sweep`, `supports_cuda_kernel`, and `prefers_cuda_kernel`
- dropping or reordering MA params in ways that break existing editors
- letting empty or malformed ids survive normalization
- treating catalog preference hints as the same thing as runtime route guarantees

## External Research Summary

- [Tauri Calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because the MA catalog is a typed backend contract crossing the Tauri command boundary.
- [Tauri State Management](https://v2.tauri.app/develop/state-management/) is relevant because the registry, kernel policy, and cached frontend consumers all depend on a stable long-lived source of truth.
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because this catalog should remain a shared service-level source of truth, not be re-normalized independently inside components.
- [Pinia Testing](https://pinia.vuejs.org/cookbook/testing.html) is relevant because catalog-driven optimize flows depend on deterministic normalization and cache-reset behavior in tests.
- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because the catalog already behaves like a structured compatibility contract, especially for parameter descriptors.

## Proposed Improvement Approach

1. Freeze the authority chain explicitly: VectorTA registry for MA identity and base capabilities, app-core policy for kernel-support and preference overlays, Tauri for serialization, frontend for normalization only.
2. Document each boolean capability field separately so UI and builder code do not collapse them into a single “GPU supported” concept.
3. Treat parameter-schema ordering, dedupe, and null-handling as compatibility behavior rather than incidental normalization cleanup.
4. Keep the frontend catalog service small and typed, with cache reset and forced refresh kept testable.
5. Add or tighten tests whenever catalog shape, kernel policy, or parameter-schema transformation changes.

## Checklist

- [ ] Document the MA catalog authority chain from VectorTA registry through frontend normalization.
- [ ] Document the meaning of each capability and preference field separately.
- [ ] Document parameter-schema ordering, dedupe, and numeric-normalization expectations.
- [ ] Document cache-reset, force-refresh, and stale-state expectations for tests.
- [ ] Add or tighten tests when new MA fields, capabilities, or params are introduced.

## Verification Plan

- run `npm --prefix web test -- src/services/movingAverageCatalogService.test.ts`
- run `npm --prefix web test -- src/services/optimizeRequestBuilder.test.ts`
- run `npm --prefix web test -- src/components/OptimizeWorkspacePanel.test.ts`
- run targeted `ma_policy` tests in `crates/app_core/src/ma_policy.rs`

## Rollout / Fallback / Compatibility Notes

- preserve current serialized field names until all frontend consumers migrate together
- prefer additive catalog fields over repurposing existing booleans
- if VectorTA adds new MA metadata, stage it through Tauri and frontend normalization in one change
- keep UI hinting advisory unless backend legality has also been updated and verified

## Related Docs

- `REPO_Improvement/03_opt/03_opt_ma_grid_editor_2026-04-09.md`
- `REPO_Improvement/03_opt/05_opt_route_gpu_capability_2026-04-09.md`
- `REPO_Improvement/03_opt/18_eng_dm_request_resolution_2026-04-09.md`
- `REPO_Improvement/03_opt/31_gpu_speedups_policy_2026-04-09.md`
