# Double MA Request Resolution Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to Double-MA-family request resolution so store reads, recommended defaults, request-builder legality, dynamic preview normalization, and command selection remain explicit and auditable.

## Scope Boundaries

In scope:

- optimize-service request construction for Double-MA-family runs
- request-builder ownership and normalization
- recommended optimization defaults
- store-field binding resolution across optimize, advanced, and strategy stores
- dynamic preview normalization into final request payloads

Out of scope:

- backend algorithm implementation
- results rendering
- options-related request building

## Current User-Facing Behavior

The frontend already resolves Double-MA-family runs through a multi-layer process. `optimizeService.ts` reads state from the optimize, advanced, strategy, and data stores; applies recommended output defaults where needed; runs dynamic preview when a dynamic strategy IR is involved; and then hands normalized inputs to the request builder.

The request builder already performs the real legality and normalization work. It normalizes backend aliases, optimization mode, objective and score mode, advanced shortlist settings, export legality, metric contracts, trade-metric behavior, and VDPS fields. It also constructs route-dependent backend payloads and embeds normalized `dynamic_strategy_ir` payloads when available.

The final command selection is then derived from the normalized strategy kind. Double-MA-family strategies use `run_double_ma_optimization`, Composite CPU families use `run_composite_cpu_optimization`, and `CompositeMo` uses its own command. This means request resolution is already sophisticated, but it needs a dedicated plan because the logic is distributed across stores, services, and builders.

## Current Code Surface

- `web/src/services/optimizeService.ts`
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/services/recommendedOptimizationDefaults.ts`
- `web/src/stores/optimizeStore.ts`
- `web/src/stores/advancedStore.ts`
- `web/src/stores/strategyStore.ts`
- `src-tauri/src/commands_optimize.rs`

## Current Verification Surface

- `web/src/services/optimizeService.test.ts`
- `web/src/services/optimizeRequestBuilder.test.ts`
- `web/src/stores/optimizeStore.actions.test.ts`
- `web/src/components/AdvancedWorkspacePanel.test.ts`
- `web/src/components/StrategyWorkspacePanel.test.ts`

## Strengths Already Present

- request construction is already centralized in service and builder layers rather than spread across components
- the request builder already owns a large amount of legality and normalization logic
- recommended output defaults are already isolated in their own helper
- dynamic preview already normalizes strategy IR before final request use
- command resolution is already explicit by strategy family

## Weaknesses / Failure Modes

- request resolution spans several stores and service helpers, which is hard to reason about without a dedicated contract
- duplicated field ownership across stores increases drift risk
- recommended defaults are helpful but easy to forget as part of request resolution
- dynamic preview normalization can be mistaken for request-builder ownership if the layering is not documented
- command selection and request building can drift if strategy-family resolution changes in one place only

## Regressions To Avoid

- bypassing the request builder and reintroducing component-local request construction
- breaking recommended output defaults for missing or invalid top-K or heatmap values
- letting dynamic preview and final request payloads diverge
- weakening legality checks for advanced settings, export, or metric contracts
- misrouting normalized requests to the wrong optimization command

## External Research Summary

- [Pinia Core Concepts](https://pinia.vuejs.org/core-concepts/) is relevant because request resolution reads from several canonical stores and must keep ownership boundaries explicit.
- [Pinia Testing](https://pinia.vuejs.org/cookbook/testing.html) is relevant because the main regression risk is cross-store state drift and normalization bugs.
- [Tauri Calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because request resolution terminates at a typed command boundary.
- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because normalized request payloads and dynamic strategy IRs behave like structured compatibility contracts.

## Proposed Improvement Approach

1. Document the full request-resolution pipeline from store reads to command invocation.
2. Freeze the request builder as the authoritative legality layer.
3. Document recommended defaults as part of request resolution rather than a side utility.
4. Separate dynamic preview normalization from final request construction in the plan.
5. Add parity checks between strategy-family resolution, builder output, and command selection.

## Checklist

- [ ] Document all store inputs used by request resolution.
- [ ] Document the role of recommended defaults in missing-value cleanup.
- [ ] Document request-builder authority for normalization and legality.
- [ ] Document dynamic preview normalization and final payload handoff.
- [ ] Add or tighten tests for store drift, defaults, and command-selection parity.

## Verification Plan

- run `npm --prefix web test -- src/services/optimizeService.test.ts`
- run `npm --prefix web test -- src/services/optimizeRequestBuilder.test.ts`
- run `npm --prefix web test -- src/stores/optimizeStore.actions.test.ts`
- run `npm --prefix web test -- src/components/AdvancedWorkspacePanel.test.ts`
- run `npm --prefix web test -- src/components/StrategyWorkspacePanel.test.ts`

## Rollout / Fallback / Compatibility Notes

- keep request-builder authority intact even if store ownership is cleaned up later
- preserve recommended default behavior unless the product explicitly changes the output-default policy
- treat dynamic preview as an additive normalization stage, not a silent override of builder legality
- if command selection changes, update this plan and optimize-service tests together

## Related Docs

- `REPO_Improvement/03_opt/14_opt_double_ma_command_2026-04-09.md`
- `REPO_Improvement/03_opt/09_advanced_search_objective_2026-04-09.md`
- `REPO_Improvement/03_opt/28_recommended_defaults_2026-04-09.md`
- `REPO_Improvement/03_opt/27_advanced_store_2026-04-09.md`
