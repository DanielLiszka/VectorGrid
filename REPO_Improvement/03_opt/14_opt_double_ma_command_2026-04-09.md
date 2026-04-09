# Optimization Double MA Command Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to the `run_double_ma_optimization` command boundary so product-mode normalization, run guards, progress emission, and backend invocation remain explicit and regression-resistant.

## Scope Boundaries

In scope:

- `src-tauri/src/commands_optimize.rs` for `run_double_ma_optimization`
- product-mode normalization for Double MA requests
- progress sink emission and cancel-token wiring
- frontend command selection and expectations for the Double MA family

Out of scope:

- backend optimization kernel internals
- validation command behavior
- options-related command surfaces

## Current User-Facing Behavior

The Double MA family is the default optimization command path for `DoubleMa`, `DynamicDoubleMa`, PIT, SET, MTS, ORC, MARRT, VFCT, and the current composite-profile family members that reuse the Double-MA engine. The frontend ultimately invokes `run_double_ma_optimization` when a selected strategy resolves into that family.

On the Tauri side, the command normalizes product mode before invoking the backend. If grid-shaped outputs are requested through `include_all`, `heatmap_bins`, or `export_all_csv_path`, the command pins the effective mode to `Grid` even if the incoming request said `Auto`, `CoarseToFine`, or `GpuVdps`. Otherwise it preserves the requested mode.

The command also enforces run-level coordination. It acquires the app run guard, captures the app cancel token, fetches candles from `AppState`, and uses a Tauri-backed progress sink to emit `double_ma_progress` events while the blocking backend call runs off the async executor through `spawn_blocking`.

## Current Code Surface

- `src-tauri/src/commands_optimize.rs`
- `web/src/services/optimizeService.ts`
- `web/src/services/optimizeRequestBuilder.ts`
- `src-tauri/src/state.rs`
- `crates/app_core/src/progress.rs`
- `crates/app_core/src/double_ma.rs`

## Current Verification Surface

- `src-tauri/src/commands_optimize.rs`
- `web/src/services/optimizeService.test.ts`
- `web/src/services/optimizeRequestBuilder.test.ts`
- `web/src/tauri/instrumentation.test.ts`

## Strengths Already Present

- product-mode normalization is already explicit and tested
- the command already uses a run guard rather than allowing overlapping runs
- progress emission already uses a shared backend progress trait
- the command boundary already keeps blocking backend work off the async runtime
- frontend command resolution already points the right strategy families at this command

## Weaknesses / Failure Modes

- product-mode normalization is easy to miss because it lives at the command boundary rather than the request builder alone
- the command contract spans frontend routing, backend invocation, and event instrumentation, which creates drift risk
- grid-output pinning can become confusing if UI messaging implies a looser mode contract than the command actually enforces
- there is no dedicated implementation doc freezing which Double-MA-family strategies use this command and why
- progress semantics are shared across several command paths but not yet documented per command

## Regressions To Avoid

- removing or weakening the grid-output normalization rule
- allowing overlapping runs by bypassing the app run guard
- breaking `double_ma_progress` event emission
- moving blocking backend work onto the async path
- misrouting Double-MA-family strategies away from this command

## External Research Summary

- [Tauri Calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because this command is a primary frontend-to-backend contract boundary.
- [Tauri State Management](https://v2.tauri.app/develop/state-management/) is relevant because run guards and cancel tokens are long-lived backend state, not frontend conveniences.
- [Calling the Frontend from Rust](https://v2.tauri.app/develop/calling-frontend/) is relevant because progress is emitted back to the frontend through the Tauri event system.
- [NVIDIA CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/pdf/CUDA_C_Best_Practices_Guide.pdf) is relevant because GPU-oriented command claims should stay tied to explicit execution and verification rules rather than vague “GPU” wording.

## Proposed Improvement Approach

1. Freeze the Double MA command boundary as the authoritative product-mode normalization point for grid-shaped outputs.
2. Document strategy-family membership for this command explicitly.
3. Document the run guard, cancel token, candle fetch, and progress sink sequence as part of the command contract.
4. Add parity checks between optimize-service command selection and the command’s documented scope.
5. Keep frontend UX notes conservative when command-level normalization can override a requested mode.

## Checklist

- [ ] Document which strategies resolve to `run_double_ma_optimization`.
- [ ] Document grid-output mode normalization and its trigger conditions.
- [ ] Document the run guard and cancel token sequence.
- [ ] Document progress-event emission and the `double_ma_progress` channel.
- [ ] Add or tighten command-selection and normalization tests if coverage is thin.

## Verification Plan

- run `cargo test -p ta_desktop_demo_app commands_optimize`
- run `npm --prefix web test -- src/services/optimizeService.test.ts`
- run `npm --prefix web test -- src/services/optimizeRequestBuilder.test.ts`
- run `npm --prefix web test -- src/tauri/instrumentation.test.ts`

## Rollout / Fallback / Compatibility Notes

- preserve current mode-normalization behavior until all UI messaging is aligned
- prefer additive diagnostics over silent changes to command routing
- keep the event name `double_ma_progress` stable until all consumers migrate together
- if strategy-family membership changes later, update service routing and this plan together

## Related Docs

- `REPO_Improvement/03_opt/02_opt_strategy_backend_2026-04-09.md`
- `REPO_Improvement/03_opt/17_opt_cancel_progress_2026-04-09.md`
- `REPO_Improvement/03_opt/18_eng_dm_request_resolution_2026-04-09.md`
- `REPO_Improvement/03_opt/05_opt_route_gpu_capability_2026-04-09.md`
