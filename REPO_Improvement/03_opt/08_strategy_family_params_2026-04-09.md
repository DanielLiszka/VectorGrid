# Strategy Family Params Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to strategy-family parameter coverage so exposed family-specific fields, staged family defaults, request-builder legality, and route-family boundaries stay coherent.

## Scope Boundaries

In scope:

- family-local sections in `StrategyWorkspacePanel.vue`
- family-local defaults and storage in `strategyStore.ts`
- family-local request-builder validation and routing
- staged families already present in the store but not fully surfaced

Out of scope:

- shared cost and risk controls
- backend algorithm redesigns
- ML Signal strategy parameters

## Current User-Facing Behavior

The strategy panel already exposes family-local controls for several strategy groups, including ASR, VCC, PIT, VFCT, ROCB, VCET, and VEMR. Those sections are visible only when their corresponding strategy is selected.

The store already carries additional staged family defaults for `Kqsr`, `Carm`, `Kvdf`, `Mkcs`, and `Cdmc`, even though those families are not fully exposed in the panel. Request-building and backend routing are already family-specific, so staged families already affect product truth even when the UI surface is partial.

Route families differ. PIT, VFCT, and composite-profile cases follow the Double-MA route family, while families like VCC, VEMR, ROCB, and VCET ride the Composite CPU product request surface with optional unified-GPU backend. That route split is fundamental to how family parameters should be documented.

## Current Code Surface

- `web/src/components/StrategyWorkspacePanel.vue`
- `web/src/stores/strategyStore.ts`
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/services/optimizeService.ts`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/composite_profile_route.rs`

## Current Verification Surface

- `web/src/services/optimizeRequestBuilder.test.ts`
- `web/src/services/optimizeService.test.ts`
- `web/src/stores/optimizeStore.actions.test.ts`
- `web/src/components/StrategyWorkspacePanel.test.ts`

## Strengths Already Present

- family-local parameter sections already exist in the product
- strategy defaults are already centralized in the strategy store
- request-builder logic already enforces family-specific routing and payloads
- staged families are at least represented in store defaults rather than hidden entirely
- route-family differences are already encoded in the repo

## Weaknesses / Failure Modes

- exposed family sections and staged store-backed families can drift apart
- family-local UI coverage is incomplete relative to the store and backend
- contributors can confuse route-family membership if there is no dedicated family matrix
- adding a family field without synchronized request-builder validation is easy in a large store
- current docs do not clearly separate “supported and surfaced” from “staged and partially surfaced”

## Regressions To Avoid

- mixing route families in the UI or request builder
- dropping staged family defaults accidentally
- adding visible family fields without validation or route support
- letting panel wording imply broader support than the product actually has
- breaking existing family defaults or stored snapshot behavior

## External Research Summary

- [Vue Form Input Bindings](https://vuejs.org/guide/essentials/forms.html) is relevant because these family sections are high-density conditional form surfaces.
- [Pinia Core Concepts](https://pinia.vuejs.org/core-concepts/) is relevant because family defaults and state ownership should stay centralized in the strategy store.
- [Pinia Testing](https://pinia.vuejs.org/cookbook/testing.html) is relevant because family-specific defaults, visibility, and request shaping need repeatable store-aware tests.
- [Tauri Calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because family-local fields become backend command payloads and must remain route-consistent.

## Proposed Improvement Approach

1. Produce one family matrix that distinguishes surfaced families, staged families, and route-family membership.
2. Freeze the ownership of defaults in `strategyStore.ts`.
3. Document which family fields are already productized and which are intentionally staged.
4. Add explicit UI-to-request parity checks for each surfaced family section.
5. Keep family route buckets visible in the implementation plan so contributors do not accidentally mix them.

## Checklist

- [ ] Document every currently surfaced family section and its fields.
- [ ] Document every staged family already present in the store.
- [ ] Document route-family membership for each strategy family.
- [ ] Add or tighten UI-to-request parity tests for surfaced family fields.
- [ ] Record a policy for adding future family fields without regressions.

## Verification Plan

- run `npm --prefix web test -- src/components/StrategyWorkspacePanel.test.ts`
- run `npm --prefix web test -- src/services/optimizeRequestBuilder.test.ts`
- run `npm --prefix web test -- src/services/optimizeService.test.ts`
- run `npm --prefix web test -- src/stores/optimizeStore.actions.test.ts`

## Rollout / Fallback / Compatibility Notes

- preserve current defaults for surfaced and staged families unless intentionally changed
- prefer explicit “staged” labeling over ambiguous partial exposure
- keep route-family truth authoritative if UI coverage lags behind store support
- document missing panel coverage rather than pretending it does not exist

## Related Docs

- `REPO_Improvement/03_opt/02_opt_strategy_backend_2026-04-09.md`
- `REPO_Improvement/03_opt/07_strategy_costs_risk_2026-04-09.md`
- `REPO_Improvement/03_opt/01_opt_panel_core_2026-04-09.md`
- `REPO_Improvement/07_strats/04_strat_breakout_momentum_2026-04-08.md`
