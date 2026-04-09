# Strategy Costs Risk Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to shared cost, execution-risk, and constraint controls so optimization requests reflect the visible strategy settings truthfully and route blockers stay understandable.

## Scope Boundaries

In scope:

- shared strategy cost fields
- execution overlays and trade-timing controls
- DM constraint surfaces
- Kaufman-stop and related route blockers
- optimize request-builder handling of these shared settings

Out of scope:

- family-local strategy parameters that belong to separate docs
- live execution policy outside shared optimization settings
- options-related execution settings

## Current User-Facing Behavior

The strategy panel already owns shared cost and risk controls such as cost presets, fee, spread, slippage, equity, stops, cooldown, signal persistence, long-only, flip behavior, and trade timing. It also exposes DM constraints and long-tail guardrails behind a grouped disclosure rather than overwhelming the default surface.

The product already derives helpful cost messaging. Cost presets update fee, spread, and slippage inputs, and the store computes a cost hint and commission fraction derived from those values. The builder turns the visible settings into common optimization configuration and constraints.

Some settings also influence route legality. Kaufman-stop and certain execution overlays already block pure GPU-kernel paths. That makes this surface more than simple UX polish; it is part of route truth.

## Current Code Surface

- `web/src/components/StrategyWorkspacePanel.vue`
- `web/src/stores/strategyStore.ts`
- `web/src/services/optimizeRequestBuilder.ts`
- `crates/app_core/src/double_ma_backend.rs`
- `crates/app_core/src/double_ma.rs`

## Current Verification Surface

- `web/src/components/StrategyWorkspacePanel.test.ts`
- `web/src/services/optimizeRequestBuilder.test.ts`
- `crates/app_core/src/double_ma_backend.rs`
- `crates/app_core/src/double_ma.rs`

## Strengths Already Present

- shared cost and risk settings are already centralized in the strategy panel and store
- cost presets already derive meaningful user-facing cost hints
- long-tail constraints are already grouped behind a clearer disclosure
- request-builder logic already maps shared settings into backend payloads
- route blockers for Kaufman stop and execution overlays already exist

## Weaknesses / Failure Modes

- shared cost or risk settings can be mistaken for strategy-local settings because they live near family sections
- route-blocker consequences are easy to lose if UI messaging and backend errors diverge
- validation and live surfaces may reuse these settings, but the ownership boundary is not explicitly documented
- the long list of constraints can drift away from request-builder validation if not frozen as a contract
- derived cost behavior can be broken by store refactors if not treated as product logic

## Regressions To Avoid

- hiding active constraints or making them invisible in the UI
- breaking derived cost hint and commission behavior
- loosening current GPU blocker rules without coordinated backend changes
- letting request-builder payloads drift from visible settings
- making shared settings diverge across optimize, validation, and live reuse paths

## External Research Summary

- [Vue Form Input Bindings](https://vuejs.org/guide/essentials/forms.html) is relevant because this surface is a dense form with interdependent fields and derived feedback.
- [Pinia Core Concepts](https://pinia.vuejs.org/core-concepts/) is relevant because derived cost state and shared setting ownership belong in canonical store state.
- [Pinia Testing](https://pinia.vuejs.org/cookbook/testing.html) is relevant because cost-preset derivation and constraint visibility are store-driven behaviors that need strong regression tests.
- [Tauri Calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because these settings ultimately become backend request contracts and route blockers.

## Proposed Improvement Approach

1. Document shared cost and execution-risk settings separately from family-local strategy parameters.
2. Freeze derived cost behavior and cost-preset application rules.
3. Make route-blocker implications for shared settings explicit in the implementation plan.
4. Document reuse expectations for validation and live paths that depend on the same settings.
5. Expand tests for overlay-driven route blocking if current coverage is insufficient.

## Checklist

- [ ] Document all shared cost and execution-risk settings.
- [ ] Document cost-preset derivation and cost-hint behavior.
- [ ] Document Kaufman-stop and execution-overlay route blockers.
- [ ] Document DM constraints and long-tail guardrail behavior.
- [ ] Add or tighten tests for request-builder parity and route blocking.

## Verification Plan

- run `npm --prefix web test -- src/components/StrategyWorkspacePanel.test.ts`
- run `npm --prefix web test -- src/services/optimizeRequestBuilder.test.ts`
- run targeted Rust route tests covering Kaufman-stop or overlay blockers
- perform one manual route-preview check when shared setting or blocker messaging changes

## Rollout / Fallback / Compatibility Notes

- preserve current shared-setting defaults and derived-cost behavior unless the product intentionally changes them
- prefer better messaging and explicit docs before changing actual route legality
- if blocker behavior changes, update request-builder, route summaries, and tests together
- keep shared settings reusable across optimize, validation, and live surfaces

## Related Docs

- `REPO_Improvement/03_opt/08_strategy_family_params_2026-04-09.md`
- `REPO_Improvement/03_opt/05_opt_route_gpu_capability_2026-04-09.md`
- `REPO_Improvement/05_validation/03_validation_window_budget_2026-04-08.md`
- `REPO_Improvement/09_live/01_execution_command_surface_2026-04-08.md`
