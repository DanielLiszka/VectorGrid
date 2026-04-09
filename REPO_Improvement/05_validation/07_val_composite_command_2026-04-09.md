# Composite Validation Command Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to the composite-family validation command path so route restrictions, strategy-family admission, cross-ticker behavior, and returned route/verdict semantics remain explicit and dependable.

## Scope Boundaries

In scope:

- `src-tauri/src/commands_validation.rs`
- `crates/app_core/src/composite_validate.rs`
- composite-family validation command routing for `CompositeCpu`, `CompositeMo`, `Rocb`, `Vcet`, `Vcc`, `Vemr`, `Mkcs`, `Kqsr`, `Carm`, `Kvdf`, `Cdmc`
- frontend preview/service behavior that depends on composite validation route semantics

Out of scope:

- Double-MA-family validation routing
- validation results table rendering
- options-related validation flows

## Current User-Facing Behavior

The frontend routes composite-family strategies through `run_composite_validation`, again carrying a `validation_context` that guards against dataset/timeframe/strategy mismatches. Current behavior is not uniform across families: plain `CompositeCpu` has GPU-route restrictions, `CompositeMo` currently uses the CPU validation route for fixed-candidate validation, and several composite-derived strategy kinds still share the composite command while preserving distinct strategy-kind identity in the request context.

The service already turns these backend and capability decisions into route summaries, warnings, and route-detail notes. In practice this means the user sees when `CompositeMo` is forced into the CPU validation route or when a requested GPU route is only conditionally supported for a narrower subset of finalists.

## Current Code Surface

- `src-tauri/src/commands_validation.rs`
- `src-tauri/src/validation_common.rs`
- `crates/app_core/src/composite_validate.rs`
- `web/src/services/validationRequestBuilder.ts`
- `web/src/services/validationService.ts`

## Current Verification Surface

- `src-tauri/src/commands_validation.rs` tests
- `web/src/services/validationRequestBuilder.test.ts`
- `web/src/services/validationService.test.ts`

## Strengths Already Present

- the command already accepts a broad but explicit set of composite-family strategy kinds
- frontend builder tests already cover important route restrictions and warnings
- composite routing differences such as `CompositeMo` CPU validation are already exposed to the user
- backend context checks already reject mismatched artifact/runtime combinations
- route summary and warning text already distinguish requested-versus-supported behavior

## Weaknesses / Failure Modes

- composite-family routing is more heterogeneous than the Double-MA path, so implicit assumptions are risky
- `CompositeMo` and plain `CompositeCpu` restrictions can be hidden if preview text and backend behavior drift
- several composite-derived strategy kinds share the same command, increasing the chance of strategy-kind admission drift
- route behavior may look simpler in the UI than it really is unless warnings and details are preserved carefully

## Regressions To Avoid

- routing composite-family strategies through the wrong command or strategy-kind validator
- losing explicit warnings for `CompositeMo` CPU validation or plain `CompositeCpu` GPU restrictions
- weakening validation-context mismatch protection
- returning route strings that no longer match frontend preview semantics
- hiding omitted-module consequences during composite cross reruns or route fallbacks

## External Research Summary

- [Calling the Frontend from Rust | Tauri](https://v2.tauri.app/es/develop/calling-frontend/) is relevant because the composite validation command is a typed frontend/backend contract, not an internal-only helper.
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because preview and post-run composite route messaging should remain derived from shared service/store state.
- [Pinia Testing Cookbook](https://pinia.vuejs.org/cookbook/testing.html) is relevant because the most important route and warning guarantees are already exercised through frontend service tests.
- [CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/pdf/CUDA_C_Best_Practices_Guide.pdf) is relevant because route restrictions should remain grounded in actual supported execution constraints rather than optimistic UI claims.
- [Playwright Best Practices](https://playwright.dev/docs/best-practices) is relevant because user-visible route warnings and summaries should remain observable through higher-level validation flows.

## Proposed Improvement Approach

1. Freeze `run_composite_validation` as a family command with explicit allowed strategy kinds and route constraints.
2. Document the current composite route matrix, especially:
   - plain `CompositeCpu` GPU restrictions
   - `CompositeMo` CPU validation route
   - family-specific identity preserved through `validation_context`
3. Keep frontend preview and backend result semantics aligned around the same route/warning vocabulary.
4. Treat omitted modules and reduced cross-rerun capabilities as part of the composite command contract, not just secondary copy.
5. Add or tighten tests for representative composite-family strategy kinds and route restrictions.

## Checklist

- [ ] Document the accepted composite-family strategy kinds for this command.
- [ ] Document current route restrictions and warnings for plain `CompositeCpu`.
- [ ] Document why `CompositeMo` currently uses the CPU validation route.
- [ ] Document how route summaries and warning summaries should map to backend behavior.
- [ ] Add or tighten tests for strategy-kind routing and mismatch rejection.
- [ ] Add or tighten tests for `CompositeMo` CPU-route messaging and `CompositeCpu` GPU restriction messaging.
- [ ] Add or tighten tests for representative family variants such as `Rocb`, `Mkcs`, and `Carm`.

## Verification Plan

- run targeted `commands_validation.rs` tests
- run `npm --prefix web test -- src/services/validationRequestBuilder.test.ts`
- run `npm --prefix web test -- src/services/validationService.test.ts`
- manually verify route summaries for one plain `CompositeCpu` case, one `CompositeMo` case, and one alternate composite-family strategy

## Rollout / Fallback / Compatibility Notes

- preserve current family routing and warning semantics unless a coordinated backend capability change is made
- keep `validation_context.strategy_kind` explicit for shared-command families
- prefer additive clarification over broad route-policy changes
- if a composite route opens or closes, update builder previews, service summaries, and tests together

## Related Docs

- `REPO_Improvement/03_opt/11_opt_request_builder_2026-04-08.md`
- `REPO_Improvement/05_validation/19_validation_request_builder_2026-04-08.md`
- `REPO_Improvement/05_validation/20_validation_service_2026-04-08.md`
- `REPO_Improvement/05_validation/09_eng_validation_verdicts_2026-04-09.md`
