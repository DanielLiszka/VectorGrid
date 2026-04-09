# Strategy Capability Surface Plan

Last updated: 2026-04-08

## Purpose

Plan improvements to the non-options strategy-capability surface so the optimize UI, request builder, and backend route preview share one truthful contract for what each strategy/backend combination can actually do.

## Scope Boundaries

In scope:

- strategy capability summaries shown in the optimize flow
- route/backend capability hints
- frontend capability helpers vs backend authority
- parity testing across capability layers

Out of scope:

- options strategy capabilities
- introducing new strategy families
- redesigning the entire optimize workspace layout

## Current User-Facing Behavior

Users see capability hints and backend availability based on the current strategy and requested backend. The frontend provides fast guidance about what is supported, but the real authority still comes from backend request resolution and route preview logic.

## Current Code Surface

- `web/src/utils/strategyOptimizationCapabilities.ts`
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/services/optimizeService.ts`
- `web/src/components/OptimizeWorkspacePanel.vue`
- `crates/app_core/src/double_ma_backend.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/composite_profile_route.rs`

## Current Verification Surface

- `web/src/services/optimizeRequestBuilder.test.ts`
- `web/src/services/optimizeService.test.ts`
- optimize workspace component tests
- backend route and strategy tests in `ta_app_core`

## Strengths Already Present

- capability logic is already partly centralized in one frontend helper
- route previews and backend resolution already exist in Rust
- the optimize UI already exposes strategy/backend awareness instead of treating all strategies the same
- request-builder tests already cover important legality combinations

## Weaknesses / Failure Modes

- frontend capability hints and backend authority can drift
- alias handling around GPU-related backends is easy to interpret inconsistently
- the current surface does not clearly separate preview-only guidance from hard enforcement
- dynamic and composite strategies increase the risk of stale capability assumptions
- a user can see a capability hint that is technically true in one route family but false in another

## Regressions To Avoid

- hiding controls that are currently legal
- showing controls as supported when Rust will reject them
- collapsing route-specific differences into one oversimplified capability badge
- weakening the current exact/fallback safety paths
- changing capability copy without updating request-builder and backend tests

## External Research Summary

- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/) reinforces keeping backend authority explicit where frontend previews are advisory.
- [Vue Computed Properties](https://vuejs.org/guide/essentials/computed) supports deriving capability presentation from a small, stable source of truth.
- [Pinia Getters](https://pinia.vuejs.org/core-concepts/getters.html) is relevant if capability summaries move toward store-backed derived state.
- [JSON Schema: Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant for documenting a structured capability matrix or preview payload.

## Proposed Improvement Approach

1. Define one authoritative capability matrix for the supported non-options strategy families and backend modes.
2. Label each rule as one of:
   - preview-only guidance
   - request-builder enforcement
   - Rust enforcement
3. Document alias behavior for backend names so standalone UI copy, request payloads, and backend resolution use the same language.
4. Add parity fixtures that compare frontend capability summaries against backend preview/resolution outputs.
5. Make route-family differences explicit instead of implying that one strategy-wide capability badge is enough.

## Checklist

- [ ] Define an authoritative capability matrix for supported strategies/backends.
- [ ] Mark each capability rule by authority layer.
- [ ] Document backend alias behavior and route-family distinctions.
- [ ] Identify capability rules that are duplicated or drifting today.
- [ ] Add parity tests between frontend summaries and Rust preview/resolution outputs.
- [ ] Tighten optimize UI copy so it does not over-promise unsupported paths.
- [ ] Verify dynamic/composite routes are covered, not just simple DoubleMa paths.
- [ ] Keep current exact/fallback safety behavior intact.

## Verification Plan

- run `npm test -- src/services/optimizeRequestBuilder.test.ts`
- run `npm test -- src/services/optimizeService.test.ts`
- run targeted optimize workspace component tests
- run targeted `ta_app_core` route/capability tests for supported strategy families

## Rollout / Fallback / Compatibility Notes

- preserve current route-resolution authority in Rust
- treat frontend capability cleanup as additive truthfulness work, not as permission to hide advanced controls casually
- if a structured capability payload is added later, keep current frontend helpers working until the new contract is fully adopted

## Related Docs

- `REPO_Improvement/03_opt/11_opt_request_builder_2026-04-08.md`
- `REPO_Improvement/03_opt/19_eng_dm_backend_routing_2026-04-08.md`
- `REPO_Improvement/03_opt/25_optimize_store_DATE.md`
- `REPO_Improvement/07_strats/20_strat_surface_matrix_DATE.md`
- `REPO_Improvement/10_tests_perf/42_tauri_command_parity_DATE.md`
