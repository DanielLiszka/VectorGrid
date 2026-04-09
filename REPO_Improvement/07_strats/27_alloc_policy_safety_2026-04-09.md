## Purpose

Define the improvement plan for the shared allocation-safety policy that uses uninitialized buffers, warmup-prefix initialization, and debug poisoning in performance-critical non-options paths.

## Scope Boundaries

In scope:

- `AGENTS.md`
- `crates/app_core/src/alloc_policy.rs`
- high-impact consumers such as:
  - `crates/app_core/src/double_ma.rs`
  - `crates/app_core/src/gpu_backtest_kernel.rs`
  - `crates/app_core/src/standalone_gpu_runtime.rs`
  - `crates/app_core/src/ml_signal.rs`
  - `crates/app_core/src/composite_mo.rs`

Out of scope:

- rewriting all hot-path allocation strategies
- unrelated memory optimizations not tied to the current allocation helpers
- options-related compute paths

## Current User-Facing Behavior

Users do not see this layer directly, but it affects two high-value product properties:

- performance of hot optimization/validation/runtime paths
- correctness safety when partially initialized buffers are used for speed

Bad changes here often appear as subtle data corruption, nondeterministic bugs, or unexplained performance cliffs rather than obvious crashes.

## Current Code Surface

- `AGENTS.md`
- `crates/app_core/src/alloc_policy.rs`
- representative consumer surfaces:
  - `crates/app_core/src/double_ma.rs`
  - `crates/app_core/src/gpu_backtest_kernel.rs`
  - `crates/app_core/src/standalone_gpu_runtime.rs`
  - `crates/app_core/src/ml_signal.rs`
  - `crates/app_core/src/composite_mo.rs`

Key current policy:

- use `Vec::with_capacity` plus `set_len`/`MaybeUninit` only when every element is guaranteed to be written before read
- initialize only observable warmup prefixes when appropriate
- keep full-fill allocation when the fill value is semantically observable
- use debug poison values in debug/test builds to catch accidental reads

## Current Verification Surface

- direct tests in `crates/app_core/src/alloc_policy.rs`
- consumer tests in `double_ma.rs`
- consumer tests in `gpu_backtest_kernel.rs`
- repo perf gates:
  - `scripts/run_cpu_perf_gate.ps1`
  - `scripts/run_gpu_perf_gate.ps1`

## Strengths Already Present

- the policy is already written in `AGENTS.md`, which gives contributors a clear intent
- `alloc_policy.rs` centralizes the common helpers rather than forcing each caller to write bespoke unsafe code
- debug poisoning is already in place, which is a strong safety net for development/test builds
- the helpers are narrow and performance-focused
- hot consumers already use the shared helpers instead of hiding unsafe allocation tricks inline everywhere

## Weaknesses / Failure Modes

- the safety policy is still enforced mostly by discipline, not by an explicit call-site inventory
- the most performance-sensitive consumers are also the easiest places to accidentally introduce a read-before-write bug
- helper semantics are clear locally, but downstream invariants are not always written beside the call sites
- a “minor cleanup” can easily widen the window where partially initialized memory becomes observable
- perf regressions can hide behind correctness-preserving changes if allocation strategy changes are not measured

## Regressions To Avoid

- any read-before-write on uninitialized tails or matrix cells
- removing debug poison behavior without replacing the detection value
- changing warmup-prefix semantics so observable prefixes become garbage
- replacing uninitialized helpers with full-fill behavior in hot loops without explicit perf review
- scattering new unsafe allocation logic outside the shared helper layer

## External Research Summary

- [Rust `MaybeUninit`](https://doc.rust-lang.org/std/mem/union.MaybeUninit.html) is relevant because the safety contract in this repo depends on using partially initialized storage correctly.
- [Rustonomicon: Uninitialized Memory](https://doc.rust-lang.org/nomicon/uninitialized.html) is relevant because it explains why accidental reads of uninitialized memory are undefined behavior even when they appear to “work.”
- [`Vec::from_raw_parts` docs](https://doc.rust-lang.org/std/vec/struct.Vec.html#method.from_raw_parts) are relevant because the helper layer rebuilds initialized vectors from manually managed storage and must maintain the required invariants.
- [CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/archive/12.5.1/cuda-c-best-practices-guide/index.html) is relevant because some consumers sit on GPU-adjacent hot paths where allocation decisions are performance-critical.
- [Criterion.rs getting started](https://bheisler.github.io/criterion.rs/book/getting_started.html) is relevant because allocation-policy changes should be measured, not justified from intuition.

## Proposed Improvement Approach

1. Treat `alloc_policy.rs` and the AGENTS policy as a shared unsafe-code contract.
2. Build a repo-local inventory of every current call site that relies on:
   - uninitialized scalar buffers
   - prefix-initialized tails
   - uninitialized matrix storage
3. At each call site, add or confirm a short invariant comment describing why full overwrite before read is guaranteed.
4. Add targeted tests for the helpers themselves and regression tests for a few representative consumers.
5. Keep any future unsafe allocation addition behind the shared helper surface unless there is a compelling reason not to.

## Checklist

- [ ] Build a call-site inventory for `alloc_policy.rs` helpers across app-core.
- [ ] Confirm every call site has a clear overwrite-before-read argument.
- [ ] Add or tighten helper tests for prefix initialization, resize behavior, and debug poison expectations.
- [ ] Add or tighten consumer tests in the hottest paths that depend on these helpers.
- [ ] Decide whether additional debug assertions are warranted at selected consumer boundaries.
- [ ] Route any proposed new unsafe allocation path through the same documented policy.
- [ ] Re-run perf gates for any allocation-strategy change in hot paths.

## Verification Plan

- run direct tests in `alloc_policy.rs`
- run representative consumer tests in `double_ma.rs`
- run representative GPU/runtime tests in `gpu_backtest_kernel.rs`
- run `scripts/run_cpu_perf_gate.ps1 -Mode candidate` for CPU-hot changes
- run `scripts/run_gpu_perf_gate.ps1 -Mode candidate` for GPU-hot changes when applicable

## Rollout / Fallback / Compatibility Notes

- prefer documentation and call-site hardening first, new unsafe tricks last
- if an optimization cannot be justified with a concrete overwrite-before-read argument, reject it
- if a perf optimization weakens auditability substantially, keep the safer clearer version
- preserve CPU-only builds and existing debug/test poison behavior

## Related Docs

- `REPO_Improvement/07_strats/25_strategy_runtime_gpu_stack_2026-04-09.md`
- `REPO_Improvement/03_opt/20_eng_dm_gpu_kernel_vram_2026-04-08.md`
- `REPO_Improvement/03_opt/21_eng_dm_vdps_fidelity_2026-04-08.md`
