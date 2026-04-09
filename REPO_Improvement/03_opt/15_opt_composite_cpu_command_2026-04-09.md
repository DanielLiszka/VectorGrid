# Optimization Composite CPU Command Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to the `run_composite_cpu_optimization` command boundary so Composite CPU family routing, mode normalization, progress reporting, and backend invocation remain explicit and correct.

## Scope Boundaries

In scope:

- `src-tauri/src/commands_optimize.rs` for `run_composite_cpu_optimization`
- strategy-family routing into the Composite CPU command
- command-level mode normalization for Composite CPU product runs
- run guard, cancel token, and progress sink behavior

Out of scope:

- Composite CPU backend algorithm details
- Composite MO command behavior
- options-related command surfaces

## Current User-Facing Behavior

The frontend already routes `Asr`, `Vcc`, `Vemr`, `Rocb`, `Vcet`, `Kqsr`, `Carm`, `Kvdf`, `Mkcs`, `Cdmc`, and `CompositeCpu` into `run_composite_cpu_optimization`. Users reach those routes through the same optimize workflow, but the command path is distinct from the Double-MA and Composite MO families.

At the command boundary, Composite CPU product mode is intentionally normalized to `Grid`. Unlike the Double-MA path, the current product contract does not preserve alternate optimization modes here. The command then acquires the app run guard, uses the shared app cancel token, pulls candles from `AppState`, and invokes the blocking Composite CPU backend through `spawn_blocking` with the Tauri progress sink.

This means the command is already productized as a specialized route rather than a generic backend wrapper, but that contract is not yet documented in its own implementation plan.

## Current Code Surface

- `src-tauri/src/commands_optimize.rs`
- `web/src/services/optimizeService.ts`
- `web/src/services/optimizeRequestBuilder.ts`
- `src-tauri/src/state.rs`
- `crates/app_core/src/composite_cpu.rs`
- `crates/app_core/src/progress.rs`

## Current Verification Surface

- `src-tauri/src/commands_optimize.rs`
- `web/src/services/optimizeService.test.ts`
- `web/src/services/optimizeRequestBuilder.test.ts`
- `web/src/tauri/instrumentation.test.ts`

## Strengths Already Present

- Composite CPU strategies already have a clear product command boundary
- mode normalization is already explicit and tested
- the command already shares the same run-guard and progress infrastructure as other optimization commands
- frontend command selection is already family-aware
- blocking backend work already runs off the async executor

## Weaknesses / Failure Modes

- route truth is split between service routing, capability prose, and the command boundary
- contributors can assume the Composite CPU path supports broader mode selection than the product actually allows
- there is no dedicated document freezing which families belong here
- progress semantics are shared but undocumented at the command-specific level
- family routing can drift if new staged strategies are surfaced without synchronized command updates

## Regressions To Avoid

- breaking the `Grid`-only product normalization for Composite CPU runs
- misrouting Composite CPU family strategies to another command
- bypassing the shared run guard or cancel token
- breaking progress-event emission for Composite CPU runs
- letting UI capability text overpromise unsupported mode behavior

## External Research Summary

- [Tauri Calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because this command is a typed frontend-to-backend contract.
- [Tauri State Management](https://v2.tauri.app/develop/state-management/) is relevant because run coordination relies on shared backend state.
- [Calling the Frontend from Rust](https://v2.tauri.app/develop/calling-frontend/) is relevant because progress events travel back through Tauri’s event system.
- [Pinia Core Concepts](https://pinia.vuejs.org/core-concepts/) is relevant because the frontend family-routing decision should stay derived from canonical state and services.

## Proposed Improvement Approach

1. Freeze the Composite CPU command as a distinct route family with explicit strategy membership.
2. Document the current `Grid`-only product normalization rule at the command boundary.
3. Keep the shared run guard, cancel, and progress sequence aligned with the Double-MA command while documenting the route differences.
4. Add parity tests between strategy-family routing and command selection if coverage is weak.
5. Make UI capability notes reflect the command’s actual product contract rather than backend hypotheticals.

## Checklist

- [ ] Document which strategies resolve to `run_composite_cpu_optimization`.
- [ ] Document the Composite CPU `Grid`-only product normalization rule.
- [ ] Document the run guard, cancel token, and progress sink behavior.
- [ ] Add or tighten routing-parity tests for Composite CPU families.
- [ ] Document the relationship between frontend capability messaging and command truth.

## Verification Plan

- run `cargo test -p ta_desktop_demo_app commands_optimize`
- run `npm --prefix web test -- src/services/optimizeService.test.ts`
- run `npm --prefix web test -- src/services/optimizeRequestBuilder.test.ts`
- run `npm --prefix web test -- src/tauri/instrumentation.test.ts`

## Rollout / Fallback / Compatibility Notes

- preserve the current product `Grid` posture unless a broader mode contract is deliberately shipped
- keep Composite CPU family routing explicit instead of inferred
- treat the command boundary as the authoritative product contract if UI copy drifts
- update staged-family routing and this plan together when new families are surfaced

## Related Docs

- `REPO_Improvement/03_opt/02_opt_strategy_backend_2026-04-09.md`
- `REPO_Improvement/03_opt/17_opt_cancel_progress_2026-04-09.md`
- `REPO_Improvement/03_opt/18_eng_dm_request_resolution_2026-04-09.md`
- `REPO_Improvement/03_opt/08_strategy_family_params_2026-04-09.md`
