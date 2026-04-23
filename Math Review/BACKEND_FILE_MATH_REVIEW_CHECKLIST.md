# Backend File Math Review Checklist

Date: `2026-04-17`

## Purpose

This checklist is the exhaustive backend coverage tracker for the math-review program.

Rules:

- every backend Rust file in scope must be reviewed
- each file stays unchecked until a sub-agent returns coverage for that exact file
- every reviewed file must be classified as either:
  - `viable`
  - `non_viable`
  - `support_only`
- if viable, map it to one or more packet IDs from
  [MASTER_MATH_REVIEW_TRACKER.md](</C:/Rust Projects/ta_desktop_demo/Math Review/MASTER_MATH_REVIEW_TRACKER.md>)
- if non-viable or support-only, record the reason briefly

## Scope

- `src-tauri/src/**/*.rs`
- `crates/app_core/src/**/*.rs`
- `crates/strategies/src/**/*.rs`
- `crates/optimizer/src/**/*.rs`

Total files in current scope: `136`

## Legend

- `reviewed`: markdown checkbox
- `viable`: `yes` / `no` / `support_only` / `TBD`
- `packet`: `MR-*` or `TBD`
- `note`: one-line reason

## Batch Summary

| Batch | Area | File count | Status |
| --- | --- | --- | --- |
| `A1` | `app_core` double-ma and kernel core | `11` | `complete` |
| `A2` | `app_core` composite and validation core | `14` | `complete` |
| `A3` | `app_core` dynamic/runtime/routing support | `17` | `complete` |
| `A4` | `app_core` meta/ml/options core | `12` | `complete` |
| `A5` | `app_core` strategy family modules | `20` | `complete` |
| `A6` | `app_core` bin and CLI surfaces | `9` | `complete` |
| `B1` | `strategies` and `optimizer` crates | `8` | `complete` |
| `C1` | `src-tauri` core/runtime surfaces | `12` | `complete` |
| `C2` | `src-tauri` main command surfaces | `10` | `complete` |
| `C3` | `src-tauri` reports/options/system commands | `10` | `complete` |
| `C4` | `src-tauri` bin surfaces | `13` | `complete` |

## Checklist

### `A1` `app_core` double-ma and kernel core

- [x] `crates/app_core/src/double_ma.rs` | viable: `yes` | packet: `MR-02,MR-03,MR-07` | note: owns effective-score algebra, shortlist rerank helpers, canonical risk or trade metric finalization, and local shortlist-stability or plateau diagnostics for Double-MA optimization.
- [x] `crates/app_core/src/double_ma_backend.rs` | viable: `yes` | packet: `MR-04` | note: static GPU and hybrid blocker logic plus capability summaries encode route-policy heuristics.
- [x] `crates/app_core/src/double_ma_ma_kind.rs` | viable: `support_only` | packet: `none` | note: registry lookup and period-based single-output admission checks only.
- [x] `crates/app_core/src/double_ma_mode.rs` | viable: `yes` | packet: `MR-04` | note: auto mode picks Grid, CoarseToFine, or GpuVdps from fixed combo and output heuristics rather than an explicit cost model.
- [x] `crates/app_core/src/double_ma_result.rs` | viable: `support_only` | packet: `none` | note: projects backend outputs into resolved params, matrix columns, and diagnostics without introducing new math.
- [x] `crates/app_core/src/double_ma_strategy.rs` | viable: `no` | packet: `none` | note: validates and repairs strategy and filter inputs but is not a standalone math-review surface.
- [x] `crates/app_core/src/double_ma_test_support.rs` | viable: `support_only` | packet: `none` | note: test-only preset forwarders into strategy presets.
- [x] `crates/app_core/src/double_ma_validate.rs` | viable: `yes` | packet: `MR-09,MR-10` | note: runs walk-forward, regime-slice, friction-stress, checkpoint-robustness, perturb, bootstrap analyses, and directly builds CSCV or SPA inputs for finalists.
- [x] `crates/app_core/src/double_ma_vdps.rs` | viable: `yes` | packet: `MR-04` | note: owns VDPS rung targeting, local-anchor thinning, and elite or differential seed generation for candidate-space reduction.
- [x] `crates/app_core/src/gpu_backtest_kernel.rs` | viable: `yes` | packet: `MR-01,MR-02,MR-03,MR-05` | note: defines GPU metric contracts, resident-market and returns-cache economics, and the live shortlist-confidence, union, and slice-robustness rerank machinery.
- [x] `crates/app_core/src/vram_ma.rs` | viable: `support_only` | packet: `none` | note: tiny CUDA bridge re-exporting vector_ta VRAM MA helpers without its own math.

### `A2` `app_core` composite and validation core

- [x] `crates/app_core/src/composite_cpu.rs` | viable: `yes` | packet: `MR-02,MR-07,MR-15` | note: owns composite objective scalarization, trade22 metric enrichment, and host-side confidence, union, Pareto, or weighted-rank shortlist reranking.
- [x] `crates/app_core/src/composite_gpu_metric_decode.rs` | viable: `yes` | packet: `MR-01,MR-07` | note: defines non-double-MA GPU metric row contract, derives Martin ratio from pnl and ulcer, and fixes placeholder trade-stat semantics.
- [x] `crates/app_core/src/composite_mo.rs` | viable: `yes` | packet: `MR-04,MR-15` | note: implements composite MO objective or constraint mapping plus GPU prescan pair narrowing and candidate-space reduction.
- [x] `crates/app_core/src/composite_profile_route.rs` | viable: `support_only` | packet: `none` | note: static profile and target-label classification that supports route policy but contains no explicit cost-model or scoring math.
- [x] `crates/app_core/src/composite_validate.rs` | viable: `yes` | packet: `MR-07,MR-09,MR-10` | note: owns composite validation math including walk-forward and reopt summaries, CSCV or PBO, SPA inputs, bootstrap wiring, regime analytics, and downstream consumption of detailed VCC analytics semantics.
- [x] `crates/app_core/src/cscv_pbo.rs` | viable: `yes` | packet: `MR-09` | note: implements CSCV or PBO split combinatorics, slice scoring, lambda quantiles, and degradation regression from slice return summaries.
- [x] `crates/app_core/src/iterative_sim.rs` | viable: `yes` | packet: `MR-07` | note: defines iterative execution semantics and re-derives pnl, Sharpe, Sortino, Omega, drawdown, ulcer, and exposure metrics.
- [x] `crates/app_core/src/sharpe_stats.rs` | viable: `yes` | packet: `MR-09` | note: implements PSR or DSR-related formulas, expected-max-Sharpe estimation, and minimum track-record-length math.
- [x] `crates/app_core/src/stats_common.rs` | viable: `support_only` | packet: `none` | note: generic quantile, median, and extreme helpers used by summaries but not a packet-owning surface by itself.
- [x] `crates/app_core/src/timeframe.rs` | viable: `support_only` | packet: `none` | note: parses timeframe labels and performs candle resampling as data-shaping support rather than a current packet target.
- [x] `crates/app_core/src/validation_core.rs` | viable: `support_only` | packet: `none` | note: thin orchestration wrapper delegating collection, sorting, and optional finalization to validation_pipeline.
- [x] `crates/app_core/src/validation_pipeline.rs` | viable: `yes` | packet: `MR-08,MR-09,MR-10` | note: holds robust-score formula, probability gating, walk-forward and reopt utility summaries, trade bootstrap, and SPA test logic.
- [x] `crates/app_core/src/validation_shared.rs` | viable: `yes` | packet: `MR-09,MR-10,MR-13` | note: hard-codes validation math defaults and thresholds including SPA slices, bootstrap samples and block length, reopt top-n, and cross-ticker gate defaults.
- [x] `crates/app_core/src/validation_stats.rs` | viable: `yes` | packet: `MR-02,MR-09` | note: defines bounded PSR and DSR outputs plus missing-as-zero probability-gate semantics used in both validation gates and optimization-path confidence reranks.

### `A3` `app_core` dynamic/runtime/routing support

- [x] `crates/app_core/src/alloc_policy.rs` | viable: `support_only` | packet: `none` | note: scratch-buffer allocation and prefix-init helpers only, with no scoring, routing, or statistical contract.
- [x] `crates/app_core/src/cuda_device.rs` | viable: `support_only` | packet: `none` | note: sticky preferred-device selection by visible VRAM as operational device policy, not a math-review surface.
- [x] `crates/app_core/src/dynamic_indicator_adapter.rs` | viable: `yes` | packet: `MR-05,MR-06` | note: owns prepared CUDA indicator inputs, host-device fallback, and source or input-kind residency for dynamic GPU reuse and algebra.
- [x] `crates/app_core/src/indicator_catalog.rs` | viable: `support_only` | packet: `none` | note: projects registry metadata and validates params or outputs without carrying the math contract itself.
- [x] `crates/app_core/src/indicator_factory.rs` | viable: `support_only` | packet: `none` | note: CPU-only MA batch wrapper over double_ma that adds no independent ranking or residency logic.
- [x] `crates/app_core/src/ma_policy.rs` | viable: `yes` | packet: `MR-04` | note: encodes kernel-capable allowlists, speedup-table preferences, and hybrid route decisions feeding backend route policy.
- [x] `crates/app_core/src/matrix_index.rs` | viable: `support_only` | packet: `none` | note: stable fast<slow grid indexing helper with deterministic layout only.
- [x] `crates/app_core/src/nvtx.rs` | viable: `support_only` | packet: `none` | note: profiling-range loader and color mapping only; instrumentation does not change math behavior.
- [x] `crates/app_core/src/prescan_predictive.rs` | viable: `yes` | packet: `MR-04` | note: predictive prescan ranks sampled fast and slow pairs by next-close log-return edge, directly affecting pruning economics.
- [x] `crates/app_core/src/progress.rs` | viable: `support_only` | packet: `none` | note: progress and VRAM-prefetch feedback sink only with no mathematical semantics.
- [x] `crates/app_core/src/request_normalization.rs` | viable: `support_only` | packet: `none` | note: normalizes optional path, top-k, include-all, and heatmap-bin inputs as request hygiene.
- [x] `crates/app_core/src/standalone_gpu_runtime.rs` | viable: `support_only` | packet: `none` | note: shared device-buffer helpers and resident cache keys for kernels, but runtime plumbing not standalone review math.
- [x] `crates/app_core/src/strategy_dynamic.rs` | viable: `yes` | packet: `MR-06` | note: enforces dynamic GPU v1 limits such as float-only outputs, two-slot shape, and CPU fallback on unsupported algebra.
- [x] `crates/app_core/src/strategy_gpu_program.rs` | viable: `yes` | packet: `MR-06` | note: defines GPU predicate-program operand or group ABI and executable fingerprints, the compact algebra encoded for runtime.
- [x] `crates/app_core/src/strategy_gpu_runtime.rs` | viable: `yes` | packet: `MR-06` | note: builds and validates GPU predicate bundles and executable caches, carrying the compiled dynamic GPU contract into runtime.
- [x] `crates/app_core/src/strategy_ir.rs` | viable: `yes` | packet: `MR-06` | note: defines StrategyIr, ValueRef, conditions, indicator nodes, and validation caps that set the representable dynamic strategy algebra.
- [x] `crates/app_core/src/strategy_plan.rs` | viable: `yes` | packet: `MR-04,MR-06` | note: compiles IR into routed indicator and condition plans using speedup hints, making it both route-policy and GPU-contract surface.

### `A4` `app_core` meta/ml/options core

- [x] `crates/app_core/src/csv_compat.rs` | viable: `support_only` | packet: `none` | note: CSV compatibility loader with header aliasing and CandleFieldFlags shaping, not a ranking or statistics surface.
- [x] `crates/app_core/src/lib.rs` | viable: `support_only` | packet: `none` | note: crate root and re-export surface only; math work lives in the underlying modules it exposes.
- [x] `crates/app_core/src/meta_select.rs` | viable: `yes` | packet: `MR-08,MR-11,MR-12` | note: owns grouped quantiles, hard shortlist gates, weighted meta-score ranking, and holdout evidence feeding ML-confidence.
- [x] `crates/app_core/src/ml_confidence.rs` | viable: `yes` | packet: `MR-12` | note: owns ML-confidence labels, logistic and GBDT training, Platt calibration, and policy-health evaluation.
- [x] `crates/app_core/src/ml_signal.rs` | viable: `yes` | packet: `MR-17` | note: owns ML-signal label policy, split-window construction, calibration transforms, score-cache coverage contracts, threshold or delta action rules, overlay execution semantics, and sweep ranking.
- [x] `crates/app_core/src/options_data_store.rs` | viable: `support_only` | packet: `none` | note: normalized options ingest, contract-series assembly, dedup, and manifest I/O support the options lane but do not own its scoring math.
- [x] `crates/app_core/src/options_expression.rs` | viable: `yes` | packet: `MR-16` | note: owns options split assignment, tuple evaluation, robust-score weighting, coverage adequacy, and finalist ranking.
- [x] `crates/app_core/src/options_expression_gpu.rs` | viable: `yes` | packet: `MR-16` | note: device-side tuple aggregate scoring, chunk planning, and GPU-residency economics are direct options math-review targets.
- [x] `crates/app_core/src/options_signal_manifest.rs` | viable: `yes` | packet: `MR-16` | note: fixes option-event timing and price semantics through decision-bar projection, entry-bar underlier price selection, UTC-session labeling, and default fold initialization.
- [x] `crates/app_core/src/options_types.rs` | viable: `yes` | packet: `MR-16` | note: carries behavior-bearing options evaluation math through fill-model admissibility, stress-fragility thresholds, supported semantics helpers, and inferred validation verdict logic.
- [x] `crates/app_core/src/strategy_presets.rs` | viable: `support_only` | packet: `none` | note: curated StrategyConfig presets and route-plan tests, not a live math or ranking engine.
- [x] `crates/app_core/src/test_support.rs` | viable: `support_only` | packet: `none` | note: test-only mutex for process-global env and state coordination with no backend math behavior.

### `A5` `app_core` strategy family modules

- [x] `crates/app_core/src/asr.rs` | viable: `yes` | packet: `MR-07` | note: owns full ASR replay in simulate_from_desired and eval_asr_metrics_from_desired, filling Metrics and extended downside or trade fields.
- [x] `crates/app_core/src/asr_gpu_kernel.rs` | viable: `support_only` | packet: `none` | note: strategy-local CUDA launcher with resident-close caching and shared non-double-MA metric decode, but no standalone scoring contract.
- [x] `crates/app_core/src/carm.rs` | viable: `no` | packet: `none` | note: indicator assembly and desired-position gating only with no shared metric or ranking math.
- [x] `crates/app_core/src/carm_gpu_kernel.rs` | viable: `support_only` | packet: `none` | note: CUDA batch wrapper around resident-close caching and shared metric decode only.
- [x] `crates/app_core/src/cdmc.rs` | viable: `no` | packet: `none` | note: signal generation and entry-exit rules only without standalone analytics or selection math.
- [x] `crates/app_core/src/cdmc_gpu_kernel.rs` | viable: `support_only` | packet: `none` | note: device-row DTO and runtime-cache plumbing with shared metric decode only.
- [x] `crates/app_core/src/kqsr.rs` | viable: `no` | packet: `none` | note: signal construction and desired-position logic only with no direct metric-finalization or shortlist surface.
- [x] `crates/app_core/src/kqsr_gpu_kernel.rs` | viable: `support_only` | packet: `none` | note: thin CUDA evaluation entrypoint with resident close and log-return cache over shared metric decode.
- [x] `crates/app_core/src/kvdf.rs` | viable: `no` | packet: `none` | note: indicator normalization, signal building, and position rules only with no standalone analytics or ranking semantics.
- [x] `crates/app_core/src/kvdf_gpu_kernel.rs` | viable: `support_only` | packet: `none` | note: GPU transport and evaluation layer over shared non-double-MA metric rows.
- [x] `crates/app_core/src/mkcs.rs` | viable: `no` | packet: `none` | note: squeeze or breakout signal preparation and desired-position rules only, with no reusable metric or scalarization math.
- [x] `crates/app_core/src/mkcs_gpu_kernel.rs` | viable: `support_only` | packet: `none` | note: CUDA row-pointer and batch plumbing plus shared metric decode only.
- [x] `crates/app_core/src/rocb.rs` | viable: `no` | packet: `none` | note: breakout and ATR-stop signal logic and position management only, with no analytics or confidence contract.
- [x] `crates/app_core/src/rocb_gpu_kernel.rs` | viable: `support_only` | packet: `none` | note: strategy-local GPU launcher and resident-close cache over shared metric decode without retained-stat design.
- [x] `crates/app_core/src/vcc.rs` | viable: `no` | packet: `none` | note: signal sweeps, parity checks, and desired-position rules only, while metric semantics are delegated elsewhere.
- [x] `crates/app_core/src/vcc_gpu_kernel.rs` | viable: `yes` | packet: `MR-07` | note: extends shared metric decode with VCC-local detailed GPU outputs consumed by composite_validate, making it a real analytics contract.
- [x] `crates/app_core/src/vcet.rs` | viable: `no` | packet: `none` | note: signal construction and desired-position logic only without standalone risk or validation math.
- [x] `crates/app_core/src/vcet_gpu_kernel.rs` | viable: `support_only` | packet: `none` | note: prepared-batch and device-row transport plus shared metric decode for VCET GPU evaluation.
- [x] `crates/app_core/src/vemr.rs` | viable: `no` | packet: `none` | note: z-score, Bollinger, and ATR sweep helpers with desired-position rules only, not reusable shortlist math.
- [x] `crates/app_core/src/vemr_gpu_kernel.rs` | viable: `yes` | packet: `MR-02` | note: owns objective_to_vemr_kernel_id, objective_score, and on-device top-k reduction for objective-driven metric-row selection.

### `A6` `app_core` bin and CLI surfaces

- [x] `crates/app_core/src/bin/cupti_sass_probe.rs` | viable: `support_only` | packet: `none` | note: CUPTI inventory and capture harness around exact GpuKernel runs, not a trading-math or ranking surface.
- [x] `crates/app_core/src/bin/profile_doublema_campaign_prefetch.rs` | viable: `support_only` | packet: `none` | note: measures staged versus host-only VRAM-prefetch latency and residency feedback rather than owning math semantics.
- [x] `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs` | viable: `support_only` | packet: `none` | note: profiling runner for contract and slice toggles, cache stats, and route diagnostics that delegates scoring and statistics to core modules.
- [x] `crates/app_core/src/bin/real_data_fidelity_audit.rs` | viable: `yes` | packet: `MR-04,MR-09,MR-10` | note: owns exact-vs-VDPS regret and overlap math plus validation-preset heuristics for walk-forward, SPA, CSCV, and approval overlap.
- [x] `crates/app_core/src/bin/real_data_optimizer_eval.rs` | viable: `yes` | packet: `MR-04` | note: owns real-data exact-vs-progressive optimizer comparison, pair-count economics, and score-regret or overlap calculations.
- [x] `crates/app_core/src/bin/stage6b_kernel_report.rs` | viable: `support_only` | packet: `none` | note: synthetic CPU/GPU sweep and parity report with top-k overlap only, useful for diagnostics but not a core math surface.
- [x] `crates/app_core/src/bin/ta_cli.rs` | viable: `support_only` | packet: `none` | note: headless request builder for CSV or synthetic DoubleMa runs with no new mathematical logic.
- [x] `crates/app_core/src/bin/ta_ml_signal_export_dataset.rs` | viable: `support_only` | packet: `none` | note: thin CLI wrapper over ML-signal dataset export without label, calibration, or policy math.
- [x] `crates/app_core/src/bin/ta_ml_signal_native_smoke.rs` | viable: `support_only` | packet: `none` | note: native CatBoost smoke harness for score-cache builds and backend selection, not a math-review target.

### `B1` `strategies` and `optimizer` crates

- [x] `crates/optimizer/src/grid.rs` | viable: `yes` | packet: `MR-15` | note: grid search hard-codes scalar score mapping, risk-objective negation, finite-score clamping, and stable top-k tie semantics.
- [x] `crates/optimizer/src/lib.rs` | viable: `yes` | packet: `MR-07,MR-15` | note: defines shared objective vocabulary plus optimization-result and heatmap contracts that constrain scoring and analytics surfaces.
- [x] `crates/optimizer/src/mo.rs` | viable: `yes` | packet: `MR-15` | note: owns Pareto dominance, crowding, hypervolume, QMC sampling, TPE heuristics, exact-grid paths, and candidate-set MO search behavior.
- [x] `crates/optimizer/src/stream.rs` | viable: `yes` | packet: `MR-15` | note: stream aggregation fixes online scalarization, top-k merge behavior, and max/mean/count heatmap semantics.
- [x] `crates/strategies/src/composite_cpu/mod.rs` | viable: `yes` | packet: `MR-07,MR-15` | note: composite desired-position logic and metric evaluation encode hard gating plus duplicated risk/trade-stat math.
- [x] `crates/strategies/src/double_ma/mod.rs` | viable: `yes` | packet: `MR-01,MR-03,MR-07` | note: shared Double-MA metric/trade structs, risk finalization, and replay-style CPU analytics feed contract and robustness packets.
- [x] `crates/strategies/src/lib.rs` | viable: `support_only` | packet: `none` | note: thin crate root that only re-exports strategy modules and has no standalone math-review target.
- [x] `crates/strategies/src/records.rs` | viable: `yes` | packet: `MR-07` | note: record-array conversions and per-column trade stats define a parallel trade-record contract that should be reviewed against the live hot-path analytics implementations.

### `C1` `src-tauri` core/runtime surfaces

- [x] `src-tauri/src/commands.rs` | viable: `support_only` | packet: `none` | note: assembly and re-export facade for command modules; math-review logic lives in child modules.
- [x] `src-tauri/src/data_cache.rs` | viable: `yes` | packet: `MR-14` | note: owns cache identity and reuse heuristics including size, mtime, and content-fingerprint fidelity economics.
- [x] `src-tauri/src/lib.rs` | viable: `support_only` | packet: `none` | note: Tauri builder and invoke registration only, wiring math-owning modules but not defining formulas.
- [x] `src-tauri/src/main.rs` | viable: `no` | packet: `none` | note: four-line binary entrypoint that only calls run_tauri.
- [x] `src-tauri/src/ma_registry.rs` | viable: `support_only` | packet: `none` | note: projects upstream MA catalog and policy flags without scoring, routing-cost, or statistical formulas.
- [x] `src-tauri/src/metric_surface.rs` | viable: `yes` | packet: `MR-14` | note: rebuilds metric-surface kind from route and strategy heuristics and projects canonical artifact metric rows.
- [x] `src-tauri/src/options_parent_adapters.rs` | viable: `support_only` | packet: `none` | note: artifact replay and signal-manifest adapter for supported options parents, not options ranking math.
- [x] `src-tauri/src/polygon.rs` | viable: `yes` | packet: `MR-14` | note: implements cache-plan hit, append, and refresh heuristics plus gap-threshold and lookback economics.
- [x] `src-tauri/src/run_artifacts.rs` | viable: `yes` | packet: `MR-04,MR-14` | note: normalizes route provenance, dataset identity, timeframe identity, and metric-surface metadata into saved artifacts.
- [x] `src-tauri/src/settings.rs` | viable: `support_only` | packet: `none` | note: persists retention and market-data-root settings but only performs basic validation and storage.
- [x] `src-tauri/src/state.rs` | viable: `yes` | packet: `MR-13,MR-14` | note: owns dataset retention, resample or alignment invalidation, live-bar mutation rules, and timeframe-inference heuristics.
- [x] `src-tauri/src/validation_common.rs` | viable: `yes` | packet: `MR-08,MR-11,MR-13` | note: defines cross-ticker aggregation math, hard gates, Pareto filtering, and downstream robust-score folding after cross-ticker aggregation.

### `C2` `src-tauri` main command surfaces

- [x] `src-tauri/src/commands_analysis.rs` | viable: `support_only` | packet: `none` | note: Tauri shims into app-core drilldown and local-stability functions with no in-file ranking or statistical math.
- [x] `src-tauri/src/commands_artifacts.rs` | viable: `support_only` | packet: `none` | note: pure artifact persistence and replay wrappers over run_artifacts without math semantics.
- [x] `src-tauri/src/commands_campaign.rs` | viable: `yes` | packet: `MR-11,MR-14` | note: owns campaign holdout splitting, validation-admission math, preload-policy economics, exact-GpuKernel prefetch eligibility, and per-dataset prefetch or runtime diagnostics.
- [x] `src-tauri/src/commands_data.rs` | viable: `support_only` | packet: `none` | note: data, cache, and live-feed command surface where packet-level math lives elsewhere.
- [x] `src-tauri/src/commands_dynamic.rs` | viable: `support_only` | packet: `MR-06` | note: user-facing IR preview and constant-bit encoding only; executable algebra lives in strategy_dynamic and GPU runtime code.
- [x] `src-tauri/src/commands_execution.rs` | viable: `yes` | packet: `MR-14` | note: owns live execution sizing and economics such as quantity rounding, delta targeting, and bracket price translation from percentage risk inputs.
- [x] `src-tauri/src/commands_meta.rs` | viable: `yes` | packet: `MR-11,MR-12,MR-14` | note: contains holdout quantiles and summaries, Stage 5 blended reranking, staleness gating, ML-confidence policy evaluation, and metric-surface comparability warnings or summary materialization.
- [x] `src-tauri/src/commands_ml_signal.rs` | viable: `support_only` | packet: `none` | note: command-layer ML-signal plumbing and manifest normalization that should be provided as context for MR-17 even though math ownership remains in app-core.
- [x] `src-tauri/src/commands_optimize.rs` | viable: `yes` | packet: `MR-04` | note: command-boundary mode normalization coerces output-heavy requests back to Grid and directly affects route and cost behavior.
- [x] `src-tauri/src/commands_validation.rs` | viable: `yes` | packet: `MR-13` | note: aligns cross-ticker horizons, aggregates success-only cross-dataset evidence, filters finalists, and rebuilds validation verdicts.

### `C3` `src-tauri` reports/options/system commands

- [x] `src-tauri/src/commands_options_data.rs` | viable: `support_only` | packet: `none` | note: imports and normalizes options datasets but does not own options ranking or validation statistics.
- [x] `src-tauri/src/commands_options_expression.rs` | viable: `yes` | packet: `MR-16` | note: owns options-expression replay and provenance validation plus the Tauri bridge into options evaluation and inspection flows.
- [x] `src-tauri/src/commands_reports.rs` | viable: `support_only` | packet: `none` | note: live path mostly dispatches to report submodules; substantive analysis workbook logic is legacy/reference only.
- [x] `src-tauri/src/commands_reports_api.rs` | viable: `support_only` | packet: `none` | note: async Tauri transport wrappers for report operations with no standalone report-math ownership.
- [x] `src-tauri/src/commands_reports_csv.rs` | viable: `support_only` | packet: `none` | note: serializes precomputed shortlist rows to CSV and does not define the underlying scoring or statistical semantics.
- [x] `src-tauri/src/commands_reports_meta_xlsx.rs` | viable: `yes` | packet: `MR-11` | note: computes meta-report workbook aggregates including per-dataset medians and histograms over meta-selection summaries.
- [x] `src-tauri/src/commands_reports_shared.rs` | viable: `support_only` | packet: `none` | note: report-path and shell-open plumbing only; quantile helper is test-only and not a live math surface.
- [x] `src-tauri/src/commands_reports_validation_common.rs` | viable: `yes` | packet: `MR-08,MR-09,MR-10,MR-13` | note: builds validation export semantics including approval, bootstrap, CSCV, reopt, plateau, and cross-ticker aggregates.
- [x] `src-tauri/src/commands_reports_validation_xlsx.rs` | viable: `support_only` | packet: `none` | note: renders workbook sheets from precomputed validation export data and does not own underlying validation math.
- [x] `src-tauri/src/commands_system.rs` | viable: `no` | packet: `none` | note: GPU diagnostics and OS window-effect plumbing only, with no trading, validation, or report-math surface.

### `C4` `src-tauri` bin surfaces

- [x] `src-tauri/src/bin/ta_app_path_faceoff.rs` | viable: `yes` | packet: `MR-04,MR-14` | note: counts valid fast or slow pairs and summarizes cold or warm route, fallback, and timing economics for app-path runs.
- [x] `src-tauri/src/bin/ta_campaign_perf.rs` | viable: `yes` | packet: `MR-14` | note: campaign perf harness averages preload and runtime costs and captures per-dataset prepare, prefetch, optimize, and validate diagnostics.
- [x] `src-tauri/src/bin/ta_double_ma_validation_cost.rs` | viable: `yes` | packet: `MR-09,MR-10,MR-13` | note: measures walk-forward, CSCV or PBO, SPA, reopt, bootstrap, and cross-ticker validation modules with cost ratios and feasibility summaries.
- [x] `src-tauri/src/bin/ta_headless_validate.rs` | viable: `support_only` | packet: `none` | note: manual optimize-to-validate wrapper forwarding validation knobs to app_core without adding new formulas.
- [x] `src-tauri/src/bin/ta_options_dataset_download.rs` | viable: `support_only` | packet: `none` | note: thin Tauri-linked CLI around provider dataset download with no local ranking, split, or metric logic.
- [x] `src-tauri/src/bin/ta_options_dataset_import.rs` | viable: `support_only` | packet: `none` | note: imports normalized options CSVs and parses price-semantics enums, while options evaluation math lives in app_core.
- [x] `src-tauri/src/bin/ta_options_expression_compare.rs` | viable: `yes` | packet: `MR-16` | note: compares saved options-expression artifacts using matched-tuple robust-score and expectancy deltas across selection, holdout, and stress lanes.
- [x] `src-tauri/src/bin/ta_options_expression_feasibility.rs` | viable: `yes` | packet: `MR-16` | note: owns Phase 0.5 gates for density, invalid-slot rate, cache reuse, GPU speedup, and backend recommendation quality.
- [x] `src-tauri/src/bin/ta_options_expression_smoke.rs` | viable: `yes` | packet: `MR-16` | note: builds synthetic options event data, checks density and holdout or stress coverage invariants, and benchmarks backend speedups.
- [x] `src-tauri/src/bin/ta_options_parent_artifact.rs` | viable: `support_only` | packet: `none` | note: bounded DoubleMa artifact generator for the options follow-up path with no local options or validation math.
- [x] `src-tauri/src/bin/ta_profile_route_matrix.rs` | viable: `yes` | packet: `MR-04,MR-14` | note: audits expected versus observed backend routes and summarizes wall or runtime evidence for route-policy regret.
- [x] `src-tauri/src/bin/ta_refactor_snapshots.rs` | viable: `support_only` | packet: `none` | note: fixture and snapshot generator that canonicalizes serialized artifacts while underlying statistics remain elsewhere.
- [x] `src-tauri/src/bin/ta_workflow_regression.rs` | viable: `yes` | packet: `MR-11` | note: deterministic workflow gate aggregates meta and holdout quantiles or pass rates and enforces stage-budget regression checks.
