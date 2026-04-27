# GPT-5.5 Pro hard-problem meta-research plan

Status: active planning document  
Created: 2026-04-27  
Scope: meta-analysis before creating GPT-5.5 Pro hard-problem dossiers

## Purpose

This document defines how to research the application and the external technical landscape before deciding what problems should be packaged for GPT-5.5 Pro.

The goal is not to ask GPT-5.5 Pro to review the whole repository blindly. The goal is to identify the subset of problems where a stronger math, science, architecture, or algorithmic reasoning model could change the direction of the project before implementation time is spent.

Use this plan to produce a later artifact such as:

- `docs/gpt55_pro_hard_problem_atlas_YYYY-MM-DD.md`
- one or more focused GPT-5.5 Pro dossier prompts
- implementation spikes for accepted recommendations
- kill notes for ideas that are interesting but not worth pursuing

## Non-goals

- Do not turn this into a generic implementation backlog.
- Do not duplicate `docs/REPO_MAP.md`.
- Do not treat model output as authority without local code review, benchmarks, and parity checks.
- Do not send broad "review the whole repo" prompts.
- Do not assume experimental hardware ideas are useful before researching whether the hardware exposes a relevant computation model.

## Why a meta-analysis pass is needed

The hard part is not writing a prompt. The hard part is choosing questions that justify GPT-5.5 Pro's cost and attention.

Good candidates usually have several of these properties:

- They combine repo-specific constraints with math, numerical methods, GPU architecture, finance, or scientific computing.
- A wrong direction could waste weeks of engineering.
- The answer may be "do not build this", and that answer would be valuable.
- The problem cannot be resolved by a local code scan alone.
- There are external papers, vendor docs, algorithms, or prior systems that should shape the answer.
- The output should become an architecture decision, feasibility verdict, research spike, or implementation plan.

## Inputs

Local inputs:

- `AGENTS.md`
- `docs/REPO_MAP.md`
- `docs/ARCHITECTURE_NOTES.md`
- `docs/RELEASE.md`
- `web/README.md`
- `docs/archive/gpt54_feature_review_map.md`
- current code and tests in the subsystem under review
- benchmark, profiling, and parity scripts under `scripts/`
- current options research docs under `docs/options_research/`

External inputs:

- official NVIDIA CUDA, OptiX, cuBLAS, cuDNN, CUTLASS, Nsight, and CUDA Graphs documentation
- peer-reviewed or widely cited papers for GPU finance, option pricing, Monte Carlo, top-k selection, reductions, and time-series algorithms
- official documentation for relevant libraries already used or plausible in this stack
- finance and market-microstructure references for options strategy semantics
- benchmark writeups and engineering posts only as secondary evidence

## Source quality rules

Prefer sources in this order:

1. Official vendor or project documentation.
2. Research papers, technical reports, and conference material.
3. Source code or issue discussions from mature open-source projects.
4. Engineering blog posts with reproducible details.
5. Forum posts only for implementation caveats, never as core proof.

Every research note should distinguish:

- established facts
- repo-specific observations
- plausible hypotheses
- speculative ideas
- kill criteria

## Process

### Phase 0: Establish local baseline

Open `docs/REPO_MAP.md` first, then the current owner files for the subsystem being considered.

For each application area, capture:

- what the app currently does
- owner files
- current verification gates
- known benchmarks or missing benchmarks
- route/fallback behavior
- data movement and residency assumptions
- command, event, artifact, and storage boundaries
- architecture ownership boundaries and strategy-local versus shared code
- user-facing workflow impact
- current failure modes, fallback reasons, and kill criteria

Output: a local baseline note per area.

### Phase 1: Comprehensive application sweep

Survey every major area of the app at least once, but keep the first pass shallow.

Coverage areas:

- optimization and search
- CUDA runtime, kernels, memory residency, and GPU fallback policy
- validation, parity, statistics, and reportable evidence
- options expression workflow and advanced options strategy feasibility
- ML Signal, score-cache validation, and policy sweeps
- meta selection and Stage 5 ML confidence
- data loading, resampling, Polygon cache, and dataset readiness
- live execution, broker safety, and streaming runtime control
- artifacts, history, replay, hydration, and reports
- frontend workflow orchestration and request shaping
- benchmark, profiling, QA, release, and packaging gates
- experimental hardware lanes: tensor cores, RT cores, OptiX, CUDA Graphs, cooperative groups, stream concurrency

Output: a coverage matrix with hard-question candidates.

The sweep is not complete until every row below has either:

- at least one candidate question
- an explicit "no GPT-5.5 Pro dossier needed yet" note
- a deferral note with the missing evidence required to revisit it

Repo-wide coverage checklist:

- optimization request shaping, backend route selection, and search modes
- validation request shaping, validation statistics, parity, and reportable evidence
- candidate detail, local stability, and follow-up analysis
- reports, shortlist export, XLSX/CSV shape, and output parity
- artifacts, run history, replay, hydration, app-local storage, and provenance
- meta selection, campaign orchestration, auto-meta, Stage 5 ML confidence, and campaign resume/retry
- ML Signal dataset export, score-cache generation, validation, policy sweep, calibration, and native-library packaging
- data loading, resampling, cache identity, Polygon historical data, Polygon live data, and options dataset import/download
- options expression optimization, embedded validation, tuple inspection, event-path replay, and options dataset replacement policy
- live execution, broker integration, dry-run/paper state, risk policy, live stream coupling, and broker reconciliation
- frontend shell, workbench workflow model, task/event UI, command palette, saved-run rehydration, and user-visible evidence
- frontend command wiring, generated command parity, capability summaries, and request-builder tests
- dynamic strategy IR, builder preview, strategy plan/program generation, and constant-bit tooling
- moving-average registry, dynamic indicator catalog, candle-field gating, and VectorTA dependency/feature wiring
- strategy catalog breadth, strategy-local CPU/GPU/hybrid routes, GPU companion files, presets, and strategy-specific benches
- CUDA runtime, kernels, device-residency, allocation policy, top-k/reduction, launch overhead, and profiling scripts
- system/runtime environment, GPU status, settings, window effects, startup/bootstrap, and desktop runtime setup
- headless tooling, route matrices, benchmark harnesses, perf gates, output parity, and diagnostic binaries
- release packaging, PTX/prebuilt-kernel behavior, CPU-only builds, CUDA feature wiring, and CI/release surfaces
- security, privacy, credential storage, broker credential handling, and external-provider API key handling where a candidate can affect them

### Phase 2: External research sweep

For each promising candidate, run targeted web research before building a GPT-5.5 Pro dossier.

The research must answer:

- What is the relevant state of the art?
- What hardware or algorithmic constraints matter?
- What existing implementation patterns exist?
- What are the known traps?
- What would make the idea impossible or low ROI for this repository?
- What local evidence is needed before asking GPT-5.5 Pro?
- What source facts are current enough to depend on, and which facts must be rechecked before implementation?

Output: short research briefs linked to candidate questions.

### Phase 3: Candidate scoring

Score each candidate from 1 to 5 on:

- impact on product capability
- impact on runtime performance or workflow latency
- depth of math/science/algorithmic reasoning required
- dependence on external research
- implementation risk
- verification feasibility
- fit with existing architecture
- auditability and provenance impact
- user-risk or broker-risk impact
- dependency, packaging, and platform risk
- chance that the correct answer is a valuable "no"

Prioritize candidates with high impact, high reasoning depth, and clear verification paths.

### Phase 4: Build GPT-5.5 Pro dossiers

Only build a full dossier after local and external research have narrowed the question.

Each dossier should contain:

- problem statement
- why GPT-5.5 Pro is needed
- current repo behavior
- exact files to provide
- relevant code excerpts if full files are too large
- external research summary with source links
- constraints and invariants
- known evidence and benchmarks
- specific questions
- expected output format
- acceptance criteria
- kill criteria
- rollback plan or no-op fallback path
- minimum local verification before implementation

### Phase 5: Triage model output

Every GPT-5.5 Pro answer must be triaged into one of:

- accepted implementation plan
- bounded research spike
- benchmark/profiling task
- documentation-only decision
- rejected / killed idea
- needs another dossier

Do not implement directly from model output without mapping it back to local files, tests, and gates.

For every accepted recommendation, record:

- which local invariant it changes or preserves
- which source claims were relied on
- which files own the implementation
- which tests, parity checks, perf gates, or manual validations are required
- what would prove the recommendation wrong

## Candidate research lanes

### Lane A: CUDA architecture and GPU residency

Local entry points:

- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/dynamic_indicator_adapter.rs`
- `crates/app_core/src/vram_ma.rs`
- `crates/app_core/src/strategy_gpu_runtime.rs`
- `crates/app_core/src/standalone_gpu_runtime.rs`
- `crates/app_core/src/kernels/`
- `scripts/run_gpu_perf_gate.ps1`
- `scripts/profile_gpu_backtest_ncu.ps1`
- `scripts/profile_gpu_backtest_nsys.ps1`

Hard questions:

- Can more optimization and validation work stay VRAM-resident without breaking parity or reportability?
- Which current host round trips are architecturally necessary versus accidental?
- Would CUDA Graphs reduce warm-path launch overhead for repeated optimization or validation passes?
- Can reductions, top-k, metric aggregation, or candidate compaction be fused safely?
- Is there a general strategy-local GPU execution shape that avoids creating a large generic execution layer?

External research targets:

- CUDA Graphs
- stream concurrency
- cooperative groups
- CUB or equivalent reductions/top-k patterns
- Nsight profiling methodology
- memory coalescing and occupancy guidance

### Lane B: Tensor-core opportunities

Local entry points:

- `crates/app_core/src/ml_signal.rs`
- `crates/app_core/src/meta_select.rs`
- `crates/app_core/src/composite_mo.rs`
- `crates/app_core/src/options_expression_gpu.rs`
- `crates/app_core/src/validation_pipeline.rs`

Hard questions:

- Is there a real matrix-shaped computation in VectorGrid large enough to justify tensor-core work?
- Could covariance/correlation, feature transforms, batched scoring, Monte Carlo blocks, or policy sweeps use tensor cores?
- Can any trading-strategy computation be reformulated into dense or block-structured math without destroying semantics?
- Would cuBLAS, CUTLASS, or an existing ML inference path be more appropriate than custom kernels?

External research targets:

- NVIDIA tensor-core programming models
- WMMA and CUTLASS
- mixed-precision numerical error bounds
- GPU matrix methods for finance and time-series workloads
- inference acceleration tradeoffs for small and medium batch sizes

Kill criteria:

- the computation is branch-heavy and not reformulable into dense blocks
- data movement dominates expected arithmetic savings
- precision requirements make mixed precision unacceptable
- the implementation would bypass existing verification surfaces

### Lane C: RT cores and OptiX feasibility

Local entry points:

- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_expression_gpu.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/validation_core.rs`

Hard questions:

- Is there any VectorGrid computation that naturally maps to ray/BVH traversal or interval intersection?
- Could OptiX accelerate path queries, barrier/hit testing, event-time detection, or multidimensional range lookup?
- Are RT cores exposed in a way that is useful outside rendering-style traversal?
- Is the engineering cost justified compared with CUDA kernels or CPU data structures?

External research targets:

- official OptiX programming model
- RT-core hardware capability boundaries
- GPGPU uses of ray tracing acceleration structures
- interval, path, and event-detection algorithms

Expected output:

- probably a proof-oriented feasibility verdict
- clear kill criteria if RT cores are not appropriate
- alternatives if the underlying query pattern is still valuable

### Lane D: Advanced options strategy feasibility

Local entry points:

- `docs/options_research/options_expression_optimizer_viable_plan_2026-04-02.md`
- `docs/options_research/options_expression_future_scope_roadmap_2026-04-07.md`
- `web/src/components/OptionsExpressionPanel.vue`
- `web/src/stores/optionsExpressionStore.ts`
- `src-tauri/src/commands_options_expression.rs`
- `src-tauri/src/options_parent_adapters.rs`
- `crates/app_core/src/options_types.rs`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_expression_gpu.rs`
- `crates/app_core/src/kernels/options_expression.cu`

Hard questions:

- Which advanced options strategies can be represented within the current expression and validation model?
- What would it take to support spreads, calendars, diagonals, condors, butterflies, straddles, strangles, covered overlays, or rolling logic?
- Which pieces are data-parallel enough for CUDA?
- What data quality is required for bid/ask, midpoint, IV, Greeks, assignment, and exercise assumptions?
- Can embedded validation remain provenance-safe as the strategy space becomes more complex?

External research targets:

- options backtesting methodology
- multi-leg payoff modeling
- implied volatility and Greeks computation
- options market microstructure
- GPU Monte Carlo option pricing
- portfolio and margin treatment for multi-leg strategies

### Lane E: Optimization pass enrichment

Local entry points:

- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/optimizer/src/stream.rs`
- `crates/optimizer/src/grid.rs`
- `crates/optimizer/src/mo.rs`
- `crates/app_core/src/composite_mo.rs`

Hard questions:

- Can each optimization pass produce more useful validation-like evidence without changing semantics?
- Can early pruning, upper bounds, lower bounds, or streaming Pareto maintenance reduce wasted candidate work?
- Can top-k, heatmap, and include-all outputs share more computation?
- Can robust statistics or stability measures be computed during optimization cheaply enough to affect ranking?

External research targets:

- branch-and-bound search
- multi-objective optimization
- streaming top-k algorithms
- GPU reduction and selection
- robust financial objective functions
- overfitting-aware strategy ranking

### Lane F: Validation theory, parity, and statistical evidence

Local entry points:

- `crates/app_core/src/validation_core.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/composite_validate.rs`
- `src-tauri/src/commands_validation.rs`
- `src-tauri/src/bin/ta_headless_validate.rs`
- `scripts/run_output_parity_check.mjs`

Hard questions:

- What is the right tolerance model for CPU/GPU parity across financial metrics?
- Which validation statistics should be exact, approximate, or confidence-bounded?
- Can validation be made faster while preserving evidence quality?
- What statistical evidence best detects overfitting, regime concentration, and slippage sensitivity?

External research targets:

- walk-forward validation
- multiple-hypothesis correction
- false discovery and overfitting in trading strategy search
- numerical reproducibility for GPU reductions
- financial backtest robustness metrics

### Lane G: ML Signal and meta-selection

Local entry points:

- `web/src/services/mlSignalService.ts`
- `src-tauri/src/commands_ml_signal.rs`
- `crates/app_core/src/ml_signal.rs`
- `crates/app_core/src/ml_confidence.rs`
- `crates/app_core/src/meta_select.rs`
- `crates/app_core/benches/ml_signal_workflow.rs`
- `crates/app_core/benches/ml_signal_validation.rs`

Hard questions:

- Are current score-cache validation and policy sweeps mathematically sufficient?
- Could model calibration, conformal intervals, uncertainty estimates, or regime-aware policies improve usefulness?
- Where is GPU acceleration actually useful in the ML Signal workflow?
- Can meta-selection avoid overfitting while still surfacing actionable candidates?

External research targets:

- probability calibration
- conformal prediction
- walk-forward ML validation
- model-selection bias
- GPU inference and batch scoring

### Lane H: Data pipeline and market-data quality

Local entry points:

- `web/src/services/dataService.ts`
- `web/src/stores/dataStore.ts`
- `src-tauri/src/commands_data.rs`
- `src-tauri/src/data_cache.rs`
- `src-tauri/src/polygon.rs`
- `src-tauri/src/settings.rs`

Hard questions:

- Which data-quality issues are most likely to invalidate optimization or validation results?
- Can cache format, resampling, or repair logic be redesigned for lower latency without losing auditability?
- What provenance model is needed for options data and live-data-derived runs?
- Can preload and dataset-readiness behavior be improved with measurable UX impact?

External research targets:

- market-data cleaning and gap repair
- OHLCV resampling semantics
- options quote quality and stale quote detection
- cache formats for time-series data

### Lane I: Live execution and broker safety

Local entry points:

- `web/src/stores/executionStore.ts`
- `web/src/services/executionService.ts`
- `src-tauri/src/commands_execution.rs`
- `src-tauri/src/execution_mocks.rs`
- `scripts/run_execution_safety_gate.ps1`

Hard questions:

- What is the correct safety boundary between research workflows and live execution?
- Can strategy validation evidence be converted into execution risk controls?
- What failure modes matter most for delayed data, stream interruption, partial fills, duplicate events, and broker reconciliation?
- Are there formal invariants that should be enforced before expanding supported execution strategies?

External research targets:

- broker order lifecycle and idempotency
- event-sourced trading systems
- risk controls for automated execution
- paper/live parity traps

### Lane J: Frontend orchestration and human decision support

Local entry points:

- `web/src/components/workbench/WorkbenchShell.vue`
- `web/src/services/optimizeService.ts`
- `web/src/services/validationService.ts`
- `web/src/services/metaService.ts`
- `web/src/stores/taskStore.ts`
- `web/src/stores/workflowArtifactsStore.ts`

Hard questions:

- Which high-complexity workflows are currently hard for users to reason about?
- Can the app surface uncertainty, provenance, route fallback, and evidence strength without adding noise?
- What frontend state model best supports long-running research workflows and resumability?
- Which UI decisions materially affect scientific correctness or user risk?

External research targets:

- human-in-the-loop model selection
- workflow provenance UX
- uncertainty visualization
- long-running desktop task UX patterns

### Lane K: Dynamic strategy IR, indicator catalog, and strategy surface

Local entry points:

- `crates/app_core/src/strategy_dynamic.rs`
- `crates/app_core/src/strategy_ir.rs`
- `crates/app_core/src/strategy_plan.rs`
- `crates/app_core/src/strategy_gpu_program.rs`
- `src-tauri/src/commands_dynamic.rs`
- `src-tauri/src/ma_registry.rs`
- `web/src/services/indicatorCatalogService.ts`
- `web/src/services/indicatorCatalogService.test.ts`
- `web/src/utils/strategyOptimizationCapabilities.ts`
- strategy-local files listed in `docs/REPO_MAP.md`

Hard questions:

- Can the dynamic strategy IR be made more mathematically expressive without creating an unsafe generic execution layer?
- Which indicator or MA catalog expansions are GPU-compatible, CPU-only, or hybrid by design?
- Can a strategy-local route policy be inferred or verified from IR features?
- What formal capability model would prevent frontend/backend drift across the strategy catalog?

External research targets:

- compiler IR design for numeric kernels
- GPU expression compilation patterns
- indicator dependency graphs
- numerical stability for recursive and adaptive indicators
- feature-gating and capability-schema design

### Lane L: Artifacts, reports, replay, and auditability

Local entry points:

- `src-tauri/src/run_artifacts.rs`
- `src-tauri/src/commands_artifacts.rs`
- `src-tauri/src/commands_reports_api.rs`
- `src-tauri/src/commands_reports_validation_common.rs`
- `src-tauri/src/commands_reports_validation_xlsx.rs`
- `src-tauri/src/commands_reports_csv.rs`
- `src-tauri/src/commands_reports_meta_xlsx.rs`
- `web/src/stores/workflowArtifactsStore.ts`
- `web/src/services/historyService.ts`
- `scripts/run_output_parity_check.mjs`
- `scripts/check_output_parity.mjs`

Hard questions:

- What is the minimum common provenance schema across run, validation, meta, ML, options, campaign, and live artifacts?
- Which replay behaviors must be exact, compatible, replacement-scoped, or intentionally blocked?
- Can report/export generation remain strategy-specific while sharing enough audit structure for decision-grade review?
- What output-parity guarantees are required before a model-recommended change is safe?

External research targets:

- reproducible research artifacts
- provenance and lineage schemas
- audit-trail design
- report and workbook validation strategies

### Lane M: Desktop runtime, command surface, events, and packaging

Local entry points:

- `src-tauri/src/lib.rs`
- `src-tauri/src/commands.rs`
- `src-tauri/src/state.rs`
- `src-tauri/src/settings.rs`
- `src-tauri/src/commands_system.rs`
- `src-tauri/src/main.rs`
- `src-tauri/tauri.conf.json`
- `src-tauri/capabilities/main.json`
- `docs/RELEASE.md`
- `scripts/check_tauri_config_sync.mjs`

Hard questions:

- Do any hard-problem candidates require new command/event boundaries, long-running task semantics, or app-local state ownership?
- Can GPU/CPU capability reporting and settings bootstrap make experimental features safe to expose?
- What release or PTX packaging constraints could invalidate a CUDA-heavy recommendation?
- How should CPU-only builds degrade when a GPT-5.5 Pro recommendation depends on CUDA features?

External research targets:

- Tauri command and capability guidance
- Windows packaging constraints
- CUDA runtime distribution and PTX compatibility
- OS credential storage and notification behavior where relevant

### Lane N: Verification authority, benchmark fidelity, and release gates

Local entry points:

- `scripts/run_repo_quality_gate.ps1`
- `scripts/run_workflow_gate.ps1`
- `scripts/run_cpu_perf_gate.ps1`
- `scripts/run_gpu_perf_gate.ps1`
- `scripts/run_gpu_validation_perf_gate.ps1`
- `scripts/run_options_expression_gpu_promotion_gate.ps1`
- `scripts/run_execution_safety_gate.ps1`
- `scripts/run_frontend_e2e_gate.ps1`
- `scripts/baselines/`
- `crates/app_core/benches/`
- `src-tauri/src/bin/`
- `docs/REPO_MAP.md`

Hard questions:

- Which benchmark or gate is authoritative for each candidate?
- Are current timing boundaries measuring the thing the model recommendation claims to improve?
- What local evidence is required before a claim can leave research status?
- Can perf and parity evidence be made reproducible enough for future model comparisons?

External research targets:

- benchmarking methodology for GPU workloads
- statistical treatment of benchmark noise
- profiling methodology with Nsight
- regression-gate design for performance-sensitive desktop applications

### Lane O: Campaign, batch, and resumable research workflows

Local entry points:

- `web/src/services/metaService.ts`
- `src-tauri/src/commands_campaign.rs`
- `web/src/services/metaServiceCampaignResume.test.ts`
- `web/src/services/metaServiceCampaignStrategy.test.ts`
- `web/src/stores/taskStoreCampaignProgress.test.ts`
- `src-tauri/src/run_artifacts.rs`

Hard questions:

- What is the right deterministic manifest model for resumable multi-dataset research campaigns?
- How should retries, partial summaries, auto-meta, auto-Stage-5, and validation payloads preserve evidence integrity?
- Which campaign failures should be resumable, restart-only, or rejected as stale?
- Can campaign orchestration become a source of hard-problem candidates rather than just a workflow wrapper?

External research targets:

- deterministic job manifests
- resumable batch workflow design
- provenance-aware retry semantics
- auditability for long-running scientific workflows

## Web-search query seed patterns

Use targeted queries. Replace bracketed terms with the active subsystem or algorithm.

- `site:docs.nvidia.com CUDA Graphs launch overhead repeated kernels`
- `site:docs.nvidia.com CUTLASS tensor cores mixed precision error`
- `site:docs.nvidia.com OptiX acceleration structure non rendering computation`
- `GPU Monte Carlo option pricing CUDA paper`
- `GPU top k selection reduction CUB`
- `walk forward validation trading strategy overfitting multiple hypothesis`
- `option backtesting multi leg spread bid ask midpoint data quality`
- `CUDA financial backtesting vectorized strategies`
- `conformal prediction trading signals calibration`
- `time series resampling OHLCV data quality gaps stale quotes`
- `Tauri command capability security documentation`
- `CUDA PTX compatibility Windows deployment documentation`
- `reproducible research artifact provenance schema`
- `GPU benchmark methodology Nsight profiling noise`
- `event sourced trading order lifecycle idempotency`

## Hard-problem candidate template

```md
## Candidate

### Problem

### Application Area

### Why This Might Need GPT-5.5 Pro

### Local Files To Inspect

### External Research Needed

### Current Evidence

### Unknowns

### Proposed GPT-5.5 Pro Questions

### Expected Output

### Acceptance Criteria

### Kill Criteria

### Verification Path

### Rollback Or No-Op Fallback
```

## GPT-5.5 Pro dossier template

```md
# GPT-5.5 Pro dossier: <problem>

## Task

Answer the feasibility/design question below using the provided repo context and external research summary.

## Context

## Current Architecture

## Files Provided

## External Research Summary

## Constraints

## Questions

## Required Output

1. Feasibility verdict
2. Reasoning and assumptions
3. Recommended architecture or algorithm
4. Counterarguments and failure modes
5. Implementation slices
6. Verification plan
7. Kill criteria
8. Rollback or CPU-only fallback
9. Source claims that must be rechecked before implementation
```

## Meta-analysis output format

The first complete pass should produce:

- a coverage matrix for all application areas
- a ranked hard-problem candidate list
- research briefs for the top candidates
- GPT-5.5 Pro dossiers for only the strongest candidates
- a rejected-ideas section explaining what not to pursue yet
- a skipped-areas section proving every repo-map area was considered
- a verification-authority table mapping each accepted candidate to gates
- a source-age table for facts that may change, such as vendor APIs, model/library docs, broker behavior, CUDA tooling, and provider entitlements

## Initial priority hypotheses

These are starting hypotheses, not conclusions:

1. GPU-resident validation and optimization-pass enrichment are likely high-value because they align with existing architecture and perf gates.
2. Advanced options strategy CUDA feasibility is high-value but requires careful data-quality and semantics research before implementation.
3. Tensor cores may be useful only if a real dense-math workload is identified in ML, covariance/correlation, Monte Carlo, or batched scoring.
4. RT cores and OptiX are likely low-probability but worth a focused kill-or-prove research pass because the upside is unusual and the answer may prevent wasted experimentation.
5. Statistical validation and overfitting control may be as valuable as raw speed if it improves trust in optimization output.

## Completion criteria for this planning phase

This planning phase is complete when:

- every major app area has been surveyed at least once
- at least one external research brief exists for each high-priority lane
- candidate problems are scored and ranked
- top GPT-5.5 Pro dossiers include exact file packs and source-backed research summaries
- low-ROI or invalid ideas have explicit kill notes
- accepted follow-up work is moved into implementation plans or research spikes
- skipped areas have explicit reasons rather than silent omission
- every accepted candidate has a verification-authority mapping
- every external dependency or provider claim has a source link and recheck date
