# Recommended Optimization Defaults Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to recommended optimization defaults so fallback repair behavior for missing or invalid output settings remains explicit, conservative, and non-opinionated.

## Scope Boundaries

In scope:

- `web/src/services/recommendedOptimizationDefaults.ts`
- startup, panel-mount, and pre-run application sites
- fallback defaults for top-K and heatmap bins
- explicit preservation of user-disabled heatmap values

Out of scope:

- preset-profile logic
- backend request legality
- non-output advanced defaults

## Current User-Facing Behavior

The repo already applies a small set of recommended optimization defaults. Those defaults are not full preset profiles. They only repair missing or invalid output settings by filling in `Top K 50` and, when the effective mode is exactly `Grid`, `heatmap bins 64`.

The behavior is already intentionally conservative. An explicit `0` heatmap value is preserved as disabled rather than overwritten. Heatmap defaults are not injected for `Auto` or other non-grid modes. The defaults can be applied at startup, on advanced-panel mount, and quietly before a run if the current state is missing or invalid.

That makes this logic a repair layer, not an opinionated profile system, and that distinction should be frozen in a dedicated plan.

## Current Code Surface

- `web/src/services/recommendedOptimizationDefaults.ts`
- `web/src/services/recommendedOptimizationDefaults.test.ts`
- `web/src/main.ts`
- `web/src/components/AdvancedWorkspacePanel.vue`
- `web/src/services/optimizeService.ts`

## Current Verification Surface

- `web/src/services/recommendedOptimizationDefaults.test.ts`
- `web/src/components/AdvancedWorkspacePanel.test.ts`
- `web/src/services/optimizeService.test.ts`
- `web/src/main.test.ts`

## Strengths Already Present

- the default-repair logic already lives in its own service
- the service already distinguishes repair from preset application
- explicit `0` heatmap behavior is already preserved
- application sites already cover startup, panel, and pre-run paths
- tests already exist for the helper behavior

## Weaknesses / Failure Modes

- contributors can confuse recommended defaults with preset profiles
- the helper is small enough that its product meaning can be missed
- application at multiple lifecycle points can drift if not documented
- heatmap defaults are mode-sensitive, which is easy to weaken accidentally
- there is no dedicated implementation doc freezing “repair, not preference” as the core rule

## Regressions To Avoid

- overwriting explicit `0` heatmap values
- applying grid heatmap defaults in non-grid or `Auto` modes
- turning recommended defaults into a hidden preset profile system
- drifting startup, panel, and pre-run application behavior
- burying this logic inside larger services or components

## External Research Summary

- [Pinia Core Concepts](https://pinia.vuejs.org/core-concepts/) is relevant because this helper repairs canonical store state rather than component-local state.
- [Pinia Testing](https://pinia.vuejs.org/cookbook/testing.html) is relevant because the main contract is deterministic state repair, which benefits from focused tests.
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because recommended defaults should remain a store-facing helper, not duplicated panel logic.
- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because the repaired fields are part of structured optimization payloads with compatibility expectations.

## Proposed Improvement Approach

1. Freeze recommended defaults as fallback repair rather than user-facing preset logic.
2. Document all application sites and why each one exists.
3. Preserve explicit `0` values and non-grid mode behavior as tested product rules.
4. Keep the helper small and isolated instead of merging it into unrelated services.
5. Add or tighten tests if lifecycle application points change.

## Checklist

- [ ] Document the exact default values and when they apply.
- [ ] Document the difference between repair defaults and preset profiles.
- [ ] Document startup, mount, and pre-run application sites.
- [ ] Document preservation of explicit `0` and non-grid mode behavior.
- [ ] Add or tighten lifecycle tests if application timing changes.

## Verification Plan

- run `npm --prefix web test -- src/services/recommendedOptimizationDefaults.test.ts`
- run `npm --prefix web test -- src/components/AdvancedWorkspacePanel.test.ts`
- run `npm --prefix web test -- src/services/optimizeService.test.ts`
- run `npm --prefix web test -- src/main.test.ts`

## Rollout / Fallback / Compatibility Notes

- preserve current small-scope repair behavior unless the product deliberately introduces broader defaults
- keep this helper separate from preset-profile application
- document lifecycle changes before moving the helper to a new call site
- if the default values change later, update helper tests and this plan together

## Related Docs

- `REPO_Improvement/03_opt/18_eng_dm_request_resolution_2026-04-09.md`
- `REPO_Improvement/03_opt/27_advanced_store_2026-04-09.md`
- `REPO_Improvement/03_opt/09_advanced_search_objective_2026-04-09.md`
- `REPO_Improvement/03_opt/10_advanced_shortlist_export_2026-04-09.md`
