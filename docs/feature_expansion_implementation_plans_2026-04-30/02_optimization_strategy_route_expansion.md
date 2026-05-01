# Optimization, Strategy, And Route Expansion Plan

Date: 2026-04-30

## Goal

Expand optimization features while keeping route policy explicit, CPU fallback intact, and frontend/backend capability descriptions aligned. New strategy work should be strategy-local first and shared only where primitives are truly cross-cutting.

## Current Owner Files

Start here:

- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/services/optimizeService.ts`
- `web/src/utils/strategyOptimizationCapabilities.ts`
- `web/src/strategies/compositeProfileGpuPolicy.ts`
- `src-tauri/src/commands_optimize.rs`
- `crates/app_core/src/double_ma_mode.rs`
- `crates/app_core/src/double_ma_backend.rs`
- `crates/app_core/src/double_ma_strategy.rs`
- `crates/app_core/src/composite_profile_route.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/composite_cpu.rs`
- `crates/app_core/src/composite_mo.rs`
- `crates/strategies/src/double_ma/mod.rs`
- `crates/strategies/src/composite_cpu/mod.rs`
- `crates/optimizer/src/grid.rs`
- `crates/optimizer/src/stream.rs`

Representative tests:

- `web/src/services/optimizeRequestBuilder.test.ts`
- `web/src/services/optimizeService.test.ts`
- `web/src/utils/strategyOptimizationCapabilities.test.ts`
- `web/src/strategies/compositeProfileGpuPolicy.test.ts`
- Rust tests in `crates/app_core/src/double_ma.rs`
- strategy-local tests as added.

## Research Basis

- Cargo resolver and feature docs support the existing `resolver = "2"` CPU/GPU feature split: <https://doc.rust-lang.org/cargo/reference/resolver.html>
- CUDA best practices favor device-resident intermediate data and batched transfers when GPU routes are expanded: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
- Criterion provides a statistics-driven Rust benchmark harness, but benchmark boundary must match the claim: <https://docs.rs/criterion/latest/criterion/>
- Backtest overfitting research argues against treating larger search spaces as better unless validation evidence scales with them: <https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2326253>
- Hansen's SPA/data-snooping work and later deflated Sharpe research support adding search-space disclosure before expanding optimizer families: <https://www.ssc.wisc.edu/~bhansen/papers/ecnmt_05.html> and <https://academic.oup.com/jrssig/article/18/6/22/7038278>

## Route Policy To Preserve

Current product policy from repo instructions:

- `CompositeTrend`: `GpuKernel` when ADX is off, `GpuHybridKernel` when ADX is on, `CpuOnly` fallback.
- `BreakoutMomentum`: `GpuKernel` when ADX is off, `GpuHybridKernel` when ADX is on, `CpuOnly` fallback.
- `RegimeHybrid`: `GpuHybridKernel` in current ADX-bearing form, `CpuOnly` fallback, `GpuKernel` only if a true no-ADX effective variant is later implemented.
- For `CompositeTrend`, `RegimeHybrid`, and `BreakoutMomentum`, the product optimization route is the `double_ma` family, not `CompositeCpu`.

Do not change this policy as part of unrelated feature expansion.

## Expansion Track A: Capability Matrix As A Contract

### Problem

Route/capability truth can drift across request builders, service labels, route metadata, and Rust execution.

### Implementation

Add a generated or test-verified matrix, not a new runtime dependency:

- backend source owner: `crates/app_core/src/composite_profile_route.rs`
- frontend source owner: `web/src/utils/strategyOptimizationCapabilities.ts`
- frontend policy owner: `web/src/strategies/compositeProfileGpuPolicy.ts`

Candidate new files:

- `crates/app_core/src/strategy_route_matrix.rs`
- `web/src/services/strategyRouteMatrix.ts`
- tests comparing known strategy/backend combinations.

Fields:

- strategy id;
- ADX/regime requirements;
- requested backend;
- resolved backend;
- realized backend;
- fallback reason;
- CPU fallback availability;
- validation support;
- release claim label.

### Tests

- frontend capability text matches Rust route matrix fixtures;
- Auto resolves deterministically for each strategy profile;
- unsupported GPU request returns explicit fallback;
- no-ADX and ADX paths are independently covered.

## Expansion Track B: Strategy-Local Expansion Template

### Problem

New strategies can sprawl into shared execution layers too early.

### Implementation Shape

For each new strategy, prefer:

- request/config module;
- CPU evaluator;
- GPU evaluator or explicit unsupported adapter;
- hybrid adapter if needed;
- validation adapter;
- route metadata;
- strategy-local tests;
- frontend request builder mapping;
- frontend capability summary;
- frontend form normalization.

Only promote shared code when it is a primitive:

- CUDA upload/buffers;
- top-k/reduction;
- common metric helpers;
- artifact/report plumbing;
- validation reduction logic.

### New Strategy Checklist

Before writing execution code:

- define CPU oracle;
- define data requirements and warmup semantics;
- define objective metrics;
- define validation compatibility;
- define route/fallback labels;
- define artifact schema additions;
- define frontend form constraints;
- define performance gate, if any.

## Expansion Track C: Dynamic Strategy IR Promotion

### Problem

The repo has dynamic strategy planning and GPU predicate program modules. Expansion needs a clear boundary between experimental dynamic support and product route policy.

### Current Files

- `crates/app_core/src/strategy_dynamic.rs`
- `crates/app_core/src/strategy_ir.rs`
- `crates/app_core/src/strategy_plan.rs`
- `crates/app_core/src/strategy_gpu_program.rs`
- `crates/app_core/src/strategy_gpu_runtime.rs`
- `src-tauri/src/commands_dynamic.rs`
- `web/src/services/dynamicIndicatorCatalogService.ts`
- `web/src/strategies/dynamicDoubleMaBuilder.ts`

### Implementation Phases

Phase 1: inventory and label.

- Add route metadata that distinguishes dynamic prototype, release-supported route, and unsupported route.
- Add CPU-only fixture tests for dynamic plans.

Phase 2: CPU oracle.

- Ensure every promoted dynamic plan has a deterministic CPU evaluator and fixture output.
- Keep GPU route behind explicit capability flags.

Phase 3: GPU candidate.

- Add GPU evaluator only for plans whose indicator/input model can be kept resident enough to matter.
- Require CPU/GPU parity fixtures and route metadata.

### Acceptance Criteria

- Dynamic routes cannot silently replace product `double_ma` routes.
- Unsupported indicators return actionable capability text.
- CPU-only build remains clean.

## Expansion Track D: Request Builder Hardening

### Problem

Optimization request shaping is a high-churn boundary between UI, Tauri commands, and app-core.

### Implementation

In `web/src/services/optimizeRequestBuilder.ts`:

- keep normalization pure and testable;
- add explicit strategy id aliases only in one place;
- include capability warnings separately from execution request payload;
- avoid adding backend-specific fields to generic forms unless they are hidden behind strategy adapters.

In `src-tauri/src/commands_optimize.rs`:

- validate backend/request combinations before running heavy work;
- return structured fallback reason;
- preserve command names and response compatibility.

### Tests

- request snapshots for each strategy family;
- invalid backend/strategy combinations;
- ADX on/off route expectations;
- no CUDA feature CPU-only behavior;
- command parity after new commands or renamed fields.

## Expansion Track E: Optimization Evidence And Overfit Guardrails

### Problem

Adding larger grids or more strategy variants can increase false discovery risk.

### Implementation

Add evidence metadata, not behavior changes:

- search-space size;
- number of candidates evaluated;
- objective function;
- train range;
- validation recommendation availability;
- DSR/PSR/CSCV/PBO availability where already computed or safely derivable;
- warning when an optimization result has no validation evidence.

Current owners:

- `crates/app_core/src/validation_stats.rs`
- `crates/app_core/src/cscv_pbo.rs`
- `crates/app_core/src/sharpe_stats.rs`
- `crates/app_core/src/validation_pipeline.rs`
- frontend workflow evidence utilities.

### Acceptance Criteria

- No ranking/scoring default changes in this track.
- Evidence labels are optional/backward-compatible in artifacts.
- UI does not present optimized-only results as validated results.

## Second-Pass Expansion: Candidate Detail And Local Stability Lane

### Problem

Users need to inspect why a finalist ranked well before they commit to validation. A candidate-detail lane should explain local stability, nearby-parameter sensitivity, and route provenance without changing the optimizer's ranking behavior.

### Current Anchors

- `web/src/services/analysisService.ts`
- `web/src/stores/analysisStore.ts`
- `web/src/components/CandidateDetailWorkspacePanel.vue`
- `src-tauri/src/commands_analysis.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/composite_cpu.rs`
- `crates/app_core/src/composite_profile_route.rs`

### Implementation

Add a candidate-detail response that can be computed for selected finalists:

- selected candidate identity and source artifact;
- neighbor grid window definition;
- local score rank within neighborhood;
- score gradient/sign-flip count around the candidate;
- drawdown sensitivity summary;
- route metadata used for the original optimization;
- CPU recompute availability;
- validation recommendation availability.

Keep it pull-based. Do not compute local sensitivity for every candidate during the main optimization result unless a later perf gate proves the cost is acceptable.

### UI Behavior

- show candidate detail from the existing drilldown workspace;
- label it as diagnostic context, not validation evidence;
- provide a direct action to start validation with the same candidate and dataset;
- block detail if the source artifact is incompatible with the loaded dataset, using the artifact replay labels.

### Tests

- deterministic local-neighborhood ordering;
- invalid source artifact produces a blocked state;
- route metadata is preserved from optimization into drilldown;
- frontend handles missing optional local-stability fields for legacy artifacts.

## Second-Pass Expansion: CompositeMo And Run-Control Boundary

### Problem

Multi-objective optimization and long-running tasks can make cancellation, progress, and result ownership ambiguous.

### Current Anchors

- `crates/app_core/src/composite_mo.rs`
- `crates/optimizer/src/stream.rs`
- `web/src/services/runControlService.ts`
- `web/src/stores/taskStore.ts`
- `src-tauri/src/commands_optimize.rs`

### Implementation

For CompositeMo or any future multi-objective expansion:

- keep the result family separate from single-objective `double_ma` route claims;
- define a Pareto-front artifact shape before adding UI tables;
- expose progress as stages: request validation, data prep, candidate evaluation, reduction, artifact save;
- make cancellation best-effort but report the stage that observed it;
- ensure no partially saved result is presented as complete.

### Acceptance Criteria

- cancel leaves either no artifact or an artifact with explicit incomplete status;
- task store state cannot show `completed` after a backend cancellation/error;
- CompositeMo is not promoted as a GPU route without a separate CPU oracle and parity plan;
- frontend route labels do not collapse Pareto results into single-score winners.

## Verification

Targeted:

- `npm --prefix web run test -- src/services/optimizeRequestBuilder.test.ts src/services/optimizeService.test.ts src/utils/strategyOptimizationCapabilities.test.ts src/strategies/compositeProfileGpuPolicy.test.ts`
- `cargo test -p ta_app_core double_ma`
- strategy-local Rust tests as added.

Performance-sensitive:

- `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate`
- `powershell -File scripts/run_cpu_perf_gate.ps1 -Mode candidate`
- `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate` for CUDA route behavior.

Broad:

- `powershell -File scripts/run_repo_quality_gate.ps1`
