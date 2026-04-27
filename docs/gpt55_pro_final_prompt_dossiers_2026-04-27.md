# GPT-5.5 Pro final prompt dossiers

Status: active prompt pack  
Created: 2026-04-27  
Depends on:

- `docs/gpt55_pro_hard_problem_meta_research_plan_2026-04-27.md`
- `docs/gpt55_pro_hard_problem_atlas_2026-04-27.md`

## Purpose

This document contains the final GPT-5.5 Pro prompt pack for VectorGrid hard problems.

Use it when a problem needs unusually strong math, science, algorithmic, GPU, statistical, or architecture reasoning before implementation begins.

Do not use these prompts as broad repo-review prompts. Each prompt has a concrete question, file pack, source context, required output format, verification path, and kill criteria.

## How To Use This Prompt Pack

1. Pick exactly one dossier unless the problem explicitly spans two dossiers.
2. Provide the global context files listed below.
3. Provide the dossier's required file pack.
4. Add optional files only when the prompt says they are relevant or when context budget allows.
5. Include the external source summaries and links listed in the prompt.
6. Ask GPT-5.5 Pro for a decision, not just ideas.
7. Triage the response locally before implementation.

## 20-File Upload Variant

If GPT-5.5 Pro enforces a 20-file upload limit, use `docs/gpt55_pro_prompt_pack_20file_2026-04-27`.

Each dossier folder in that package is flat and contains 20 files or fewer. Required/global source files are bundled into `context_part_*.md` files with explicit file-boundary delimiters, and `02_context_manifest.md` maps original repository paths to bundle files.

Generated or unusually low-signal files, such as prebuilt PTX, may be summarized in `03_summarized_or_omitted_files.md` instead of included verbatim. Treat those summaries as context hints only. If the model needs an optional or summarized file to make a final implementation recommendation, it should ask for that exact file path before proceeding.

## Future Prompt Ideas

For researched high-ROI prompt candidates beyond these 12 dossiers, see `docs/gpt55_pro_high_roi_future_prompt_ideas_2026-04-27.md`.

## Global Context Files

Provide these for every GPT-5.5 Pro dossier unless context is extremely constrained:

- `AGENTS.md`
- `docs/REPO_MAP.md`
- `docs/ARCHITECTURE_NOTES.md`
- `docs/gpt55_pro_hard_problem_meta_research_plan_2026-04-27.md`
- `docs/gpt55_pro_hard_problem_atlas_2026-04-27.md`
- root `Cargo.toml`
- `crates/app_core/Cargo.toml`
- `src-tauri/Cargo.toml`
- `web/package.json`

Add these only when the dossier touches release, packaging, desktop runtime, or CUDA distribution:

- `docs/RELEASE.md`
- `src-tauri/tauri.conf.json`
- `tauri.conf.json`
- `scripts/check_tauri_config_sync.mjs`

Add these only when a recommendation requires Tauri command changes, signature changes, event changes, capability changes, or command-codegen changes:

- `src-tauri/src/lib.rs`
- `src-tauri/src/commands.rs`
- `src-tauri/capabilities/main.json`
- `src-tauri/capabilities/dev.json`
- `web/src/tauri/commands.ts`
- `web/scripts/generate-tauri-commands.mjs`
- `web/scripts/check-tauri-parity.mjs`
- `web/scripts/check-known-tauri-commands.mjs`
- `web/scripts/check-tauri-capabilities.mjs`
- `src-tauri/tests/command_runtime_wiring.rs`

Global Tauri-boundary question:

- Does this recommendation require command additions, signature changes, event changes, or capability changes, and what parity/capability checks must move with it?

## Global Prompt Contract

Prepend this contract to every dossier prompt.

```md
You are reviewing VectorGrid, a Tauri desktop application for GPU-accelerated trading strategy optimization, validation, options-expression research, ML Signal workflows, and live execution support.

Use the provided repository files as the source of truth for current implementation. Use the external research summary only for technical background and constraints. Do not propose broad rewrites unless you prove they are necessary.

Required response format:

1. Feasibility verdict: build, spike, defer, or kill.
2. Main reasoning: separate repo facts, external source facts, and assumptions.
3. Recommended architecture or algorithm.
4. Alternatives considered and why they lose.
5. Implementation slices, ordered by smallest safe evidence-producing step.
6. Verification plan with exact test/gate categories.
7. CPU-only fallback, rollback path, or no-op behavior.
8. Risk register: parity, performance, numerical, user-risk, packaging, and maintenance risks.
9. Kill criteria: what evidence would prove this is not worth building.
10. Questions that cannot be answered from the provided context.

Constraints:

- CPU-only builds must keep working.
- CUDA paths must preserve CPU oracle/parity expectations.
- Strategy-local organization is preferred over a large generic execution layer.
- Do not weaken artifact provenance, validation reportability, or route/fallback transparency.
- Do not recommend new UI or product surfaces unless they are required to preserve scientific correctness or user safety.
- If a source fact may have changed, flag it as requiring recheck before implementation.
```

## Dossier Index

| Dossier | Problem | Primary Decision |
| --- | --- | --- |
| 1 | GPU-resident validation, optimization-pass enrichment, and CUDA residency | Architecture and algorithm design |
| 2 | Options objective, advanced fixed-geometry structures, and CPU/GPU parity | Finance/math objective and structure admission |
| 3 | Quote-aware options data, market-data quality, and provenance | Provider/data-quality policy |
| 4 | Validation evidence theory, ML Signal calibration, and Stage 5 leakage control | Statistical evidence model |
| 5 | Tensor-core workload discovery | Prove useful dense workload or kill |
| 6 | OptiX/RT-core prove-or-kill | Prove ray/BVH mapping or kill |
| 7 | Live execution safety state machine | Formal invariants and evidence-to-action boundary |
| 8 | Artifact, replay, report, and evidence UX contract | Auditability and schema decision |
| 9 | Dynamic strategy IR, indicator catalog, and strategy surface | Capability model and route-policy proof |
| 10 | Frontend workflow orchestration and evidence-to-decision UX | State model and decision-support constraints |
| 11 | Campaign, batch, and resumable research workflows | Deterministic resume/retry model |
| 12 | Verification, benchmark fidelity, release, and packaging authority | Evidence and deployment authority |

---

## Dossier 1: GPU-Resident Validation, Optimization-Pass Enrichment, And CUDA Residency

### Prompt

```md
Apply the global prompt contract.

Problem:

VectorGrid has mature CPU and CUDA optimization paths, validation paths, route diagnostics, top-k/include-all/heatmap outputs, and GPU perf gates. The hard question is whether the optimization and validation pipeline can do more useful work per GPU pass while preserving CPU parity, reportability, deterministic ordering, route/fallback transparency, and CPU-only builds.

Decide whether VectorGrid should pursue a GPU-resident validation and optimization-pass enrichment architecture. Include top-k/reduction design, candidate compaction, heatmap handling, validation finalist/stress lanes, CPU/GPU tolerance policy, CUDA residency ownership, and CUDA Graphs/launch-overhead viability.

Specific questions:

1. Which current CPU tails or host readbacks look architecturally necessary, and which are accidental?
2. Can validation candidate evaluation, perturbation/stress lanes, finalist revalidation, and summary evidence stay GPU-resident?
3. Should top-k, heatmap, Pareto, shortlist union, confidence slices, and export-all behavior share a fused or staged reduction path?
4. Is CUB DeviceTopK or another GPU selection primitive appropriate despite tie-order/nondeterminism caveats?
5. What deterministic tie-breaking and non-finite score policy is required?
6. Would CUDA Graphs help in this repository, and what launch-shape stability is needed before a spike?
7. What is the smallest benchmarkable implementation slice?
8. What should be killed or deferred?
9. For each standalone strategy profile, which CPU prep, host readbacks, and validation routes are architecturally required versus accidental?
10. Which mixed validation routes, especially `Vcc`, `Carm`, and `Cdmc`, prevent a fully GPU-resident validation design?
11. How do `cross_ticker`, aux analytics, walk-forward, reopt, SPA, perturb, and bootstrap support differ by strategy family and route?
```

### Required File Pack

Core routing and mode files:

- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/double_ma_backend.rs`
- `crates/app_core/src/double_ma_mode.rs`
- `crates/app_core/src/double_ma_result.rs`
- `crates/app_core/src/double_ma_strategy.rs`
- `crates/app_core/src/composite_profile_route.rs`

CUDA/runtime files:

- `crates/app_core/src/alloc_policy.rs`
- `crates/app_core/src/cuda_device.rs`
- `crates/app_core/src/nvtx.rs`
- `crates/app_core/src/progress.rs`
- `crates/app_core/src/request_normalization.rs`
- `crates/app_core/src/timeframe.rs`
- `crates/app_core/src/matrix_index.rs`
- `crates/app_core/src/double_ma_vdps.rs`
- `crates/app_core/src/prescan_predictive.rs`
- `crates/app_core/src/composite_gpu_metric_decode.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/vram_ma.rs`
- `crates/app_core/src/dynamic_indicator_adapter.rs`
- `crates/app_core/src/strategy_gpu_runtime.rs`
- `crates/app_core/src/standalone_gpu_runtime.rs`
- `crates/app_core/src/strategy_gpu_program.rs`

Validation/statistics files:

- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/validation_core.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/validation_stats.rs`
- `crates/app_core/src/validation_shared.rs`
- `crates/app_core/src/stats_common.rs`
- `crates/app_core/src/sharpe_stats.rs`
- `crates/app_core/src/iterative_sim.rs`
- `crates/app_core/src/cscv_pbo.rs`
- `crates/app_core/src/composite_validate.rs`
- `src-tauri/src/commands_validation.rs`
- `src-tauri/src/validation_common.rs`
- `src-tauri/src/metric_surface.rs`

Application request and route boundary:

- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/services/optimizeRequestBuilder.test.ts`
- `web/src/services/optimizeService.ts`
- `src-tauri/src/commands_optimize.rs`
- `web/src/services/validationRequestBuilder.ts`
- `web/src/services/validationRequestBuilder.test.ts`
- `web/src/services/validationService.ts`
- `web/src/services/validationService.test.ts`
- `web/src/utils/strategyOptimizationCapabilities.ts`
- `web/src/utils/strategyOptimizationCapabilities.test.ts`

Optimizer support:

- `crates/optimizer/src/grid.rs`
- `crates/optimizer/src/stream.rs`
- `crates/optimizer/src/mo.rs`
- `crates/optimizer/src/lib.rs`
- `crates/app_core/src/composite_mo.rs`

Verification and profiling:

- `scripts/run_gpu_perf_gate.ps1`
- `scripts/run_gpu_validation_perf_gate.ps1`
- `scripts/run_gpu_followup_perf_gate.ps1`
- `scripts/run_gpu_alma_contract_rows_perf_gate.ps1`
- `scripts/run_cpu_perf_gate.ps1`
- `scripts/run_workflow_gate.ps1`
- `scripts/profile_gpu_backtest_ncu.ps1`
- `scripts/profile_gpu_backtest_nsys.ps1`
- `crates/app_core/benches/double_ma_opt.rs`
- `crates/app_core/benches/double_ma_followup_workflow.rs`
- `crates/app_core/benches/double_ma_metrics.rs`
- `crates/app_core/benches/double_ma_validation_workflow.rs`
- `crates/app_core/benches/composite_cpu_opt.rs`
- `src-tauri/src/bin/ta_double_ma_validation_cost.rs`
- `src-tauri/src/bin/ta_workflow_regression.rs`

### Optional File Pack

Use if the question expands to app-path latency, route matrices, or frontend request shaping:

- `web/src/strategies/compositeProfileGpuPolicy.ts`
- `src-tauri/src/bin/ta_app_path_faceoff.rs`
- `src-tauri/src/bin/ta_profile_route_matrix.rs`

### External Context To Include

- NVIDIA CUDA Programming Guide: https://docs.nvidia.com/cuda/cuda-programming-guide/
- NVIDIA CUB documentation: https://docs.nvidia.com/cuda/cub/index.html
- CUB DeviceTopK API: https://nvidia.github.io/cccl/cub/api/structcub_1_1DeviceTopK.html
- RadiK GPU radix top-k paper: https://arxiv.org/abs/2501.14336
- Deflated Sharpe Ratio: https://www.davidhbailey.com/dhbpapers/deflated-sharpe.pdf
- Probability of Backtest Overfitting: https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2326253

### Required Output Emphasis

- deterministic GPU selection/reduction policy
- CPU/GPU parity and tolerance model
- route/fallback reporting changes
- benchmark and profiling plan
- smallest safe implementation slice
- kill criteria for CUDA Graphs, fused top-k, and GPU-resident validation

### Local Verification Authority

- `cargo test -p ta_app_core`
- `cargo test -p ta_optimizer`
- `powershell -File scripts/run_gpu_validation_perf_gate.ps1 -Mode candidate`
- `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`
- `powershell -File scripts/run_cpu_perf_gate.ps1 -Mode candidate`
- `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate`

---

## Dossier 2: Options Objective, Advanced Fixed-Geometry Structures, And CPU/GPU Parity

### Prompt

```md
Apply the global prompt contract.

Problem:

VectorGrid's options-expression workflow currently optimizes fixed-shape options tuples over validated parent artifacts. It has a CPU oracle, CUDA scoring path, three active structure families, embedded validation, retained-tuple replay, robust scoring, and strict provenance constraints. The hard question is what objective, ranking model, and advanced fixed-geometry structures should be admitted without breaking tensor density, CPU/GPU parity, or decision-grade validation.

Decide what the next options-expression objective and strategy-admission model should be.

Specific questions:

1. What objective function should rank options tuples when search, holdout, stress, harsher stress, invalid rate, premium efficiency, liquidity, and quote realism all matter?
2. Should the ranking be search-first, validation-first, stress-first, robust-score based, or multi-objective?
3. Which advanced structures can be supported while preserving fixed geometry and deterministic tensor compilation?
4. Should straddles, strangles, butterflies, condors, ladders, calendars, diagonals, rolling, short premium, assignment/exercise, or margin modeling be admitted, deferred, or killed?
5. What data-quality and quote-quality assumptions are mandatory for each admitted structure?
6. What CPU/GPU parity constraints must the CUDA scorer satisfy?
7. What host exactification is allowed after GPU scoring without invalidating top-k or robust-score ordering?
8. What is the smallest safe implementation sequence?
```

### Required File Pack

Current and future options scope:

- `docs/options_research/options_expression_optimizer_viable_plan_2026-04-02.md`
- `docs/options_research/options_expression_future_scope_roadmap_2026-04-07.md`
- `docs/options_research/options_expression_polygon_validation_hardening_implementation_plan_2026-04-25.md`

Backend and CUDA:

- `crates/app_core/src/options_types.rs`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_expression_gpu.rs`
- `crates/app_core/src/options_signal_manifest.rs`
- `crates/app_core/src/options_data_store.rs`
- `crates/app_core/src/kernels/options_expression.cu`
- `src-tauri/src/commands_options_expression.rs`
- `src-tauri/src/options_parent_adapters.rs`

Frontend:

- `web/src/components/OptionsExpressionPanel.vue`
- `web/src/components/OptionsExpressionPanel.test.ts`
- `web/src/stores/optionsExpressionStore.ts`
- `web/src/stores/optionsExpressionStore.test.ts`
- `web/src/services/optionsExpressionService.ts`
- `web/src/services/optionsExpressionService.test.ts`

Verification:

- `scripts/run_options_expression_phase05.ps1`
- `scripts/run_options_expression_gpu_promotion_gate.ps1`
- `scripts/profile_options_expression_compute_sanitizer.ps1`
- `scripts/profile_options_expression_ncu.ps1`
- `scripts/profile_options_expression_nsys.ps1`
- `src-tauri/src/bin/ta_options_expression_feasibility.rs`
- `crates/app_core/benches/options_expression_candidate_compile.rs`
- `crates/app_core/benches/options_expression_summary_compile.rs`
- `crates/app_core/benches/options_expression_cpu_score.rs`
- `crates/app_core/benches/options_expression_gpu_score.rs`
- `crates/app_core/benches/options_expression_bench_support.rs`

### Optional File Pack

Use if parent artifact support or validation replay is central:

- `crates/app_core/src/strategy_presets.rs`
- `crates/app_core/src/double_ma_test_support.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `src-tauri/src/commands_artifacts.rs`
- `src-tauri/src/run_artifacts.rs`
- `web/src/services/historyService.ts`
- `web/src/stores/workflowArtifactsStore.ts`

### External Context To Include

- Polygon/Massive Options API docs: https://www.polygon.io/docs/options
- Cboe strategy-based margin: https://www.cboe.com/en/markets/us/options/margin/strategy-based-margin/
- OCC options disclosure document page: https://www.theocc.com/company-information/documents-and-archives/options-disclosure-document
- NVIDIA Monte Carlo option pricing material: https://developer.download.nvidia.com/assets/cuda/files/MonteCarlo.pdf
- Deflated Sharpe Ratio: https://www.davidhbailey.com/dhbpapers/deflated-sharpe.pdf
- Probability of Backtest Overfitting: https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2326253

### Required Output Emphasis

- options objective/ranking design
- strategy admission matrix
- "admit/defer/kill" decision for each candidate structure
- CPU/GPU rank-parity requirements
- quote/data requirements per structure
- minimum implementation sequence
- explicit out-of-scope list

### Local Verification Authority

- `cargo test -p ta_app_core options_expression`
- `powershell -File scripts/run_options_expression_phase05.ps1`
- `powershell -File scripts/run_options_expression_gpu_promotion_gate.ps1`
- `powershell -File scripts/profile_options_expression_compute_sanitizer.ps1`
- `npm --prefix web run test -- optionsExpression`

---

## Dossier 3: Quote-Aware Options Data, Market-Data Quality, And Provenance

### Prompt

```md
Apply the global prompt contract.

Problem:

VectorGrid supports Polygon historical data, Polygon live stock streaming, options-expression data import/download, aggregate-lane direct provider downloads, quote-aware imported datasets, cache/provenance metadata, exact replay, and explicit replacement/cross-lane policies. The hard question is whether quote-aware direct options acquisition and stronger market-data quality auditing should be built, and what provenance schema is sufficient for decision-grade replay.

Decide the quote-aware acquisition and market-data provenance policy.

Specific questions:

1. Can direct quote-aware Polygon/Massive options acquisition be made trustworthy enough for optimization and validation?
2. What endpoint, entitlement, paging, timestamp, and cost-preview constraints must be researched or rechecked?
3. What quote-quality summary is sufficient: bid/ask availability, midpoint validity, spread, stale quote age, NBBO assumptions, underlying alignment, contract-reference freshness?
4. How should exact replay, same-root replacement, cross-lane comparison, and legacy provider aliases behave?
5. What data-quality issues most threaten optimization validity for stock/FX/crypto/options datasets?
6. Is sampled file fingerprinting enough for decision-grade audit, or is a stronger cache identity needed?
7. What should be hard rejected, warned, repaired, or allowed with provenance flags?
8. How should the options dataset store under the effective market-data root relate to Polygon cache root migration, exact replay, and provider-download/import identity?
```

### Required File Pack

Options data and provenance:

- `docs/options_research/options_expression_polygon_validation_hardening_implementation_plan_2026-04-25.md`
- `crates/app_core/src/options_types.rs`
- `crates/app_core/src/options_data_store.rs`
- `src-tauri/src/commands_options_data.rs`
- `src-tauri/src/commands_options_expression.rs`
- `src-tauri/src/bin/ta_options_dataset_import.rs`
- `src-tauri/src/bin/ta_options_dataset_download.rs`
- `web/src/components/OptionsExpressionPanel.vue`
- `web/src/components/OptionsExpressionPanel.test.ts`
- `web/src/stores/optionsExpressionStore.ts`
- `web/src/services/optionsExpressionService.ts`
- `web/src/utils/core/artifactIdentity.ts`
- `web/src/services/historyService.ts`

General data pipeline:

- `src-tauri/src/polygon.rs`
- `src-tauri/src/data_cache.rs`
- `src-tauri/src/state.rs`
- `src-tauri/src/settings.rs`
- `src-tauri/src/commands_data.rs`
- `crates/app_core/src/bin/real_data_fidelity_audit.rs`
- `crates/app_core/src/bin/real_data_optimizer_eval.rs`
- `web/src/services/dataService.ts`
- `web/src/services/dataService.test.ts`
- `web/src/stores/dataStore.ts`
- `web/src/stores/dataStore.actions.test.ts`
- `web/src/utils/core/datasetMeta.ts`
- `web/src/utils/core/datasetMeta.test.ts`

Tests and gates:

- `web/src/services/optionsExpressionService.test.ts`
- `web/src/stores/optionsExpressionStore.test.ts`
- `web/src/utils/core/artifactIdentity.test.ts`
- `scripts/run_output_parity_check.mjs`
- `scripts/check_output_parity.mjs`

### Optional File Pack

Use if live-data mutation or live execution evidence matters:

- `src-tauri/src/commands_execution.rs`
- `web/src/services/executionService.ts`
- `web/src/stores/executionStore.ts`

### External Context To Include

- Polygon/Massive Options API docs: https://www.polygon.io/docs/options
- OCC options disclosure document page: https://www.theocc.com/company-information/documents-and-archives/options-disclosure-document
- OPRA/options market data references if available
- Research or documentation on stale quote detection, bid/ask/midpoint execution assumptions, and OHLCV resampling semantics

### Required Output Emphasis

- provider and endpoint feasibility verdict
- quote-quality admission policy
- exact replay/replacement/cross-lane provenance model
- cache identity and repair policy
- manual live-credential evidence required before productization
- implementation sequence that preserves aggregate-lane behavior

### Local Verification Authority

- `cargo test -p ta_desktop_demo_app --lib polygon`
- data cache and state resampling tests
- `npm --prefix web run test -- dataService dataStore`
- `npm --prefix web run test -- optionsExpression`
- `node scripts/run_output_parity_check.mjs`
- `node scripts/check_output_parity.mjs`
- live provider run only as manual evidence

---

## Dossier 4: Validation Evidence Theory, ML Signal Calibration, And Stage 5 Leakage Control

### Prompt

```md
Apply the global prompt contract.

Problem:

VectorGrid has validation statistics, PBO/CSCV-related code, PSR/DSR fields, meta-selection, Stage 5 ML confidence, ML Signal score-cache generation, calibration metadata, policy sweeps, score-scope policies, and frontend evidence summaries. The hard question is whether the current evidence model is statistically sufficient and how to improve it without creating false confidence, leakage, or overfit model-selection behavior.

Design a validation evidence theory for optimization, meta-selection, and ML Signal.

Specific questions:

1. Which metrics should be exact, approximate, confidence-bounded, or warning-only?
2. How should PSR, DSR, PBO/CSCV, walk-forward evidence, holdout pass rate, tail risk, and sample-size/coverage flags be combined?
3. What tolerance model is appropriate for CPU/GPU metric parity?
4. Can ML Signal use calibration, conformal abstention, or uncertainty intervals safely for time-series data?
5. What leakage controls are mandatory for Stage 5, meta-selection, ML confidence, source exclusion, grouped CV, and campaign outputs?
6. What evidence should be surfaced to users versus kept as artifact metadata?
7. What current confidence or score fields should be demoted, renamed, or gated?
8. Which validation evidence gaps are backend-only versus report/export/frontend-summary gaps?
9. What invariants bind dataset export manifest, split manifest, feature schema, score-cache manifest, model bundle manifest, and saved artifact replay?
10. What should hard fail versus warn for score CSV coverage, duplicate timestamps, missing calibrated probabilities, and schema-version drift?
11. What evidence is required before a native CPU CatBoost bundle or app-level fallback DLL is release-worthy?
12. How should Stage 5 compatibility names, Stage 4 aliases, saved policy-eval artifacts, and user-facing Meta copy stay coherent?
```

### Required File Pack

Validation/statistics:

- `crates/app_core/src/validation_stats.rs`
- `crates/app_core/src/stats_common.rs`
- `crates/app_core/src/sharpe_stats.rs`
- `crates/app_core/src/iterative_sim.rs`
- `crates/app_core/src/validation_shared.rs`
- `crates/app_core/src/cscv_pbo.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/validation_core.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/composite_validate.rs`
- `src-tauri/src/commands_validation.rs`
- `src-tauri/src/validation_common.rs`
- `src-tauri/src/metric_surface.rs`
- `src-tauri/src/commands_reports_shared.rs`
- `src-tauri/tests/validation_xlsx_report_tests.rs`
- `web/src/services/validationRequestBuilder.ts`
- `web/src/services/validationService.ts`

Meta and ML:

- `crates/app_core/src/meta_select.rs`
- `crates/app_core/src/ml_confidence.rs`
- `crates/app_core/src/ml_signal.rs`
- `src-tauri/src/commands_meta.rs`
- `src-tauri/src/commands_ml_signal.rs`
- `web/src/services/metaRequestBuilder.ts`
- `web/src/services/metaStage4RequestBuilder.ts`
- `web/src/services/metaMlScopeBuilder.ts`
- `web/src/services/metaService.ts`
- `web/src/services/mlSignalService.ts`
- `web/src/stores/optimizeStore.ts`
- `web/src/stores/analysisStore.ts`
- `web/src/components/MetaWorkspacePanel.vue`
- `web/src/services/metaBatchRequestBuilder.ts`
- `crates/app_core/src/bin/ta_ml_signal_export_dataset.rs`
- `crates/app_core/src/bin/ta_ml_signal_native_smoke.rs`
- `scripts/train_ml_signal_catboost.py`
- `scripts/run_ml_signal_quality_gate.ps1`
- `scripts/run_ml_signal_native_smoke.ps1`
- `scripts/build_catboost_native_windows.ps1`
- `configs/ml_signal_catboost_default.json`
- `docs/catboost_ml_signal_improvement_plan_2026-04-24.md`

Tests and benches:

- `crates/app_core/benches/meta_select_workflow.rs`
- `crates/app_core/benches/ml_signal_workflow.rs`
- `crates/app_core/benches/ml_signal_validation.rs`
- `crates/app_core/benches/e2e_workflow_ml.rs`
- `web/src/services/metaRequestBuilder.test.ts`
- `web/src/services/metaStage4RequestBuilder.test.ts`
- `web/src/services/metaMlScopeBuilder.test.ts`
- `web/src/services/metaBatchRequestBuilder.test.ts`
- `web/src/services/metaServiceCampaignStrategy.test.ts`
- `web/src/services/metaServiceCampaignResume.test.ts`
- `web/src/services/mlSignalService.test.ts`

### Optional File Pack

Use when the question includes options validation:

- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_types.rs`
- `src-tauri/src/commands_options_expression.rs`
- `web/src/stores/optionsExpressionStore.ts`

### External Context To Include

- Probability of Backtest Overfitting: https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2326253
- Deflated Sharpe Ratio: https://www.davidhbailey.com/dhbpapers/deflated-sharpe.pdf
- Adaptive conformal prediction for time series: https://proceedings.mlr.press/v162/zaffran22a.html
- Probability calibration and walk-forward ML validation references

### Required Output Emphasis

- validation statistic policy
- leakage-control invariants
- ML Signal calibration/conformal verdict
- artifact metadata and frontend evidence requirements
- minimum safe implementation slices
- terms or metrics to kill, rename, or gate

### Local Verification Authority

- `cargo test -p ta_app_core meta_select`
- `cargo test -p ta_app_core ml_confidence`
- `cargo test -p ta_app_core ml_signal`
- `cargo test -p ta_app_core validation`
- `npm --prefix web run test -- src/services/metaService.test.ts src/services/mlSignalService.test.ts`
- `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate`
- `powershell -File scripts/run_ml_signal_quality_gate.ps1`
- `powershell -File scripts/run_ml_signal_native_smoke.ps1`

---

## Dossier 5: Tensor-Core Workload Discovery

### Prompt

```md
Apply the global prompt contract.

Problem:

VectorGrid is GPU-focused, but most strategy backtests are branch-heavy and may not map to tensor cores. The hard question is whether the application contains, or should add, any dense or block-structured workload large enough and numerically tolerant enough to justify tensor-core, CUTLASS, cuBLAS, or mixed-precision work.

Do a prove-or-kill tensor-core workload discovery.

Specific questions:

1. Which existing computations are dense matrix-shaped or can be made block-structured without corrupting semantics?
2. Are ML Signal inference, feature transforms, covariance/correlation, cross-asset validation, Monte Carlo, policy sweeps, clustering, or similarity workloads plausible?
3. What are the arithmetic intensity, memory movement, batch size, precision, and parity requirements for each plausible workload?
4. Would cuBLAS, CUTLASS, ONNX Runtime, CatBoost-native paths, or custom CUDA be the right tool?
5. What exact spike would prove value?
6. What evidence should kill tensor-core work for now?
```

### Required File Pack

ML/meta and dense-candidate surfaces:

- `crates/app_core/src/ml_signal.rs`
- `crates/app_core/src/ml_confidence.rs`
- `crates/app_core/src/meta_select.rs`
- `crates/app_core/src/composite_mo.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/options_expression_gpu.rs`
- `crates/app_core/src/options_expression.rs`

Runtime and build context:

- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/Cargo.toml`
- `src-tauri/Cargo.toml`
- `docs/RELEASE.md`

Benchmarks:

- `crates/app_core/benches/ml_signal_workflow.rs`
- `crates/app_core/benches/ml_signal_validation.rs`
- `crates/app_core/benches/meta_select_workflow.rs`
- `crates/app_core/benches/composite_mo_opt.rs`

### Optional File Pack

Use if options Monte Carlo or advanced options payoff modeling becomes central:

- `docs/options_research/options_expression_future_scope_roadmap_2026-04-07.md`
- `crates/app_core/src/options_types.rs`
- `crates/app_core/src/kernels/options_expression.cu`

### External Context To Include

- NVIDIA CUDA Programming Guide: https://docs.nvidia.com/cuda/cuda-programming-guide/
- NVIDIA CUTLASS overview: https://docs.nvidia.com/cutlass/latest/overview.html
- NVIDIA Monte Carlo option-pricing material: https://developer.download.nvidia.com/assets/cuda/files/MonteCarlo.pdf
- mixed-precision numerical-error references relevant to financial metrics

### Required Output Emphasis

- workload inventory table
- arithmetic-intensity and data-movement estimate
- precision/tolerance model
- build-vs-buy recommendation
- one minimal spike or kill note

### Local Verification Authority

- microbenchmark only after a specific workload is proven
- `cargo test -p ta_app_core`
- relevant ML/meta/options benches
- `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate` only if a GPU path is prototyped

---

## Dossier 6: OptiX And RT-Core Prove-Or-Kill

### Prompt

```md
Apply the global prompt contract.

Problem:

RT cores and OptiX are attractive experimentally, but they are designed around ray traversal and acceleration structures, not general CUDA computation. The hard question is whether any VectorGrid problem naturally maps to ray/BVH traversal, interval intersection, path hit testing, or range queries strongly enough to justify further work.

Do a prove-or-kill RT-core/OptiX feasibility analysis.

Specific questions:

1. Is there any natural ray/BVH formulation for barrier hits, event-path thresholds, interval intersection, path-dependent options, or multidimensional candidate lookup?
2. Would OptiX acceleration structures beat simpler CUDA kernels, CPU interval indexes, or preprocessing?
3. What data layout and build/update cost would an OptiX approach require?
4. Does the OptiX programming model fit a Rust/Tauri/CUDA desktop application without unacceptable packaging risk?
5. What exact evidence would justify a spike?
6. What exact evidence kills the idea?
```

### Required File Pack

Potential mapping surfaces:

- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_expression_gpu.rs`
- `crates/app_core/src/kernels/options_expression.cu`
- `crates/app_core/src/validation_core.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`

Build/release context:

- `crates/app_core/Cargo.toml`
- `src-tauri/Cargo.toml`
- `docs/RELEASE.md`
- `scripts/run_gpu_perf_gate.ps1`
- `scripts/run_options_expression_gpu_promotion_gate.ps1`

### Optional File Pack

Use if the prompt explores options path replay or event inspection:

- `web/src/components/OptionsExpressionPanel.vue`
- `web/src/stores/optionsExpressionStore.ts`
- `src-tauri/src/commands_options_expression.rs`

### External Context To Include

- NVIDIA OptiX developer page: https://developer.nvidia.com/rtx/ray-tracing/optix
- NVIDIA CUDA Programming Guide: https://docs.nvidia.com/cuda/cuda-programming-guide/
- GPGPU ray traversal or BVH interval-query references if available

### Required Output Emphasis

- build/spike/defer/kill verdict
- mapping table for each candidate VectorGrid computation
- packaging and maintenance risk
- simpler alternative for every plausible mapping
- kill note if no natural RT-core workload exists

### Local Verification Authority

- no implementation gate until a specific spike is accepted
- if spiked: CUDA/OptiX build proof, CPU parity tests, and GPU perf/profile evidence

---

## Dossier 7: Live Execution Safety State Machine

### Prompt

```md
Apply the global prompt contract.

Problem:

VectorGrid can bridge research outputs to live or paper broker actions. Current live execution supports limited strategy coverage, Alpaca over REST, dry-run/paper behavior, one active live stream, one active runner, risk-policy persistence, broker reconciliation, and event-based runner status. The hard question is what formal state machine and invariants are required before expanding live execution or using richer research evidence for broker actions.

Design the live execution safety model.

Specific questions:

1. What states and transitions should govern stream readiness, runner readiness, order intent, broker submission, partial fill, rejection, cancellation, reconciliation, halt, and recovery?
2. What invariants must hold before a strategy artifact can trigger an order?
3. How should delayed data, stream interruption, duplicate/out-of-order bars, stale broker snapshots, partial fills, and unresolved intents be handled?
4. How should validation evidence and risk policy map to live execution limits?
5. What fail-closed behavior is mandatory?
6. What is required before adding execution support for more strategies?
7. What broker-review freshness and optimistic-local-order reconciliation is required before re-arming broker actions?
8. What quantity normalization, min-size rejection, bracket attachment, and local-paper persistence rules are part of the state machine?
```

### Required File Pack

Backend execution:

- `src-tauri/src/commands_execution.rs`
- `src-tauri/src/execution_policy.rs`
- `src-tauri/src/execution_quantity.rs`
- `src-tauri/src/broker_paper.rs`
- `src-tauri/src/execution_mocks.rs`
- `src-tauri/src/state.rs`
- `src-tauri/src/settings.rs`
- `src-tauri/src/commands_data.rs`
- `src-tauri/src/polygon.rs`

Frontend execution:

- `web/src/services/executionService.ts`
- `web/src/services/executionChartService.ts`
- `web/src/utils/executionRisk.ts`
- `web/src/stores/executionStore.ts`
- `web/src/stores/taskStore.ts`
- `web/src/tauri/events.ts`
- `web/src/components/ExecutionWorkspacePanel.vue`

Artifacts/history:

- `web/src/services/historyService.ts`
- `web/src/stores/workflowArtifactsStore.ts`
- `src-tauri/src/run_artifacts.rs`

Verification:

- `scripts/run_execution_safety_gate.ps1`
- `scripts/run_live_execution_safety_harness.ps1`
- `web/e2e/browser/execution-review-surface.spec.ts`
- execution service/store tests listed in `docs/REPO_MAP.md`

### Optional File Pack

Use if new strategy execution adapters are part of the question:

- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/composite_cpu.rs`
- `web/src/utils/strategyOptimizationCapabilities.ts`
- `src-tauri/src/commands_analysis.rs`

### External Context To Include

- Alpaca order documentation: https://docs.alpaca.markets/docs/trading/orders/
- NIST AI Risk Management Framework: https://www.nist.gov/itl/ai-risk-management-framework
- event-sourced order lifecycle and idempotency references

### Required Output Emphasis

- formal state machine
- invariants and fail-closed rules
- strategy expansion admission criteria
- broker reconciliation rules
- frontend evidence/readiness requirements
- safety gate additions

### Local Verification Authority

- `powershell -File scripts/run_execution_safety_gate.ps1`
- targeted Rust execution tests
- frontend execution service/store tests
- broker mock partial-fill/reject/timeout/cancel-failure tests

---

## Dossier 8: Artifact, Replay, Report, And Evidence UX Contract

### Prompt

```md
Apply the global prompt contract.

Problem:

VectorGrid persists many artifact families: optimization runs, validation, meta, Stage 5, ML Signal, options expression, campaign, reports, and live-session evidence. These share storage and history surfaces but have different replay, provenance, reportability, and backward-compatibility requirements. The hard question is what common contract should exist without forcing every workflow into a generic artifact model.

Design the artifact/replay/report evidence contract.

Specific questions:

1. What minimum common provenance schema should all artifact families share?
2. Which fields must remain workflow-specific?
3. Which replay behaviors should be exact, compatible, replacement-scoped, or blocked?
4. What output parity guarantees are needed for JSON, CSV, and XLSX reports?
5. How should frontend evidence summaries expose route fallback, score scope, data provenance, validation freshness, and report status?
6. What schema versioning and migration policy is safest?
7. Which report save/open/reveal operations mutate artifact evidence, and which are shell conveniences?
8. How should report-path failures and artifact-update warnings be represented in history and replay?
9. Which app-local roots are part of provenance: `runs`, `reports`, `data_cache`, Polygon cache, and options datasets?
```

### Required File Pack

Backend artifacts and reports:

- `crates/app_core/src/csv_compat.rs`
- `src-tauri/src/run_artifacts.rs`
- `src-tauri/src/commands_artifacts.rs`
- `src-tauri/src/commands_reports_api.rs`
- `src-tauri/src/commands_reports_shared.rs`
- `src-tauri/src/commands_reports_validation_common.rs`
- `src-tauri/src/commands_reports_validation_xlsx.rs`
- `src-tauri/src/commands_reports_csv.rs`
- `src-tauri/src/commands_reports_meta_xlsx.rs`
- `src-tauri/src/commands_reports.rs`
- `src-tauri/src/bin/ta_headless_validate.rs`

Frontend artifacts/history:

- `web/src/stores/workflowArtifactsStore.ts`
- `web/src/services/workflowArtifactsSync.ts`
- `web/src/services/historyService.ts`
- `web/src/stores/historyStore.ts`
- `web/src/services/validationService.ts`
- `web/src/services/metaService.ts`
- `web/src/services/mlSignalService.ts`
- `web/src/services/optionsExpressionService.ts`
- `web/src/utils/core/artifactIdentity.ts`

Verification:

- `scripts/run_output_parity_check.mjs`
- `scripts/check_output_parity.mjs`
- `scripts/check_output_semantic_parity.mjs`
- `scripts/output_parity_pairs.json`
- `scripts/headless_output_parity_pack.json`
- `scripts/output_parity_semantic_pairs.json`
- `src-tauri/tests/validation_xlsx_report_tests.rs`
- `web/src/services/validationService.test.ts`
- `web/src/stores/workflowArtifactsStore.test.ts`
- `web/src/services/historyService.test.ts`

### Optional File Pack

Use if campaign or live-session artifacts are central:

- `src-tauri/src/commands_campaign.rs`
- `src-tauri/src/commands_execution.rs`
- `web/src/services/executionService.ts`
- `web/src/services/metaServiceCampaignResume.test.ts`

### External Context To Include

- reproducible research artifact and provenance schema references
- audit-trail design references
- spreadsheet/report validation references

### Required Output Emphasis

- common schema proposal
- workflow-specific carve-outs
- replay compatibility matrix
- output parity authority
- frontend evidence summary requirements
- migration/backward-compatibility strategy

### Local Verification Authority

- `node scripts/run_output_parity_check.mjs`
- `node scripts/check_output_parity.mjs`
- `node scripts/check_output_semantic_parity.mjs`
- Rust artifact/report tests
- `npm --prefix web run test -- src/stores/workflowArtifactsStore.test.ts src/services/historyService.test.ts`

---

## Dossier 9: Dynamic Strategy IR, Indicator Catalog, And Strategy Surface

### Prompt

```md
Apply the global prompt contract.

Problem:

VectorGrid has a broad strategy catalog, strategy-local CPU/GPU/hybrid routes, dynamic strategy IR, moving-average and indicator catalogs, candle-field gating, frontend capability summaries, and VectorTA CUDA dependency wiring. The hard question is whether the strategy/capability model can become more expressive and safer without creating a broad generic execution layer or frontend/backend drift.

Design the strategy capability and dynamic IR policy.

Specific questions:

1. Can the dynamic strategy IR safely express more strategy logic while preserving strategy-local ownership?
2. Which indicator/MA features imply CPU-only, GPU-only, GpuKernel, GpuHybridKernel, or unsupported routes?
3. Can route policy be inferred from IR features, or must it remain strategy-specific?
4. What capability schema would keep frontend request builders, capability text, and backend route decisions aligned?
5. How should VectorTA dependency and feature wiring constrain future catalog expansion?
6. What strategy families are mathematically poor fits for current GPU routes?
7. For each standalone profile, which CPU prep/readbacks are architecturally required versus accidental?
8. Which mixed validation routes, especially `Vcc`, `Carm`, and `Cdmc`, prevent fully GPU-resident validation?
9. How do `cross_ticker`, aux analytics, walk-forward, reopt, SPA, perturb, bootstrap, candidate detail, and local-stability support differ by family?
```

### Required File Pack

Dynamic strategy and IR:

- `crates/app_core/src/strategy_dynamic.rs`
- `crates/app_core/src/strategy_ir.rs`
- `crates/app_core/src/strategy_plan.rs`
- `crates/app_core/src/strategy_gpu_program.rs`
- `src-tauri/src/commands_dynamic.rs`

Indicators and MA catalog:

- `crates/app_core/src/indicator_catalog.rs`
- `crates/app_core/src/indicator_factory.rs`
- `crates/app_core/src/ma_policy.rs`
- `crates/app_core/src/double_ma_ma_kind.rs`
- `src-tauri/src/ma_registry.rs`
- `crates/app_core/src/dynamic_indicator_adapter.rs`
- `crates/app_core/src/vram_ma.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `web/src/services/movingAverageCatalogService.ts`
- `web/src/services/movingAverageCatalogService.test.ts`
- `web/src/services/dynamicIndicatorCatalogService.ts`
- `web/src/services/dynamicIndicatorCatalogService.test.ts`
- `web/src/strategies/dynamicDoubleMaBuilder.ts`
- `web/src/utils/core/candleFields.ts`
- `web/src/utils/strategyOptimizationCapabilities.ts`

Strategy route examples:

- `crates/app_core/src/composite_profile_route.rs`
- `crates/app_core/src/double_ma_backend.rs`
- `web/src/strategies/compositeProfileGpuPolicy.ts`
- representative strategy-local files from `docs/REPO_MAP.md`

### Optional File Pack

Use if exact strategy-family decisions are requested:

- `crates/app_core/src/asr.rs`
- `crates/app_core/src/asr_gpu_kernel.rs`
- `crates/app_core/src/vcc.rs`
- `crates/app_core/src/vcc_gpu_kernel.rs`
- `crates/app_core/src/vemr.rs`
- `crates/app_core/src/vemr_gpu_kernel.rs`
- `crates/app_core/src/rocb.rs`
- `crates/app_core/src/rocb_gpu_kernel.rs`
- `crates/app_core/src/vcet.rs`
- `crates/app_core/src/vcet_gpu_kernel.rs`
- `crates/app_core/src/mkcs.rs`
- `crates/app_core/src/mkcs_gpu_kernel.rs`
- `crates/app_core/src/kqsr.rs`
- `crates/app_core/src/kqsr_gpu_kernel.rs`
- `crates/app_core/src/carm.rs`
- `crates/app_core/src/carm_gpu_kernel.rs`
- `crates/app_core/src/kvdf.rs`
- `crates/app_core/src/kvdf_gpu_kernel.rs`
- `crates/app_core/src/cdmc.rs`
- `crates/app_core/src/cdmc_gpu_kernel.rs`
- `crates/app_core/src/composite_cpu.rs`
- `crates/app_core/src/composite_mo.rs`
- `crates/strategies/src/lib.rs`
- `crates/strategies/src/double_ma/mod.rs`
- `crates/strategies/src/composite_cpu/mod.rs`

Use if candidate detail, local stability, or analysis replay is central:

- `web/src/services/analysisService.ts`
- `web/src/services/analysisService.test.ts`
- `web/src/stores/analysisStore.ts`
- `src-tauri/src/commands_analysis.rs`
- `web/src/components/TopWorkspacePanel.test.ts`
- `web/src/components/CandidateDetailWorkspacePanel.test.ts`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/composite_cpu.rs`

### External Context To Include

- GPU expression compilation and numeric IR references
- numerical stability references for recursive/adaptive indicators
- CUDA and VectorTA-related documentation if changing feature wiring

### Required Output Emphasis

- capability schema proposal
- route-policy proof obligations
- IR expansion admission rules
- frontend/backend drift prevention
- strategy families to avoid or keep strategy-local

### Local Verification Authority

- `cargo test -p ta_app_core`
- `npm --prefix web run test -- movingAverageCatalogService dynamicIndicatorCatalogService strategyOptimizationCapabilities`
- `npm --prefix web run parity`
- GPU perf gates only for accepted GPU-route changes

---

## Dossier 10: Frontend Workflow Orchestration And Evidence-To-Decision UX

### Prompt

```md
Apply the global prompt contract.

Problem:

VectorGrid exposes complex research workflows through a desktop frontend: data loading, optimization, validation, meta, ML Signal, options expression, live execution, saved artifacts, command palette actions, task center, and long-running runs. The hard question is which frontend state model and evidence presentation protect scientific correctness and user safety without becoming a noisy generic workflow engine.

Design the frontend orchestration and evidence-to-decision model.

Specific questions:

1. Which workflow transitions should be formalized, blocked, warned, or left flexible?
2. How should route fallback, score scope, validation freshness, quote/data provenance, ML confidence, and live readiness be surfaced?
3. What should be visible in normal workflow panels versus inspector/history/detail views?
4. How should long-running task progress, cancellation, resumability, and artifact hydration be modeled?
5. Which UI decisions materially affect scientific correctness or user risk?
6. What tests should become authority for workflow behavior?
7. How do command palette, inspector, action bar, native menu bridge, and global shortcuts stay synchronized?
8. What is the authority order between `workflowModel.ts`, `WorkbenchShell.vue`, `workflowStore.ts`, and `workflowBootstrap.ts` when restored state disagrees with visible UI?
9. How should Tauri invoke/listen failures, preflight state, runtime banners, task center state, footer state, and OS notifications compose into one user-facing task model?
10. Which persisted localStorage migrations or stale shell states must be blocked versus repaired?
```

### Required File Pack

Shell and workflow:

- `web/src/App.vue`
- `web/src/components/workbench/WorkbenchShell.vue`
- `web/src/components/workbench/WorkbenchShell.test.ts`
- `web/src/components/workbench/WorkbenchInspector.vue`
- `web/src/components/workbench/WorkbenchInspector.test.ts`
- `web/src/components/workbench/WorkbenchRail.vue`
- `web/src/components/workbench/ResearchCockpit.vue`
- `web/src/components/workbench/WorkbenchActionBar.vue`
- `web/src/components/GlobalShortcutBridge.vue`
- `web/src/components/GlobalShortcutBridge.test.ts`
- `web/src/components/AppRuntimeBanner.vue`
- `web/src/components/AppRuntimeBanner.test.ts`
- `web/src/components/CommandPaletteOverlay.vue`
- `web/src/components/CommandPaletteOverlay.test.ts`
- `web/src/components/TaskCenterOverlay.vue`
- `web/src/components/TaskCenterOverlay.test.ts`
- `web/src/components/RunFooterPanel.vue`
- `web/src/components/RunFooterPanel.test.ts`
- `web/src/components/SettingsWorkspacePanel.vue`
- `web/src/components/SettingsWorkspacePanel.test.ts`
- `web/src/components/WorkflowQuickSwitch.vue`
- `web/src/components/WorkflowQuickSwitch.test.ts`
- `web/src/utils/core/workflowModel.ts`
- `web/src/utils/workbenchActionModel.ts`
- `web/src/utils/workbenchNavigationModel.ts`
- `web/src/stores/workflowStore.ts`
- `web/src/stores/taskStore.ts`
- `web/src/stores/workflowArtifactsStore.ts`
- `web/src/stores/runtimeStore.ts`
- `web/src/stores/commandPaletteStore.ts`
- `web/src/stores/commandPaletteStore.test.ts`
- `web/src/stores/commandActionStore.ts`
- `web/src/stores/commandActionStore.test.ts`
- `web/src/stores/settingsStore.ts`
- `web/src/stores/settingsStore.actions.test.ts`
- `web/src/preflight.ts`
- `web/src/preflight.test.ts`
- `web/src/tauri/commands.ts`
- `web/src/tauri/commands.test.ts`
- `web/src/tauri/invoke.ts`
- `web/src/tauri/invoke.contract.test.ts`
- `web/src/tauri/events.ts`
- `web/src/tauri/events.test.ts`
- `web/src/tauri/instrumentation.ts`
- `web/src/tauri/instrumentation.test.ts`
- `web/src/tauri/runtimeError.ts`
- `web/src/tauri/runtimeError.test.ts`
- `web/src/tauri/runtimeWiringPolicy.test.ts`

Services:

- `web/src/services/optimizeService.ts`
- `web/src/services/validationService.ts`
- `web/src/services/metaService.ts`
- `web/src/services/mlSignalService.ts`
- `web/src/services/optionsExpressionService.ts`
- `web/src/services/executionService.ts`
- `web/src/services/historyService.ts`
- `web/src/services/workflowArtifactsSync.ts`
- `web/src/services/desktopProductization.ts`
- `web/src/services/runControlService.ts`
- `web/src/services/runControlService.test.ts`
- `web/src/services/settingsBootstrap.ts`
- `web/src/services/settingsBootstrap.test.ts`
- `web/src/services/settingsService.ts`
- `web/src/services/settingsService.test.ts`
- `web/src/utils/storage.ts`
- `web/src/utils/storage.test.ts`
- `web/src/utils/core/storageKeys.ts`
- `web/src/utils/shellShortcutRegistry.ts`
- `web/src/utils/statusPresentation.ts`
- `web/src/utils/desktopNotificationPolicy.ts`
- `web/src/utils/inspectorModel.ts`
- `web/src/composables/useResearchCockpitModel.ts`
- `web/src/composables/useTaskLifecycleSummary.ts`

Verification:

- `web/e2e/browser/qa/workflowMatrix.spec.ts`
- `web/e2e/browser/qa/actionCoverageManifest.ts`
- `web/e2e/accessibility-review-matrix.md`
- `scripts/run_frontend_e2e_gate.ps1`

### Optional File Pack

Use if desktop chrome or startup is part of the question:

- `web/src/components/DesktopTitlebar.vue`
- `web/src/components/CommandPaletteOverlay.vue`
- `web/src/components/TaskCenterOverlay.vue`
- `web/src/components/RunFooterPanel.vue`
- `web/src/main.ts`
- `web/src/services/settingsBootstrap.ts`
- `web/src/services/workflowBootstrap.ts`

### External Context To Include

- human-in-the-loop model selection references
- uncertainty visualization references
- provenance UX references
- long-running desktop task UX references
- NIST AI RMF if live execution or consequential decisions are included: https://www.nist.gov/itl/ai-risk-management-framework

### Required Output Emphasis

- workflow state model
- evidence visibility matrix
- invalid transition policy
- user-risk and scientific-correctness UI requirements
- test authority map
- changes to avoid because they add noise without improving decisions

### Local Verification Authority

- `npm --prefix web run test`
- `npm --prefix web run typecheck`
- `npm --prefix web run lint`
- `npm --prefix web run test:e2e:workflow`
- `npm --prefix web run a11y:smoke`
- `powershell -File scripts/run_frontend_e2e_gate.ps1`

---

## Dossier 11: Campaign, Batch, And Resumable Research Workflows

### Prompt

```md
Apply the global prompt contract.

Problem:

VectorGrid supports campaign-style research workflows across datasets, execution modes, preload policy, validation payloads, auto-meta, auto-Stage-5, progress events, saved summaries, retry, and resume semantics. The hard question is what deterministic manifest and replay model should govern resumable research so partial results are useful but not misleading.

Design the campaign and resumable research model.

Specific questions:

1. What must a deterministic campaign manifest contain?
2. Which failures are resumable, restart-only, stale, or unsafe to continue?
3. How should dataset identity, validation payload, preload policy, auto-meta, auto-Stage-5, and saved summary artifacts interact?
4. How should progress events and partial summaries avoid false completion or stale evidence?
5. How should campaign outputs feed meta-selection and ML confidence without leakage?
6. What is the smallest implementation slice to improve reliability?
7. How should campaign contracts remain compatible with accepted strategy families when `DynamicDoubleMa` and `MlSignal` are intentionally rejected or unsupported in campaign mode?
```

### Required File Pack

Campaign orchestration:

- `web/src/services/metaService.ts`
- `web/src/services/metaBatchRequestBuilder.ts`
- `web/src/components/MetaWorkspacePanel.vue`
- `web/src/stores/analysisStore.ts`
- `src-tauri/src/commands_campaign.rs`
- `src-tauri/src/run_artifacts.rs`
- `web/src/stores/taskStore.ts`
- `web/src/stores/workflowArtifactsStore.ts`
- `web/src/tauri/events.ts`
- `web/src/tauri/instrumentation.ts`

Tests:

- `web/src/services/metaServiceCampaignResume.test.ts`
- `web/src/services/metaServiceCampaignStrategy.test.ts`
- `web/src/services/metaBatchRequestBuilder.test.ts`
- `web/src/stores/taskStoreCampaignProgress.test.ts`
- `web/src/stores/taskStoreEventThrottle.test.ts`
- relevant campaign tests under `src-tauri` if present

Related meta/validation:

- `web/src/services/metaRequestBuilder.ts`
- `web/src/services/metaStage4RequestBuilder.ts`
- `web/src/services/metaMlScopeBuilder.ts`
- `src-tauri/src/commands_meta.rs`
- `crates/app_core/src/meta_select.rs`
- `crates/app_core/src/ml_confidence.rs`

### Optional File Pack

Use if dataset preloading or cross-ticker validation is central:

- `src-tauri/src/commands_data.rs`
- `src-tauri/src/data_cache.rs`
- `src-tauri/src/state.rs`
- `src-tauri/src/commands_validation.rs`
- `src-tauri/src/validation_common.rs`
- `src-tauri/src/bin/ta_campaign_perf.rs`
- `crates/app_core/src/bin/profile_doublema_campaign_prefetch.rs`
- `scripts/workflow_benchmark_pack.json`

### External Context To Include

- deterministic job manifest references
- resumable batch workflow design references
- provenance-aware retry and auditability references
- PBO/DSR references if campaign results drive model selection

### Required Output Emphasis

- campaign manifest schema
- retry/resume state machine
- stale evidence detection
- leakage controls
- progress/event semantics
- verification plan

### Local Verification Authority

- campaign frontend tests
- Rust campaign tests
- `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate`
- full browser workflow E2E for UI-facing changes

---

## Dossier 12: Verification, Benchmark Fidelity, Release, And Packaging Authority

### Prompt

```md
Apply the global prompt contract.

Problem:

VectorGrid has many tests, gates, profiling scripts, baselines, headless binaries, release paths, CUDA feature flags, PTX/prebuilt-kernel packaging, frontend E2E lanes, output parity checks, and disabled CI surfaces. The hard question is which evidence is authoritative for each hard-problem candidate and whether existing gates measure the claims future GPT-5.5 Pro recommendations will make.

Design the verification and release authority model for hard-problem implementation.

Specific questions:

1. Which gates are authoritative for each subsystem and candidate dossier?
2. Which current benchmarks measure product-relevant latency versus microbenchmark-only behavior?
3. What timing boundaries are missing for CUDA Graphs, GPU residency, options scoring, validation, ML Signal, frontend workflow, and live execution?
4. How should benchmark noise and baseline drift be handled?
5. What CPU-only, CUDA, CUDA kernel, PTX, Windows packaging, and release constraints can invalidate implementation plans?
6. What evidence should be required before an accepted GPT-5.5 Pro recommendation moves from research to implementation?
7. What is the authority order among release doctor, release candidate gate, readiness JSON, provenance JSON/TXT, release manifest, `SHA256SUMS`, and packaged smoke summary?
8. Which release variants require CPU-only, `cuda`, `cuda-backtest-kernel`, prebuilt PTX, packaged smoke, and native CatBoost DLL evidence?
9. What evidence should force hotfix, withdrawal, or rollback of a released build?
```

### Required File Pack

Verification scripts:

- `scripts/run_repo_quality_gate.ps1`
- `scripts/run_workspace_tests_stable.ps1`
- `scripts/run_workflow_gate.ps1`
- `scripts/run_cpu_perf_gate.ps1`
- `scripts/run_gpu_perf_gate.ps1`
- `scripts/run_gpu_validation_perf_gate.ps1`
- `scripts/run_gpu_followup_perf_gate.ps1`
- `scripts/run_gpu_alma_contract_rows_perf_gate.ps1`
- `scripts/run_options_expression_gpu_promotion_gate.ps1`
- `scripts/run_options_expression_phase05.ps1`
- `scripts/run_ml_signal_quality_gate.ps1`
- `scripts/run_ml_signal_native_smoke.ps1`
- `scripts/run_execution_safety_gate.ps1`
- `scripts/run_frontend_e2e_gate.ps1`
- `scripts/run_release_doctor.ps1`
- `scripts/run_release_candidate_gate.ps1`
- `scripts/generate_release_manifest.ps1`
- `scripts/capture_release_provenance.ps1`
- `scripts/run_packaged_smoke.ps1`
- `scripts/check_output_parity.mjs`
- `scripts/check_output_semantic_parity.mjs`
- `scripts/run_output_parity_check.mjs`
- `scripts/refresh_output_parity_baseline.mjs`
- `scripts/export_baseline_samples.mjs`
- `scripts/check_migration_freeze_scope.mjs`
- `scripts/baselines/workflow_regression_baseline.json`
- `scripts/baselines/cpu_perf_baseline.json`
- `scripts/baselines/gpu_perf_baseline.json`
- `scripts/baselines/options_expression_perf_baseline.json`
- `scripts/baselines/gpu_followup_perf_baseline.json`
- `scripts/baselines/gpu_alma_contract_rows/`
- `scripts/baselines/gpu_validation_perf_baseline.json`

Benchmarks and headless tools:

- `crates/app_core/benches/double_ma_opt.rs`
- `crates/app_core/benches/double_ma_followup_workflow.rs`
- `crates/app_core/benches/double_ma_metrics.rs`
- `crates/app_core/benches/double_ma_validation_workflow.rs`
- `crates/app_core/benches/composite_cpu_opt.rs`
- `crates/app_core/benches/composite_validation_workflow.rs`
- `crates/app_core/benches/composite_mo_opt.rs`
- `crates/app_core/benches/gpu_native_expansion_opt.rs`
- `crates/app_core/benches/asr_opt.rs`
- `crates/app_core/benches/vcc_opt.rs`
- `crates/app_core/benches/vemr_opt.rs`
- `crates/app_core/benches/vcet_opt.rs`
- `crates/app_core/benches/rocb_opt.rs`
- `crates/app_core/benches/mkcs_opt.rs`
- `crates/app_core/benches/dynamic_strategy_cpu.rs`
- `crates/app_core/benches/meta_select_workflow.rs`
- `crates/app_core/benches/ml_signal_workflow.rs`
- `crates/app_core/benches/ml_signal_validation.rs`
- `crates/app_core/benches/e2e_workflow_ml.rs`
- `crates/app_core/benches/options_expression_candidate_compile.rs`
- `crates/app_core/benches/options_expression_summary_compile.rs`
- `crates/app_core/benches/options_expression_cpu_score.rs`
- `crates/app_core/benches/options_expression_gpu_score.rs`
- `crates/app_core/benches/options_expression_bench_support.rs`
- `src-tauri/src/bin/ta_workflow_regression.rs`
- `src-tauri/src/bin/ta_headless_validate.rs`
- `src-tauri/src/bin/ta_profile_route_matrix.rs`
- `src-tauri/src/bin/ta_double_ma_validation_cost.rs`
- `src-tauri/src/bin/ta_app_path_faceoff.rs`
- `src-tauri/src/bin/ta_campaign_perf.rs`
- `src-tauri/src/bin/ta_release_readiness.rs`
- `src-tauri/src/bin/ta_refactor_snapshots.rs`
- `src-tauri/src/bin/ta_options_expression_compare.rs`
- `src-tauri/src/bin/ta_options_expression_smoke.rs`
- `src-tauri/src/bin/ta_options_parent_artifact.rs`
- `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs`
- `crates/app_core/src/bin/profile_doublema_campaign_prefetch.rs`
- `crates/app_core/src/bin/cupti_sass_probe.rs`
- `crates/app_core/src/bin/stage6b_kernel_report.rs`
- `crates/app_core/src/bin/real_data_fidelity_audit.rs`
- `crates/app_core/src/bin/real_data_optimizer_eval.rs`
- `crates/app_core/src/bin/ta_cli.rs`
- `crates/app_core/src/bin/ta_ml_signal_export_dataset.rs`
- `crates/app_core/src/bin/ta_ml_signal_native_smoke.rs`

Release and packaging:

- `docs/RELEASE.md`
- `src-tauri/tauri.conf.json`
- `src-tauri/tauri.windows.conf.json`
- `tauri.conf.json`
- `tauri.windows.conf.json`
- `src-tauri/capabilities/main.json`
- `src-tauri/capabilities/dev.json`
- `src-tauri/Cargo.toml`
- `crates/app_core/Cargo.toml`
- `crates/app_core/build.rs`
- `crates/app_core/src/kernels/double_crossover.prebuilt.ptx`
- `crates/app_core/src/kernels/options_expression.prebuilt.ptx`
- `src-tauri/src/release_readiness.rs`
- `src-tauri/src/commands_system.rs`
- `src-tauri/src/desktop_menu.rs`
- `src-tauri/src/main.rs`
- `web/scripts/check-tauri-capabilities.mjs`

Frontend QA:

- `web/package.json`
- `web/scripts/check-a11y-smoke.mjs`
- `web/scripts/check-action-coverage-manifest.ts`
- `web/scripts/check-qa-selector-inventory.mjs`
- `web/scripts/check-e2e-artifact-fixtures.mjs`
- `web/scripts/check-ui-perf-budget.mjs`
- `web/scripts/capture-baseline-ui.mjs`
- `web/scripts/check-bundle-budget.mjs`
- `web/e2e/browser/README.md`
- `web/e2e/accessibility-review-matrix.md`
- `web/e2e/browser/qa/workbenchShell.spec.ts`
- `web/e2e/browser/qa/actionCoverage.spec.ts`
- `web/e2e/browser/qa/responsiveWorkflow.spec.ts`
- `web/e2e/browser/qa/layoutRegression.spec.ts`
- `web/e2e/browser/qa/regressionRepros.spec.ts`
- `web/e2e/browser/qa/visualBaselines.spec.ts`
- `web/e2e/browser/qa/exploratorySweep.spec.ts`
- `web/e2e/browser/qa/workflowMatrix.spec.ts`
- `web/e2e/browser/qa/actionCoverageManifest.ts`
- `web/wdio.desktop.conf.ts`
- `web/e2e/desktop/README.md`
- `web/e2e/desktop/specs/*.spec.ts`

### Optional File Pack

Use if the dossier is tied to GitHub Actions or release CI reactivation:

- `.github/workflows.disabled/ci.yml.disabled`
- `.github/workflows.disabled/desktop-smoke.yml.disabled`

### External Context To Include

- NVIDIA Nsight profiling methodology
- CUDA/PTX deployment and compatibility documentation
- benchmark noise/statistical methodology references
- Tauri build and capability documentation

### Required Output Emphasis

- verification authority table
- benchmark gap list
- source-of-truth baseline policy
- release/package constraints
- evidence threshold to promote recommendations to implementation
- gates to avoid using as authority

### Local Verification Authority

This dossier defines the authority model. It should not implement code directly.

Minimum local validation for changes to the authority model:

- `powershell -File scripts/run_repo_quality_gate.ps1`
- affected script dry runs where safe
- `powershell -File scripts/run_release_doctor.ps1`
- `powershell -File scripts/run_release_candidate_gate.ps1`
- `powershell -File scripts/run_packaged_smoke.ps1` only for package-facing changes
- no simultaneous benchmark runs

---

## Prompt Pack Source Links

CUDA and GPU:

- NVIDIA CUDA Programming Guide: https://docs.nvidia.com/cuda/cuda-programming-guide/
- NVIDIA CUTLASS overview: https://docs.nvidia.com/cutlass/latest/overview.html
- NVIDIA CUB documentation: https://docs.nvidia.com/cuda/cub/index.html
- CUB DeviceTopK API: https://nvidia.github.io/cccl/cub/api/structcub_1_1DeviceTopK.html
- NVIDIA OptiX developer page: https://developer.nvidia.com/rtx/ray-tracing/optix
- NVIDIA Monte Carlo option-pricing CUDA material: https://developer.download.nvidia.com/assets/cuda/files/MonteCarlo.pdf
- RadiK GPU radix top-k selection: https://arxiv.org/abs/2501.14336

Options and market data:

- Polygon/Massive Options REST API overview: https://www.polygon.io/docs/options
- Cboe strategy-based margin: https://www.cboe.com/en/markets/us/options/margin/strategy-based-margin/
- OCC options disclosure document page: https://www.theocc.com/company-information/documents-and-archives/options-disclosure-document

Validation and ML uncertainty:

- The Probability of Backtest Overfitting: https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2326253
- The Deflated Sharpe Ratio: https://www.davidhbailey.com/dhbpapers/deflated-sharpe.pdf
- Adaptive Conformal Predictions for Time Series: https://proceedings.mlr.press/v162/zaffran22a.html

Execution and governance:

- Alpaca placing orders documentation: https://docs.alpaca.markets/docs/trading/orders/
- NIST AI Risk Management Framework: https://www.nist.gov/itl/ai-risk-management-framework

## Local Triage Template After GPT-5.5 Pro Responds

Use this after each model response before implementation.

```md
# GPT-5.5 Pro response triage: <dossier>

## Verdict

- Accepted implementation plan:
- Research spike:
- Documentation-only decision:
- Rejected or killed:
- Needs another dossier:

## Repo Facts Used Correctly

## Repo Facts Missing Or Wrong

## External Source Claims To Recheck

## Implementation Ownership

## Required Tests And Gates

## Rollback Or Fallback

## Follow-Up File Changes
```
