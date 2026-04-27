# GPT-5.5 Pro hard-problem atlas

Status: first meta-analysis pass  
Created: 2026-04-27  
Precursor plan: `docs/gpt55_pro_hard_problem_meta_research_plan_2026-04-27.md`

## Purpose

This document is the first repo-wide and web-research-backed analysis of which VectorGrid problems are complex enough to justify GPT-5.5 Pro dossier treatment.

It does not replace `docs/REPO_MAP.md`, and it is not an implementation backlog. It is an atlas of hard questions, source-backed constraints, local file packs, and recommended next dossiers.

## Method

Local analysis used:

- `AGENTS.md`
- `docs/REPO_MAP.md`
- `docs/ARCHITECTURE_NOTES.md`
- `docs/options_research/options_expression_optimizer_viable_plan_2026-04-02.md`
- `docs/options_research/options_expression_future_scope_roadmap_2026-04-07.md`
- targeted owner files under `crates/app_core`, `src-tauri`, `web`, and `scripts`
- three read-only repo-analysis sub-agents covering GPU/runtime/validation, options/data/provenance, and ML/meta/live/frontend surfaces

External research used official vendor docs and primary research where possible. The source list is at the end of this file.

## Executive Verdict

The best GPT-5.5 Pro targets are not generic feature ideas. They are problems where finance semantics, numerical validity, GPU architecture, and repo-specific constraints interact.

Highest-value first dossiers:

1. GPU-resident validation, top-k/reduction, and optimization-pass enrichment.
2. Options-expression objective/ranking redesign, advanced fixed-geometry options feasibility, and quote-quality admission.
3. Validation evidence theory across PSR/DSR, PBO/CSCV, walk-forward windows, and ML/meta-selection leakage.
4. CUDA runtime residency ownership, launch overhead, and CUDA Graphs viability.
5. Tensor-core and RT-core prove-or-kill hardware dossiers.

The current research suggests:

- CUDA Graphs and CUB top-k/reduction are plausible near-term research targets because the repo already has repeated GPU stages, top-k limits, heatmaps, row-first/score-first selection, and perf gates.
- Tensor cores should not be pursued until the repo identifies a real dense or block-structured workload, such as batched model scoring, covariance/correlation, Monte Carlo blocks, or large matrix transforms.
- RT cores and OptiX are a low-probability but bounded prove-or-kill lane. They only look relevant if a VectorGrid problem can be expressed as ray/BVH traversal or interval/path intersection.
- Advanced options strategy expansion is a strong GPT-5.5 Pro candidate, but only inside the repo's fixed-shape, same-expiry, compiled-tensor philosophy.
- Validation/statistical evidence may be as valuable as raw speed because high-throughput optimization increases overfitting risk.

## External Research Findings

### CUDA, Graphs, And Runtime Shape

NVIDIA's CUDA guide is the primary source for CUDA features, including CUDA Graphs, cooperative groups, stream-ordered allocation, and dynamic parallelism. The guide frames CUDA Graphs as a first-class feature for capturing and replaying sequences of GPU work.

Repo implication:

- A CUDA Graph dossier is justified only for repeated, stable launch graphs in `gpu_backtest_kernel.rs`, `options_expression_gpu.rs`, or dynamic strategy runtimes.
- It should measure launch overhead separately from kernel execution, memory allocation, host sync, and data transfer.
- It should not be assumed useful for highly variable request shapes or paths with frequent fallback.

Primary local files:

- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/options_expression_gpu.rs`
- `crates/app_core/src/strategy_gpu_runtime.rs`
- `crates/app_core/src/standalone_gpu_runtime.rs`
- `scripts/profile_gpu_backtest_ncu.ps1`
- `scripts/profile_gpu_backtest_nsys.ps1`
- `scripts/profile_options_expression_ncu.ps1`
- `scripts/profile_options_expression_nsys.ps1`

### CUB, Top-K, Reductions, And Determinism

CUB provides device-wide primitives such as sort, scan, reduce, histogram, selection, and top-k. Current CUB `DeviceTopK` exists, but its documentation explicitly notes unordered output and possible nondeterminism for ties unless the caller accepts that behavior.

Repo implication:

- CUB can be a strong candidate for top-k, candidate compaction, and reduction experiments.
- A GPT-5.5 Pro dossier should require deterministic tie-breaking or a post-pass sort/tie canonicalization plan before product adoption.
- This maps directly to existing `top_k`, `include_all`, heatmap, row-score, score-first, Pareto, and shortlist semantics.

Primary local files:

- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/double_ma.rs`
- `crates/optimizer/src/stream.rs`
- `crates/optimizer/src/grid.rs`
- `crates/optimizer/src/mo.rs`

### Tensor Cores

NVIDIA documents tensor-core style warp matrix operations as matrix multiply-accumulate work, and CUTLASS is explicitly built around high-performance GEMM and related dense linear algebra. CUTLASS also has Windows build caveats in its current docs.

Repo implication:

- Tensor cores should be treated as a workload-discovery problem first.
- Good candidates: batched ML scoring, covariance/correlation matrices, dense feature transforms, large Monte Carlo blocks, or explicit matrix methods.
- Weak candidates: branch-heavy trade simulation, irregular options chain selection, per-candidate control-flow-heavy backtests.

Primary local files:

- `crates/app_core/src/ml_signal.rs`
- `crates/app_core/src/ml_confidence.rs`
- `crates/app_core/src/meta_select.rs`
- `crates/app_core/src/composite_mo.rs`
- `crates/app_core/src/options_expression_gpu.rs`
- `crates/app_core/src/validation_pipeline.rs`

### OptiX And RT Cores

NVIDIA positions OptiX as a programmable GPU ray-tracing framework with a ray-tracing pipeline, single-ray shader model, acceleration structures, and RT-core acceleration.

Repo implication:

- RT cores are not a general CUDA substitute.
- The only plausible VectorGrid mapping is a problem that naturally looks like path traversal, interval intersection, range hit testing, or BVH queries.
- This should be a prove-or-kill dossier with explicit kill criteria.

Possible local files if a mapping exists:

- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_expression_gpu.rs`
- `crates/app_core/src/validation_core.rs`
- `crates/app_core/src/double_ma.rs`

### Options Data, Strategy Semantics, And Market Microstructure

Polygon/Massive documents options data through OPRA and lists historical tick-level data, trades, quotes, IV, Greeks, open interest, and underlying integration as supported API use cases. Cboe's margin material makes clear that spreads, straddles, and writer margin requirements are strategy-dependent and complex. OCC's standardized options disclosure is the primary risk context for exercise, assignment, and options suitability.

Repo implication:

- The current options roadmap is right to keep v1/v2 inside fixed-shape, same-expiry, long or debit-style structures.
- Short premium, assignment/exercise, margin, rolling, mixed-expiry calendars, and generic scripting should remain out of scope unless a future dossier proves a bounded model.
- Direct quote-aware acquisition is a separate hard problem from scoring. It needs entitlement, endpoint, timestamp, quote staleness, NBBO, and cost model research.

Primary local files:

- `docs/options_research/options_expression_optimizer_viable_plan_2026-04-02.md`
- `docs/options_research/options_expression_future_scope_roadmap_2026-04-07.md`
- `docs/options_research/options_expression_polygon_validation_hardening_implementation_plan_2026-04-25.md`
- `crates/app_core/src/options_types.rs`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_expression_gpu.rs`
- `crates/app_core/src/kernels/options_expression.cu`
- `src-tauri/src/commands_options_expression.rs`
- `src-tauri/src/commands_options_data.rs`
- `src-tauri/src/options_parent_adapters.rs`
- `web/src/stores/optionsExpressionStore.ts`
- `web/src/components/OptionsExpressionPanel.vue`

### GPU Monte Carlo For Options

NVIDIA's Monte Carlo option-pricing material supports the broad claim that option pricing can be GPU-suitable when work is path-parallel or option-parallel. It also highlights accuracy and reduction design, not just throughput.

Repo implication:

- Monte Carlo is relevant for future options research, but it is not automatically aligned with the current compiled event/outcome tensor scorer.
- A dossier should ask whether Monte Carlo belongs as a separate future options lane, a stress lane, or not at all.
- If used, reduction accuracy and CPU/GPU parity must be first-class.

### Statistical Validation And Overfitting

Bailey, Borwein, Lopez de Prado, and Zhu's PBO work directly targets backtest overfitting. Bailey and Lopez de Prado's Deflated Sharpe Ratio work directly addresses selection bias, non-normality, and many-trial performance inflation. Adaptive conformal prediction literature is relevant because time series are not generally exchangeable, which matters for ML Signal confidence and uncertainty reporting.

Repo implication:

- High-throughput optimization makes overfitting controls more important, not less.
- Validation evidence theory deserves a GPT-5.5 Pro dossier independent of raw performance work.
- Conformal or calibrated confidence should be explored for ML Signal and Stage 5 only with explicit time-series assumptions and leakage controls.

Primary local files:

- `crates/app_core/src/validation_stats.rs`
- `crates/app_core/src/cscv_pbo.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/validation_core.rs`
- `crates/app_core/src/meta_select.rs`
- `crates/app_core/src/ml_confidence.rs`
- `crates/app_core/src/ml_signal.rs`
- `web/src/services/mlSignalService.ts`
- `web/src/services/metaService.ts`

### Live Execution And Human Risk Controls

Alpaca's order docs emphasize client order IDs, order status changes, streaming order updates, buying-power checks, and bracket order behavior. NIST's AI RMF is not trading-specific, but it is useful as a governance source for risk management, measurement, monitoring, and human oversight when model outputs influence consequential actions.

Repo implication:

- Live execution expansion should not be framed as "just add adapters."
- It is a state-machine and evidence-to-action problem involving stream freshness, idempotency, partial fills, broker reconciliation, and fail-closed behavior.
- GPT-5.5 Pro is useful here only if the dossier asks for formal invariants and state transition analysis, not UI ideas.

Primary local files:

- `src-tauri/src/commands_execution.rs`
- `src-tauri/src/execution_mocks.rs`
- `src-tauri/src/state.rs`
- `web/src/services/executionService.ts`
- `web/src/stores/executionStore.ts`
- `scripts/run_execution_safety_gate.ps1`

## Coverage Matrix

| Area | Current repo signal | Hard-question candidates | GPT-5.5 Pro fit |
| --- | --- | --- | --- |
| Double-MA optimization | Main hotspot, `GpuKernel`, route diagnostics, top-k/include-all/heatmap semantics | GPU top-k, pass enrichment, launch graphs, exact-vs-heuristic pruning | High |
| CUDA runtime and residency | VRAM route, dynamic adapter, device matrices, runtime caches | ownership/lifetime model, host round-trip elimination, CUDA Graphs | High |
| Validation | Double-MA, composite, cross-ticker, PBO/PSR/DSR surfaces | GPU-resident validation, per-window evidence, tolerance theory | Very high |
| Composite profiles | Double-MA family product route for CompositeTrend, BreakoutMomentum, RegimeHybrid | ADX-bearing hybrid routes, route-policy proof, true GPU promotion | Medium-high |
| Options expression | CPU oracle, CUDA scorer, three active fixed structures, embedded validation | objective redesign, advanced fixed geometry, quote quality, provenance | Very high |
| Data and Polygon | CSV/cache, Polygon historical/live, options provider path | session-aware gaps, quote-aware acquisition, provenance, repair kill rules | High |
| ML Signal | score cache, calibration metadata, policy sweep, PSR/DSR gates | calibrated deployability, conformal abstention, leakage-safe policy sweep | High |
| Meta selection | grouped evidence, holdout, confidence, clustering | model-selection bias, Stage 5 confidence, source exclusion | High |
| Live execution | Alpaca, paper state, one stream/runner, risk policy | formal state machine, evidence-to-action boundary, adapter expansion | Medium-high |
| Artifacts/reports | shared persistence, replay, output parity | unified provenance schema, reproducible audit trail, report contract | Medium |
| Frontend orchestration | workbench, task store, long-running workflows | explicit workflow state model, uncertainty/provenance UX | Medium |
| Experimental hardware | No RT/Tensor-core product path yet | tensor-core workload discovery, OptiX prove-or-kill | Medium as research, low as implementation |

## Ranked Candidate Problems

### 1. GPU-resident validation and validation evidence theory

Priority: highest  
Expected output: architecture decision plus implementation slices

Question:

Can validation be redesigned so more candidate evaluation, perturbation/stress lanes, top-k finalist validation, and evidence summaries stay GPU-resident while preserving CPU fallback, parity, reproducibility, and reportability?

Why this needs GPT-5.5 Pro:

- It combines GPU architecture, validation statistics, numerical reproducibility, and repo route policy.
- A wrong design could make results fast but scientifically weaker.
- It needs a proof of which CPU tails are necessary versus accidental.

File pack:

- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/validation_core.rs`
- `crates/app_core/src/composite_validate.rs`
- `crates/app_core/src/validation_stats.rs`
- `crates/app_core/src/cscv_pbo.rs`
- `src-tauri/src/commands_validation.rs`
- `src-tauri/src/bin/ta_double_ma_validation_cost.rs`

External research to include:

- PBO/CSCV
- Deflated Sharpe Ratio
- GPU reduction reproducibility
- CUB reductions/top-k
- walk-forward validation literature

Verification:

- `cargo test -p ta_app_core`
- `powershell -File scripts/run_gpu_validation_perf_gate.ps1 -Mode candidate`
- `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate`
- `npm --prefix web run parity:outputs`

### 2. GPU top-k, heatmap, candidate compaction, and optimization-pass enrichment

Priority: highest  
Expected output: algorithm design with deterministic tie policy

Question:

Can optimization passes produce more useful evidence per computation pass while reducing memory traffic and CPU readbacks through fused top-k, heatmap, Pareto, candidate compaction, or early-pruning designs?

Why this needs GPT-5.5 Pro:

- It requires algorithm selection, not only code review.
- CUB `DeviceTopK` is promising but has nondeterminism caveats.
- The repo has product-visible semantics for `top_k`, `include_all`, heatmap bins, route diagnostics, and export-all behavior.

File pack:

- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/optimizer/src/stream.rs`
- `crates/optimizer/src/grid.rs`
- `crates/optimizer/src/mo.rs`
- `crates/app_core/src/composite_mo.rs`

External research to include:

- CUB `DeviceTopK`, `DeviceReduce`, `DeviceRadixSort`
- RadiK GPU radix top-k
- CUDA Graphs if launch count dominates

Verification:

- `cargo test -p ta_optimizer`
- `cargo test -p ta_app_core`
- `powershell -File scripts/run_cpu_perf_gate.ps1 -Mode candidate`
- `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`
- Nsight profiles before and after

### 3. CUDA residency ownership and launch-overhead model

Priority: high  
Expected output: feasibility verdict plus runtime ownership map

Question:

Can the repo define a safe, strategy-local GPU residency model for prepared indicators, device matrices, MA buffers, regime filters, and repeated launch shapes without creating a large generic execution layer?

Why this needs GPT-5.5 Pro:

- Pointer ownership and lifetime in GPU memory are correctness-sensitive.
- Strategy-local architecture is a repo invariant.
- CUDA Graphs may help only if launch shapes are stable enough to capture and replay.

File pack:

- `crates/app_core/src/dynamic_indicator_adapter.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/vram_ma.rs`
- `crates/app_core/src/strategy_gpu_runtime.rs`
- `crates/app_core/src/standalone_gpu_runtime.rs`
- `crates/app_core/src/strategy_gpu_program.rs`
- `src-tauri/src/state.rs`
- `src-tauri/src/data_cache.rs`

External research to include:

- CUDA Graphs
- stream-ordered allocation
- cooperative groups where relevant
- Nsight methodology

Verification:

- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel`
- `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`
- Nsight Compute/Systems captures

### 4. Options objective/ranking redesign with CPU/GPU parity

Priority: very high  
Expected output: objective function proposal plus parity constraints

Question:

What is the right objective and ranking system for options-expression tuples when search, holdout, stress, harsher stress, invalid rate, premium efficiency, liquidity, and quote realism all matter?

Why this needs GPT-5.5 Pro:

- This is finance/statistics-heavy and tightly coupled to GPU scorer shape.
- The CPU evaluator is the oracle, but CUDA scoring and host exactification must remain rank-compatible.
- Bad objective design can produce attractive but fragile results.

File pack:

- `crates/app_core/src/options_types.rs`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_expression_gpu.rs`
- `crates/app_core/src/kernels/options_expression.cu`
- `src-tauri/src/commands_options_expression.rs`
- `web/src/stores/optionsExpressionStore.ts`
- `web/src/components/OptionsExpressionPanel.vue`

External research to include:

- options backtesting methodology
- bid/ask and midpoint execution assumptions
- DSR/PBO overfitting controls
- GPU reduction accuracy

Verification:

- `cargo test -p ta_app_core options_expression`
- `powershell -File scripts/run_options_expression_gpu_promotion_gate.ps1`
- `powershell -File scripts/profile_options_expression_compute_sanitizer.ps1`
- `npm --prefix web run test -- optionsExpression`

### 5. Advanced fixed-geometry options strategy feasibility

Priority: very high  
Expected output: strategy admission matrix plus kill criteria

Question:

Which advanced options structures can be admitted while preserving same-expiry or otherwise fixed-shape tensor regularity, deterministic compilation, bounded leg counts, and provenance-safe validation?

Candidate structures:

- same-expiry call/put debit spreads beyond current verticals
- fixed-ratio ladders
- straddles and strangles
- butterflies and condors if long/debit and fixed shape
- calendars, diagonals, rolling, short premium, assignment/exercise, and margin only as likely out-of-scope or separate future dossiers

Why this needs GPT-5.5 Pro:

- The hard part is not adding enum values.
- The hard part is deciding which strategy families preserve the product's GPU-first compiled-tensor model.

File pack:

- `docs/options_research/options_expression_optimizer_viable_plan_2026-04-02.md`
- `docs/options_research/options_expression_future_scope_roadmap_2026-04-07.md`
- `crates/app_core/src/options_types.rs`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_expression_gpu.rs`
- `web/src/components/OptionsExpressionPanel.vue`

External research to include:

- OCC options risks
- Cboe margin and spread treatment
- OPRA/options quote data capabilities
- multi-leg fill assumptions

Verification:

- `powershell -File scripts/run_options_expression_phase05.ps1`
- `powershell -File scripts/run_options_expression_gpu_promotion_gate.ps1`
- CPU/GPU ranking parity tests
- tuple inspection and event-path tests

### 6. Quote-aware options data acquisition and provenance policy

Priority: high  
Expected output: provider/data-quality decision document

Question:

Can the app support direct quote-aware Polygon/Massive acquisition with enough quote-quality, endpoint entitlement, timestamp alignment, and cost-preview discipline to make quote-aware options validation trustworthy?

Why this needs GPT-5.5 Pro:

- Data quality and provenance are more important than coding the endpoint.
- Quote freshness, OPRA timing, underlying alignment, fill-lane semantics, and replacement validation policy interact.

File pack:

- `docs/options_research/options_expression_polygon_validation_hardening_implementation_plan_2026-04-25.md`
- `src-tauri/src/commands_options_data.rs`
- `src-tauri/src/commands_options_expression.rs`
- `crates/app_core/src/options_data_store.rs`
- `crates/app_core/src/options_types.rs`
- `web/src/stores/optionsExpressionStore.ts`

External research to include:

- Polygon/Massive options docs and entitlement limits
- OPRA quote/trade semantics
- options bid/ask/midpoint research
- stale quote and quote coverage standards

Verification:

- mocked provider tests
- quote-quality summary tests
- frontend lane-gating tests
- live credential run only as manual evidence

### 7. ML Signal calibration, conformal abstention, and Stage 5 leakage control

Priority: high  
Expected output: statistical design and deployment gate model

Question:

Can ML Signal and Stage 5 meta-selection produce calibrated, leakage-safe, decision-useful confidence that survives walk-forward and grouped validation?

Why this needs GPT-5.5 Pro:

- The problem mixes calibration, conformal prediction, time-series non-exchangeability, source exclusion, model-selection bias, and UI evidence.
- It is easy to create confidence numbers that look rigorous but leak or overfit.

File pack:

- `crates/app_core/src/ml_signal.rs`
- `crates/app_core/src/ml_confidence.rs`
- `crates/app_core/src/meta_select.rs`
- `src-tauri/src/commands_ml_signal.rs`
- `src-tauri/src/commands_meta.rs`
- `web/src/services/mlSignalService.ts`
- `web/src/services/metaService.ts`
- `web/src/services/metaMlScopeBuilder.ts`

External research to include:

- adaptive conformal prediction for time series
- probability calibration
- PBO/DSR
- offline policy evaluation

Verification:

- `cargo test -p ta_app_core ml_signal`
- `cargo test -p ta_app_core meta_select ml_confidence`
- `npm --prefix web run test -- src/services/mlSignalService.test.ts`
- ML workflow/validation benches

### 8. Tensor-core workload discovery

Priority: medium-high as research, low until proven for implementation  
Expected output: proof of useful dense workload or kill note

Question:

Does VectorGrid contain or plausibly need a dense matrix workload large enough and numerically tolerant enough to justify tensor-core/CUTLASS work?

Likely candidate workloads:

- batched ML inference or feature transforms
- covariance/correlation matrices for cross-asset validation
- Monte Carlo blocks
- dense policy-sweep scoring
- similarity or clustering workloads in meta selection

Likely poor workloads:

- branch-heavy per-bar trade simulation
- irregular options chain search
- strategy kernels dominated by control flow and memory irregularity

File pack:

- `crates/app_core/src/ml_signal.rs`
- `crates/app_core/src/ml_confidence.rs`
- `crates/app_core/src/meta_select.rs`
- `crates/app_core/src/composite_mo.rs`
- `crates/app_core/src/options_expression_gpu.rs`

External research to include:

- CUDA warp matrix functions
- CUTLASS docs
- mixed-precision error models
- tensor-core finance or time-series papers

Verification:

- microbench spike only after a workload is proven
- CPU/GPU numerical tolerance design before product code

### 9. RT-core and OptiX prove-or-kill

Priority: medium as research, likely low for implementation  
Expected output: kill note unless a natural BVH/ray mapping is found

Question:

Is there any VectorGrid computation that maps naturally to ray traversal, BVH intersection, path hit testing, interval intersection, or multi-dimensional range queries such that OptiX/RT cores could provide value?

Possible mapping hypotheses:

- barrier/hit detection over many price paths
- event-path intersection with thresholds
- interval/range query over compiled options outcomes
- exotic path-dependent options research

Kill criteria:

- no natural ray/BVH formulation
- data movement dominates
- CUDA kernels or CPU interval indexes solve the same problem more simply
- integration requires graphics/ray tracing abstractions unrelated to repo value

File pack:

- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_expression_gpu.rs`
- `crates/app_core/src/validation_core.rs`
- `crates/app_core/src/double_ma.rs`

External research to include:

- NVIDIA OptiX programming model
- RT-core hardware boundaries
- GPGPU ray traversal papers

### 10. Live execution safety state machine and evidence-to-action boundary

Priority: medium-high  
Expected output: formal state machine and invariants

Question:

What invariants must hold before research artifacts can trigger broker orders, and how should the app model stream freshness, idempotent orders, partial fills, cancellation, reconciliation, and emergency halt behavior?

Why this needs GPT-5.5 Pro:

- This is a safety and state-machine problem.
- It must bridge research evidence, delayed/live data, broker constraints, and user-visible controls.

File pack:

- `src-tauri/src/commands_execution.rs`
- `src-tauri/src/execution_mocks.rs`
- `src-tauri/src/state.rs`
- `web/src/services/executionService.ts`
- `web/src/stores/executionStore.ts`
- `scripts/run_execution_safety_gate.ps1`

External research to include:

- Alpaca order lifecycle and bracket order docs
- idempotent order design
- event-sourced audit trail patterns
- automated trading risk controls

Verification:

- `powershell -File scripts/run_execution_safety_gate.ps1`
- execution service/store tests
- broker mock partial-fill/reject/cancel tests

### 11. Artifact, replay, report, and evidence UX contract

Priority: medium  
Expected output: schema and workflow decision document

Question:

Can artifacts, replay, reports, provenance diffs, and UI evidence summaries be unified enough to support research-grade auditability without making every workflow generic?

Why this needs GPT-5.5 Pro:

- The repo has many artifact families with different replay semantics.
- The hard problem is deciding the minimum common schema and the surfaces that must remain strategy-specific.

File pack:

- `src-tauri/src/run_artifacts.rs`
- `src-tauri/src/commands_artifacts.rs`
- `src-tauri/src/commands_reports_api.rs`
- `src-tauri/src/commands_reports_validation_common.rs`
- `src-tauri/src/commands_reports_validation_xlsx.rs`
- `src-tauri/src/commands_reports_csv.rs`
- `src-tauri/src/commands_reports_meta_xlsx.rs`
- `web/src/stores/workflowArtifactsStore.ts`
- `web/src/services/historyService.ts`

Verification:

- `npm --prefix web run parity:outputs`
- `node scripts/run_output_parity_check.mjs`
- Rust report/artifact tests
- workflow artifact store and history service tests

## Recommended GPT-5.5 Pro Dossier Order

### Dossier 1: GPU-Resident Validation And Optimization-Pass Enrichment

Combine candidates 1 and 2.

Reason:

- Highest expected performance and product impact.
- Strong external source base: CUDA, CUB, PBO/DSR.
- Clear verification gates already exist.

Required output:

- architecture verdict
- deterministic top-k/reduction policy
- CPU/GPU tolerance model
- implementation slices
- benchmark plan
- kill criteria

### Dossier 2: Options Expression Objective, Advanced Geometry, And Quote Quality

Combine candidates 4, 5, and 6.

Reason:

- This is the richest finance/math/product question.
- The current options workflow already has CPU/GPU implementation and future roadmap docs.
- It can produce an admission matrix that prevents low-quality feature creep.

Required output:

- advanced-structure admission matrix
- objective/ranking proposal
- quote-quality gate model
- CPU/GPU parity constraints
- artifact/provenance requirements
- rejected strategies and why

### Dossier 3: Validation Evidence Theory For Meta And ML Signal

Combine candidates 1, 7, and relevant parts of 11.

Reason:

- High-throughput optimization and ML confidence both need better statistical guardrails.
- PBO, DSR, adaptive conformal prediction, and time-series leakage are GPT-5.5 Pro-shaped problems.

Required output:

- validation statistic policy
- ML confidence/calibration model
- leakage and source-exclusion rules
- frontend evidence display requirements
- implementation and verification plan

### Dossier 4: Tensor-Core Workload Discovery

Reason:

- Potentially high upside, but only if a dense workload exists.
- Should be a prove-or-kill dossier before any implementation.

Required output:

- workload inventory
- expected arithmetic intensity
- precision/tolerance analysis
- minimal spike plan or kill note

### Dossier 5: OptiX/RT-Core Prove-Or-Kill

Reason:

- Unusual but likely low probability.
- A single focused dossier can prevent wasted experimentation.

Required output:

- mapping attempt
- hardware/API constraints
- alternatives
- kill verdict unless evidence is strong

## Ideas To Reject Or Defer For Now

- Broad "review the whole repo" GPT-5.5 Pro prompts.
- Generic strategy-engine rewrites.
- Options calendars, diagonals, rolling, assignment/exercise, short premium, or margin modeling before a fixed-geometry options dossier.
- Tensor-core implementation before a dense workload is proven.
- OptiX implementation before a natural BVH/ray formulation is proven.
- Live execution expansion without a formal state-machine and evidence-readiness dossier.
- UI-only uncertainty features before the underlying evidence model is defined.

## Immediate Next Work

1. Create Dossier 1 with exact files, excerpts, external source summary, and required output format.
2. Create Dossier 2 after one more focused Polygon/Massive quote endpoint and entitlement research pass.
3. Create Dossier 3 after collecting the current validation-stat implementations and examples of artifact evidence fields.
4. Keep Dossiers 4 and 5 as bounded research prompts with explicit kill criteria.

## Source List

CUDA and GPU:

- [NVIDIA CUDA Programming Guide](https://docs.nvidia.com/cuda/cuda-programming-guide/)
- [NVIDIA CUTLASS overview](https://docs.nvidia.com/cutlass/latest/overview.html)
- [NVIDIA CUB documentation](https://docs.nvidia.com/cuda/cub/index.html)
- [CUB DeviceTopK API](https://nvidia.github.io/cccl/cub/api/structcub_1_1DeviceTopK.html)
- [NVIDIA OptiX developer page](https://developer.nvidia.com/rtx/ray-tracing/optix)
- [NVIDIA Monte Carlo option-pricing CUDA material](https://developer.download.nvidia.com/assets/cuda/files/MonteCarlo.pdf)
- [RadiK: Scalable and Optimized GPU-Parallel Radix Top-K Selection](https://arxiv.org/abs/2501.14336)

Options and market data:

- [Polygon/Massive Options REST API overview](https://www.polygon.io/docs/options)
- [Cboe strategy-based margin](https://www.cboe.com/en/markets/us/options/margin/strategy-based-margin/)
- [OCC options disclosure document page](https://www.theocc.com/company-information/documents-and-archives/options-disclosure-document)

Validation and ML uncertainty:

- [The Probability of Backtest Overfitting](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2326253)
- [The Deflated Sharpe Ratio](https://www.davidhbailey.com/dhbpapers/deflated-sharpe.pdf)
- [Adaptive Conformal Predictions for Time Series](https://proceedings.mlr.press/v162/zaffran22a.html)

Execution and governance:

- [Alpaca placing orders documentation](https://docs.alpaca.markets/docs/trading/orders/)
- [NIST AI Risk Management Framework](https://www.nist.gov/itl/ai-risk-management-framework)

