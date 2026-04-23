# Strategy Runtime GPU Stack Plan

Last updated: 2026-04-10

## Purpose

Define the improvement plan for the non-options GPU runtime stack that turns compiled strategy plans into executable GPU predicate bundles, caches them, and evaluates them through the current CUDA kernel/runtime pipeline.

## Scope Boundaries

In scope:

- `crates/app_core/src/strategy_gpu_program.rs`
- `crates/app_core/src/strategy_gpu_runtime.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/standalone_gpu_runtime.rs`
- `crates/app_core/src/vram_ma.rs`
- dynamic-runtime probe and route bins that exercise this stack

Out of scope:

- frontend capability presentation
- options-related GPU execution
- inventing a different GPU backend
- changing high-level route policy except where tighter invariants are needed

## Current User-Facing Behavior

Users do not touch this stack directly, but they experience its effects through:

- whether a dynamic strategy is considered GPU-capable
- whether GPU routes actually execute or fall back
- how stable GPU runtime reuse is across repeated runs
- whether route diagnostics and performance probes match real execution
- whether cache reuse reduces repeated overhead without leaking stale plan/device state

This stack is one of the most performance-sensitive and regression-sensitive surfaces in the repo.

## Current Code Surface

- `crates/app_core/src/strategy_gpu_program.rs`
- `crates/app_core/src/strategy_gpu_runtime.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/standalone_gpu_runtime.rs`
- `crates/app_core/src/vram_ma.rs`
- related inputs/consumers:
  - `crates/app_core/src/strategy_plan.rs`
  - `crates/app_core/src/strategy_dynamic.rs`
  - `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs`
  - `src-tauri/src/bin/ta_profile_route_matrix.rs`
  - `src-tauri/src/bin/ta_app_path_faceoff.rs`

Current ownership split:

- `strategy_gpu_program.rs` compiles plans into GPU predicate programs and ABIs with fingerprints
- `strategy_gpu_runtime.rs` builds handles, caches compiled bundles/executables, and tracks cache stats
- `gpu_backtest_kernel.rs` owns the CUDA upload/orchestration, persistent device buffers, shortlist/exact paths, runtime diagnostics, and dynamic-program execution
- `standalone_gpu_runtime.rs` provides lower-level device-buffer/runtime helpers
- `vram_ma.rs` is the thin VRAM-oriented MA surface

## Current Verification Surface

- unit tests in `strategy_gpu_program.rs`
- unit tests in `strategy_gpu_runtime.rs`
- extensive tests inside `gpu_backtest_kernel.rs`
- dynamic runtime probes via `profile_gpu_dynamic_runner.rs`
- route/probe coverage via `ta_profile_route_matrix.rs`
- faceoff/perf tooling via `ta_app_path_faceoff.rs` and `crates/app_core/benches/double_ma_opt.rs`

## Strengths Already Present

- the stack already separates compiled-plan generation from runtime cache ownership
- executable fingerprints and ABI versions are explicit
- runtime cache stats are surfaced rather than hidden
- the kernel runtime already carries detailed diagnostics and feature gating
- persistent VRAM and resident-series ideas already exist instead of being bolted on as one-off optimizations
- the repo already has multiple probe and benchmark entry points for this stack

## Weaknesses / Failure Modes

- `gpu_backtest_kernel.rs` is very large and mixes many concerns, which makes it difficult to audit boundaries
- it is easy for compile-time plan semantics, runtime cache keys, and uploaded device state to drift apart
- a change that preserves unit tests can still invalidate route diagnostics or reuse assumptions
- persistent caches and resident-series logic can create stale-state bugs if invalidation rules are underspecified
- runtime feature flags and diagnostics are powerful, but they also create many combinatorial behavior branches

## Regressions To Avoid

- breaking fingerprint or ABI stability between compile and upload stages
- reusing cached or uploaded executables across incompatible plan/device states
- changing fallback conditions in ways that silently alter user-visible routing
- regressing VRAM residency or persistent-buffer behavior while keeping only correctness tests green
- introducing performance regressions by “cleaning up” the runtime without measurement
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/archive/12.5.1/cuda-c-best-practices-guide/index.html) is relevant because the runtime stack is explicitly balancing upload cost, buffer persistence, kernel-launch policy, and diagnostic visibility.
- [Rust `OnceLock`](https://doc.rust-lang.org/std/sync/struct.OnceLock.html) is relevant because the runtime/cache layer already uses process-global singleton caches and needs clear initialization/invalidation rules.
- [cust crate docs](https://docs.rs/cust/latest/cust/) are relevant because the runtime stack is built on top of Rust CUDA device-buffer, stream, and module abstractions that should be treated as contract-sensitive dependencies.
- [Serde enum representations](https://serde.rs/enum-representations.html) are relevant because executable/cache metadata and route/program contracts are serialized across tooling surfaces.
- [Cargo targets](https://doc.rust-lang.org/cargo/reference/cargo-targets.html) are relevant because probe binaries and runtime verification tools are part of the intended maintenance surface for this stack.
- [cargo test](https://doc.rust-lang.org/cargo/commands/cargo-test.html)
  - relevant because strategy-local behavior is easy to regress silently unless tests stay close to the owning strategy/runtime surface

## Proposed Improvement Approach

1. Freeze the current stack into explicit layers before restructuring anything:
   - plan compilation
   - executable metadata and cache keys
   - upload/runtime cache ownership
   - kernel execution and diagnostics
2. Document cache invalidation rules for:
   - compiled bundle cache
   - executable cache
   - uploaded thread-local cache
   - resident market-series cache
3. Add explicit cross-layer tests that start from a compiled plan and verify:
   - fingerprint consistency
   - ABI consistency
   - cache-key consistency
   - route diagnostics consistency
4. Create an inventory of environment flags and diagnostic switches so runtime behavior is auditable.
5. Only after the contract is clear, consider surgical file decomposition inside `gpu_backtest_kernel.rs`.
6. Treat strategy-surface code quality as part of the implementation scope: keep strategy-local ownership, test proximity, truth anchors, diagnostics, and route semantics explicit instead of hiding them in generic layers.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the compile -> ABI -> executable -> upload lifecycle in one place.
- [x] Document all GPU runtime caches and their invalidation triggers.
- [x] Add or tighten tests for fingerprint and ABI stability across repeated builds of the same plan.
- [x] Add or tighten tests for cache-hit behavior on compatible repeated runs and cache-miss behavior on incompatible inputs.
- [x] Add or tighten tests for route-diagnostic stability when dynamic GPU handles are present or absent.
- [x] Inventory runtime env flags, diagnostics toggles, and persistence switches in `gpu_backtest_kernel.rs`.
- [x] Decide whether a surgical internal split of `gpu_backtest_kernel.rs` improves auditability without altering behavior.
- [x] Re-run targeted perf/probe tooling for any cache or residency change.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Name the truth anchor, representative fixture or dataset, and any exact-vs-approximate or perf gate evidence required to close this strategy/runtime item.

## Verification Plan

- run targeted tests in `strategy_gpu_program.rs`
- run targeted tests in `strategy_gpu_runtime.rs`
- run targeted tests in `gpu_backtest_kernel.rs`
- run route/probe binaries such as `profile_gpu_dynamic_runner.rs` and `ta_profile_route_matrix.rs`
- run performance-sensitive checks such as `crates/app_core/benches/double_ma_opt.rs` or the repo perf gates when cache/runtime behavior changes
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant Rust quality gates for touched strategy/runtime surfaces: `cargo check --workspace`, the closest crate or strategy tests, and the applicable workflow, CPU, or GPU perf gate when behavior or performance claims change
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- keep the first wave additive: docs, cache inventories, and cross-layer tests
- do not merge structural cleanup without before/after probe evidence
- preserve CPU fallback and existing route gating throughout
- if a cleanup makes route or cache behavior harder to explain, stop and restore the simpler shape
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closed by tightening the runtime cache invalidation contract in [strategy_gpu_runtime.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/strategy_gpu_runtime.rs):
  - added `reset_gpu_strategy_executable_cache_clears_entries_and_counters`
  - freezes the expectation that compiled-bundle and executable caches fully clear their entry counts and hit/miss counters after reset
- Tightened same-plan GPU program stability in [strategy_gpu_program.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/strategy_gpu_program.rs):
  - added `compile_gpu_program_repeated_builds_preserve_fingerprint_and_abi`
  - freezes the expectation that repeated compilation of the same compiled plan preserves program fingerprints and ABI payloads
- Kept scope intentionally additive:
  - no GPU route-policy changes
  - no cache-key changes
  - no CUDA upload/runtime behavior changes
- Acceptance evidence:
  - `cargo test -p ta_app_core strategy_gpu_runtime --lib`
  - `cargo test -p ta_app_core strategy_gpu_program --lib`

## Related Docs

- `REPO_Improvement/07_strats/11_eng_composite_route_policy_2026-04-08.md`
- `REPO_Improvement/07_strats/23_strategy_plan_presets_2026-04-09.md`
- `REPO_Improvement/03_opt/20_eng_dm_gpu_kernel_vram_2026-04-08.md`
- `REPO_Improvement/03_opt/31_gpu_speedups_policy_2026-04-08.md`
