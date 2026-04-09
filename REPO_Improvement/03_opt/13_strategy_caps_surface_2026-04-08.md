# Strategy Capability Surface Plan

Last updated: 2026-04-09

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
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/) reinforces keeping backend authority explicit where frontend previews are advisory.
- [Vue Computed Properties](https://vuejs.org/guide/essentials/computed) supports deriving capability presentation from a small, stable source of truth.
- [Pinia Getters](https://pinia.vuejs.org/core-concepts/getters.html) is relevant if capability summaries move toward store-backed derived state.
- [JSON Schema: Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant for documenting a structured capability matrix or preview payload.
- [NVIDIA CUDA C Best Practices Guide](https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/)
  - relevant because GPU and route-policy improvements should preserve correctness, observability, and profiling discipline instead of chasing speed blindly

## Proposed Improvement Approach

1. Define one authoritative capability matrix for the supported non-options strategy families and backend modes.
2. Label each rule as one of:
   - preview-only guidance
   - request-builder enforcement
   - Rust enforcement
3. Document alias behavior for backend names so standalone UI copy, request payloads, and backend resolution use the same language.
4. Add parity fixtures that compare frontend capability summaries against backend preview/resolution outputs.
5. Make route-family differences explicit instead of implying that one strategy-wide capability badge is enough.
6. Treat optimizer-surface code quality as part of the implementation scope: keep request shaping, route policy, GPU ownership, diagnostics, and strategy-local boundaries readable and testable in a public repo.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Define an authoritative capability matrix for supported strategies/backends.
- [ ] Mark each capability rule by authority layer.
- [ ] Document backend alias behavior and route-family distinctions.
- [ ] Identify capability rules that are duplicated or drifting today.
- [ ] Add parity tests between frontend summaries and Rust preview/resolution outputs.
- [ ] Tighten optimize UI copy so it does not over-promise unsupported paths.
- [ ] Verify dynamic/composite routes are covered, not just simple DoubleMa paths.
- [ ] Keep current exact/fallback safety behavior intact.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm test -- src/services/optimizeRequestBuilder.test.ts`
- run `npm test -- src/services/optimizeService.test.ts`
- run targeted optimize workspace component tests
- run targeted `ta_app_core` route/capability tests for supported strategy families
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve current route-resolution authority in Rust
- treat frontend capability cleanup as additive truthfulness work, not as permission to hide advanced controls casually
- if a structured capability payload is added later, keep current frontend helpers working until the new contract is fully adopted
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/03_opt/11_opt_request_builder_2026-04-08.md`
- `REPO_Improvement/03_opt/19_eng_dm_backend_routing_2026-04-08.md`
- `REPO_Improvement/03_opt/25_optimize_store_2026-04-08.md`
- `REPO_Improvement/07_strats/20_strat_surface_matrix_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/42_tauri_command_parity_2026-04-09.md`
