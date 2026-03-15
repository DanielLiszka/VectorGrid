# GPT-5.4 feature review map

This file is a repo-grounded map of which files own which optimization and validation features, so each implementation can be presented to a reviewer model in isolation.

## Recommended review order

1. `double_ma` optimization backend
2. composite strategy route policy
3. validation stage core
4. cross-ticker validation orchestration
5. frontend request shaping
6. frontend optimization orchestration
7. presets and tests

## 1. Double MA optimization backend

### Primary files

- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/double_ma_backend.rs`
- `crates/app_core/src/double_ma_mode.rs`
- `crates/app_core/src/double_ma_result.rs`
- `crates/app_core/src/double_ma_strategy.rs`
- `src-tauri/src/commands_optimize.rs`

### What each file owns

- `crates/app_core/src/double_ma.rs`
  - Main optimization engine for the Double MA family.
  - Owns request types, backend enums, optimization execution, objective scoring, drilldown, local stability, GPU/CPU execution paths, regime filters, and VDPS/grid search logic.
  - Main entry points:
    - `run_double_ma_optimization_blocking_with_candles`
    - `compute_double_ma_drilldown_blocking_with_candles`
    - `compute_double_ma_local_stability_blocking_with_candles`
  - Strategy-specific logic is also concentrated here:
    - `compute_trend_quality_regime`
    - `compute_combined_regime_filter`
    - `compute_breakout_momentum_regime`
    - `compute_pressure_impulse_trend_regime`
    - `compute_slope_efficiency_trend_regime`
    - `compute_momentum_triple_stack_regime`
    - `compute_oscillator_reversion_confluence_regime`
    - `compute_volume_flow_confirmation_regime`

- `crates/app_core/src/double_ma_backend.rs`
  - Backend gating and route-summary helper layer.
  - Owns backend labels, hybrid enable toggle, GPU kernel blockers, and per-indicator route-plan summaries.
  - Review this when the question is "why did a request take this backend path or fall back?"

- `crates/app_core/src/double_ma_mode.rs`
  - Optimization mode selection and validation.
  - Owns the `Auto -> Grid/CoarseToFine` decision and validation for `CoarseToFine` and `GpuVdps`.

- `crates/app_core/src/double_ma_result.rs`
  - Converts raw optimizer outputs into the public `BackendOptimizationResult`.
  - Owns resolved-parameter mapping, matrix-index generation, top/best column metadata, and output assembly.

- `crates/app_core/src/double_ma_strategy.rs`
  - Validation layer for `StrategyConfig` within the Double MA flow.
  - Owns configuration correctness rules and incompatibility guards between regime profiles.
  - This is where many "allowed combination vs forbidden combination" questions should start.

- `src-tauri/src/commands_optimize.rs`
  - Tauri command boundary for optimization.
  - Owns run cancellation wiring, progress emission, candle lookup, async `spawn_blocking` handoff, and product-level normalization.
  - Important detail: it forces product optimization mode to `Grid`.

### Review focus

- CPU/GPU parity and fallback behavior
- objective scoring correctness, especially inverted objectives like volatility and max drawdown
- backend blocker logic and route summaries
- mode-selection correctness and hidden product constraints
- regime-profile interactions and configuration incompatibility rules
- drilldown/local-stability consistency with main optimization results

## 2. Composite strategy route policy

### Primary files

- `crates/app_core/src/composite_profile_route.rs`
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/utils/strategyOptimizationCapabilities.ts`

### What each file owns

- `crates/app_core/src/composite_profile_route.rs`
  - Maps `StrategyConfig` flags to runtime profile identity.
  - Decides whether a composite strategy targets `CpuOnly`, `GpuKernel`, or `GpuHybridKernel`.
  - Important mappings:
    - `trend_quality_enable` -> `CompositeTrend`
    - `breakout_momentum_enable` -> `BreakoutMomentum`
    - `regime_hybrid_enable` -> `RegimeHybrid`
  - ADX-enabled variants flip `CompositeTrend` and `BreakoutMomentum` from `GpuKernel` to `GpuHybridKernel`.

- `web/src/services/optimizeRequestBuilder.ts`
  - Frontend request builder for optimization payloads.
  - Owns strategy-kind-specific request normalization, backend payload shaping, MA support checks, range/sweep parsing, and user-facing guardrails before the request reaches Rust.
  - Main entry points:
    - `buildDoubleMaOptimizationRequest`
    - `buildCompositeCpuOptimizationRequest`
    - `buildCompositeMoOptimizationRequest`

- `web/src/utils/strategyOptimizationCapabilities.ts`
  - Frontend capability matrix.
  - Owns which global controls a given strategy kind exposes, such as backend choice, mode, top-k, heatmap, export behavior, and related affordances.
  - Main entry point: `describeStrategyOptimizationCapabilities`

### Review focus

- frontend/backend route-policy drift
- ADX branch consistency
- capability exposure matching actual backend support
- invalid combinations being blocked on the frontend before Tauri invocation

## 3. Double MA validation stage

### Primary files

- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `src-tauri/src/commands_validation.rs`

### What each file owns

- `crates/app_core/src/double_ma_validate.rs`
  - Main Double MA validation implementation.
  - Owns candidate scoring, walk-forward validation, perturbation stats, trade bootstrap, SPA/re-optimization branches, GPU fixed-candidate preparation, backend runtime selection, and final `DoubleMaValidationResult` assembly.
  - Main entry points:
    - `run_double_ma_validation_blocking_with_candles`
    - `try_prepare_gpu_fixed_candidate_validation`
    - `compute_cpu_perturb_stats_for_candidate`

- `crates/app_core/src/validation_pipeline.rs`
  - Shared validation reduction/math layer.
  - Owns hard-gate sanitation, probability metrics, Pareto marking, CSCV/PBO folding, final verdict construction, walk-forward helpers, perturbation/bootstrap helpers, and parallel candidate collection.
  - Key abstraction boundary: `ValidationPostCandidate`

- `src-tauri/src/commands_validation.rs`
  - Tauri orchestration layer for validation.
  - Owns command entry points, progress/cancel behavior, validation context checks, and cross-ticker orchestration.
  - Main entry points include:
    - `run_double_ma_validation`
    - `run_composite_validation`
    - `apply_cross_ticker_validation`
    - `apply_cross_ticker_validation_double_ma`
    - `apply_cross_ticker_validation_double_ma_headless`

### Review focus

- GPU validation support rules and fallback reasons
- candidate reduction correctness and gate precedence
- deterministic behavior in perturbation/bootstrap branches
- correctness of cross-ticker aggregation and finalist pruning
- cancellation and async boundary integrity

## 4. Cross-ticker validation orchestration

### Primary files

- `src-tauri/src/commands_validation.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `src-tauri/src/validation_common.rs`

### What this bundle is for

- Review this bundle separately when the question is specifically about multi-dataset validation behavior rather than single-dataset scoring.
- `commands_validation.rs` owns cross-ticker reruns, candidate-key grouping, overlap alignment, and final shortlist assembly.
- `validation_pipeline.rs` owns the reduction rules that decide whether those rerun results become warnings, failures, or robust candidates.
- `validation_common.rs` is the adjacent helper surface for candidate identity/canonicalization and should be inspected whenever candidates seem to merge, split, or disappear across datasets.

### Main entry points

- `src-tauri/src/commands_validation.rs`
  - `apply_cross_ticker_validation`
  - `apply_cross_ticker_validation_double_ma`
  - `apply_cross_ticker_validation_double_ma_headless`
- `src-tauri/src/validation_common.rs`
  - start with the candidate-key canonicalization helpers

### Recommended edit order

1. `src-tauri/src/commands_validation.rs`
2. `src-tauri/src/validation_common.rs`
3. `crates/app_core/src/validation_pipeline.rs`

### Review focus

- horizon alignment and empty-overlap handling
- duplicate or dropped candidates caused by canonical keying
- whether cross-ticker gates run before or after base validation decisions
- preserved-finalist behavior versus fully reranked behavior

## 5. Frontend optimization request shaping

### Primary files

- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/services/optimizeRequestBuilder.test.ts`
- `web/src/utils/strategyOptimizationCapabilities.ts`

### What each file owns

- `web/src/services/optimizeRequestBuilder.ts`
  - Converts frontend inputs into typed optimization payloads.
  - Owns input normalization, validation errors, backend field construction, MA parameter sweep shaping, data-field preconditions, and request-size guardrails.

- `web/src/services/optimizeRequestBuilder.test.ts`
  - Contract tests for request construction.
  - Covers positive and negative cases such as alias normalization, MA whitelist errors, and composite routing behavior.

- `web/src/utils/strategyOptimizationCapabilities.ts`
  - Single-source capability matrix used to decide what the UI should expose for each strategy kind.

### Review focus

- malformed or partial input handling
- request-shape compatibility with Rust-side request structs
- guardrail coverage for extreme ranges and sweep counts
- capability matrix drift versus real backend support

## 6. Frontend optimization orchestration

### Primary files

- `web/src/services/optimizeService.ts`
- `web/src/services/optimizeService.test.ts`
- `src-tauri/src/commands_optimize.rs`

### What each file owns

- `web/src/services/optimizeService.ts`
  - Frontend orchestration boundary for optimization actions.
  - Owns reading store state, choosing the backend command, invoking Tauri, surfacing backend metadata, triggering parity checks, updating UI status, and related post-run flows such as validation/artifact handling.
  - Main entry points include:
    - `applyPreset`
    - `presetKernel`
    - `refreshGpuStatus`
    - `runGpuParityCheck`
    - `buildOptimizationRequestFromVueStores`
    - `runOptimization`
    - `cancelOptimization`
    - `runValidation`

- `web/src/services/optimizeService.test.ts`
  - Integration-style tests for service orchestration.
  - Covers command selection, preset application, auto-build behavior, artifact persistence, backend metadata surfacing, and auto-validation triggers.

- `src-tauri/src/commands_optimize.rs`
  - Rust-side command boundary paired with the frontend service.
  - Review together with `optimizeService.ts` when debugging UI-to-backend command mismatches.

### Review focus

- coupling between UI state, DOM/state reads, and command dispatch
- mismatch risk between frontend command selection and Rust command surface
- parity-check assumptions and tolerance thresholds
- error-path coverage for failures, missing metadata, and cancellation races

## 7. Presets and test fixtures

### Primary files

- `crates/app_core/src/strategy_presets.rs`
- `crates/app_core/src/double_ma_test_support.rs`
- `web/src/services/optimizeRequestBuilder.test.ts`
- `web/src/services/optimizeService.test.ts`

### What each file owns

- `crates/app_core/src/strategy_presets.rs`
  - Canonical strategy fixtures and named kernel-oriented preset constructors.
  - Useful when reviewing whether benchmark/test inputs still represent the intended product behavior.

- `crates/app_core/src/double_ma_test_support.rs`
  - Test-only wrappers exposing preset builders into `double_ma.rs` test code.

- `web/src/services/optimizeRequestBuilder.test.ts`
  - Request construction contract coverage.

- `web/src/services/optimizeService.test.ts`
  - Frontend orchestration coverage.

### Review focus

- preset drift versus production defaults
- missing edge-case coverage
- silent contract drift between test helpers and production code

## 8. Tauri command registry and module boundaries

### Primary files

- `src-tauri/src/main.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/src/commands.rs`
- `src-tauri/src/commands_optimize.rs`
- `src-tauri/src/commands_validation.rs`

### What this bundle is for

- `src-tauri/src/commands.rs` is the public command aggregation layer.
- Review this bundle when the issue is about which commands are exposed, how the Tauri boundary is organized, or where a feature enters the app from the frontend.
- If a command exists in one module but is not reachable from the app, inspect `src-tauri/src/lib.rs` next.

## Suggested isolated review bundles for GPT-5.4

### Bundle A: Double MA optimizer internals

- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/double_ma_backend.rs`
- `crates/app_core/src/double_ma_mode.rs`
- `crates/app_core/src/double_ma_result.rs`
- `crates/app_core/src/double_ma_strategy.rs`

Ask for:

- backend-routing review
- CPU/GPU parity review
- optimization-mode review
- strategy-config validation review

### Bundle B: Validation engine

- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/validation_pipeline.rs`

Ask for:

- validation-statistics review
- hard-gate/verdict review
- determinism/reproducibility review

### Bundle C: Cross-ticker validation orchestration

- `src-tauri/src/commands_validation.rs`
- `crates/app_core/src/validation_pipeline.rs`

Ask for:

- cross-ticker aggregation review
- async/cancellation review
- finalist-selection review

### Bundle D: Frontend request shaping

- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/utils/strategyOptimizationCapabilities.ts`
- `web/src/services/optimizeRequestBuilder.test.ts`

Ask for:

- request-contract review
- frontend/backend support-matrix review
- guardrail/error-message review

### Bundle E: Frontend orchestration

- `web/src/services/optimizeService.ts`
- `web/src/services/optimizeService.test.ts`
- `src-tauri/src/commands_optimize.rs`

Ask for:

- service-boundary review
- command-selection review
- failure/cancellation review

## Backend-wide repo map extension

This section expands the map beyond optimization/validation so it covers the rest of the backend/runtime repo, excluding frontend/UI code.

## 9. Workspace and crate boundaries

### Workspace members

- `src-tauri`
- `crates/app_core`
- `crates/strategies`
- `crates/optimizer`

### What each crate owns

- `src-tauri`
  - Desktop host, Tauri command boundary, runtime integration, app state, external side effects, file dialogs, and desktop-facing orchestration.
  - Start here when the question is "how does the app expose this backend feature?"

- `crates/app_core`
  - Main business-logic/orchestration crate.
  - Owns optimization, validation, strategy planning, dynamic indicator/GPU plumbing, composite strategy runtimes, meta-selection, ML confidence, statistics helpers, and public request/result types used by Tauri.

- `crates/strategies`
  - Pure strategy/math crate.
  - Owns strategy evaluation primitives, metrics, trade/curve types, and strategy record structures.

- `crates/optimizer`
  - Generic optimization/search crate.
  - Owns objective definitions, optimization modes, grid search, multi-objective search, and streaming aggregation.

## 10. Tauri application shell and command registry

### Primary files

- `src-tauri/src/main.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/src/commands.rs`

### What these files own

- `src-tauri/src/main.rs`
  - Thin executable entry point.
  - Delegates to the library app bootstrap.

- `src-tauri/src/lib.rs`
  - Real Tauri app bootstrap and state initialization.
  - Command registration boundary and app-wide dependency wiring.

- `src-tauri/src/commands.rs`
  - Aggregates command modules and re-exports their public command surface.
  - Best file for identifying which backend capabilities are exposed to the desktop app.

### Review focus

- command exposure completeness and accidental omissions
- global state setup and lifecycle ownership
- feature wiring from app shell into `ta_app_core`

## 11. Data ingestion, catalog, and market-data operations

### Primary files

- `src-tauri/src/commands_data.rs`
- `crates/app_core/src/csv_compat.rs`
- `crates/app_core/src/timeframe.rs`
- `crates/app_core/src/indicator_catalog.rs`
- `crates/app_core/src/indicator_factory.rs`
- `crates/app_core/src/cuda_device.rs`
- `src-tauri/src/lib.rs`

### What each file owns

- `src-tauri/src/commands_data.rs`
  - Backend data-service surface.
  - Owns:
    - CSV loading and resampling
    - moving-average and dynamic-indicator listing
    - GPU speedup table loading
    - sample/repo dataset discovery
    - market-data directory configuration
    - Polygon cache/key management
    - Polygon live stream start/stop/status
    - Polygon dataset validation, repair, and 1-minute download/load flow
    - native file/folder pickers
  - Main entry points:
    - `load_price_data`
    - `resample_price_data`
    - `list_moving_averages`
    - `list_dynamic_indicators`
    - `list_gpu_speedups`
    - `list_repo_data_csvs`
    - `get_market_data_dir_info`
    - `set_market_data_dir`
    - `polygon_start_live_candle_stream`
    - `polygon_stop_live_candle_stream`
    - `polygon_validate_dataset`
    - `polygon_repair_dataset`
    - `polygon_download_1m_and_load`

- `crates/app_core/src/csv_compat.rs`
  - Candle CSV compatibility/loading helpers.
  - Review this when investigating CSV schema tolerance or ingestion edge cases.

- `crates/app_core/src/timeframe.rs`
  - Timeframe parsing and candle resampling primitives.
  - Review this for resample correctness and base/derived timeframe identity handling.

- `crates/app_core/src/indicator_catalog.rs`
  - Dynamic indicator metadata and descriptor layer.

- `crates/app_core/src/indicator_factory.rs`
  - Indicator construction/instantiation layer.

- `crates/app_core/src/cuda_device.rs`
  - Preferred CUDA-device discovery and selection helpers.

### Recommended edit order

1. `src-tauri/src/commands_data.rs`
2. `crates/app_core/src/csv_compat.rs` or `crates/app_core/src/timeframe.rs`, depending on whether the issue is ingestion or resampling
3. `crates/app_core/src/indicator_catalog.rs` and `crates/app_core/src/indicator_factory.rs` for indicator-support issues
4. `src-tauri/src/lib.rs` if the problem is command exposure or state wiring

### Review focus

- CSV schema compatibility and silent coercion risk
- resampling correctness and timeframe identity semantics
- Polygon operational safety, cache behavior, and stream lifecycle
- indicator discovery metadata staying aligned with runtime support

## 12. Strategy math layer

### Primary files

- `crates/strategies/src/lib.rs`
- `crates/strategies/src/double_ma/mod.rs`
- `crates/strategies/src/composite_cpu/mod.rs`
- `crates/strategies/src/records.rs`

### What each file owns

- `crates/strategies/src/double_ma/mod.rs`
  - Core Double MA strategy math.
  - Owns `DoubleMaParams`, `Metrics`, `Trade`, `TradeStats`, `StrategyConfig`, grid expansion, MA matrix construction, and the raw evaluation functions for curves and metrics across f64/f32 and filtered/unfiltered variants.

- `crates/strategies/src/composite_cpu/mod.rs`
  - Composite CPU signal and metric evaluation layer.
  - Owns composite desired-position logic, execution overlays, and composite metric/curve evaluation.

- `crates/strategies/src/records.rs`
  - Trade record containers and record-oriented helpers.

- `crates/strategies/src/lib.rs`
  - Export boundary for the strategy crate.

### Review focus

- raw strategy-evaluation correctness
- consistency between fast and full evaluation variants
- overlay semantics and trade accounting
- data-structure contracts used by `app_core` and `optimizer`

## 13. Generic optimization/search engine

### Primary files

- `crates/optimizer/src/lib.rs`
- `crates/optimizer/src/grid.rs`
- `crates/optimizer/src/stream.rs`
- `crates/optimizer/src/mo.rs`

### What each file owns

- `crates/optimizer/src/lib.rs`
  - Public optimization contracts.
  - Owns `ObjectiveKind`, `OptimizationMode`, `BacktestEngine`, `CpuEngine`, `OptimizationResult`, and `OptimizationHeatmap`.

- `crates/optimizer/src/grid.rs`
  - Generic single-objective grid search.

- `crates/optimizer/src/stream.rs`
  - Streaming aggregation layer for optimization result collection and heatmap aggregation.

- `crates/optimizer/src/mo.rs`
  - Multi-objective search engine.
  - Owns parameter domains/search spaces, NSGA-II, random search, QMC search, and TPE-based search.

### Review focus

- objective/scoring consistency with strategy backends
- result aggregation determinism
- multi-objective constraint handling and Pareto logic
- search-space explosion and guardrail behavior

## 14. Composite strategy runtimes and strategy-specific modules

### Primary files

- `crates/app_core/src/composite_cpu.rs`
- `crates/app_core/src/composite_mo.rs`
- `crates/app_core/src/composite_validate.rs`
- `crates/app_core/src/asr.rs`
- `crates/app_core/src/vcc.rs`
- `crates/app_core/src/vemr.rs`
- `crates/app_core/src/rocb.rs`
- `crates/app_core/src/mkcs.rs`
- `crates/app_core/src/kqsr.rs`
- `crates/app_core/src/kvdf.rs`
- `crates/app_core/src/carm.rs`
- `crates/app_core/src/cdmc.rs`

### GPU companion modules

- `crates/app_core/src/asr_gpu_kernel.rs`
- `crates/app_core/src/vcc_gpu_kernel.rs`
- `crates/app_core/src/vemr_gpu_kernel.rs`
- `crates/app_core/src/rocb_gpu_kernel.rs`
- `crates/app_core/src/mkcs_gpu_kernel.rs`
- `crates/app_core/src/kqsr_gpu_kernel.rs`
- `crates/app_core/src/kvdf_gpu_kernel.rs`
- `crates/app_core/src/carm_gpu_kernel.rs`
- `crates/app_core/src/cdmc_gpu_kernel.rs`

### Adjacent files

- `crates/app_core/src/composite_profile_route.rs`
- `crates/app_core/src/strategy_presets.rs`

### What this bundle owns

- Strategy-specific parameter models and runtime logic for the composite strategy family.
- CPU optimization/validation path for composite strategies.
- Multi-objective composite optimization path.
- GPU-kernel implementations for the supported composite strategies listed above.

### Recommended edit order

1. strategy-specific file such as `crates/app_core/src/asr.rs` or `crates/app_core/src/vcc.rs`
2. `crates/app_core/src/composite_cpu.rs` or `crates/app_core/src/composite_mo.rs`
3. matching `*_gpu_kernel.rs` file if GPU parity is in scope
4. `crates/app_core/src/composite_validate.rs`
5. `crates/app_core/src/composite_profile_route.rs` if route selection must change

### Review focus

- parity between CPU strategy modules and GPU kernel modules
- consistency of parameter semantics across strategies
- composite validation correctness
- route policy alignment with which strategies truly support GPU kernels

## 15. Dynamic strategy planning and GPU program generation

### Primary files

- `crates/app_core/src/strategy_ir.rs`
- `crates/app_core/src/strategy_plan.rs`
- `crates/app_core/src/strategy_gpu_program.rs`
- `crates/app_core/src/strategy_dynamic.rs`
- `crates/app_core/src/dynamic_indicator_adapter.rs`
- `crates/app_core/src/ma_policy.rs`
- `crates/app_core/src/vram_ma.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`

### Recommended edit order

1. `crates/app_core/src/strategy_ir.rs`
2. `crates/app_core/src/strategy_plan.rs`
3. `crates/app_core/src/strategy_gpu_program.rs` or `crates/app_core/src/strategy_dynamic.rs`
4. `crates/app_core/src/dynamic_indicator_adapter.rs`
5. `crates/app_core/src/ma_policy.rs`, `crates/app_core/src/vram_ma.rs`, and `crates/app_core/src/gpu_backtest_kernel.rs` for route/runtime issues

### What this bundle owns

- Internal strategy representation and planning for execution/runtime paths.
- GPU program shape and dynamic strategy compilation/plumbing.
- Dynamic indicator CUDA adaptation and device-data bridging.
- MA capability/routing policy and VRAM-resident MA support.
- Shared GPU backtest-kernel runtime plumbing.

### Review focus

- IR-to-runtime fidelity
- dynamic indicator/device-data correctness
- MA route planning and support-matrix correctness
- GPU kernel shared-runtime assumptions and reuse boundaries

## 16. Analysis and diagnostic surfaces

### Primary files

- `src-tauri/src/commands_analysis.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/composite_cpu.rs`
- `crates/app_core/src/matrix_index.rs`

### What this bundle owns

- Drilldown and local-stability command surfaces for Double MA and composite CPU.
- Matrix indexing and result-coordinate support for analysis output.
- Use this bundle when the question is about explanation, sensitivity analysis, or post-optimization inspection rather than raw optimization.

### Main entry points

- `src-tauri/src/commands_analysis.rs`
  - `compute_double_ma_drilldown`
  - `compute_composite_cpu_drilldown`
  - `compute_double_ma_local_stability`
  - `compute_composite_cpu_local_stability`
- `crates/app_core/src/double_ma.rs`
  - `compute_double_ma_drilldown_blocking_with_candles`
  - `compute_double_ma_local_stability_blocking_with_candles`

### Review focus

- drilldown/local-stability numerical consistency
- matrix-coordinate correctness
- analysis output stability relative to optimization output

## 17. Validation statistics and robustness math

### Primary files

- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/validation_core.rs`
- `crates/app_core/src/validation_stats.rs`
- `crates/app_core/src/cscv_pbo.rs`
- `crates/app_core/src/sharpe_stats.rs`
- `crates/app_core/src/stats_common.rs`
- `crates/app_core/src/prescan_predictive.rs`
- `crates/app_core/src/iterative_sim.rs`

### What this bundle owns

- Shared validation/reduction math and verdict logic
- CSCV/PBO and related robustness statistics
- Sharpe/statistical helper functions
- common stats utilities
- predictive prescan and iterative-simulation helpers used in advanced validation or meta workflows

### Review focus

- statistical soundness
- determinism and seeded randomness
- gate/failure semantics
- reuse boundaries between core validation and meta-selection workflows

## 18. Live execution and broker integration

### Primary files

- `src-tauri/src/commands_execution.rs`
- `src-tauri/src/commands.rs`
- `src-tauri/src/lib.rs`

### What this file owns

- Single-shot strategy signal execution
- live execution runner lifecycle
- live runner status reporting
- broker connectivity checks
- live execution snapshots for orders, positions, and risk state

### Main entry points

- `execute_strategy_signal_once`
- `start_live_execution_runner`
- `stop_live_execution_runner`
- `live_execution_runner_status`
- `test_alpaca_connection`
- `get_live_execution_snapshot`

### Recommended edit order

1. `src-tauri/src/commands_execution.rs`
2. `src-tauri/src/commands.rs`
3. `src-tauri/src/lib.rs` if lifecycle wiring or state registration must change

### Review focus

- live runner lifecycle and cancellation safety
- broker request/response handling
- state snapshot correctness
- boundaries between paper/live execution modes and risk overrides

## 19. Meta-selection and ML confidence workflows

### Primary files

- `src-tauri/src/commands_meta.rs`
- `crates/app_core/src/meta_select.rs`
- `crates/app_core/src/ml_confidence.rs`
- `crates/app_core/src/cscv_pbo.rs`
- `crates/app_core/src/validation_stats.rs`

### What each file owns

- `src-tauri/src/commands_meta.rs`
  - Tauri command surface for:
    - ML confidence model status/train/apply/delete
    - meta stage 4 experimental run
    - meta stage 5 ML confidence run
    - ML confidence policy evaluation
    - meta-selection run orchestration
  - Main entry points:
    - `ml_confidence_model_status`
    - `train_ml_confidence_model`
    - `apply_ml_confidence_model`
    - `delete_ml_confidence_model`
    - `run_meta_stage4_experimental`
    - `run_meta_stage5_ml_confidence`
    - `evaluate_ml_confidence_policy`
    - `run_meta_selection`

- `crates/app_core/src/meta_select.rs`
  - Core meta-selection logic, result models, and selection workflow primitives.

- `crates/app_core/src/ml_confidence.rs`
  - ML confidence model types, training/apply logic, policy artifacts, and related scoring interfaces.

- `crates/app_core/src/cscv_pbo.rs`
  - Statistical robustness helpers reused by meta/validation workflows.

- `crates/app_core/src/validation_stats.rs`
  - Validation-statistics support layer that often sits next to meta-ranking decisions.

### Recommended edit order

1. `src-tauri/src/commands_meta.rs`
2. `crates/app_core/src/meta_select.rs`
3. `crates/app_core/src/ml_confidence.rs`
4. `crates/app_core/src/cscv_pbo.rs` and `crates/app_core/src/validation_stats.rs` if the issue touches scoring methodology

### Review focus

- artifact-to-meta-selection contract correctness
- model lifecycle and persistence safety
- stage 4/stage 5 scoring semantics
- policy-evaluation methodology and leakage risk

## 20. Reporting and exported deliverables

### Primary files

- `src-tauri/src/commands_reports.rs`
- `src-tauri/src/commands_reports_api.rs`
- `src-tauri/src/run_artifacts.rs`

### What each file owns

- `src-tauri/src/commands_reports.rs`
  - Headless report-generation internals, including XLSX byte construction and related report helpers.

- `src-tauri/src/commands_reports_api.rs`
  - Tauri-facing report/export API:
    - build report XLSX
    - build validation shortlist CSV
    - save report XLSX
    - open reports directory
    - reveal/open a generated path

- `src-tauri/src/run_artifacts.rs`
  - Adjacent artifact schema/persistence context.
  - Inspect this when a report field seems stale, missing, or inconsistent with saved run metadata.

### Recommended edit order

1. `src-tauri/src/commands_reports_api.rs`
2. `src-tauri/src/commands_reports.rs`
3. `src-tauri/src/run_artifacts.rs`

### Review focus

- export correctness and format stability
- file-path sanitization and reveal/open safety
- consistency between report content and stored artifacts

## 21. Run artifacts and persistence

### Primary files

- `src-tauri/src/commands_artifacts.rs`
- `src-tauri/src/run_artifacts.rs`
- `src-tauri/src/validation_common.rs`

### What each file owns

- `src-tauri/src/commands_artifacts.rs`
  - Artifact API surface:
    - save run artifact
    - save batch summary
    - list run/meta/stage4/stage5/ML-policy artifacts
    - load artifact
    - delete artifact
    - update artifact tags
    - open runs directory

- `src-tauri/src/run_artifacts.rs`
  - Artifact persistence internals and summary extraction.
  - Owns artifact-kind recognition, metadata normalization, fingerprint extraction, summary loading, and path-safety checks.

- `src-tauri/src/validation_common.rs`
  - Shared validation/artifact helper layer.
  - Important when reviewing candidate-keying or validation artifact identity behavior.

### Review focus

- artifact kind/version compatibility
- config fingerprint correctness
- path safety and in-directory enforcement
- metadata normalization and summary extraction drift

## 22. System/platform utilities

### Primary files

- `src-tauri/src/commands_system.rs`

### What this file owns

- GPU status reporting
- window effect/platform-specific shell integration

### Main entry points

- `get_gpu_status`
- `set_window_effect`

### Review focus

- runtime environment detection
- platform guard correctness
- whether system-facing commands leak UI-specific concerns into backend logic

## Suggested backend-only review bundles for GPT-5.4

### Bundle F: Data and market-data operations

- `src-tauri/src/commands_data.rs`
- `crates/app_core/src/csv_compat.rs`
- `crates/app_core/src/timeframe.rs`
- `crates/app_core/src/indicator_catalog.rs`
- `crates/app_core/src/indicator_factory.rs`
- `crates/app_core/src/cuda_device.rs`

Ask for:

- ingestion/resampling review
- market-data operational review
- indicator catalog/runtime support review

### Bundle G: Strategy math and generic optimization

- `crates/strategies/src/double_ma/mod.rs`
- `crates/strategies/src/composite_cpu/mod.rs`
- `crates/strategies/src/records.rs`
- `crates/optimizer/src/lib.rs`
- `crates/optimizer/src/grid.rs`
- `crates/optimizer/src/stream.rs`
- `crates/optimizer/src/mo.rs`

Ask for:

- strategy-math review
- optimizer correctness review
- multi-objective search review

### Bundle H: Composite strategy runtimes

- `crates/app_core/src/composite_cpu.rs`
- `crates/app_core/src/composite_mo.rs`
- `crates/app_core/src/composite_validate.rs`
- `crates/app_core/src/asr.rs`
- `crates/app_core/src/vcc.rs`
- `crates/app_core/src/vemr.rs`
- `crates/app_core/src/rocb.rs`
- `crates/app_core/src/mkcs.rs`
- `crates/app_core/src/kqsr.rs`
- `crates/app_core/src/kvdf.rs`
- `crates/app_core/src/carm.rs`
- `crates/app_core/src/cdmc.rs`

Ask for:

- strategy-runtime review
- parameter-contract review
- CPU/GPU parity review

### Bundle I: Dynamic strategy and GPU infrastructure

- `crates/app_core/src/strategy_ir.rs`
- `crates/app_core/src/strategy_plan.rs`
- `crates/app_core/src/strategy_gpu_program.rs`
- `crates/app_core/src/strategy_dynamic.rs`
- `crates/app_core/src/dynamic_indicator_adapter.rs`
- `crates/app_core/src/ma_policy.rs`
- `crates/app_core/src/vram_ma.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`

Ask for:

- IR/planning review
- dynamic GPU pipeline review
- support-matrix and route-policy review

### Bundle J: Live execution and broker integration

- `src-tauri/src/commands_execution.rs`

Ask for:

- live-execution safety review
- broker integration review
- state-machine/race review

### Bundle K: Meta-selection and ML confidence

- `src-tauri/src/commands_meta.rs`
- `crates/app_core/src/meta_select.rs`
- `crates/app_core/src/ml_confidence.rs`
- `crates/app_core/src/cscv_pbo.rs`
- `crates/app_core/src/validation_stats.rs`

Ask for:

- meta-selection methodology review
- ML-confidence pipeline review
- leakage/overfitting-risk review

### Bundle L: Reporting and artifact persistence

- `src-tauri/src/commands_reports.rs`
- `src-tauri/src/commands_reports_api.rs`
- `src-tauri/src/commands_artifacts.rs`
- `src-tauri/src/run_artifacts.rs`
- `src-tauri/src/validation_common.rs`

Ask for:

- artifact-contract review
- report/export review
- persistence/path-safety review

## General high-quality review prompt

Use this prompt when sending one feature bundle to another model for evaluation.

```text
You are reviewing a specific backend feature implementation from my Rust/Tauri project.

Your job is to evaluate this feature for:
1. correctness of behavior and feature implementation
2. architectural quality and code quality
3. optimization/performance opportunities
4. edge cases, failure modes, and regression risks
5. test coverage gaps
6. maintainability and future-change risk

Important constraints:
- Base your review only on the files I provide.
- Do not assume behavior that is not supported by the code.
- If something is unclear, say what is unclear and what file likely contains the missing context.
- Prefer concrete findings over general advice.
- Prioritize bugs, behavioral mismatches, hidden assumptions, and performance risks over style comments.
- When making a claim, cite the file and the relevant function or module name.
- Distinguish clearly between:
  - confirmed issue
  - likely issue
  - open question or missing context
  - optimization opportunity
- Do not give shallow praise. Be critical and specific.

Review this feature as if I may ask another model to implement improvements afterward.

For each finding, use this format:
- Severity: critical / high / medium / low
- Type: correctness / performance / code quality / architecture / testing / maintainability
- Location: file + function or module
- Why it matters
- Recommended change

Then provide these sections:
1. Feature understanding
   - What this feature appears to do
   - Main entry points
   - Main dependency files in the provided set

2. Findings
   - Ordered by severity, most important first

3. Optimization opportunities
   - Only include changes that are realistically worth making
   - Separate safe optimization from higher-risk refactor

4. Implementation-risk review
   - Areas where the feature may not faithfully implement intended behavior
   - For validation-stage logic, inspect fallback-path drift, determinism, candidate filtering, metric interpretation, and cross-stage leakage risk

5. Test review
   - What is well covered
   - What is missing
   - Highest-value tests to add

6. Suggested improvement plan
   - Small, medium, and large improvement options
   - Include which file should be edited first for each

If the implemented behavior appears to differ from the intended feature behavior, prioritize that over performance or style comments.

Feature under review:
[describe the feature here in 1-3 sentences]

Files to review:
[paste file list here]

Extra context:
[paste intended behavior, product expectation, or known concerns here]
```

### Optional add-on for validation-stage reviews

```text
Pay extra attention to:
- whether the validation logic matches the intended methodology
- fallback-path consistency
- determinism and reproducibility
- candidate filtering and gate correctness
- metric interpretation
- cross-stage leakage or selection bias
```

### Optional add-on for optimization-heavy reviews

```text
Pay extra attention to:
- unnecessary allocations
- repeated recomputation
- CPU/GPU parity risks
- fallback-path performance cliffs
- large-search-space inefficiencies
- opportunities for simpler or safer optimizations before major refactors
```
