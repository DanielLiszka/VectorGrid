# Strategy Store Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to the strategy store so shared strategy state, family defaults, derived fields, and snapshot rehydration remain explicit, testable, and non-regressive.

## Scope Boundaries

In scope:

- `web/src/stores/strategyStore.ts`
- strategy-store action behavior and state ownership
- derived fields such as `commission` and `costHint`
- snapshot and history rehydration for strategy state

Out of scope:

- strategy panel presentation details by themselves
- advanced-store ownership
- backend request-builder legality beyond the store contract

## Current User-Facing Behavior

The strategy store already owns a large amount of canonical strategy state for the product. That includes shared cost settings, strategy-family parameters, exit settings, overlay controls, and many staged or surfaced family defaults.

Some of the store’s fields are not passive. Cost presets apply concrete fee, spread, and slippage defaults, and the store derives `commission` and `costHint` from those values. Snapshot rehydration restores a very large state surface, and save or restore flows already depend on that behavior. The store also enforces a notable semantic rule: `tradeOnNext` is forced true in checkbox handling.

Because the store already contains behavior, not just inventory, it deserves a dedicated implementation plan separate from the strategy panel doc.

## Current Code Surface

- `web/src/stores/strategyStore.ts`
- `web/src/stores/strategyStore.actions.test.ts`
- `web/src/components/StrategyWorkspacePanel.vue`
- `web/src/services/historyService.ts`
- `web/src/services/optimizeRequestBuilder.ts`

## Current Verification Surface

- `web/src/stores/strategyStore.actions.test.ts`
- `web/src/components/StrategyWorkspacePanel.test.ts`
- `web/src/services/historyService.test.ts`
- `web/src/services/optimizeRequestBuilder.test.ts`

## Strengths Already Present

- the strategy store is already the canonical owner of a large strategy-state surface
- derived cost fields are already centralized instead of reconstructed in the panel
- cost presets already behave like a real stateful policy rather than static labels
- snapshot and history rehydration already exist
- tests already cover Vue-first store action behavior

## Weaknesses / Failure Modes

- the store is large enough that it is easy to treat it as a dump of fields instead of a behavior contract
- derived-field behavior and semantic rules such as `tradeOnNext` can be changed accidentally
- there is no dedicated implementation doc distinguishing passive fields from behavior-bearing fields
- snapshot replay compatibility is important but under-documented
- ownership boundaries between strategy store and builder or panel can drift

## Regressions To Avoid

- breaking cost-preset application and derived `costHint` or `commission`
- changing `tradeOnNext` semantics silently
- weakening snapshot or history rehydration
- moving canonical strategy truth back into component-local state
- adding new store behavior without tests that freeze it

## External Research Summary

- [Pinia Core Concepts](https://pinia.vuejs.org/core-concepts/) is relevant because this store is a canonical ownership surface, not just a convenience wrapper.
- [Pinia Testing](https://pinia.vuejs.org/cookbook/testing.html) is relevant because derived state and snapshot replay behavior need store-aware tests.
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because the strategy store should stay authoritative over panel-local drafts.
- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because history and snapshot payloads behave like serialized state contracts.

## Proposed Improvement Approach

1. Separate strategy-store inventory from strategy-store behavior in the implementation plan.
2. Freeze derived cost-field behavior and `tradeOnNext` semantics explicitly.
3. Document snapshot and history rehydration expectations.
4. Keep the strategy panel as a consumer of this store rather than a parallel owner.
5. Add or tighten tests around behavior-bearing fields before store cleanup work.

## Checklist

- [ ] Document canonical strategy-store ownership.
- [ ] Document derived cost behavior and cost-preset application.
- [ ] Document special semantic rules such as `tradeOnNext`.
- [ ] Document snapshot and history rehydration expectations.
- [ ] Add or tighten tests for behavior-bearing fields.

## Verification Plan

- run `npm --prefix web test -- src/stores/strategyStore.actions.test.ts`
- run `npm --prefix web test -- src/components/StrategyWorkspacePanel.test.ts`
- run `npm --prefix web test -- src/services/historyService.test.ts`
- run `npm --prefix web test -- src/services/optimizeRequestBuilder.test.ts`

## Rollout / Fallback / Compatibility Notes

- preserve current derived-field behavior until a deliberate product change is made
- prefer clearer ownership docs before splitting or shrinking the store
- keep snapshot compatibility explicit during refactors
- if semantic rules change, update store tests and consumer docs together

## Related Docs

- `REPO_Improvement/03_opt/07_strategy_costs_risk_2026-04-09.md`
- `REPO_Improvement/03_opt/08_strategy_family_params_2026-04-09.md`
- `REPO_Improvement/03_opt/18_eng_dm_request_resolution_2026-04-09.md`
- `REPO_Improvement/03_opt/27_advanced_store_2026-04-09.md`
