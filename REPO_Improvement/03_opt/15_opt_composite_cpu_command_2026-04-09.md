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
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Tauri Calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because this command is a typed frontend-to-backend contract.
- [Tauri State Management](https://v2.tauri.app/develop/state-management/) is relevant because run coordination relies on shared backend state.
- [Calling the Frontend from Rust](https://v2.tauri.app/develop/calling-frontend/) is relevant because progress events travel back through Tauri’s event system.
- [Pinia Core Concepts](https://pinia.vuejs.org/core-concepts/) is relevant because the frontend family-routing decision should stay derived from canonical state and services.
- [NVIDIA CUDA C Best Practices Guide](https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/)
  - relevant because GPU and route-policy improvements should preserve correctness, observability, and profiling discipline instead of chasing speed blindly

## Proposed Improvement Approach

1. Freeze the Composite CPU command as a distinct route family with explicit strategy membership.
2. Document the current `Grid`-only product normalization rule at the command boundary.
3. Keep the shared run guard, cancel, and progress sequence aligned with the Double-MA command while documenting the route differences.
4. Add parity tests between strategy-family routing and command selection if coverage is weak.
5. Make UI capability notes reflect the command’s actual product contract rather than backend hypotheticals.
6. Treat optimizer-surface code quality as part of the implementation scope: keep request shaping, route policy, GPU ownership, diagnostics, and strategy-local boundaries readable and testable in a public repo.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document which strategies resolve to `run_composite_cpu_optimization`.
- [x] Document the Composite CPU `Grid`-only product normalization rule.
- [x] Document the run guard, cancel token, and progress sink behavior.
- [x] Add or tighten routing-parity tests for Composite CPU families.
- [x] Document the relationship between frontend capability messaging and command truth.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `cargo test -p ta_desktop_demo_app commands_optimize`
- run `npm --prefix web run test -- src/services/optimizeService.test.ts`
- run `npm --prefix web run test -- src/services/optimizeRequestBuilder.test.ts`
- run `npm --prefix web run test -- src/tauri/instrumentation.test.ts`
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve the current product `Grid` posture unless a broader mode contract is deliberately shipped
- keep Composite CPU family routing explicit instead of inferred
- treat the command boundary as the authoritative product contract if UI copy drifts
- update staged-family routing and this plan together when new families are surfaced
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Revalidated the Composite CPU command as a distinct product route rather than a backend detail. `commands_optimize.rs` already pins Composite CPU product mode to `Grid`, and the internal command tests cover that normalization directly.
- Frontend command routing is already frozen by tests. `optimizeService.test.ts` and `optimizeRequestBuilder.test.ts` continue to prove Composite CPU request construction and command selection for `Asr`, `Vcc`, `Vemr`, `Rocb`, `Vcet`, the staged families, and `CompositeCpu` itself.
- No code change was required in this tranche because the targeted audit did not reveal a command-boundary or routing mismatch; the existing implementation already matches the documented contract.
- Acceptance evidence captured during execution: `cargo test -p ta_desktop_demo_app commands_optimize --lib`, `npm.cmd --prefix web run test -- src/services/optimizeService.test.ts src/services/optimizeRequestBuilder.test.ts src/components/OptimizeWorkspacePanel.test.ts src/services/runControlService.test.ts src/tauri/instrumentation.test.ts src/stores/taskStore.lifecycle.test.ts src/utils/statusPresentation.test.ts`, `npm.cmd --prefix web run lint`, and `npm.cmd --prefix web run typecheck`.

## Related Docs

- `REPO_Improvement/03_opt/02_opt_strategy_backend_2026-04-09.md`
- `REPO_Improvement/03_opt/17_opt_cancel_progress_2026-04-09.md`
- `REPO_Improvement/03_opt/18_eng_dm_request_resolution_2026-04-09.md`
- `REPO_Improvement/03_opt/08_strategy_family_params_2026-04-09.md`
