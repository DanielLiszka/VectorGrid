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
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Tauri Calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because this command is a primary typed boundary for a strategy-owned route.
- [Tauri State Management](https://v2.tauri.app/develop/state-management/) is relevant because run coordination still relies on the same backend state model as other commands.
- [Calling the Frontend from Rust](https://v2.tauri.app/develop/calling-frontend/) is relevant because progress still returns to the frontend through events.
- [Pinia Core Concepts](https://pinia.vuejs.org/core-concepts/) is relevant because Composite MO’s frontend special casing should remain explicit and centralized.
- [NVIDIA CUDA C Best Practices Guide](https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/)
  - relevant because GPU and route-policy improvements should preserve correctness, observability, and profiling discipline instead of chasing speed blindly

## Proposed Improvement Approach

1. Document Composite MO as a distinct command family with strategy-owned route semantics.
2. Freeze the current differences between Composite MO and the other optimize commands.
3. Keep shared run coordination common while documenting the product-contract differences.
4. Add or tighten service-routing tests for `CompositeMo`.
5. Make UI capability notes reflect that Composite MO does not simply inherit all global optimize controls.
6. Treat optimizer-surface code quality as part of the implementation scope: keep request shaping, route policy, GPU ownership, diagnostics, and strategy-local boundaries readable and testable in a public repo.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document `CompositeMo` command routing and special-case ownership.
- [ ] Document which global optimize controls do not directly govern this command.
- [ ] Document the shared run guard, cancel token, and progress sink behavior.
- [ ] Add or tighten command-routing tests for Composite MO.
- [ ] Document the separation between strategy-owned and product-owned route semantics.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `cargo test -p ta_desktop_demo_app commands_optimize`
- run `npm --prefix web run test -- src/services/optimizeService.test.ts`
- run `npm --prefix web run test -- src/services/optimizeRequestBuilder.test.ts`
- run `npm --prefix web run test -- src/components/OptimizeWorkspacePanel.test.ts`
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve current special-case routing for Composite MO unless the product deliberately broadens or narrows it
- prefer clearer documentation before unifying any code paths
- keep strategy-owned route behavior explicit in UI and service layers
- update this plan alongside any Composite MO control-surface changes
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/03_opt/01_opt_panel_core_2026-04-09.md`
- `REPO_Improvement/03_opt/02_opt_strategy_backend_2026-04-09.md`
- `REPO_Improvement/03_opt/17_opt_cancel_progress_2026-04-09.md`
- `REPO_Improvement/03_opt/18_eng_dm_request_resolution_2026-04-09.md`
