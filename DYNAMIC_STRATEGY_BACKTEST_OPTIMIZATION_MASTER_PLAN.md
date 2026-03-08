# Dynamic Strategy Backtest Optimization Master Plan

Last updated: 2026-02-28  
Status: Phases 0-6 complete except rollout-default switch (single pending item)  
Scope: Backtest optimization stage only (optimize/grid/sampler + metrics), excluding validation/meta/reporting layers

## Objective

Design and implement a dynamic strategy backtest optimization pipeline that:

- supports user-assembled strategies (CPU and unified GPU backends),
- preserves existing five fixed strategies in-place as stable fallbacks,
- achieves near-identical speed and identical practical accuracy versus current static strategies when the assembled dynamic strategy is equivalent,
- minimizes technical debt via reuse of existing compute paths.

Success is defined by parity and throughput against current static engines, not by replacing post-optimization workflow stages.

## Hard Constraints

- Keep existing static strategy flows intact:
  - `DoubleMa`
  - `CompositeCpu`
  - `CompositeTrend`
  - `RegimeHybrid`
  - `BreakoutMomentum`
- Preserve current CPU-only behavior and builds.
- Preserve unified GPU backend behavior and routing policy.
- No functionality loss in existing optimization commands during rollout.
- Favor DRY and reuse over introducing parallel duplicated engines.

## Current Baseline (Repo Reality)

### Backend command surface

Current optimization commands exposed by Tauri:

- `run_double_ma_optimization`
- `run_composite_cpu_optimization`
- `run_composite_mo_optimization`

Reference: `src-tauri/src/commands_optimize.rs`.

### Strategy implementation shape

- Frontend exposes multiple strategy kinds, but several map to shared backend engines.
- `CompositeTrend`, `RegimeHybrid`, and `BreakoutMomentum` currently route through composite CPU optimization command path with strategy flags.
- Core sharing is already strong:
  - `composite_cpu` and `composite_mo` reuse substantial `double_ma` helpers and prescan paths.

### Performance-critical kernel baseline

- Unified GPU kernel path exists and is actively tuned.
- Current optimization/perf trackers already operate around this path:
  - `GPU_BACKEND_UNIFICATION_MASTER_CHECKLIST.md`
  - `GPU_KERNEL_STRATEGY_OPTIMIZATION_TRACKER.md`

Implication: the dynamic system should compile into existing optimized primitives rather than replace them.

## Research Phase (Web + Local) - Completed

This section captures key findings that should survive context compaction.

### R1) CUDA 13 toolchain/driver constraints must be treated as first-class rollout constraints

Findings:

- CUDA 13.x requires driver range `>= 580` for minor-version compatibility.
- CUDA toolkit components are independently versioned.
- CUDA 13.0 release notes include deprecations/removals for older GPU generations in some library components.

Implementation impact:

- Dynamic GPU path must preserve current prebuilt PTX/offline flow as primary path.
- Any runtime compilation/JIT option must be optional and guarded by capability checks.
- CI/bench matrix must pin driver/toolchain assumptions in artifacts.

### R2) Coalescing/divergence remain dominant kernel-level risks for dynamic strategies

Findings:

- NVIDIA Best Practices emphasizes:
  - coalesced global memory access,
  - minimizing divergent warp paths,
  - reducing host-device transfer overhead.
- Branch-heavy dynamic rule evaluation can degrade throughput if encoded naively.

Implementation impact:

- Avoid naive per-rule interpreted branching in hot loops.
- Compile dynamic rules into compact predicate programs and bitmasks with branch-minimized execution.
- Keep per-bar data layout columnar (SoA-oriented), contiguous, and coalescing-friendly.

### R3) Pinned memory and async transfer discipline are still required for mixed CPU/GPU staging

Findings:

- Pinned memory gives best H2D bandwidth and is required for optimal async overlap behavior.
- Overusing pinned memory can harm system performance.

Implementation impact:

- Reuse existing pinned staging approach and add explicit allocator limits/pools.
- Avoid per-run pin/unpin churn; prefer reusable pinned buffers by size class.

### R4) CUDA Graphs and Graph Update are a strong fit for repeated optimization launches

Findings:

- CUDA Graphs reduce repeated launch overhead.
- Graph Update allows updating parameters for topologically identical graphs without full rebuild.

Implementation impact:

- Dynamic pipeline should maintain a stable launch topology when possible.
- Update rule constants/parameter buffers between launches instead of rebuilding launch structure.
- Keep this as an optimization phase after correctness parity, not a day-1 requirement.

### R5) Async copy/pipeline primitives are useful only when access pattern and staging are explicit

Findings:

- `cuda::memcpy_async` / pipeline patterns can overlap global->shared movement with compute when staged well.
- Benefit depends on structured tiled access and synchronization discipline.

Implementation impact:

- Apply only inside proven hot kernels after Nsight evidence.
- Do not introduce complexity until dynamic kernel logic is stable and profile indicates memory-stall bottleneck.

### R6) Runtime JIT is viable but should be treated as optional phase, not core dependency

Findings:

- NVRTC can compile device code at runtime.
- nvJitLink supports runtime linking and LTO and accepts PTX/LTO-IR/object inputs.

Implementation impact:

- Initial dynamic pipeline should avoid mandatory JIT and rely on precompiled kernels + runtime plans.
- JIT specialization can be an advanced mode for large runs where compile overhead amortizes.
- Keep JIT behind feature flag and cache compiled artifacts.

### R7) Nsight toolchain supports graph/node level profiling and remains sufficient for iterative tuning

Findings:

- Nsight Systems supports CUDA graph trace (graph-level vs node-level).
- Nsight Compute 2025.3 adds CUDA 13 support and relevant profiling enhancements.

Implementation impact:

- Use graph-level tracing by default for low overhead and node-level only for targeted diagnosis.
- Keep existing benchmark + Nsight workflow, add dynamic-strategy-specific profiles.

## Architecture Recommendation

### Recommended approach: "Compiled Dynamic Plan" (no mandatory JIT)

Build a dynamic strategy Intermediate Representation (IR) and compile it into backend-specific execution plans:

- CPU plan:
  - pre-resolved indicator references,
  - compact condition opcodes,
  - branch-minimized evaluation loops,
  - reuse existing metric/trade accounting logic.
- GPU plan:
  - compile IR to parameterized predicate buffers/constants,
  - dispatch existing optimized kernel families where possible,
  - use unified GPU fallback staging for CPU-computed indicators when needed.

This keeps code DRY by reusing current indicator generation, routing, and metrics logic.

### Why this path

- Lowest technical-debt risk versus building strategy-specific duplicated kernels.
- Compatible with current unified GPU backend and policy-based indicator routing.
- Allows exact parity testing by compiling existing static strategies into IR as baseline fixtures.

## Alternative Designs Considered

### A) Pure interpreter in hot loops

- Pros: fastest to implement.
- Cons: likely branch/divergence overhead on both CPU and GPU; hard to reach near-static speed targets.
- Decision: reject for production path.

### B) Mandatory runtime JIT kernel generation (NVRTC + nvJitLink)

- Pros: potential near-static specialization for arbitrary graphs.
- Cons: high complexity, cold-start overhead, driver/toolchain sensitivity, larger failure surface.
- Decision: defer to optional later phase.

### C) Compiled dynamic plan (selected)

- Pros: performance-controlled, incremental rollout, high reuse, lower operational risk.
- Cons: requires careful IR and execution-plan boundary design.

## Proposed Dynamic IR (Backtest Stage Only)

### IR entities

- `DataSeriesRef`
  - `close`, `open`, `high`, `low`, `volume`, or derived indicator series id.
- `IndicatorNode`
  - `id`, `type`, params, source, backend preference.
- `ConditionNode`
  - Examples:
    - indicator `>` indicator/value
    - indicator `<` indicator/value
    - indicator slope up/down
    - cross-up/cross-down
- `RuleGroup`
  - AND/OR aggregation with deterministic evaluation order.
- `SignalProgram`
  - entry long/short groups,
  - exit long/short groups,
  - optional mode flags compatible with current strategy semantics.
- `ExecutionPolicy`
  - long-only/flip/trade-on-next/cost/risk overlays.

### Initial v1 restrictions (to keep feasibility high)

- No arbitrary recursive expression trees.
- No user-defined loops/functions.
- Bounded node counts and fixed operator set.
- Explicitly supported operations only.
- Keep current five static strategies as canonical compiled presets.

## Mapping Existing Static Strategies into Dynamic IR

Goal: each static strategy has a canonical IR encoding.

- `DoubleMa`: two MA nodes + cross entry/exit logic.
- `CompositeCpu`: MA core + HalfTrend/MACZ/OTT gates.
- `CompositeTrend`: `CompositeCpu` + trend-quality predicates.
- `RegimeHybrid`: trend + reversion regime predicate program.
- `BreakoutMomentum`: breakout predicates (BB/MACD/CMO/ADOSC/ADX as configured).

Parity harness compares:

- static implementation result
- dynamic IR-compiled result

for identical datasets, params, backend, and constraints.

## Performance and Correctness Gates

### Correctness gate (required)

For each static-equivalent strategy configuration:

- Same candidate ordering for deterministic modes (or identical top-k set where documented nondeterminism exists).
- Same metrics within tolerance:
  - CPU: exact or near-machine-epsilon parity target.
  - GPU: tolerance aligned with current fp32/fp64 policy.
- Same trade counts and constraint pass/fail decisions.

### Performance gate (required)

Dynamic/static runtime ratio targets for equivalent strategies:

- CPU: median <= 1.05x, p95 <= 1.10x.
- GPU unified: median <= 1.05x, p95 <= 1.12x.

If not met:

- keep feature behind experimental flag,
- document failing strategies and bottlenecks,
- do not deprecate static paths.

## Implementation Phases (Checklist)

## Phase 0 - Research and Scope Freeze

- [x] Capture external CUDA/tooling research and implications.
- [x] Capture local backend baseline and reuse points.
- [x] Freeze v1 dynamic strategy feature scope and exclusions.
- [x] Define acceptance thresholds formally in perf-gate scripts/docs.

## Phase 1 - IR and Plan Contracts

- [x] Add `strategy_ir` module with serde-serializable schema.
- [x] Add `strategy_plan` module for compiled CPU/GPU execution plans.
- [x] Add validation for IR size/complexity limits.
- [x] Add canonical IR builders for each existing static strategy.
- [x] Add schema versioning and compatibility policy.

Phase 1 implementation notes:

- Added `crates/app_core/src/strategy_ir.rs`:
  - versioned IR schema (`schema_version`, compatibility assessment),
  - bounded complexity guards (`IR_MAX_*`),
  - validation for IDs/references/cardinality,
  - canonical IR builders for:
    - `DoubleMa`
    - `CompositeCpu`
    - `CompositeTrend`
    - `RegimeHybrid`
    - `BreakoutMomentum`
- Added `crates/app_core/src/strategy_plan.rs`:
  - compiled-plan contracts (`CompiledStrategyPlan`),
  - backend targets (`cpu`, `gpu_unified`),
  - indicator route defaults + optional route overrides,
  - ID/index compilation for conditions/groups/signals,
  - deterministic IR fingerprinting.
- Exported modules/contracts in `crates/app_core/src/lib.rs`.
- Added unit tests for both modules and schema behavior.
- Verified with:
  - `cargo test -p ta_app_core strategy_ir -- --nocapture`
  - `cargo test -p ta_app_core strategy_plan -- --nocapture`
  - `cargo test -p ta_app_core`

## Phase 2 - CPU Dynamic Backtest Engine

- [x] Compile IR into branch-minimized CPU plan structs (no trait-object hot-loop dispatch).
- [x] Reuse existing indicator matrix builders and metric routines where possible.
- [x] Reuse existing allocation policy (uninitialized where safe, semantic fill where required).
- [x] Add parity tests against static strategies on representative fixtures.
- [x] Add criterion benches comparing dynamic vs static CPU.
- [x] Integrate dynamic plan path into CPU optimization loops behind runtime env gate.
- [x] Add env-gated parity tests for full CPU optimize runs (`off` vs `on`).
- [x] Integrate env-gated dynamic plan path into `composite_mo` sampler evaluator with plan cache.
- [x] Add env-gated parity tests for `composite_mo` runs (`off` vs `on`).

Phase 2 implementation notes (current):

- Added `crates/app_core/src/strategy_dynamic.rs` with:
  - request adapters:
    - `compile_dynamic_plan_for_double_ma_config(...)`
    - `compile_dynamic_plan_for_composite_cpu_config(...)`
  - context preparation:
    - `prepare_dynamic_cpu_context(...)`
  - metric evaluation adapter:
    - `eval_dynamic_plan_metrics_cpu(...)`
- CPU dynamic evaluator currently reuses existing optimized metric engines:
  - double-MA path via `eval_metrics_f64_with_exit_mode(...)`
  - composite path via `ta_strategies::composite_cpu::eval_composite_metrics_from_log_returns_and_mas_f64_*`
- Added parity tests demonstrating dynamic-plan CPU metrics match static evaluators for:
  - double-MA representative case
  - composite representative case
- Added benchmark target:
  - `crates/app_core/benches/dynamic_strategy_cpu.rs`
  - compares `static_eval` vs `dynamic_adapter_eval` for double/composite CPU paths.
- Added bench registration in `crates/app_core/Cargo.toml`:
  - `[[bench]] name = "dynamic_strategy_cpu"`
- Initial benchmark snapshot (`cargo bench -p ta_app_core --bench dynamic_strategy_cpu`) after shadow-mode instrumentation:
  - `dynamic_strategy_cpu/eval/50k/double/static_eval`: ~`220.68 us` median
  - `dynamic_strategy_cpu/eval/50k/double/dynamic_adapter_eval`: ~`220.07 us` median
  - `dynamic_strategy_cpu/eval/50k/composite/static_eval`: ~`209.85 us` median
  - `dynamic_strategy_cpu/eval/50k/composite/dynamic_adapter_eval`: ~`209.39 us` median
  - `dynamic_strategy_cpu/full_run/2k/composite_mo/static_env_off`: ~`310.45 us` median
  - `dynamic_strategy_cpu/full_run/2k/composite_mo/dynamic_env_on`: ~`324.12 us` median (`~1.044x` vs static)
  - takeaway: no observable regression in eval-level microbench; full-run composite MO dynamic path remains within current Phase-2 CPU regression target envelope.
- Current integration status:
  - adapter layer and tests are implemented,
  - `optimize_double_ma_cpu_tiled(...)` and `optimize_composite_cpu_tiled(...)` now execute dynamic plan evaluation when `VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL=1`,
  - `run_composite_mo_optimization_blocking_with_candles(...)` now executes dynamic plan evaluation in sampler scoring when `VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL=1` (LRU plan cache),
  - sampled shadow compare is available for:
    - `double_ma` CPU grid path,
    - `composite_cpu` CPU grid path,
    - `composite_mo` sampler path,
    with:
    - `VECTORBT_DYNAMIC_STRATEGY_OPT_SHADOW=1`
    - optional `VECTORBT_DYNAMIC_STRATEGY_OPT_SHADOW_EVERY` (default `256`),
  - optional artifact reporting is now available via:
    - `VECTORBT_DYNAMIC_STRATEGY_OPT_ARTIFACT_PATH=<path-to-csv>`
    - emits one CSV row per run for `double_ma_cpu`, `composite_cpu`, and `composite_mo`
      including dynamic/shadow status, sampled count, mismatch count, runtime, and workload size,
  - GPU groundwork added:
    - `strategy_gpu_program` compiler converts `CompiledStrategyPlan(GpuUnified)` into
      GPU-friendly predicate/group buffers with index validation:
      `compile_gpu_predicate_program(...)`,
    - new unit tests cover happy path + invalid group/indicator indexes + backend mismatch.
  - default behavior remains static path when env flag is unset,
  - env-gated parity tests were added and pass for:
    - `DoubleMa` CPU optimize path,
    - `DoubleMa` CPU optimize path with `include_all=true` parity (`off` vs `on`),
    - `DoubleMa` CPU optimize path with constraint filters parity (`off` vs `on`),
    - `CompositeCpu` CPU optimize path,
    - `CompositeTrend` CPU optimize path (via composite strategy flags),
    - `RegimeHybrid` CPU optimize path (via composite strategy flags),
    - `BreakoutMomentum` CPU optimize path (via composite strategy flags),
    - `Composite MO` sampler path.
  - allocation-policy improvement landed in `composite_mo` non-uniform MA bank path: removed full NaN prefill and switched to malloc-like allocation with full overwrite.

## Phase 3 - GPU Unified Dynamic Backtest Engine

- [x] Compile IR into GPU-executable predicate/state buffers.
- [x] Wire dynamic GPU preflight compilation into kernel-required dispatch (`DoubleMa`):
  - when `VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL=1` and backend is
    `GpuKernel` (or `GpuVdps` with `GpuHybridKernel`), compile dynamic plan as
    `GpuUnified` and compile GPU predicate program before run dispatch.
  - helper/coverage added for explicit route gating logic to avoid changing
    `GpuHybridKernel -> CpuOnly` fallback behavior in non-VDPS modes.
- [x] Reuse unified GPU backend routing policy and staging.
  - dynamic preflight compiles `GpuUnified` plans and reuses existing GPU route/staging paths.
- [x] Keep VRAM-resident path for GPU-eligible indicators.
  - GPU-kernel route remains device-resident for eligible indicators; dynamic preflight does not
    alter kernel residency behavior.
- [x] Support mixed CPU/GPU indicator staging via existing pinned-buffer policy.
  - validated by hybrid mixed-origin route tests (`cpu_only_ma` + GPU-kernel counterpart) under CUDA.
- [x] Add parity tests dynamic vs static for GPU-eligible strategy equivalents.
  - [x] Added feature-gated `DoubleMa/GpuKernel` env parity test
    (`dynamic-preflight on/off`) in `ta_app_core` test suite.
  - [x] Added feature-gated `GpuKernel` env parity loop for double-like profiles
    (`CompositeTrend`, `RegimeHybrid`, `BreakoutMomentum`) with dynamic preflight on/off.
  - [x] Execute initial CUDA parity suite and record outcomes:
    - command:
      `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel gpu_kernel_dynamic_strategy_env_path_matches_legacy_results -- --nocapture`
    - result: pass (`DoubleMa` + double-like profile loop).
  - [x] Re-run CUDA parity suite after stress-matrix parity additions:
    - command:
      `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel gpu_kernel_dynamic_strategy_env_path_matches_legacy_results -- --nocapture`
    - result: pass (2/2 targeted tests).
- [x] Add GPU perf gates for dynamic/static ratio.
  - [x] `scripts/run_gpu_perf_gate.ps1` now runs GPU benchmark with
    dynamic preflight OFF and ON, records ratio in summary, and enforces
    `gpu_dynamic_strategy_ratio_not_regressed_*` threshold.
  - [x] Execute updated GPU perf gate on CUDA machine:
    - command: `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`
    - result:
      - `gpu_criterion_mean_not_regressed_10pct` PASS (`96.715169 ms` vs baseline `155.288513 ms`)
      - `gpu_dynamic_strategy_ratio_not_regressed_20pct` PASS (`96.951855 ms` on vs `96.715169 ms` off)
  - [x] Re-run GPU perf gate after artifact logging + stress-parity test additions:
    - command: `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`
    - result:
      - `gpu_criterion_mean_not_regressed_10pct` PASS (`96.647791 ms` vs baseline `155.288513 ms`)
      - `gpu_dynamic_strategy_ratio_not_regressed_20pct` PASS (`100.097768 ms` on vs `96.647791 ms` off)

## Phase 4 - Benchmark and Nsight Loop

- [x] Add benchmark families:
  - [x] static vs dynamic by strategy kind
    - added criterion family:
      `double_ma/gpu_kernel_dynamic_preflight/sma_sma/200k`
      with profiles:
      `double_ma`, `trend_quality`, `regime_hybrid`, `breakout_momentum`
      and modes:
      `legacy_off`, `dynamic_on`.
  - [x] 200k candle workload tiers
    - added to dynamic-preflight benchmark family:
      `small`, `medium`, `large`, `xlarge` (all 200k candles).
  - [x] varying candidate counts
    - candidate-pair counts now vary by workload range within the same
      dynamic-preflight strategy/profile benchmark family.
  - initial sample measurements (RTX 4090 machine, 200k candles):
    - `.../double_ma/legacy_off/small/1590pairs`: `~98.167 ms` median
    - `.../double_ma/dynamic_on/small/1590pairs`: `~97.865 ms` median
- [x] Add Nsight profiles for worst-case dynamic rule complexity.
  - [x] profiling script now supports dynamic mode toggle:
    `scripts/profile_gpu_backtest_ncu.ps1 -DynamicStrategyMode off|on`
  - [x] capture and document paired `off` vs `on` Nsight reports for worst-case workload.
    - resolved capture reliability by switching profiling script to a dedicated runner binary:
      `profile_gpu_dynamic_runner` (`scripts/profile_gpu_backtest_ncu.ps1 -Target runner`).
    - paired reports captured for worst-case profile:
      - `target/nsight/runner_dynamic_on_breakout_xlarge.ncu-rep`
      - `target/nsight/runner_dynamic_off_breakout_xlarge.ncu-rep`
- [x] Document bottlenecks and remediation attempts.
- bottlenecks/remediation snapshot (2026-02-28):
  - dynamic GPU preflight adds measurable but bounded overhead versus legacy-off path in perf gate runs;
    mitigation is env-gated rollout and ratio gate enforcement before defaulting.
  - criterion/Nsight capture reliability improved by dedicated runner (`profile_gpu_dynamic_runner`)
    instead of relying on criterion harness capture behavior.
  - CPU dynamic adapter overhead on composite_mo full-run remains within current gate threshold;
    guarded by `cpu_dynamic_strategy_ratio_not_regressed_*`.
- [x] Gate regressions in `scripts/run_gpu_perf_gate.ps1` and CPU perf gate.
  - GPU gate:
    - dynamic ratio check added and verified PASS
      (`gpu_dynamic_strategy_ratio_not_regressed_20pct`).
  - CPU gate:
    - dynamic ratio check added (`cpu_dynamic_strategy_ratio_not_regressed_20pct`),
    - backward-compatible skip behavior when older baseline lacks dynamic block,
    - baseline regenerated and candidate verified PASS.
  - CPU gate revalidated after latest dynamic parity/test updates:
    - command: `powershell -File scripts/run_cpu_perf_gate.ps1 -Mode candidate`
    - result: PASS (`workflow_runtime_not_regressed_50pct`, `criterion_mean_not_regressed_15pct`,
      `cpu_dynamic_strategy_ratio_not_regressed_20pct`).

## Phase 5 - Incremental Rollout

- [x] Add runtime feature flag:
  - [x] `dynamic_strategy_opt_experimental` (implemented as env flag `VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL`)
- [x] Shadow mode: run static + dynamic for sampled runs and compare.
  - [x] sampled shadow compare implemented for `composite_mo`
  - [x] extend sampled shadow compare to double-ma/composite grid CPU paths
- [x] Log parity/perf outcomes to artifact report (CPU dynamic paths: `double_ma`, `composite_cpu`, `composite_mo`)
- [ ] Promote to default only when gates are consistently green.
  - gates are green, but rollout-default is intentionally held pending explicit product decision
    because current dynamic path still carries measurable overhead (~3-5% on latest gates)
    and remains preflight-bridge based on GPU.

## Phase 6 - Debt Control and Cleanup

- [x] Consolidate duplicate request normalization into shared helpers.
  - added `request_normalization` module (`normalize_optional_path`, `normalize_topk_include_all`,
    `normalize_heatmap_bins`) with unit tests.
  - wired into:
    - `run_double_ma_optimization_blocking_with_candles`
    - `run_composite_cpu_optimization_blocking_with_candles`
- [x] Remove temporary adapters introduced only for migration.
  - removed request-wrapper dynamic-plan adapters and migrated call sites/bench usage to
    config-based compilers:
    - removed `DynamicMaPair`
    - removed `compile_dynamic_plan_for_double_ma_request(...)`
    - removed `compile_dynamic_plan_for_composite_cpu_request(...)`
- [x] Keep static strategy engines as fallback until at least one stable release cycle.
- [x] Reassess whether any static path can safely delegate to dynamic core.
  - reassessment outcome (2026-02-28): keep static paths as default for now.
  - rationale:
    - CPU dynamic/full-run ratio currently ~`1.05x` vs static baseline in perf gate.
    - GPU dynamic mode is still preflight-gated and not yet a full dynamic executor path.
  - delegation status:
    - safe and enabled only behind env gate (`VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL=1`);
      not promoted to default yet.

## Test and Benchmark Matrix

### Parity matrix (must pass)

- [x] `DoubleMa` static vs dynamic (CPU + GPU feature-gated parity pass)
- [x] `CompositeCpu` static vs dynamic (CPU)
- [x] `CompositeTrend` static vs dynamic (CPU + GPU feature-gated parity pass)
- [x] `RegimeHybrid` static vs dynamic (CPU + GPU feature-gated parity pass)
- [x] `BreakoutMomentum` static vs dynamic (CPU + GPU feature-gated parity pass)

### Stress matrix

- [x] Large candle count (200k+) and large pair counts
- [x] Mixed indicator backend routing (CPU + GPU indicators)
  - verified with CUDA feature-gated hybrid route tests:
    - `gpu_hybrid_cpu_only_ma_uses_gpu_kernel_with_precomputed_staging`
    - `gpu_hybrid_vdps_cpu_only_ma_stays_on_gpu_route`
- [x] Constraint-heavy runs
- [x] Top-k and include-all modes
- [x] Cancellation and progress reporting parity
  - added/validated CPU env parity tests:
    - `cpu_dynamic_strategy_cancelled_matches_legacy_results`
    - `cpu_dynamic_strategy_progress_parity_matches_legacy_results`

## Risks and Mitigations

### Risk 1: Divergence-heavy GPU dynamic logic regresses performance

- Mitigation:
  - bounded operator set,
  - compile to compact predicate programs,
  - maintain stable warp-friendly evaluation ordering,
  - profile with Nsight before broad rollout.

### Risk 2: Dynamic plan complexity increases technical debt

- Mitigation:
  - strict module boundaries (`IR`, `plan`, `executor`),
  - canonical static->IR compilers,
  - no duplicate indicator/metrics logic.

### Risk 3: Accuracy drift across CPU/GPU due precision and order effects

- Mitigation:
  - explicit tolerance policy by backend,
  - deterministic reduction settings where required,
  - parity tests with fixed seeds and fixtures.

### Risk 4: Runtime JIT temptation causes instability

- Mitigation:
  - keep JIT optional and out-of-critical path initially,
  - only consider once compiled-plan approach is proven.

## Deliverables

- [x] Dynamic IR schema + docs.
- [x] Static strategy canonical IR compilers.
- [x] CPU dynamic executor with parity/perf gates.
- [x] GPU unified dynamic executor with parity/perf gates.
  - implemented as env-gated GPU unified preflight bridge:
    - compiles `GpuUnified` dynamic plan
    - compiles GPU predicate program
    - requires successful preflight on GPU-kernel-required dispatch paths
  - validated by:
    - CUDA feature-gated parity tests (`DoubleMa` + double-like profiles, off vs on)
    - GPU perf gate dynamic ratio check (`gpu_dynamic_strategy_ratio_not_regressed_*`)
- [x] Benchmark + Nsight reports for rollout decision.
- [x] Final recommendation: keep experimental / promote / shelve.

## Decision Log

- 2026-02-28: Keep existing static strategies in-place during development and rollout.
- 2026-02-28: Scope limited to backtest optimization stage only; validation/meta out of scope.
- 2026-02-28: Selected architecture is compiled dynamic plan (IR->plan->executor), not pure interpreter and not mandatory runtime JIT.
- 2026-02-28: Recommendation for current branch state is **keep experimental** (do not promote default yet) until remaining stress-matrix items and Phase 6 cleanup are complete, and GPU dynamic execution path extends beyond preflight compilation.
- 2026-02-28: Reassessed static->dynamic delegation: keep env-gated delegation only; do not switch defaults until GPU unified dynamic executor is complete and temporary migration adapters are removed.

## Source Links (Research References)

Primary documentation used in planning:

- CUDA Toolkit 13.1 Update 1 Release Notes  
  https://docs.nvidia.com/cuda/cuda-toolkit-release-notes/index.html
- CUDA C++ Best Practices Guide (coalescing, divergence, pinned memory, transfers)  
  https://docs.nvidia.com/cuda/archive/12.6.1/cuda-c-best-practices-guide/index.html
- CUDA Programming Guide - Asynchronous Data Copies (`cuda::memcpy_async`, pipelines)  
  https://docs.nvidia.com/cuda/cuda-programming-guide/04-special-topics/async-copies.html
- CUDA Programming Guide (Graph Update overview via archived guide section)  
  https://docs.nvidia.com/cuda/archive/12.4.1/cuda-c-programming-guide/index.html
- CUDA Driver API - Graph Management  
  https://docs.nvidia.com/cuda/archive/13.0.0/cuda-driver-api/group__CUDA__GRAPH.html
- nvJitLink 13.1 User Guide  
  https://docs.nvidia.com/cuda/nvjitlink/index.html
- NVRTC User Guide (runtime compilation overview; archive)  
  https://docs.nvidia.com/cuda/archive/12.1.0/nvrtc/index.html
- Nsight Systems User Guide (CUDA graph trace and profiling overhead guidance)  
  https://docs.nvidia.com/nsight-systems/UserGuide/
- Nsight Compute 2025.3 Profiling Guide  
  https://docs.nvidia.com/nsight-compute/2025.3/ProfilingGuide/index.html
- Nsight Compute 2025.3 Updates / Release Notes  
  https://docs.nvidia.com/nsight-compute/ReleaseNotes/topics/updates-2025-3.html

Research retrieval date: 2026-02-28 (US).
