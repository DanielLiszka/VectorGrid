# GPT-5.4 optimization/performance review map

This file is a repo-grounded map for sending tightly scoped optimization and performance review bundles to another model.

It is intentionally narrower than a general backend feature map.

## Purpose

Use this document when you want a reviewer model to:

- audit optimization paths
- audit performance-sensitive fallback behavior
- audit validation cost and throughput
- audit benchmark fidelity and regression-gate alignment
- suggest measured, parity-safe improvements

Do not use this document to ask for broad feature ideation.

## Hard scope guard

The current repo posture is optimization-first, benchmark-first, and parity-preserving.

The review prompt should not drift into:

- new strategies unless directly required to remove a measured performance bottleneck
- new UI surfaces or workflow panels
- generic architecture rewrites
- GPU expansion claims without route, fallback, and parity evidence
- "add more knobs" proposals that are not tied to benchmark-backed value

Out of scope unless directly implicated in a measured optimization problem:

- live execution and broker integration
- reporting/export UX
- artifact management
- meta-selection / ML confidence feature expansion
- broad data-ingestion work beyond residency/cache effects on optimize/validate latency

## Current repo optimization posture

The highest-signal current themes are:

1. `double_ma` remains the main optimization hotspot and route owner.
2. Composite strategy performance is now largely about route policy:
   `CompositeTrend`, `BreakoutMomentum`, and `RegimeHybrid` route through the `double_ma` family with `GpuKernel` vs `GpuHybridKernel` vs CPU fallback decisions.
3. Warm-path latency matters more than speculative redesign.
4. Price-data residency and VRAM reuse are active performance surfaces.
5. Validation cost and fallback behavior are still important and not fully "done".
6. Strategy-local GPU-native routes are performance-critical enough to merit their own isolated reviews.
7. `CompositeMo` is its own search-efficiency surface because sampler cost and GPU prescan can dominate.
8. Perf-gate discipline already exists, so proposed improvements should be benchmark- and gate-aware.

## Recommended review order

1. `double_ma` optimization backend and route/fallback behavior
2. composite route policy and frontend request shaping
3. GPU runtime, residency, and dynamic-strategy overhead
4. validation throughput and fallback rules
5. strategy-local GPU-native optimization/validation routes
6. `CompositeMo` search efficiency and prescan behavior
7. benchmark harnesses, perf gates, and timing-boundary fidelity
8. frontend orchestration and benchmark-backed defaults
9. frontend validation orchestration and artifact-heavy validation flows

## 1. Double MA optimization backend

### Primary files

- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/double_ma_backend.rs`
- `crates/app_core/src/double_ma_mode.rs`
- `crates/app_core/src/double_ma_result.rs`
- `crates/app_core/src/double_ma_strategy.rs`
- `src-tauri/src/commands_optimize.rs`

### What this bundle owns

- main optimization execution for the Double-MA family
- backend selection, fallback reasons, and route summaries
- mode selection including exact grid and VDPS-related search behavior
- result shaping for top-k / include-all / heatmap flows
- strategy-config validation that affects route eligibility and work shape
- Tauri command boundary and product-level normalization

### Performance review focus

- CPU/GPU parity and fallback-path performance cliffs
- route-summary correctness and blocker transparency
- repeated recomputation or unnecessary allocations in hot paths
- hybrid-kernel downgrade behavior
- search-efficiency questions that are already in-repo, especially VDPS
- consistency between optimization output and drilldown/local-stability follow-up paths

### Ask for

- route/fallback performance review
- hot-path allocation/recompute review
- VDPS/search-efficiency review
- parity-risk review

## 2. Composite route policy and request shaping

### Primary files

- `crates/app_core/src/composite_profile_route.rs`
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/utils/strategyOptimizationCapabilities.ts`
- `web/src/strategies/compositeProfileGpuPolicy.ts`
- `web/src/services/optimizeRequestBuilder.test.ts`

### What this bundle owns

- runtime profile identity for `CompositeTrend`, `BreakoutMomentum`, and `RegimeHybrid`
- `GpuKernel` vs `GpuHybridKernel` vs CPU fallback targeting
- frontend request normalization and guardrails
- capability summaries that control which optimization knobs are exposed
- request-size guardrails for pair counts and `include_all`

### Performance review focus

- frontend/backend route drift
- stale backend vocabulary or legacy naming drift
- request shaping that can accidentally explode work size
- ADX branch correctness and fallback visibility
- whether capability exposure matches real backend support
- whether defaults and guardrails are benchmark-backed rather than arbitrary

### Ask for

- route-policy fidelity review
- combinatorial guardrail review
- support-matrix drift review
- benchmark-backed defaults review

## 3. GPU runtime, residency, and dynamic-strategy overhead

### Primary files

- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/dynamic_indicator_adapter.rs`
- `crates/app_core/src/strategy_dynamic.rs`
- `crates/app_core/src/strategy_plan.rs`
- `crates/app_core/src/strategy_gpu_program.rs`
- `crates/app_core/src/vram_ma.rs`
- `src-tauri/src/state.rs`
- `src-tauri/src/data_cache.rs`
- `src-tauri/src/bin/ta_app_path_faceoff.rs`

### Supporting research / tracker files

- `research/PRICE_DATA_RESIDENCY_AND_VRAM_CACHE_IMPLEMENTATION_PLAN_2026-03-12.md`
- `research/DOUBLE_MA_WARM_APP_PATH_OPTIMIZATION_PLAN_2026-03-13.md`
- `research/DOUBLE_MA_GPU_KERNEL_OPTIMIZATION_CHECKLIST_2026-03-13.md`
- `research/OPEN_IMPLEMENTATION_COMPLETION_TRACKER_2026-03-12.md`

### What this bundle owns

- shared GPU runtime behavior for the Double-MA family
- resident market-series caching and raw-field reuse
- dynamic indicator/device staging reuse
- dynamic-strategy experimental/shadow perf surfaces
- app-path timing that includes real request + runtime behavior rather than only Criterion microbench numbers

### Performance review focus

- upload reuse and cache invalidation correctness
- warm-path vs cold-path timing boundaries
- dynamic-strategy preflight overhead
- route metadata and runtime attribution
- whether proposed work is smaller and safer than a redesign
- process-level instability or bimodality before deeper feature work

### Ask for

- residency/cache review
- dynamic-strategy overhead review
- warm app-path latency review
- benchmark-boundary review

## 4. Validation throughput and fallback behavior

### Primary files

- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `src-tauri/src/commands_validation.rs`
- `src-tauri/src/validation_common.rs`
- `web/src/services/validationRequestBuilder.ts`
- `web/src/services/validationRequestBuilder.test.ts`

### Supporting benchmark / tool files

- `src-tauri/src/bin/ta_double_ma_validation_cost.rs`
- `crates/app_core/benches/double_ma_validation_workflow.rs`
- `crates/app_core/benches/composite_validation_workflow.rs`

### What this bundle owns

- Double-MA validation execution and fixed-candidate GPU validation support
- shared reduction/gate/verdict logic
- command-layer cross-ticker orchestration
- frontend validation backend summaries and backend-selection expectations
- measurement surfaces for validation cost relative to optimization

### Performance review focus

- GPU validation support boundaries and fallback reasons
- validation-stage work duplication or rerun amplification
- cross-ticker cost and finalist-preservation tradeoffs
- deterministic behavior in perturb/bootstrap paths that affect measurement fidelity
- CPU-preferred vs GPU-preferred cases encoded in current frontend/backend logic

### Ask for

- validation-cost review
- fallback transparency review
- cross-ticker throughput review
- deterministic-measurement review

## 5. Strategy-local GPU-native optimization and validation routes

### Primary strategy files

- `crates/app_core/src/asr.rs`
- `crates/app_core/src/vcc.rs`
- `crates/app_core/src/vemr.rs`
- `crates/app_core/src/rocb.rs`
- `crates/app_core/src/mkcs.rs`
- `crates/app_core/src/kqsr.rs`
- `crates/app_core/src/carm.rs`
- `crates/app_core/src/kvdf.rs`
- `crates/app_core/src/cdmc.rs`

### GPU companion files

- `crates/app_core/src/asr_gpu_kernel.rs`
- `crates/app_core/src/vcc_gpu_kernel.rs`
- `crates/app_core/src/vemr_gpu_kernel.rs`
- `crates/app_core/src/rocb_gpu_kernel.rs`
- `crates/app_core/src/mkcs_gpu_kernel.rs`
- `crates/app_core/src/kqsr_gpu_kernel.rs`
- `crates/app_core/src/carm_gpu_kernel.rs`
- `crates/app_core/src/kvdf_gpu_kernel.rs`
- `crates/app_core/src/cdmc_gpu_kernel.rs`

### Supporting benchmark files

- `crates/app_core/benches/gpu_native_expansion_opt.rs`
- `crates/app_core/benches/composite_validation_workflow.rs`

### What this bundle owns

- strategy-local GPU-native optimization logic
- strategy-local CPU reference vs GPU scoring parity surfaces
- per-strategy staging costs, fp32-stability constraints, and route blockers
- strategy-specific validation cost differences that can be lost inside broad composite reviews

### Performance review focus

- one strategy at a time, not all strategies in one review
- strategy-local staging or upload waste
- CPU-reference prep that can erase GPU benefit
- per-strategy fallback reasons and fp32-stability blockers
- strategy-specific validation throughput, not just optimizer throughput
- whether a strategy should remain CPU-preferred on current measured workloads

### Ask for

- strategy-local hot-path review
- CPU/GPU payoff review
- staging/fallback review
- per-strategy benchmark review

## 6. `CompositeMo` search efficiency and GPU prescan

### Primary files

- `crates/app_core/src/composite_mo.rs`
- `crates/optimizer/src/mo.rs`
- `crates/optimizer/src/lib.rs`
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/utils/strategyOptimizationCapabilities.ts`

### Supporting benchmark files

- `crates/app_core/benches/dynamic_strategy_cpu.rs`

### What this bundle owns

- multi-objective search-space construction and evaluation cost
- sampler selection (`Random`, `Qmc`, `Tpe`, `Nsga2`)
- optional GPU prescan narrowing for MA domains
- request-shaping and capability exposure for `CompositeMo`

### Performance review focus

- search-space explosion and avoidable evaluation waste
- whether GPU prescan actually narrows cost in worthwhile ways
- stale or over-broad defaults for sampler/prescan settings
- progress/reporting overhead relative to evaluation cost
- constraint handling or domain-shaping behavior that silently multiplies work

### Ask for

- search-efficiency review
- GPU-prescan payoff review
- sampler-overhead review
- request-shape and default-value review

## 7. Benchmark harnesses, perf gates, and timing boundaries

### Primary files

- `scripts/run_cpu_perf_gate.ps1`
- `scripts/run_gpu_perf_gate.ps1`
- `scripts/run_workflow_gate.ps1`
- `scripts/profile_gpu_backtest_ncu.ps1`
- `scripts/profile_gpu_backtest_nsys.ps1`
- `scripts/run_vectorbt_vs_vectorgrid_faceoff.ps1`
- `scripts/run_vectorgrid_double_ma_faceoff.ps1`

### Primary benchmark binaries / benches

- `crates/app_core/benches/double_ma_opt.rs`
- `crates/app_core/benches/double_ma_metrics.rs`
- `crates/app_core/benches/dynamic_strategy_cpu.rs`
- `crates/app_core/benches/gpu_native_expansion_opt.rs`
- `crates/app_core/benches/double_ma_validation_workflow.rs`
- `crates/app_core/benches/composite_validation_workflow.rs`
- `src-tauri/src/bin/ta_profile_route_matrix.rs`
- `src-tauri/src/bin/ta_app_path_faceoff.rs`
- `src-tauri/src/bin/ta_double_ma_validation_cost.rs`

### Useful benchmark/tracker artifacts

- `benchmark_run_summary_2026-02-26.md`
- `benchmark_unification_matrix_alma_2026-02-28.md`
- `benchmark_unification_matrix_alma_2026-02-28.csv`
- `benchmark_asr_gpu_route_200k_2026-03-05.csv`
- `gpu_speedups.csv`

### What this bundle owns

- regression-budget enforcement
- criterion-based engine measurement
- warm app-path measurement
- route-matrix and validation-cost measurement
- Nsight capture entrypoints
- CPU dynamic-strategy overhead ratio checks

### Performance review focus

- whether a claimed improvement is measured on the correct timing boundary
- whether the gates actually reflect the intended production hotspot
- stale benchmark targets or benchmark-shape mismatch
- route metadata visibility inside perf evidence
- whether perf conclusions are supported by existing harnesses before suggesting new ones

### Ask for

- perf-evidence quality review
- timing-boundary review
- perf-gate relevance review
- stale-benchmark-target review

## 8. Frontend orchestration and defaults

### Primary files

- `web/src/services/optimizeService.ts`
- `web/src/services/optimizeService.test.ts`
- `web/src/services/recommendedOptimizationDefaults.ts`
- `src-tauri/src/commands_optimize.rs`

### What this bundle owns

- optimization command selection
- GPU status refresh and parity-check orchestration
- coupling from optimization into artifacts, analysis refresh, and optional validation
- benchmark-sensitive defaults such as top-k / heatmap shaping

### Performance review focus

- unnecessary orchestration work around the actual optimization run
- failure/cancellation behavior that can create wasted work
- stale default shaping that no longer matches measured workloads
- command-selection mismatch with backend route reality

### Ask for

- orchestration-overhead review
- command-selection and cancellation review
- benchmark-backed default review

## 9. Frontend validation orchestration and artifact-heavy flows

### Primary files

- `web/src/services/validationService.ts`
- `web/src/services/validationService.test.ts`
- `web/src/services/validationRequestBuilder.ts`
- `web/src/services/validationRequestBuilder.test.ts`
- `src-tauri/src/commands_validation.rs`

### What this bundle owns

- frontend validation command selection and request execution
- route-summary text, warning summaries, and backend expectation shaping
- validation table/shortlist shaping and cross-ticker summaries
- artifact persistence and export-adjacent validation follow-up work

### Performance review focus

- redundant frontend work around validation execution
- stale route-summary or CPU/GPU preference messaging
- cross-ticker and shortlist shaping overhead
- artifact persistence or export follow-up that may inflate validation latency
- mismatch between frontend route expectations and backend reality

### Ask for

- validation orchestration overhead review
- route-summary fidelity review
- shortlist/artifact follow-up review
- frontend/backend validation drift review

## Prompt rules for GPT-5.4

Use these rules for every bundle.

- Base the review only on the provided files and explicitly cited adjacent files.
- Prioritize measured optimization/performance issues over broad architecture advice.
- Do not propose new product features unless they are directly required to remove a confirmed performance bottleneck or fallback bug.
- Prefer smaller parity-safe changes before larger refactors.
- Separate confirmed issue, likely issue, open question, and optimization opportunity.
- Cite the file and function/module for every material claim.
- When discussing performance, say which timing boundary the claim belongs to:
  app-path, Criterion, validation-cost harness, route matrix, or Nsight diagnostic evidence.
- Treat existing perf gates and benchmark scripts as the default evidence path before suggesting new instrumentation.
- Keep each review bundle small enough for one deep long-think run.
- For strategy-local routes, prefer one strategy family per prompt unless two files are tightly coupled.

## General optimization/performance review prompt

```text
You are reviewing a specific optimization/performance surface from my Rust/Tauri project.

Your job is to evaluate this implementation for:
1. confirmed performance bottlenecks or wasted work
2. CPU/GPU route correctness, fallback transparency, and parity risk
3. benchmark fidelity and timing-boundary correctness
4. regression risk, edge cases, and test gaps
5. realistic, measured, parity-safe optimization opportunities

Important constraints:
- Base your review only on the files I provide.
- Do not propose broad new product features, new strategies, or UI expansion unless directly required to address a confirmed performance problem.
- Prefer benchmark-backed or gate-backed conclusions over speculation.
- If something is unclear, say what is unclear and what adjacent file likely holds the missing context.
- Distinguish clearly between:
  - confirmed issue
  - likely issue
  - open question
  - optimization opportunity
- When making a performance claim, identify the relevant timing boundary:
  - warm app-path runtime
  - Criterion benchmark
  - validation-cost harness
  - route-matrix / route-summary evidence
  - Nsight / profiling diagnostic evidence
- Prioritize route/fallback bugs, repeated recomputation, allocation waste, cache misuse, validation throughput cliffs, and stale benchmark assumptions over style comments.
- Do not give generic praise.

For each finding, use this format:
- Severity: critical / high / medium / low
- Type: performance / correctness / parity / benchmark-fidelity / testing / maintainability
- Location: file + function or module
- Timing boundary affected: app-path / Criterion / validation-cost / route-matrix / profiling-only / unknown
- Why it matters
- Recommended change

Then provide these sections:
1. Surface understanding
2. Findings
3. Safe optimization opportunities
4. Higher-risk optimization opportunities
5. Benchmark and perf-gate review
6. Test review
7. Suggested improvement plan

Surface under review:
[describe the surface here in 1-3 sentences]

Files to review:
[paste file list here]

Extra context:
[paste intended behavior, benchmark context, current route policy, or known perf concern here]
```

## Optional add-on: route/fallback audit

```text
Pay extra attention to:
- requested backend vs backend used
- fallback reasons and blocker visibility
- frontend/backend route-policy drift
- hybrid-route toggle sensitivity
- whether route summaries are preserved into benchmark and user-visible paths
```

## Optional add-on: validation-cost audit

```text
Pay extra attention to:
- duplicate work between optimization and validation
- fixed-candidate GPU validation support boundaries
- cross-ticker rerun amplification
- deterministic behavior in perturb/bootstrap branches
- CPU-preferred versus GPU-preferred cases already encoded in the current code
```

## Optional add-on: benchmark-fidelity audit

```text
Pay extra attention to:
- whether app-path, Criterion, and profiling evidence are being mixed incorrectly
- whether the current benchmark shape still matches the claimed production hotspot
- whether perf-gate targets are stale or too narrow
- whether route metadata and fallback evidence are visible in the benchmark output
```

## Quick bundle picks

Use these when you want a narrow review.

### Bundle A: Double-MA optimizer internals

- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/double_ma_backend.rs`
- `crates/app_core/src/double_ma_mode.rs`
- `crates/app_core/src/double_ma_result.rs`
- `crates/app_core/src/double_ma_strategy.rs`

Ask for:

- hot-path performance review
- route/fallback review
- VDPS/search-efficiency review

### Bundle B: Composite route policy and frontend shaping

- `crates/app_core/src/composite_profile_route.rs`
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/utils/strategyOptimizationCapabilities.ts`
- `web/src/strategies/compositeProfileGpuPolicy.ts`

Ask for:

- route-policy fidelity review
- request explosion / guardrail review
- stale terminology and support-matrix review

### Bundle C: GPU runtime and residency

- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/dynamic_indicator_adapter.rs`
- `crates/app_core/src/strategy_dynamic.rs`
- `crates/app_core/src/vram_ma.rs`
- `src-tauri/src/state.rs`
- `src-tauri/src/bin/ta_app_path_faceoff.rs`

Ask for:

- warm-path latency review
- cache/residency review
- dynamic-strategy overhead review

### Bundle D: Validation throughput

- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `src-tauri/src/commands_validation.rs`
- `web/src/services/validationRequestBuilder.ts`

Ask for:

- validation-cost review
- fallback transparency review
- cross-ticker throughput review

### Bundle E: Perf harnesses and gates

- `scripts/run_cpu_perf_gate.ps1`
- `scripts/run_gpu_perf_gate.ps1`
- `scripts/run_workflow_gate.ps1`
- `crates/app_core/benches/double_ma_opt.rs`
- `crates/app_core/benches/dynamic_strategy_cpu.rs`
- `src-tauri/src/bin/ta_profile_route_matrix.rs`
- `src-tauri/src/bin/ta_double_ma_validation_cost.rs`

Ask for:

- perf-evidence quality review
- timing-boundary review
- stale-benchmark-target review

### Bundle F: One GPU-native strategy at a time

Pick one strategy family such as:

- `crates/app_core/src/asr.rs` + `crates/app_core/src/asr_gpu_kernel.rs`
- `crates/app_core/src/vcc.rs` + `crates/app_core/src/vcc_gpu_kernel.rs`
- `crates/app_core/src/vemr.rs` + `crates/app_core/src/vemr_gpu_kernel.rs`
- `crates/app_core/src/rocb.rs` + `crates/app_core/src/rocb_gpu_kernel.rs`
- `crates/app_core/src/mkcs.rs` + `crates/app_core/src/mkcs_gpu_kernel.rs`
- `crates/app_core/src/kqsr.rs` + `crates/app_core/src/kqsr_gpu_kernel.rs`
- `crates/app_core/src/carm.rs` + `crates/app_core/src/carm_gpu_kernel.rs`
- `crates/app_core/src/kvdf.rs` + `crates/app_core/src/kvdf_gpu_kernel.rs`
- `crates/app_core/src/cdmc.rs` + `crates/app_core/src/cdmc_gpu_kernel.rs`

Optionally add:

- `crates/app_core/benches/gpu_native_expansion_opt.rs`
- `crates/app_core/benches/composite_validation_workflow.rs`

Ask for:

- strategy-local hot-path review
- staging/payoff review
- fallback/parity review

### Bundle G: `CompositeMo` search and prescan

- `crates/app_core/src/composite_mo.rs`
- `crates/optimizer/src/mo.rs`
- `crates/optimizer/src/lib.rs`
- `web/src/services/optimizeRequestBuilder.ts`

Ask for:

- search-space efficiency review
- sampler-overhead review
- GPU-prescan review

### Bundle H: Frontend validation orchestration

- `web/src/services/validationService.ts`
- `web/src/services/validationService.test.ts`
- `web/src/services/validationRequestBuilder.ts`
- `src-tauri/src/commands_validation.rs`

Ask for:

- validation orchestration overhead review
- route-summary fidelity review
- shortlist/artifact follow-up review
