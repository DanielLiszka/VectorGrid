# Double Crossover M17/M22 Valid Implementation Plan

Last updated: 2026-03-28

## Purpose

This document converts the still-valid parts of
`CUDA kernel review/double_crossover_performance_review_final.md`
into a repo-grounded implementation plan for improving the 17-row and 22-row
full-metric Double MA CUDA path.

This plan is based on the current code, not on speculative cubin analysis.

Primary code surfaces:

- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/double_ma_strategy.rs`
- `crates/app_core/src/double_ma.rs`

## Current Repo State That Matters

- The current kernel row contracts are:
  - base reduced rows: `7`
  - product reduced rows: `10`
  - full rows: `17`
  - trade rows: `22`
- The current optimize path often prefers the 10-row reduced contract for the
  Sharpe/Volatility product family.
- The current code does not have true fixed-contract M17 or M22 backtest
  wrappers. It does have M7 wrappers and lockstep M10 regime-off no-risk
  wrappers.
- The current dispatch matrix still lacks a generic combined
  `lockstep_time + fast_warp_dedup` route.
- The current `windowed` path still routes only to the generic baseline wrapper.
- Compact valid-pair output is already the default, and the host-side pair map
  already preserves dense/pf-major ordering.
- The current GPU host path always enables `STRAT_SIGNED_EXPOSURE`.
- The current strategy validation rejects `trade_on_next_bar=false`.
- The current host path already validates effective commission range and already
  converts reduced-row max-drawdown thresholds into search/log space before
  launching the device constraint kernels.
- `regime_range_exit_rel` is still threaded through the kernel ABI but is not
  used in the regime helper logic.
- The current backtest pipeline still materializes metrics rows first and then
  rereads them in later score/filter kernels instead of emitting direct side
  outputs such as `score_out` or `feasible_out`.

## What Is In Scope

- Faster M17 full-row execution.
- Faster M22 trade-row execution.
- Route coverage needed to make those fast paths reachable.
- Specialization work that matches the current product contract, not stale
  contracts that the current app never uses.
- Correctness cleanup that is still relevant to the raw kernel family or to any
  new route work.

## What Is Not In Scope

- Re-implementing compact valid-pair output. That is already done.
- Adding specializations for `trade_on_next_bar=false`. The app forbids that
  request contract today.
- Adding specializations for `STRAT_SIGNED_EXPOSURE=false` in the current GPU
  product path. The host always enables signed exposure there.
- Treating cubin-estimated register counts or occupancy guesses as acceptance
  criteria without fresh profiling.
- Spending time first on broad CUDA tuning experiments such as
  `__launch_bounds__(128,2)`, `__maxnreg__`, `__grid_constant__`, packed
  `float2` returns, or cache-policy A/Bs before the missing route/specialization
  work lands.

## Implementation Priorities

### Priority 0

- Add true M17 and M22 fixed-contract backtest wrappers.
- Make host dispatch select them for the current common full-row regime-off
  no-risk cases.
- Close the missing generic `lockstep + fast_warp_dedup` dispatch gap.

### Priority 1

- Add the zero-commission and zero-epsilon variants for the M17/M22 wrappers.
- Hoist host-fixed flags into compile-time contracts for the new full-row
  wrappers, using the current real app contract.
- Add lockstep warp-uniform load support only if profiling still shows the
  lockstep full-row path is hot after Priority 0 lands.

### Priority 2

- Decide whether `windowed` needs matching optimized coverage based on actual
  benchmark usage.
- Consider direct `score_out` / `feasible_out` side outputs for the
  backtest-to-ranking pipeline.
- Clean up latent raw-kernel correctness debt that becomes relevant if the
  kernel family is widened beyond current host-launched contracts.

## Phase Plan

## Phase 0: Baseline And Guardrails

Goal: make the 17-row and 22-row work measurable and keep the plan tied to the
current route contract.

Deliverables:

- Confirm the current full-row trigger points in `gpu_backtest_kernel.rs`.
- Add or refresh targeted tests for:
  - metrics-count route selection for 17 and 22 rows
  - symbol loading for new wrapper names
  - dispatch selection for lockstep vs non-lockstep full-row paths
- Add or reuse benchmark coverage that isolates:
  - M17 baseline
  - M17 lockstep
  - M22 baseline
  - M22 lockstep
  - `windowed` full-row cases if that path is used in practice

Notes:

- Keep benchmark runs serialized.
- Treat existing reduced 10-row product policy as a separate optimization track.

## Phase 1: Add True M17/M22 Wrappers

Goal: land the missing fixed-contract full-row kernels.

Initial wrapper set:

- non-lockstep M17 regime-off no-risk
- non-lockstep M22 regime-off no-risk
- lockstep M17 regime-off no-risk
- lockstep M22 regime-off no-risk

Second wrapper wave:

- `nocomm`
- `eps0`
- `eps0_nocomm`

Implementation notes:

- Follow the existing M10 wrapper pattern first for low-risk landing.
- Ignore runtime `M` and `metrics_out_stride` in the fixed wrappers and hardwire
  the contract to `17/17` or `22/22`.
- If the initial wrappers show real wins, refactor the body toward a fixed-`M`
  templated path instead of relying only on constant propagation from wrappers.

Primary files:

- `crates/app_core/src/kernels/double_crossover.cu`

## Phase 2: Extend Host Loading And Dispatch

Goal: make the new wrappers reachable from the existing host runtime.

Required host changes:

- Load the new M17/M22 symbols beside the current M7/M10 symbol set.
- Extend the dispatch decision tree so the new wrappers are chosen when:
  - `metrics_count == 17` or `metrics_count == 22`
  - regime is off
  - risk is off
  - `lockstep_time` is selected or not selected
  - `eps_zero_mode` and `zero_commission_mode` match the available wrappers
- Preserve the current M7 and M10 behavior for reduced-row routes.

Primary files:

- `crates/app_core/src/gpu_backtest_kernel.rs`

Acceptance condition:

- Full-row requests reach the new wrappers without regressing reduced-row
  product dispatch.

## Phase 3: Fill The Remaining Generic Route Gaps

Goal: close the still-missing route coverage that blocks full-row performance.

Required work:

- Add a generic combined `lockstep_time + fast_warp_dedup` wrapper/body route.
- Add host dispatch support for that combined route.
- Evaluate whether `windowed` needs:
  - only the combined generic route, or
  - matching M17/M22 specializations

Decision rule:

- Do not widen the `windowed` specialization matrix unless benchmark evidence
  shows the path is materially used.

Primary files:

- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/gpu_backtest_kernel.rs`

## Phase 4: Promote Current Host-Fixed Flags To Compile-Time Contracts

Goal: remove flag logic that is effectively fixed in the current shipped path.

Use the real current contract:

- `TRADE_ON_NEXT_BAR=true`
- `STRAT_SIGNED_EXPOSURE=true`

Optional follow-up only if benchmark skew justifies it:

- `LONG_ONLY`
- `NO_FLIP`

Important constraint:

- Do not spend time on `trade_on_next_bar=false` or
  `signed_exposure=false` specializations for the app path.

Primary files:

- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/double_ma_strategy.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`

## Phase 5: Hot-Loop Cleanup That Matches The Current Route

Goal: remove remaining per-iteration overhead only after the missing fast paths
 exist.

Candidate work:

- lockstep warp-uniform broadcast helper for `lr`, `ret1`, and preloaded regime
  values
- pointer-walking or reduced index arithmetic in the full-row loop

Gate:

- Only keep these changes if fresh profiling shows wins after Phases 1 through 4.

Primary files:

- `crates/app_core/src/kernels/double_crossover.cu`

## Phase 6: Backtest-To-Ranking Pipeline Reduction

Goal: avoid rereading full metric rows when ranking/filtering follows backtest
immediately.

Candidate work:

- optional `score_out`
- optional `feasible_out`
- route the global top-k / shortlist kernels to use those side outputs when they
  are available

Recommended landing order:

- exact/global-top-k path first
- broader reuse only after parity and perf are clear

Primary files:

- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/gpu_backtest_kernel.rs`

## Phase 7: Correctness Cleanup That Remains Relevant

Goal: clean up the still-valid correctness debt without confusing it with issues
the current host path already neutralizes.

Keep in scope:

- either implement real `regime_range_exit_rel` hysteresis semantics or remove
  the dead parameter from the raw kernel ABI
- centralize the row log-space rule in device code if new row contracts are
  added or exposed

Lower-priority latent debt:

- the calmar `M < 15` vs writer `M < 17` mismatch
- raw kernel-side `max_dd` threshold semantics if used without host conversion
- raw kernel-side `metrics_out_stride` underflow risk if external callers are
  ever allowed to provide arbitrary strides

Important caveat:

- These latent issues are real kernel-family debt, but they are not all active
  app-path bugs today.

## File Touchpoints

- `crates/app_core/src/kernels/double_crossover.cu`
  - new wrappers
  - new combined route
  - optional flag-specialized bodies
  - optional warp-uniform loads
  - optional side outputs
  - regime exit cleanup
- `crates/app_core/src/gpu_backtest_kernel.rs`
  - symbol loading
  - dispatch conditions
  - route metadata
  - optional side-output consumers
  - tests for metrics-count and dispatch behavior
- `crates/app_core/src/double_ma_strategy.rs`
  - preserve request-contract assumptions used for specialization
- `crates/app_core/benches/double_ma_opt.rs`
  - add or extend full-row benchmark coverage as needed

## Checklist

- [ ] Phase 0.A: Confirm the current M17 and M22 trigger conditions in the host runtime.
- [ ] Phase 0.B: Add or refresh tests for full-row metrics-count route selection.
- [ ] Phase 0.C: Add or refresh tests for full-row symbol-loading failures.
- [ ] Phase 0.D: Add or refresh benchmarks that isolate M17 and M22 baseline and lockstep runs.

- [ ] Phase 1.A: Add non-lockstep M17 regime-off no-risk wrapper.
- [ ] Phase 1.B: Add non-lockstep M22 regime-off no-risk wrapper.
- [ ] Phase 1.C: Add lockstep M17 regime-off no-risk wrapper.
- [ ] Phase 1.D: Add lockstep M22 regime-off no-risk wrapper.
- [ ] Phase 1.E: Add `nocomm` variants for M17 and M22.
- [ ] Phase 1.F: Add `eps0` variants for M17 and M22.
- [ ] Phase 1.G: Add `eps0_nocomm` variants for M17 and M22.

- [ ] Phase 2.A: Load the new M17 wrapper symbols in the host runtime.
- [ ] Phase 2.B: Load the new M22 wrapper symbols in the host runtime.
- [ ] Phase 2.C: Route non-lockstep full-row regime-off no-risk requests to the new wrappers.
- [ ] Phase 2.D: Route lockstep full-row regime-off no-risk requests to the new wrappers.
- [ ] Phase 2.E: Keep reduced-row M7/M10 dispatch behavior unchanged.

- [ ] Phase 3.A: Add a generic `lockstep_time + fast_warp_dedup` backtest route.
- [ ] Phase 3.B: Add host dispatch support for the combined generic route.
- [ ] Phase 3.C: Benchmark whether `windowed` needs matching optimized coverage.
- [ ] Phase 3.D: Add `windowed` full-row optimized coverage only if the benchmark says it is worth it.

- [ ] Phase 4.A: Convert `TRADE_ON_NEXT_BAR=true` into a compile-time full-row contract where profitable.
- [ ] Phase 4.B: Convert `STRAT_SIGNED_EXPOSURE=true` into a compile-time full-row contract where profitable.
- [ ] Phase 4.C: Benchmark whether `LONG_ONLY` or `NO_FLIP` deserve their own full-row specializations.

- [ ] Phase 5.A: Add lockstep warp-uniform loads for `lr`.
- [ ] Phase 5.B: Add lockstep warp-uniform loads for `ret1`.
- [ ] Phase 5.C: Add lockstep warp-uniform regime-value broadcast if the helper signature is updated.
- [ ] Phase 5.D: Keep or reject pointer-walking based on measured results, not style preference.

- [ ] Phase 6.A: Prototype optional `score_out`.
- [ ] Phase 6.B: Prototype optional `feasible_out`.
- [ ] Phase 6.C: Wire one exact/global-top-k route to consume the side outputs.
- [ ] Phase 6.D: Keep the side-output path only if parity and perf both hold.

- [ ] Phase 7.A: Decide whether `regime_range_exit_rel` will be implemented or removed.
- [ ] Phase 7.B: If new row contracts are exposed, centralize the device-side log-space row rule.
- [ ] Phase 7.C: If raw kernel contracts are widened, fix the latent calmar reduced-row mismatch.
- [ ] Phase 7.D: If raw external stride contracts are widened, add explicit host/kernel guardrails for stride validity.

- [ ] Verification.A: Run `cargo test -p ta_app_core`.
- [ ] Verification.B: Run targeted CUDA tests for full-row dispatch and parity.
- [ ] Verification.C: Run `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate` after performance-sensitive kernel changes.
- [ ] Verification.D: Use Nsight Compute or Nsight Systems only after ensuring no other benchmark or profiling process is active.

## Acceptance Criteria

- M17 and M22 full-row runs have dedicated reachable wrappers in the current host
  runtime.
- The host dispatch matrix no longer forces full-row requests through only the
  generic baseline, generic lockstep, or generic fast-dedup routes.
- Reduced 10-row product behavior does not regress while landing the full-row
  work.
- Any kept hot-loop cleanup or side-output path is justified by fresh benchmark
  or profiler evidence.
- The plan does not reopen work the repo has already completed, such as compact
  valid-pair output defaults or commission validation.

## Verification Surfaces

- Unit and integration tests in `ta_app_core`
- CUDA dispatch and parity tests in `crates/app_core/src/gpu_backtest_kernel.rs`
- Benchmark surface:
  - `crates/app_core/benches/double_ma_opt.rs`
- Gate:
  - `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`
- Profiling:
  - `powershell -File scripts/profile_gpu_backtest_ncu.ps1`
  - `powershell -File scripts/profile_gpu_backtest_nsys.ps1`

## Notes For The Implementer

- Treat the missing M17/M22 wrappers and missing route coverage as the first real
  implementation work. Do not start with generic CUDA tuning A/Bs.
- Preserve the current dense/pf-major valid-pair ordering. That work is already
  correct and helps the existing dedup logic.
- Keep reduced-row correctness debt separated from active app-path bugs when
  writing commit messages and validation notes.
