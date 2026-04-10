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
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Pinia Core Concepts](https://pinia.vuejs.org/core-concepts/) is relevant because this store is a canonical ownership surface, not just a convenience wrapper.
- [Pinia Testing](https://pinia.vuejs.org/cookbook/testing.html) is relevant because derived state and snapshot replay behavior need store-aware tests.
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because the strategy store should stay authoritative over panel-local drafts.
- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because history and snapshot payloads behave like serialized state contracts.
- [NVIDIA CUDA C Best Practices Guide](https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/)
  - relevant because GPU and route-policy improvements should preserve correctness, observability, and profiling discipline instead of chasing speed blindly

## Proposed Improvement Approach

1. Separate strategy-store inventory from strategy-store behavior in the implementation plan.
2. Freeze derived cost-field behavior and `tradeOnNext` semantics explicitly.
3. Document snapshot and history rehydration expectations.
4. Keep the strategy panel as a consumer of this store rather than a parallel owner.
5. Add or tighten tests around behavior-bearing fields before store cleanup work.
6. Treat optimizer-surface code quality as part of the implementation scope: keep request shaping, route policy, GPU ownership, diagnostics, and strategy-local boundaries readable and testable in a public repo.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document canonical strategy-store ownership.
- [x] Document derived cost behavior and cost-preset application.
- [x] Document special semantic rules such as `tradeOnNext`.
- [x] Document snapshot and history rehydration expectations.
- [x] Add or tighten tests for behavior-bearing fields.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/stores/strategyStore.actions.test.ts`
- run `npm --prefix web run test -- src/components/StrategyWorkspacePanel.test.ts`
- run `npm --prefix web run test -- src/services/historyService.test.ts`
- run `npm --prefix web run test -- src/services/optimizeRequestBuilder.test.ts`
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve current derived-field behavior until a deliberate product change is made
- prefer clearer ownership docs before splitting or shrinking the store
- keep snapshot compatibility explicit during refactors
- if semantic rules change, update store tests and consumer docs together
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closed a real contract gap in saved-run replay. `historyService.ts` no longer writes cost fields and `tradeOnNext` directly into `strategyStore`; it now routes replayed strategy values through store setters so derived `commission`/`costHint` recompute correctly and the store’s forced `tradeOnNext=true` rule still applies on hydration.
- Tightened direct store coverage to match the doc’s behavior claims. `strategyStore.actions.test.ts` now includes explicit replay-oriented coverage showing that `applyLegacySnapshot(...)` preserves the store’s forced `tradeOnNext` semantics while still restoring cost fields and behavior-bearing values.
- Revalidated the consumer side too: `historyService.test.ts` now proves an `MlSignal` saved-run replay recomputes strategy cost fields and preserves `tradeOnNext=true` even when the saved strategy payload requests `trade_on_next_bar=false`.
- Acceptance evidence captured during execution: `npm.cmd --prefix web run test -- src/services/historyService.test.ts src/stores/strategyStore.actions.test.ts src/stores/advancedStore.actions.test.ts src/services/dynamicIndicatorCatalogService.test.ts src/strategies/dynamicDoubleMaBuilder.test.ts`, `npm.cmd --prefix web run lint`, and `npm.cmd --prefix web run typecheck`.

## Related Docs

- `REPO_Improvement/03_opt/07_strategy_costs_risk_2026-04-09.md`
- `REPO_Improvement/03_opt/08_strategy_family_params_2026-04-09.md`
- `REPO_Improvement/03_opt/18_eng_dm_request_resolution_2026-04-09.md`
- `REPO_Improvement/03_opt/27_advanced_store_2026-04-09.md`
