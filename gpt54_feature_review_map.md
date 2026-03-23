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

The first 10 bundles are still the core optimizer-first map.

Bundles 11+ are optional broader application-performance bundles for when you want end-to-end responsiveness coverage beyond the optimizer core.

## Hard scope guard

The current repo posture is optimization-first, benchmark-first, and parity-preserving.

The review prompt should not drift into:

- new strategies unless directly required to remove a measured performance bottleneck
- new UI surfaces or workflow panels
- generic architecture rewrites
- GPU expansion claims without route, fallback, and parity evidence
- "add more knobs" proposals that are not tied to benchmark-backed value

Out of scope unless directly implicated in a measured optimization problem:

- live execution and broker integration, except where a selected bundle explicitly targets runner, snapshot, or stream responsiveness
- reporting/export UX, except where report generation or reveal/open fallback materially affects validation or ML workflow latency
- artifact management, except where artifact persistence / replay materially affects validation or ML workflow latency
- meta-selection / Stage 5 ML confidence feature expansion, except where a selected bundle explicitly targets scan, train, apply, or batch-throughput cost
- broad data-ingestion work beyond residency/cache effects on optimize/validate latency, except where a selected bundle explicitly targets load, resample, or cache latency

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
8. `MlSignal` is now a real workflow with its own dataset-export, score-cache, validation, and policy-sweep cost surfaces, especially around `pattern_recognition`, artifact persistence, and replay.
9. Perf-gate discipline already exists, so proposed improvements should be benchmark- and gate-aware.
10. Broader application responsiveness now has distinct second-tier review surfaces:
    `CompositeCpu` generic hot paths, meta/Stage 5 throughput, analysis/drilldown hydration, data load/resample/cache latency, and live execution/streaming control paths.
11. Allocation and copy-elision discipline is now a cross-cutting review surface because `alloc_policy` exists, but adoption is still uneven across row-major buffers, host readbacks, and NaN-prefix series builders.

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
10. ML signal dataset export, score-cache validation, and replay flows
11. `CompositeCpu` optimizer hot paths and request shaping
12. meta-selection and Stage 5 ML confidence throughput
13. analysis/drilldown and artifact-hydration responsiveness
14. data loading, resampling, and cache latency
15. live execution, Polygon streaming, and runtime-control responsiveness
16. allocation policy, prefix initialization, and copy-elision discipline

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

## 2. Composite route policy and request shaping  ***** _______

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

## 3. GPU runtime, residency, and dynamic-strategy overhead *****  ______

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

## 4. Validation throughput and fallback behavior  ******** _____

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

### Standalone strategy-local primary files

- `crates/app_core/src/asr.rs`
- `crates/app_core/src/vcc.rs`
- `crates/app_core/src/vemr.rs`
- `crates/app_core/src/rocb.rs`
- `crates/app_core/src/mkcs.rs`
- `crates/app_core/src/kqsr.rs`
- `crates/app_core/src/carm.rs`
- `crates/app_core/src/kvdf.rs`
- `crates/app_core/src/cdmc.rs`
- `crates/app_core/src/vcet.rs`

### Standalone strategy-local GPU companion files

- `crates/app_core/src/asr_gpu_kernel.rs`
- `crates/app_core/src/vcc_gpu_kernel.rs`
- `crates/app_core/src/vemr_gpu_kernel.rs`
- `crates/app_core/src/rocb_gpu_kernel.rs`
- `crates/app_core/src/mkcs_gpu_kernel.rs`
- `crates/app_core/src/kqsr_gpu_kernel.rs`
- `crates/app_core/src/carm_gpu_kernel.rs`
- `crates/app_core/src/kvdf_gpu_kernel.rs`
- `crates/app_core/src/cdmc_gpu_kernel.rs`
- `crates/app_core/src/vcet_gpu_kernel.rs`

### Shared-kernel family surfaces that also matter for current repo state

- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/composite_profile_route.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `web/src/utils/strategyOptimizationCapabilities.ts`

### Supporting benchmark files

- `crates/app_core/benches/gpu_native_expansion_opt.rs`
- `crates/app_core/benches/composite_validation_workflow.rs`

### What this bundle owns

- strategy-local GPU-native optimization logic
- strategy-local CPU reference vs GPU scoring parity surfaces
- per-strategy staging costs, fp32-stability constraints, and route blockers
- strategy-specific validation cost differences that can be lost inside broad composite reviews
- current GPU-native route policy for Double-MA-family composite profiles that execute through shared-kernel surfaces instead of standalone `*_gpu_kernel.rs` files

### Scope note

- This section still primarily targets the standalone strategy-local GPU-native paths.
- Use the shared-kernel Double-MA-family surfaces above when the current repo route policy or validation path flows through them.
- The specific Double-MA contract-split benchmark and product-gap history belong in Bundle P below, not in the standalone strategy-local docs.

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
- `crates/app_core/benches/double_ma_followup_workflow.rs`
- `crates/app_core/benches/double_ma_metrics.rs`
- `crates/app_core/benches/dynamic_strategy_cpu.rs`
- `crates/app_core/benches/gpu_native_expansion_opt.rs`
- `crates/app_core/benches/double_ma_validation_workflow.rs`
- `crates/app_core/benches/composite_validation_workflow.rs`
- `crates/app_core/benches/e2e_workflow_ml.rs`
- `src-tauri/src/bin/ta_profile_route_matrix.rs`
- `src-tauri/src/bin/ta_app_path_faceoff.rs`
- `src-tauri/src/bin/ta_double_ma_validation_cost.rs`
- `src-tauri/src/bin/ta_workflow_regression.rs`

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
- workflow-regression and end-to-end ML workflow timing
- Nsight capture entrypoints
- CPU dynamic-strategy overhead ratio checks

### Performance review focus

- whether a claimed improvement is measured on the correct timing boundary
- whether the gates actually reflect the intended production hotspot
- stale benchmark targets or benchmark-shape mismatch
- workflow-regression coverage gaps across optimize / validate / ML flows
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
- `web/src/services/historyService.ts`
- `web/src/stores/workflowArtifactsStore.ts`
- `src-tauri/src/commands_validation.rs`
- `src-tauri/src/commands_reports.rs`
- `src-tauri/src/commands_reports_validation_common.rs`

### What this bundle owns

- frontend validation command selection and request execution
- route-summary text, warning summaries, and backend expectation shaping
- validation table/shortlist shaping and cross-ticker summaries
- artifact persistence, saved-run replay, and export-adjacent validation follow-up work

### Performance review focus

- redundant frontend work around validation execution
- stale route-summary or CPU/GPU preference messaging
- cross-ticker and shortlist shaping overhead
- artifact persistence or export follow-up that may inflate validation latency
- saved-run reload/rerun overhead and workflow-artifact drift
- mismatch between frontend route expectations and backend reality

### Ask for

- validation orchestration overhead review
- route-summary fidelity review
- shortlist/artifact/replay follow-up review
- frontend/backend validation drift review

## 10. ML signal dataset export, score-cache validation, and replay flows

### Primary files

- `crates/app_core/src/ml_signal.rs`
- `src-tauri/src/commands_ml_signal.rs`
- `web/src/services/mlSignalService.ts`
- `web/src/services/mlSignalService.test.ts`
- `web/src/components/MlSignalWorkspacePanel.vue`
- `web/src/services/historyService.ts`
- `web/src/stores/workflowArtifactsStore.ts`

### Supporting benchmark / research files

- `crates/app_core/benches/e2e_workflow_ml.rs`
- `research/ML_SIGNAL_STRATEGY_IMPLEMENTATION_PLAN_2026-03-18.md`
- `research/ml_signal/README.md`

### What this bundle owns

- `MlSignal` dataset export, including `pattern_recognition` feature extraction and derived-series shaping
- split-manifest and ticker-holdout expansion logic that changes work size and artifact size
- score-cache build, timestamp alignment, and skipped-row behavior
- policy-to-position validation and bounded policy-sweep execution
- model-bundle status checks, artifact persistence, and saved-run replay/rerun behavior

### Performance review focus

- repeated dynamic-indicator recomputation or avoidable serialization during dataset export
- `pattern_recognition` matrix density, shape, and cost visibility
- split/holdout settings that silently multiply work or artifact size
- score-cache alignment waste, skipped-row amplification, and failure transparency
- policy-sweep combinatorics, `top_k` truncation behavior, and artifact persistence overhead
- replay/rerun drift between saved ML artifacts and current loaded dataset context
- keeping offline Python training adjacent but out of the hot path unless the provided files show a concrete blocking cost

### Ask for

- feature-extraction cost review
- score-cache alignment and sweep-bounds review
- artifact/replay overhead review
- timing-boundary and benchmark-gap review

## 11. `CompositeCpu` optimizer hot paths and request shaping

### Primary files

- `crates/app_core/src/composite_cpu.rs`
- `src-tauri/src/commands_optimize.rs`
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/services/optimizeService.ts`
- `web/src/utils/strategyOptimizationCapabilities.ts`
- `web/src/services/optimizeRequestBuilder.test.ts`

### Supporting benchmark files

- `crates/app_core/benches/composite_cpu_opt.rs`
- `crates/app_core/benches/composite_validation_workflow.rs`
- `src-tauri/src/bin/ta_workflow_regression.rs`

### What this bundle owns

- generic `CompositeCpu` request normalization and command selection
- base composite grid-search work shape, including `top_k`, `include_all`, and heatmap shaping
- optional GPU-prescan narrowing and its effect on total evaluation cost
- backend selection, fallback reasons, and strategy-config branching inside the shared `CompositeCpu` path

### Performance review focus

- request shapes that silently explode pair counts or heatmap cost
- `gpu_prescan` payoff versus added overhead on realistic workloads
- repeated indicator/cache work in the shared `CompositeCpu` path
- frontend/backend drift around backend selection, `signal_mode`, and strategy-specific guardrails
- benchmark shapes that no longer match the actual `CompositeCpu` product path

### Ask for

- composite hot-path review
- GPU-prescan payoff review
- request-shape and route/fallback review
- benchmark-shape review

## 12. Meta-selection and Stage 5 ML confidence throughput

### Primary files

- `crates/app_core/src/meta_select.rs`
- `crates/app_core/src/ml_confidence.rs`
- `src-tauri/src/commands_meta.rs`
- `web/src/services/metaService.ts`
- `web/src/services/metaRequestBuilder.ts`
- `web/src/services/metaStage4RequestBuilder.ts`

### Supporting benchmark / workflow files

- `crates/app_core/benches/meta_select_workflow.rs`
- `crates/app_core/benches/e2e_workflow_ml.rs`
- `src-tauri/src/bin/ta_workflow_regression.rs`

### What this bundle owns

- run-artifact scan, grouping, shortlist generation, and holdout evaluation cost
- Stage 4 / Stage 5 model-status, train, apply, and policy-evaluation throughput
- batch/meta workflow orchestration and artifact persistence around the meta pipeline

### Performance review focus

- artifact-scan fanout, JSON churn, and grouping overhead
- holdout and batch flows that amplify validation or retraining cost
- model status / auto-train / auto-retrain paths that add user-visible latency
- saved-artifact reuse versus repeated recomputation
- prompt drift from throughput review into broad ML feature ideation

### Ask for

- artifact-scan and grouping-cost review
- model-status/train/apply latency review
- batch/holdout amplification review
- saved-artifact and workflow-latency review

## 13. Analysis/drilldown and artifact-hydration responsiveness

### Primary files

- `src-tauri/src/commands_analysis.rs`
- `web/src/services/analysisService.ts`
- `web/src/services/workflowArtifactsSync.ts`
- `web/src/stores/workflowArtifactsStore.ts`
- `web/src/components/DrillWorkspacePanel.vue`
- `web/src/components/HeatmapWorkspacePanel.vue`

### Supporting benchmark / workflow files

- `crates/app_core/benches/double_ma_followup_workflow.rs`
- `src-tauri/src/bin/ta_workflow_regression.rs`

### What this bundle owns

- drilldown and local-stability command execution
- analysis-table, heatmap, and comparison hydration from workflow artifacts
- legacy-runtime to Vue artifact sync and refresh behavior

### Performance review focus

- repeated artifact load/parse work and redundant recompute
- drill/stability cache invalidation or duplication
- heavy chart/table hydration that blocks user-visible interactions
- workflow-artifact sync that replays more work than needed

### Ask for

- drilldown/local-stability latency review
- artifact-hydration overhead review
- cache and recompute review
- user-visible responsiveness review

## 14. Data loading, resampling, and cache latency

### Primary files

- `src-tauri/src/commands_data.rs`
- `src-tauri/src/data_cache.rs`
- `src-tauri/src/state.rs`
- `web/src/services/dataService.ts`
- `web/src/services/workflowBootstrap.ts`
- `web/src/services/settingsBootstrap.ts`

### What this bundle owns

- local CSV load, cache lookup, and cache-path visibility
- timeframe resample behavior and loaded-data state transitions
- repo-data and market-data directory discovery that affects load/startup time
- workflow bootstrap paths that touch dataset state on startup

### Performance review focus

- cold versus warm load-path mismatch
- resample duplication, cache invalidation, or redundant parsing
- metadata scans that add avoidable startup or dataset-switch latency
- frontend bootstrap work that blocks dataset readiness

### Ask for

- load/resample latency review
- cache hit-path review
- metadata-scan overhead review
- startup/bootstrap latency review

## 15. Live execution, Polygon streaming, and runtime-control responsiveness

### Primary files

- `src-tauri/src/commands_execution.rs`
- `src-tauri/src/polygon.rs`
- `src-tauri/src/commands_data.rs`
- `web/src/services/executionService.ts`
- `web/src/services/runControlService.ts`
- `web/src/stores/executionStore.ts`
- `web/src/tauri/events.ts`

### Supporting test / workflow files

- `web/src/services/executionService.test.ts`
- `web/src/services/runControlService.test.ts`

### What this bundle owns

- live/paper signal execution entrypoints
- Polygon stream lifecycle and live-runner status paths
- snapshot polling, event handling, and runtime-control orchestration

### Performance review focus

- duplicate polling or repeated snapshot fetches
- stream/runner lifecycle churn and state drift
- blocking network work inside command paths
- command cancellation and status-refresh behavior that adds avoidable latency
- user-visible lag in execution controls without drifting into broker-feature ideation

### Ask for

- command-side latency review
- polling/event-churn review
- runner/stream lifecycle review
- user-visible responsiveness review

## 16. Allocation policy, prefix initialization, and copy-elision discipline

### Primary files

- `crates/app_core/src/alloc_policy.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/ml_signal.rs`

### Adjacent hot-path files when strategy-local GPU buffers are implicated

- `crates/app_core/src/asr_gpu_kernel.rs`
- `crates/app_core/src/vcc_gpu_kernel.rs`
- `crates/app_core/src/vemr_gpu_kernel.rs`
- `crates/app_core/src/rocb_gpu_kernel.rs`
- `crates/app_core/src/mkcs_gpu_kernel.rs`
- `crates/app_core/src/kqsr_gpu_kernel.rs`
- `crates/app_core/src/kvdf_gpu_kernel.rs`
- `crates/app_core/src/vcet_gpu_kernel.rs`
- `crates/app_core/src/carm_gpu_kernel.rs`
- `crates/app_core/src/cdmc_gpu_kernel.rs`

### What this bundle owns

- shared `MaybeUninit` allocation helpers and debug-poison conventions
- prefix-initialized series allocation where warmup values such as `NaN` or `0.0` are required but the tail is overwrite-only
- uninitialized host buffers used for device readback where `copy_to(...)` fully populates the destination
- row-major matrix builders where full fill, partial prefix init, and later `copy_from_slice(...)` work need to be distinguished carefully

### Performance review focus

- zero-fill or `vec![..]` allocations whose contents are fully overwritten before any read
- series builders that need only a warm prefix initialized, not a full buffer fill
- 2D matrix builders that could use `make_uninit_matrix_*` plus `init_matrix_prefixes_*`
- host-side copy/readback buffers that are prefilled even though the device writes every element
- places where removing a fill would be unsafe because the value is behaviorally observable before guaranteed overwrite
- whether a remaining cost is really an allocation problem or an unavoidable producer-to-destination copy boundary

### Ask for

- safe `MaybeUninit` adoption review
- prefix-initialization and NaN-warmup review
- host-readback copy-elision review
- unavoidable-versus-avoidable copy-boundary review

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
- Distinguish `MlSignal` from Stage 5 ML confidence work. Do not turn an `MlSignal` review into a general ML-platform redesign.
- For `MlSignal`, do not propose replacing score-cache validation with a new trade engine or moving offline training into Tauri unless the provided files show a measured blocking cost.
- For bundles 11+, prioritize measured application latency and responsiveness problems over UI redesign or product-surface expansion.
- For meta and live-execution bundles, focus on throughput, blocking cost, status drift, and cancellation behavior rather than new trading or ML product ideas.
- Prefer the shared helpers in `crates/app_core/src/alloc_policy.rs` before proposing new local unsafe allocation code.
- Only recommend uninitialized allocation when every element is guaranteed to be written before any read; keep full-fill allocation when the fill value is observable behavior.

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
- `crates/app_core/benches/double_ma_followup_workflow.rs`
- `crates/app_core/benches/dynamic_strategy_cpu.rs`
- `crates/app_core/benches/e2e_workflow_ml.rs`
- `src-tauri/src/bin/ta_profile_route_matrix.rs`
- `src-tauri/src/bin/ta_double_ma_validation_cost.rs`
- `src-tauri/src/bin/ta_workflow_regression.rs`

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
- `crates/app_core/src/vcet.rs` + `crates/app_core/src/vcet_gpu_kernel.rs`

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
- `web/src/services/historyService.ts`
- `web/src/stores/workflowArtifactsStore.ts`
- `src-tauri/src/commands_reports.rs`
- `src-tauri/src/commands_reports_validation_common.rs`
- `src-tauri/src/commands_validation.rs`

Ask for:

- validation orchestration overhead review
- route-summary fidelity review
- shortlist/artifact/replay follow-up review

### Bundle I: ML signal workflow

- `crates/app_core/src/ml_signal.rs`
- `src-tauri/src/commands_ml_signal.rs`
- `web/src/services/mlSignalService.ts`
- `web/src/services/mlSignalService.test.ts`
- `web/src/components/MlSignalWorkspacePanel.vue`
- `web/src/services/historyService.ts`
- `web/src/stores/workflowArtifactsStore.ts`

Optionally add:

- `crates/app_core/benches/e2e_workflow_ml.rs`
- `research/ml_signal/README.md`

Ask for:

- dataset-export feature-cost review
- score-cache alignment / skipped-row review
- policy-sweep bound review
- artifact persistence and replay review

### Bundle J: `CompositeCpu` hot path

- `crates/app_core/src/composite_cpu.rs`
- `src-tauri/src/commands_optimize.rs`
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/services/optimizeService.ts`
- `web/src/utils/strategyOptimizationCapabilities.ts`

Optionally add:

- `crates/app_core/benches/composite_cpu_opt.rs`
- `src-tauri/src/bin/ta_workflow_regression.rs`

Ask for:

- composite hot-path review
- GPU-prescan payoff review
- request-shape and fallback review

### Bundle K: Meta / Stage 5 throughput

- `crates/app_core/src/meta_select.rs`
- `crates/app_core/src/ml_confidence.rs`
- `src-tauri/src/commands_meta.rs`
- `web/src/services/metaService.ts`
- `web/src/services/metaRequestBuilder.ts`
- `web/src/services/metaStage4RequestBuilder.ts`

Optionally add:

- `crates/app_core/benches/meta_select_workflow.rs`
- `src-tauri/src/bin/ta_workflow_regression.rs`

Ask for:

- artifact-scan and grouping-cost review
- model-status/train/apply latency review
- batch/holdout amplification review

### Bundle L: Analysis / drilldown responsiveness

- `src-tauri/src/commands_analysis.rs`
- `web/src/services/analysisService.ts`
- `web/src/services/workflowArtifactsSync.ts`
- `web/src/stores/workflowArtifactsStore.ts`

Optionally add:

- `crates/app_core/benches/double_ma_followup_workflow.rs`

Ask for:

- drilldown/local-stability latency review
- artifact-hydration overhead review
- cache/recompute review

### Bundle M: Data load / resample / cache

- `src-tauri/src/commands_data.rs`
- `src-tauri/src/data_cache.rs`
- `src-tauri/src/state.rs`
- `web/src/services/dataService.ts`
- `web/src/services/workflowBootstrap.ts`

Ask for:

- load/resample latency review
- cache hit-path review
- metadata-scan overhead review

### Bundle N: Live execution / stream responsiveness

- `src-tauri/src/commands_execution.rs`
- `src-tauri/src/polygon.rs`
- `web/src/services/executionService.ts`
- `web/src/services/runControlService.ts`
- `web/src/stores/executionStore.ts`
- `web/src/tauri/events.ts`

Ask for:

- command-side latency review
- polling/event-churn review
- runner/stream lifecycle review

### Bundle O: Allocation policy and copy elision

- `crates/app_core/src/alloc_policy.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/ml_signal.rs`

Optionally add:

- `crates/app_core/src/asr_gpu_kernel.rs`
- `crates/app_core/src/vcc_gpu_kernel.rs`
- `crates/app_core/src/vcet_gpu_kernel.rs`

Ask for:

- safe `MaybeUninit` adoption review
- prefix-init / NaN-warmup review
- host-readback copy-elision review

### Bundle P: Double MA search-only vs product-path gap

- `crates/app_core/benches/double_ma_opt.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/optimizer/src/stream.rs`
- `research/MA review/double_ma_validated_implementation_tracking_plan_2026-03-23.md`

Optionally add:

- `crates/app_core/benches/double_ma_metrics.rs`
- `research/DOUBLE_MA_ALMA_GPU_FULL_OPTIMIZATION_PASS_2026-03-21.md`
- `research/double_ma_cuda13_optimization_guide.md`

Current repo state:

- The preferred `Sharpe` / `Volatility` product path is now a promoted 10-row single-pass contract with exact `omega`.
- `VECTORBT_KERNEL_PRODUCT_REDUCED_ROWS=0` forces the current 7-row + replay diagnostic baseline.
- The old two-pass finalist-recompute path is no longer the default product route for this benchmark family.

Ask for:

- review whether the remaining `search_only` versus `end_to_end_product` delta is still meaningful on the current promoted single-pass path
- direct first-pass / `search_only` optimization review now that the product path no longer depends on finalist replay by default
- review whether the legacy 7-row + replay diagnostic baseline should remain available as-is or be simplified further
- review whether the promoted single-pass full-metrics contract should be generalized to adjacent shared-kernel routes or objectives
- small-finalist GPU execution-shape review only if the legacy replay path is intentionally revisited
- device-resident reuse / state-carry-forward review
