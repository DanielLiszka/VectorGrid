# Master Math Review Tracker

Date: `2026-04-17`

## Purpose

This document tracks repo-level opportunities where a higher-level mathematical review could:

- materially improve performance
- preserve or improve correctness through stronger invariants
- widen useful feature scope with low marginal compute cost
- replace repeated recomputation with derivation, reuse, or better factorization
- improve backend route policy through explicit cost models instead of heuristic drift

This is not a generic cleanup list. Each target here should justify a math-focused review with:

- a concrete execution surface
- a specific mathematical or statistical question
- a plausible payoff class
- a measurable verification plan

## Review Standard

Every target should eventually be evaluated against these questions:

1. What are the minimal primitive quantities that must be computed exactly?
2. What outputs can be derived from those primitives without replay or redundant passes?
3. What grouping, factorization, or reuse structure already exists but is underexploited?
4. What cost model governs the current implementation?
5. What correctness invariants must remain fixed?
6. What benchmark, parity, and gate evidence would prove the proposal is worth keeping?

## Payoff Classes

- `metric_contract_expansion`
- `prepared_state_reuse`
- `single_pass_statistics`
- `partial_selection_or_order_statistics`
- `candidate_space_factorization`
- `route_policy_cost_model`
- `gpu_residency_or_transfer_math`
- `scoring_or_ranking_math`
- `validation_or_holdout_statistics`
- `feature_expansion_via_existing_primitives`

## Evidence Classes

- `performance`
- `correctness`
- `feature_expansion`
- `route_transparency`
- `artifact_or_provenance_consistency`

## Status Legend

- `inventory`
- `candidate`
- `prompt_ready`
- `analysis_in_progress`
- `analysis_complete`
- `accepted`
- `rejected`
- `deferred`

## Initial Scan Scope

The initial backend scan covers:

- `src-tauri/src/**/*.rs`
- `crates/app_core/src/**/*.rs`
- `crates/strategies/src/**/*.rs`
- `crates/optimizer/src/**/*.rs`

## Second-Pass Audit Snapshot

- reviewed backend Rust files: `136`
- viable math-review files: `65`
- support-only context files: `59`
- non-viable files: `12`
- current packet inventory: `MR-01` through `MR-17`

Primary files and concrete files listed below are anchor/context lists, not exhaustive packet coverage. Exhaustive packet coverage lives in:

- [BACKEND_FILE_MATH_REVIEW_CHECKLIST.md](</C:/Rust Projects/ta_desktop_demo/Math Review/BACKEND_FILE_MATH_REVIEW_CHECKLIST.md>)
- [MATH_REVIEW_PROMPT_PACKETS.md](</C:/Rust Projects/ta_desktop_demo/Math Review/MATH_REVIEW_PROMPT_PACKETS.md>)

Execution note:

- `MR-*` remains the exhaustive coverage and synthesis inventory.
- `MATH_REVIEW_PROMPT_PACKETS.md` now executes through parallel-safe `PR-*`
  prompts that crosswalk back onto the `MR-*` inventory after response
  collection.

## Target Inventory

| ID | Target | Primary files | Payoff classes | Evidence classes | Why math review is justified | Status |
| --- | --- | --- | --- | --- | --- | --- |
| `MR-01` | GPU metric-contract lattice and sufficient-stat bundles | `crates/app_core/src/gpu_backtest_kernel.rs`, `crates/app_core/src/double_ma.rs`, `crates/strategies/src/double_ma/mod.rs` | `metric_contract_expansion`, `single_pass_statistics`, `feature_expansion_via_existing_primitives` | `performance`, `correctness`, `feature_expansion` | Current GPU search/output contracts are organized around named row counts (`1/3/7/10/17/22`) rather than minimal sufficient-stat bundles; this is the exact class of problem where a math review can remove replay and widen exact downstream work at low marginal cost. | `prompt_ready` |
| `MR-02` | Score-first top-k scalarization and confidence reranking basis | `crates/app_core/src/gpu_backtest_kernel.rs`, `crates/app_core/src/double_ma.rs`, `crates/app_core/src/validation_stats.rs` | `single_pass_statistics`, `scoring_or_ranking_math` | `performance`, `correctness` | Rich score modes currently widen contracts and disable fast shortlist paths even when the score may be derivable from a smaller basis; confidence reranking also relies on a narrow late-stage stat set. | `prompt_ready` |
| `MR-03` | Shortlist robustness algebra instead of replay-heavy slice recompute | `crates/app_core/src/gpu_backtest_kernel.rs`, `crates/app_core/src/double_ma.rs`, `crates/strategies/src/double_ma/mod.rs` | `prepared_state_reuse`, `single_pass_statistics` | `performance`, `correctness` | Slice and checkpoint robustness currently pay replay-like costs that may be reducible with prefix, segment, or checkpoint algebra. | `prompt_ready` |
| `MR-04` | Candidate-space factorization and route-policy cost modeling | `crates/app_core/src/double_ma.rs`, `crates/app_core/src/ma_policy.rs`, `crates/app_core/src/strategy_plan.rs`, `crates/app_core/src/double_ma_backend.rs`, `src-tauri/src/commands_optimize.rs`, `src-tauri/src/run_artifacts.rs` | `candidate_space_factorization`, `route_policy_cost_model` | `performance`, `route_transparency`, `feature_expansion` | Pair-space generation and backend routing still rely heavily on Cartesian products plus mostly static policy; this is fertile ground for mathematical workload planning rather than incremental heuristics. | `prompt_ready` |
| `MR-05` | Dynamic GPU market-state unification and residency economics | `crates/app_core/src/dynamic_indicator_adapter.rs`, `crates/app_core/src/gpu_backtest_kernel.rs`, `crates/app_core/src/double_ma.rs`, `crates/app_core/src/vram_ma.rs` | `gpu_residency_or_transfer_math`, `prepared_state_reuse` | `performance`, `feature_expansion` | The dynamic indicator path and the kernel path maintain separate residency/caching economics; a higher-level math review can unify what must stay resident, what can be derived, and when uploads are economically justified. | `prompt_ready` |
| `MR-06` | Dynamic GPU executable algebra widening | `crates/app_core/src/strategy_dynamic.rs`, `crates/app_core/src/dynamic_indicator_adapter.rs`, `crates/app_core/src/strategy_ir.rs`, `crates/app_core/src/strategy_gpu_program.rs`, `crates/app_core/src/strategy_gpu_runtime.rs`, `crates/app_core/src/strategy_plan.rs` | `feature_expansion_via_existing_primitives`, `gpu_residency_or_transfer_math` | `performance`, `feature_expansion`, `correctness` | Dynamic GPU v1 currently supports a narrow executable algebra; a math review could widen expressivity via scalar lowering, bitmask contracts, or alternative on-device representations instead of CPU fallback. | `prompt_ready` |
| `MR-07` | Canonical risk and trade analytics semantics | `crates/app_core/src/double_ma.rs`, `crates/strategies/src/double_ma/mod.rs`, `crates/strategies/src/records.rs`, `crates/strategies/src/composite_cpu/mod.rs`, `crates/optimizer/src/lib.rs` | `single_pass_statistics`, `feature_expansion_via_existing_primitives` | `correctness`, `feature_expansion` | Risk and trade metrics are central, duplicated in places, and currently omit some high-value analytics that could be derived cheaply if the canonical statistic basis is clarified. | `prompt_ready` |
| `MR-08` | Validation robust-score redesign | `crates/app_core/src/validation_pipeline.rs`, `crates/app_core/src/validation_shared.rs`, `crates/app_core/src/meta_select.rs` | `scoring_or_ranking_math`, `validation_or_holdout_statistics` | `correctness`, `performance` | A fixed hand-tuned linear robust score feeds multiple downstream validators and meta ranking; a better mathematical combiner could improve the whole stack. | `prompt_ready` |
| `MR-09` | Probability, bootstrap, and superiority-test rigor | `crates/app_core/src/validation_pipeline.rs`, `crates/app_core/src/validation_stats.rs`, `crates/app_core/src/sharpe_stats.rs`, `crates/app_core/src/validation_shared.rs` | `validation_or_holdout_statistics` | `correctness`, `feature_expansion` | PSR/DSR, bootstrap, and SPA-style statistics currently use simplified assumptions that are mathematically reviewable without changing product scope. | `prompt_ready` |
| `MR-10` | Reopt walk-forward and regime-validation mathematics | `crates/app_core/src/validation_pipeline.rs`, `crates/app_core/src/double_ma_validate.rs`, `crates/app_core/src/composite_validate.rs`, `crates/app_core/src/validation_shared.rs` | `validation_or_holdout_statistics`, `feature_expansion_via_existing_primitives` | `correctness`, `feature_expansion` | Reoptimization and regime analysis already compute rich path data but summarize it with coarse heuristics and bucketization. | `prompt_ready` |
| `MR-11` | Meta-select shrinkage, quantile stability, and holdout economics | `crates/app_core/src/meta_select.rs`, `src-tauri/src/commands_meta.rs`, `src-tauri/src/validation_common.rs`, `crates/app_core/src/double_ma_validate.rs`, `crates/app_core/src/composite_validate.rs` | `partial_selection_or_order_statistics`, `validation_or_holdout_statistics`, `route_policy_cost_model` | `correctness`, `performance`, `artifact_or_provenance_consistency` | Meta ranking, shortlist admission, and holdout budgeting are all using point estimates and static weights where uncertainty-aware math could materially improve both ranking quality and compute allocation. | `prompt_ready` |
| `MR-12` | ML-confidence labels, calibration, and policy-evaluation math | `crates/app_core/src/ml_confidence.rs`, `crates/app_core/src/meta_select.rs`, `src-tauri/src/commands_meta.rs` | `scoring_or_ranking_math`, `validation_or_holdout_statistics` | `correctness`, `feature_expansion` | The ML lane collapses rich holdout outcomes into coarse labels and then evaluates policies mainly through narrow precision-based summaries. | `prompt_ready` |
| `MR-13` | Cross-ticker robustness and dataset-alignment math | `src-tauri/src/validation_common.rs`, `src-tauri/src/commands_validation.rs`, `src-tauri/src/state.rs`, `crates/app_core/src/validation_shared.rs`, `crates/app_core/src/double_ma_validate.rs`, `crates/app_core/src/composite_validate.rs` | `validation_or_holdout_statistics`, `route_policy_cost_model` | `correctness`, `feature_expansion` | Cross-ticker aggregation currently uses simple scoring and alignment rules that do not fully account for coverage, overlap, or missing-data economics. | `prompt_ready` |
| `MR-14` | Metric-surface fidelity, route regret, and execution/cache economics | `src-tauri/src/metric_surface.rs`, `src-tauri/src/run_artifacts.rs`, `src-tauri/src/commands_execution.rs`, `src-tauri/src/state.rs`, `src-tauri/src/data_cache.rs` | `route_policy_cost_model`, `feature_expansion_via_existing_primitives` | `route_transparency`, `performance`, `correctness` | Several product-facing backend decisions are still heuristic: metric comparability, route provenance, execution sizing, dataset retention, and timeframe inference all invite explicit mathematical treatment. | `prompt_ready` |
| `MR-15` | Composite scoring, grid/stream scalarization, heatmap stability, and MO search math | `crates/strategies/src/composite_cpu/mod.rs`, `crates/optimizer/src/lib.rs`, `crates/optimizer/src/grid.rs`, `crates/optimizer/src/stream.rs`, `crates/optimizer/src/mo.rs` | `scoring_or_ranking_math`, `feature_expansion_via_existing_primitives`, `candidate_space_factorization` | `performance`, `feature_expansion`, `correctness` | Shared optimizer and composite surfaces currently rely on raw scores, hard gates, simple heatmaps, and 2D MO heuristics where mathematical review could improve stability and unlock new low-cost product capability. | `prompt_ready` |
| `MR-16` | Options split rigor, ranking economics, and device-side tail summaries | `crates/app_core/src/options_expression.rs`, `crates/app_core/src/options_expression_gpu.rs`, `crates/app_core/src/options_signal_manifest.rs`, `crates/app_core/src/options_types.rs`, `crates/app_core/src/kernels/options_expression.cu` | `validation_or_holdout_statistics`, `single_pass_statistics`, `gpu_residency_or_transfer_math` | `correctness`, `performance`, `feature_expansion` | The options lane already computes richer moments than it currently ranks on, and the GPU scorer is a natural place to maintain stronger tail-aware summaries that could reduce CPU exactification. | `prompt_ready` |
| `MR-17` | ML-signal export, score-cache calibration, and threshold-policy sweep math | `crates/app_core/src/ml_signal.rs`, `src-tauri/src/commands_ml_signal.rs`, `crates/app_core/src/validation_pipeline.rs`, `crates/app_core/src/iterative_sim.rs` | `validation_or_holdout_statistics`, `scoring_or_ranking_math`, `feature_expansion_via_existing_primitives` | `correctness`, `performance`, `feature_expansion` | The ML-signal lane already owns label construction, calibration transforms, threshold sweeps, and overlay execution semantics, making it a direct math-review surface rather than a thin command wrapper. | `prompt_ready` |

## Findings By Surface

### `MR-01` GPU metric-contract lattice and sufficient-stat bundles

- Current pattern:
  `gpu_backtest_kernel.rs` exposes hard metric row contracts and downstream features widen rows or force replay when they need more than the current contract carries.
- Math-review angle:
  Replace named row widths with sufficient-stat bundles for return moments, drawdown-path summaries, and trade-tail summaries so downstream features request mathematical content rather than a specific row count.
- Concrete files:
  `crates/app_core/src/gpu_backtest_kernel.rs`
  `crates/app_core/src/double_ma.rs`
  `crates/strategies/src/double_ma/mod.rs`

### `MR-02` Score-first top-k scalarization and confidence reranking basis

- Current pattern:
  Composite score modes force richer metric floors and disable score-first top-k even when the score could be expressed from a smaller statistic basis; confidence reranking uses limited retained moments.
- Math-review angle:
  Derive minimal per-objective statistic bases and on-device scalarization contracts; define the true sufficient statistics for PSR/DSR-like reranking.
- Concrete files:
  `crates/app_core/src/gpu_backtest_kernel.rs`
  `crates/app_core/src/double_ma.rs`
  `crates/app_core/src/validation_stats.rs`

### `MR-03` Shortlist robustness algebra instead of replay-heavy slice recompute

- Current pattern:
  Slice robustness and related shortlist diagnostics re-evaluate shortlisted combinations repeatedly across ranges.
- Math-review angle:
  Investigate prefix/path/checkpoint algebra or other segment mathematics that can replace repeated full re-evaluation.
- Concrete files:
  `crates/app_core/src/gpu_backtest_kernel.rs`
  `crates/app_core/src/double_ma.rs`
  `crates/strategies/src/double_ma/mod.rs`

### `MR-04` Candidate-space factorization and route-policy cost modeling

- Current pattern:
  Pair generation still starts from large Cartesian products; backend auto-routing and optimization-mode normalization are mostly heuristic.
- Math-review angle:
  Build mathematically meaningful parameter families, dominance-aware fronts, and workload cost models that include pair count, bar count, metric contract, replay needs, and residency state.
- Concrete files:
  `crates/app_core/src/double_ma.rs`
  `crates/app_core/src/ma_policy.rs`
  `crates/app_core/src/strategy_plan.rs`
  `crates/app_core/src/double_ma_backend.rs`
  `src-tauri/src/commands_optimize.rs`
  `src-tauri/src/run_artifacts.rs`

### `MR-05` Dynamic GPU market-state unification and residency economics

- Current pattern:
  The generic dynamic indicator path and the kernel path keep separate dataset caches, return buffers, and residency decisions.
- Math-review angle:
  Define one market-state ownership and derivation model for uploads, warmup semantics, derived returns, and reusability across indicator and backtest execution.
- Concrete files:
  `crates/app_core/src/dynamic_indicator_adapter.rs`
  `crates/app_core/src/gpu_backtest_kernel.rs`
  `crates/app_core/src/double_ma.rs`
  `crates/app_core/src/vram_ma.rs`

### `MR-06` Dynamic GPU executable algebra widening

- Current pattern:
  Dynamic GPU v1 is intentionally narrow: two MA slots, floating scalar outputs, simple operand classes, and constrained device chaining.
- Math-review angle:
  Explore scalar lowering, bitmask encodings, and other representations that widen the executable algebra without forcing CPU fallback.
- Concrete files:
  `crates/app_core/src/strategy_dynamic.rs`
  `crates/app_core/src/dynamic_indicator_adapter.rs`
  `crates/app_core/src/strategy_ir.rs`
  `crates/app_core/src/strategy_gpu_program.rs`
  `crates/app_core/src/strategy_gpu_runtime.rs`
  `crates/app_core/src/strategy_plan.rs`

### `MR-07` Canonical risk and trade analytics semantics

- Current pattern:
  Risk finalizers exist in more than one place, and trade analytics still emphasize counts and averages over richer distributional summaries.
- Math-review angle:
  Define one canonical risk/trade statistic basis and identify high-value derived outputs such as SQN, payoff-ratio families, uncertainty bands, duration-aware expectancy, and tail-aware trade summaries.
- Concrete files:
  `crates/app_core/src/double_ma.rs`
  `crates/strategies/src/double_ma/mod.rs`
  `crates/strategies/src/records.rs`
  `crates/strategies/src/composite_cpu/mod.rs`
  `crates/optimizer/src/lib.rs`

### `MR-08` Validation robust-score redesign

- Current pattern:
  `validation_pipeline.rs` relies on a fixed linear robust score that propagates into robust Pareto, CSCV, and meta ranking.
- Math-review angle:
  Replace it with a monotone certainty-equivalent, calibrated combiner, or another mathematically justified utility that explicitly handles uncertainty and window count.
- Concrete files:
  `crates/app_core/src/validation_pipeline.rs`
  `crates/app_core/src/validation_shared.rs`
  `crates/app_core/src/meta_select.rs`

### `MR-09` Probability, bootstrap, and superiority-test rigor

- Current pattern:
  PSR/DSR use simplified assumptions, bootstrap uses heuristic block lengths and coarse outputs, and SPA-style testing is a lightweight approximation.
- Math-review angle:
  Add effective sample size, empirical higher moments, dependence-aware studentization, stronger multiple-testing control, and richer bootstrap outputs such as Sharpe, CVaR, or ruin probability.
- Concrete files:
  `crates/app_core/src/validation_pipeline.rs`
  `crates/app_core/src/validation_stats.rs`
  `crates/app_core/src/sharpe_stats.rs`
  `crates/app_core/src/validation_shared.rs`

### `MR-10` Reopt walk-forward and regime-validation mathematics

- Current pattern:
  Reoptimization summarizes winner churn coarsely; regime validation relies on tertiles, median splits, and fixed checkpoint partitions.
- Math-review angle:
  Add regret distributions, switching-cost-aware utility, parameter-space distance, latent-regime or change-point methods, and multiscale segmentation.
- Concrete files:
  `crates/app_core/src/validation_pipeline.rs`
  `crates/app_core/src/double_ma_validate.rs`
  `crates/app_core/src/composite_validate.rs`
  `crates/app_core/src/validation_shared.rs`

### `MR-11` Meta-select shrinkage, quantile stability, and holdout economics

- Current pattern:
  Meta ranking uses static weights on point estimates; shortlist admission uses hard cliffs; holdout batches are chosen by simple top-n plus “nearest 0.5” exploration and currently rerun on CPU-only.
- Math-review angle:
  Use hierarchical shrinkage, uncertainty-aware gates, better quantile estimators, and value-of-information or bandit-style holdout budget allocation.
- Concrete files:
  `crates/app_core/src/meta_select.rs`
  `src-tauri/src/commands_meta.rs`
  `src-tauri/src/validation_common.rs`
  `crates/app_core/src/double_ma_validate.rs`
  `crates/app_core/src/composite_validate.rs`

### `MR-12` ML-confidence labels, calibration, and policy-evaluation math

- Current pattern:
  Rich holdout outcomes become binary labels; training and calibration remain relatively lightweight; policy evaluation emphasizes precision-style metrics.
- Math-review angle:
  Move to continuous or ordinal targets, better calibration, time-aware evaluation, class-imbalance-aware training, and broader policy metrics such as Brier, ECE/MCE, recall/coverage, and regret.
- Concrete files:
  `crates/app_core/src/ml_confidence.rs`
  `crates/app_core/src/meta_select.rs`
  `src-tauri/src/commands_meta.rs`

### `MR-13` Cross-ticker robustness and dataset-alignment math

- Current pattern:
  Cross-ticker aggregation uses simple score formulas and hard gates; alignment falls back to base-range crop or last-N bars and missing reruns are not treated as explicit negative evidence.
- Math-review angle:
  Add overlap ratio, effective sample size, common-calendar alignment, coverage-aware utility, and missing-dataset penalties or shrinkage.
- Concrete files:
  `src-tauri/src/validation_common.rs`
  `src-tauri/src/commands_validation.rs`
  `src-tauri/src/state.rs`
  `crates/app_core/src/validation_shared.rs`
  `crates/app_core/src/double_ma_validate.rs`
  `crates/app_core/src/composite_validate.rs`

### `MR-14` Metric-surface fidelity, route regret, and execution/cache economics

- Current pattern:
  Metric-surface kind is partly heuristic, route provenance is qualitative, execution sizing is simple sign times quantity with fixed bracket percentages, dataset retention is LRU-like, and timeframe inference uses median-gap heuristics.
- Math-review angle:
  Turn these into explicit cost or fidelity models: metric comparability schemas, route regret summaries, volatility-aware execution sizing, value-based retention, and confidence-scored timeframe inference.
- Concrete files:
  `src-tauri/src/metric_surface.rs`
  `src-tauri/src/run_artifacts.rs`
  `src-tauri/src/commands_execution.rs`
  `src-tauri/src/state.rs`
  `src-tauri/src/data_cache.rs`

### `MR-15` Composite scoring, grid/stream scalarization, heatmap stability, and MO search math

- Current pattern:
  Composite logic uses hard sign gates and persistence heuristics; grid/stream use raw scores; heatmaps emphasize maxima; MO search uses 2D-specific domination, crowding, HV, and QMC heuristics.
- Math-review angle:
  Introduce evidence-style composite scoring, ranking shrinkage, occupancy-aware or variance-aware heatmaps, and mathematically reviewed MO domination, hypervolume weighting, and discrete low-discrepancy sampling.
- Concrete files:
  `crates/strategies/src/composite_cpu/mod.rs`
  `crates/optimizer/src/lib.rs`
  `crates/optimizer/src/grid.rs`
  `crates/optimizer/src/stream.rs`
  `crates/optimizer/src/mo.rs`

### `MR-16` Options split rigor, ranking economics, and device-side tail summaries

- Current pattern:
  Options splits are chronological without purge/embargo; ranking relies heavily on expectancy-per-premium; GPU scoring keeps limited moments and CPU exactifies shortlist tails later.
- Math-review angle:
  Add purged/embargoed splits, certainty-equivalent or tail-aware ranking, shrinkage for low-trade tuples, and device-side quantile/CVaR/confidence summaries to reduce exactify volume.
- Concrete files:
  `crates/app_core/src/options_expression.rs`
  `crates/app_core/src/options_expression_gpu.rs`
  `crates/app_core/src/options_signal_manifest.rs`
  `crates/app_core/src/options_types.rs`
  `crates/app_core/src/kernels/options_expression.cu`

### `MR-17` ML-signal export, score-cache calibration, and threshold-policy sweep math

- Current pattern:
  `ml_signal.rs` owns label construction, calibration transforms, sweep policies, and overlay execution semantics, but it was previously omitted from the packet inventory because the first pass treated the surrounding command layer as the primary review surface.
- Math-review angle:
  Review the ML-signal lane as its own mathematical product surface: label deadbands, split-window construction, calibration interpolation, score-cache coverage rules, threshold/delta policy geometry, persistence/cooldown overlays, and sweep-budget truncation.
- Concrete files:
  `crates/app_core/src/ml_signal.rs`
  `src-tauri/src/commands_ml_signal.rs`
  `crates/app_core/src/validation_pipeline.rs`
  `crates/app_core/src/iterative_sim.rs`

## Current Priority Order

Initial high-yield order for GPT-driven math review:

1. `MR-01` GPU metric-contract lattice and sufficient-stat bundles
2. `MR-04` Candidate-space factorization and route-policy cost modeling
3. `MR-05` Dynamic GPU market-state unification and residency economics
4. `MR-11` Meta-select shrinkage, quantile stability, and holdout economics
5. `MR-12` ML-confidence labels, calibration, and policy-evaluation math
6. `MR-17` ML-signal export, score-cache calibration, and threshold-policy sweep math
7. `MR-16` Options split rigor, ranking economics, and device-side tail summaries
8. `MR-08` Validation robust-score redesign
9. `MR-15` Composite scoring, grid/stream scalarization, heatmap stability, and MO search math

## Prompt Handoff Checklist

Before a target moves to `prompt_ready`, capture:

- exact file list to provide as context
- current behavior and invariants to preserve
- candidate mathematical review angle
- benchmark or parity surface to use after implementation
- kill criteria if the proposed math does not materially improve repo economics

## Notes

- This tracker is intentionally repo-grounded. Abstract mathematical ideas that do not map to concrete files or verification surfaces should not be promoted here.
- Feature-expansion targets are valid if the feature can be justified as low-marginal-cost work derived from already-computed primitives.
