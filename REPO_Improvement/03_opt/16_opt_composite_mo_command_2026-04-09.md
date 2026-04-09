# Optimization Composite MO Command Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to the `run_composite_mo_optimization` command boundary so Composite MO remains clearly separated from the other optimization families and its product contract stays understandable.

## Scope Boundaries

In scope:

- `src-tauri/src/commands_optimize.rs` for `run_composite_mo_optimization`
- frontend command resolution for `CompositeMo`
- run guard, cancel token, and progress sink wiring
- separation of Composite MO from global backend and global mode expectations

Out of scope:

- Composite MO backend search internals
- Composite CPU or Double-MA command details
- options-related command surfaces

## Current User-Facing Behavior

`CompositeMo` already uses its own optimization command path. The frontend resolves that strategy directly to `run_composite_mo_optimization`, and the optimize panel already hides some global controls because the strategy owns more of its own route behavior.

At the Tauri boundary, the command acquires the app run guard, reuses the shared cancel token and candle-loading path, and runs the Composite MO backend through `spawn_blocking` with the same Tauri progress sink infrastructure used elsewhere. Unlike the product-facing Double-MA and Composite CPU commands, there is no separate product-mode normalization step in this command.

The result is a clear special-case family in code, but the repo still needs a dedicated plan documenting why this command is distinct and which assumptions from other optimization families do not apply.

## Current Code Surface

- `src-tauri/src/commands_optimize.rs`
- `web/src/services/optimizeService.ts`
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/components/OptimizeWorkspacePanel.vue`
- `src-tauri/src/state.rs`
- `crates/app_core/src/composite_mo.rs`

## Current Verification Surface

- `web/src/services/optimizeService.test.ts`
- `web/src/services/optimizeRequestBuilder.test.ts`
- `src-tauri/src/commands_optimize.rs`
- `web/src/components/OptimizeWorkspacePanel.test.ts`

## Strengths Already Present

- Composite MO already has a distinct command identity
- frontend command routing already treats `CompositeMo` specially
- the optimize panel already reflects that Composite MO owns more of its route behavior
- the command still reuses the shared run-guard and progress infrastructure
- blocking backend work is already isolated from the async path

## Weaknesses / Failure Modes

- contributors can mistakenly project Double-MA assumptions onto Composite MO
- there is no dedicated command-level doc freezing which global controls do and do not apply
- the special-case routing is easy to understand in code but under-documented in the improvement program
- result and progress semantics can drift if Composite MO is treated as “just another optimize command”
- frontend and backend could diverge about how much of Composite MO is product-managed versus strategy-owned

## Regressions To Avoid

- routing `CompositeMo` to the wrong command
- reintroducing global mode or backend assumptions into Composite MO without deliberate product changes
- bypassing the shared run guard or cancel token
- breaking progress-event behavior for Composite MO runs
- hiding the distinction between Composite MO and the other optimization families

## External Research Summary

- [Tauri Calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because this command is a primary typed boundary for a strategy-owned route.
- [Tauri State Management](https://v2.tauri.app/develop/state-management/) is relevant because run coordination still relies on the same backend state model as other commands.
- [Calling the Frontend from Rust](https://v2.tauri.app/develop/calling-frontend/) is relevant because progress still returns to the frontend through events.
- [Pinia Core Concepts](https://pinia.vuejs.org/core-concepts/) is relevant because Composite MO’s frontend special casing should remain explicit and centralized.

## Proposed Improvement Approach

1. Document Composite MO as a distinct command family with strategy-owned route semantics.
2. Freeze the current differences between Composite MO and the other optimize commands.
3. Keep shared run coordination common while documenting the product-contract differences.
4. Add or tighten service-routing tests for `CompositeMo`.
5. Make UI capability notes reflect that Composite MO does not simply inherit all global optimize controls.

## Checklist

- [ ] Document `CompositeMo` command routing and special-case ownership.
- [ ] Document which global optimize controls do not directly govern this command.
- [ ] Document the shared run guard, cancel token, and progress sink behavior.
- [ ] Add or tighten command-routing tests for Composite MO.
- [ ] Document the separation between strategy-owned and product-owned route semantics.

## Verification Plan

- run `cargo test -p ta_desktop_demo_app commands_optimize`
- run `npm --prefix web test -- src/services/optimizeService.test.ts`
- run `npm --prefix web test -- src/services/optimizeRequestBuilder.test.ts`
- run `npm --prefix web test -- src/components/OptimizeWorkspacePanel.test.ts`

## Rollout / Fallback / Compatibility Notes

- preserve current special-case routing for Composite MO unless the product deliberately broadens or narrows it
- prefer clearer documentation before unifying any code paths
- keep strategy-owned route behavior explicit in UI and service layers
- update this plan alongside any Composite MO control-surface changes

## Related Docs

- `REPO_Improvement/03_opt/01_opt_panel_core_2026-04-09.md`
- `REPO_Improvement/03_opt/02_opt_strategy_backend_2026-04-09.md`
- `REPO_Improvement/03_opt/17_opt_cancel_progress_2026-04-09.md`
- `REPO_Improvement/03_opt/18_eng_dm_request_resolution_2026-04-09.md`
