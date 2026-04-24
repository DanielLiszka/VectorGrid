# Backtest Optimization GPU Performance Implementation Plan

## Intent

Improve end-to-end backtest optimization performance by pushing the right parts of the workflow further into measured, GPU-executed paths. The plan is intentionally workflow-first: it treats CUDA kernel speed, validation cost, route residency, fallback behavior, artifact/report visibility, and benchmark authority as one system.

Last updated: 2026-04-23.

## Current Evidence Snapshot

The current GPU path is already valuable, but the value is uneven across workload sizes and workflow stages.

Observed local benchmark artifacts:

- `double_ma/cpu/sma_sma/200k/small`: about `146.411 ms`.
- `double_ma/gpu_kernel/sma_sma/200k/small`: about `67.158 ms`.
- Small SMA speedup: about `2.18x`.
- `double_ma/cpu/alma_alma/200k/large`: about `4848.405 ms`.
- `double_ma/gpu_kernel/alma_alma/200k/large`: about `70.247 ms`.
- Large ALMA speedup: about `69.02x`.
- `double_ma/gpu_kernel/alma_alma/200k/large_batched_1gb`: about `157.965 ms`, currently slower than the non-batched large ALMA case.

Observed workflow timing posture:

- `scripts/baselines/workflow_regression_baseline.json` shows the deterministic workflow baseline at about `229 ms`.
- In that baseline, optimize is about `22 ms`, validation is about `119 ms`, meta-select is about `14 ms`, and holdout is about `55 ms`.
- A recent local workflow summary showed total runtime about `336 ms`, with validation about `217 ms`, which reinforces validation as the more variable and more important workflow cost center.

Existing perf authority:

- `scripts/run_gpu_perf_gate.ps1` gates the default GPU optimization cases:
  - `gpu_sma_small`
  - `gpu_alma_large`
- `scripts/run_gpu_alma_contract_rows_perf_gate.ps1` gates specialized ALMA contract-row cases, but is explicitly not the default PR perf authority.
- `scripts/run_cpu_perf_gate.ps1` gates the workflow regression and CPU optimization cases.
- Validation and follow-up Criterion benches exist, but they are not first-class default perf gates.

Important current gaps:

- GPU optimization is well represented by default perf gates; GPU validation is not.
- The workflow gate validates only a basic CPU-oriented surface and does not exercise the full validation module stack.
- Hybrid route labels can hide different residency qualities unless telemetry is inspected.
- GPU fixed-candidate validation still has intentional CPU fallback cases, most notably generic Double-MA TrendQuality without ADX confirmation, including `CompositeTrend`.
- CPU-tail work remains in GPU validation for perturbation, trade bootstrap, and aux finalist analytics.

## Goals

1. Make performance authority match the product workflow.
2. Reduce validation latency and variance where it affects real user workflows.
3. Prevent silent GPU route degradation and accidental CPU fallback.
4. Push high-speedup indicators and strategy profiles into GPU-executed paths only when measured evidence supports it.
5. Preserve CPU-only builds and CPU fallback behavior.
6. Make route, residency, fallback, and CPU-tail costs visible in artifacts, reports, and UI summaries.

## Recommended Scope Expansion

The performance review should extend beyond the conventional backtest optimization hot path. The backtest optimizer is only one stage in the product workflow; validation and options-expression work can dominate user-perceived cost and carry separate GPU/product risks.

Recommended umbrella scope:

1. **Conventional optimization**
   - Double-MA family optimization.
   - Composite-profile routed optimization.
   - Composite/advanced strategy optimization.
   - Dynamic strategy preflight and GPU program cache behavior.
   - Follow-up analysis after optimization: drilldown, local stability, heatmap/export, and shortlist diagnostics.

2. **Conventional validation**
   - In-sample fixed-candidate validation.
   - Walk-forward.
   - Reopt walk-forward.
   - CSCV/PBO.
   - SPA.
   - Perturbation.
   - Trade bootstrap.
   - Cross-ticker preserved-finalists validation.
   - Aux analytics and report generation.
   - Route, fallback, residency, and CPU-tail telemetry.

3. **Options-expression optimization**
   - Artifact replay and parent validation.
   - Signal-event manifest preparation.
   - Options dataset manifest validation.
   - Candidate tuple compilation.
   - CPU reference scoring.
   - CUDA compiled-tensor scoring.
   - GPU chunk planning, cache reuse, kernel launch count, and memory budget behavior.
   - Exactification, sealed-finalist rescoring, holdout-aware ranking, and artifact persistence.

4. **Options-expression embedded validation**
   - Top validation verdict computation.
   - Holdout/sealed-finalist robustness.
   - Stress lanes for fill realism, fees, spread, liquidity, and invalid-slot behavior.
   - Backend comparison and promotion gates.
   - Compare/reopen/replay performance for saved options artifacts.

The expanded review should keep these lanes separate in evidence and gate output. Conventional validation and options-expression validation should share vocabulary where possible, but they should not be forced into one implementation model because the options path is compiler/tensor-oriented and intentionally separate from the Double-MA hot kernel path.

### Why The Expansion Is Worth It

- Validation is already a larger and more variable part of the deterministic workflow than optimization in the existing baseline.
- Options-expression optimization has a distinct CPU oracle plus CUDA product path; it can regress independently from conventional backtests.
- Options workflows already expose GPU-specific telemetry such as ranking time, tensor scoring time, exactification time, sealed-stage time, GPU speedup, cache hits/misses, chunk planning, launch count, and memory budget.
- Options promotion already has a dedicated gate script, `scripts/run_options_expression_gpu_promotion_gate.ps1`, but it is not integrated with the conventional GPU validation perf plan.
- Full product performance depends on the complete journey: optimize, validate, inspect, report, reopen, compare, and rerun.

### Scope Boundaries For The Expanded Review

- Keep conventional strategy optimization and options-expression optimization as separate runtime families.
- Do not make options-expression work mutate the conventional validation artifact contract unless a specific shared field is needed.
- Do not use options GPU promotion evidence to justify conventional strategy GPU route policy, or vice versa.
- Keep options CPU reference scoring as the correctness oracle.
- Treat options CUDA compiled-tensor scoring as the differentiation path.
- Keep provider download and quote-lane expansion outside this performance plan unless the measured bottleneck is local parsing, manifest validation, or replay.

## Non-Goals

- Do not remove CPU fallback.
- Do not make CUDA mandatory for normal builds or tests.
- Do not optimize small interactive runs at the expense of large exact-grid or validation workloads.
- Do not promote a strategy to GPU-preferred `Auto` routing based only on theoretical GPU support.
- Do not treat advisory GPU speedup telemetry as a legal capability source. Capability still comes from backend descriptors, route planners, and request builders.
- Do not run overlapping benchmarks, faceoffs, Nsight sessions, or CUPTI sessions.
- Do not remove, weaken, or hide existing validation features to make performance numbers look better.
- Do not narrow supported strategy, backend, artifact replay, report export, or options-expression behavior unless a separate product decision explicitly changes scope.
- Do not replace correctness checks with performance gates. Performance gates are additive and must not become a substitute for parity, semantic, artifact, or report tests.

## Regression And Feature Preservation Guardrails

Performance work in this plan must be non-regressive by default. A faster path is not acceptable if it silently changes strategy semantics, validation verdicts, supported inputs, artifact replay behavior, report content, or CPU-only availability.

### Behavioral Compatibility

- Existing strategy kinds remain available.
- Existing validation modules remain available:
  - in-sample metrics,
  - walk-forward,
  - reopt walk-forward,
  - CSCV/PBO,
  - SPA,
  - perturbation,
  - trade bootstrap,
  - cross-ticker preserved-finalists validation,
  - aux analytics,
  - report generation.
- Existing options-expression structure families remain available:
  - `long_single`,
  - `debit_vertical`,
  - `long_pair`.
- Existing backend choices remain accepted where they are currently accepted:
  - `Auto`,
  - `CpuOnly`,
  - `GpuOnly`,
  - `GpuHybridKernel`,
  - `GpuKernel`,
  - options `auto`,
  - options `cpu_reference`,
  - options `gpu_compiled_tensor`.
- CPU fallback remains available and truthful.
- CPU-only builds continue to compile and run supported CPU workflows.
- CUDA-specific improvements stay feature-gated and runtime-gated.

### Result Compatibility

- CPU remains the reference oracle for conventional validation parity unless a strategy-specific oracle is explicitly documented.
- Options CPU reference scoring remains the correctness oracle for options-expression work.
- GPU results must preserve validation verdicts within documented tolerance.
- Any changed tolerance must be documented, tested, and justified with deterministic fixtures.
- Randomized validation modules must remain seed-deterministic.
- Candidate ordering changes are allowed only when they are a consequence of a documented scoring or tie-break rule, not incidental nondeterminism.
- Route changes must be explicit in preview, result metadata, artifacts, and reports.

### Artifact And Report Compatibility

- Existing saved runs and validation artifacts must remain loadable.
- New performance fields must be additive where practical.
- Legacy artifacts missing new fields must use compatibility defaults rather than failing normal load/reopen paths.
- XLSX/report exports must not drop existing sheets, columns, warnings, or verdict fields as a side effect of performance work.
- Options-expression compare/reopen/replay must preserve provenance checks.
- Report copy must not overstate GPU residency, GPU validation coverage, or options GPU eligibility.

### UI And Workflow Compatibility

- Existing frontend workflow entry points remain reachable.
- Existing route/backend controls remain available unless replaced by an equivalent or clearer control.
- New presets must be additive; users must still be able to configure advanced validation features.
- Performance shortcuts must not bypass user-selected validation modules.
- Auto-routing improvements must not remove explicit CPU or GPU override paths.

### Rollback Criteria

Any performance change should be reverted or held behind an opt-in flag if it:

- changes validation pass/fail verdicts without an approved parity explanation,
- removes or hides a validation module,
- breaks legacy artifact loading,
- breaks report export compatibility,
- breaks CPU-only builds,
- creates an unexpected CPU fallback on a gated GPU route,
- makes a default workflow slower beyond the configured regression budget,
- reduces options-expression candidate coverage or structure-family support,
- degrades cache/replay correctness.

## Success Metrics

### Perf Authority

- A dedicated GPU validation perf gate exists and writes a structured summary JSON.
- The summary includes timing, backend route, fallback reason, residency, CPU-tail time, GPU materialization time, candidate count, candle count, and validation module mix.
- Default GPU perf authority includes:
  - one optimization small-overhead case,
  - one optimization large-throughput case,
  - one Double-MA GPU validation case,
  - one composite/advanced GPU validation case,
  - one follow-up cache case.
- Each benchmark case is serialized and guarded against concurrent perf processes.

### Route Correctness

- Expected `GpuKernel` and `GpuHybridKernel` route cases assert no unintended fallback.
- Expected CPU fallback cases assert the exact fallback reason.
- `StrictGpuResident` or equivalent test coverage catches accidental CPU-staged route use when a fully resident path is required.

### Validation Performance

- GPU validation summary reports `execution_profile.cpu_tail_wall_ms`.
- CPU-tail budget is established from a fresh baseline before enforcement.
- Validation stage runtime has stage-level regression budgets.
- The full validation stack has a measured profile:
  - in-sample metrics,
  - walk-forward,
  - reopt walk-forward,
  - CSCV/PBO,
  - SPA,
  - perturbation,
  - trade bootstrap,
  - aux finalist analytics.

### User-Facing Clarity

- Optimization and validation reports display actual route, fallback, residency, and CPU-tail summary when available.
- UI route preview and final result route summary do not imply fully GPU-resident execution when hybrid or CPU-staged work occurred.

### Regression Protection

- Every performance gate is paired with behavioral or parity verification for the same route family.
- Legacy artifact and report compatibility tests pass after new performance fields are added.
- Full validation feature coverage remains reachable from frontend request builders and backend commands.
- Options-expression CPU reference and CUDA scorer comparisons remain available after optimization.
- No performance improvement is considered complete until it has both speed evidence and non-regression evidence.

## Workstream A: Measurement And Perf Gates

### A1. Add A GPU Validation Perf Gate

Create a dedicated gate rather than overloading `scripts/run_gpu_perf_gate.ps1` immediately.

Proposed files:

- `scripts/run_gpu_validation_perf_gate.ps1`
- `scripts/baselines/gpu_validation_perf_baseline.json`
- `output/benchmarks/gpu_validation_perf_summary.json`

Initial cases:

- `double_ma_validate/gpu_kernel/sparse_finalists/100k/8candidates/wf_reopt_cscv_spa`
- `composite_validate/gpu_kernel/asr/100k/64candidates/wf_cscv`
- `composite_validate/gpu_kernel/vcet/100k/64candidates/wf_reopt_cscv_spa`
- `double_ma_followup/gpu_kernel/50k/drilldown_then_local_stability_cached`

Implementation notes:

- Reuse the process serialization pattern from `scripts/run_gpu_perf_gate.ps1`.
- Build benches with `--features cuda,cuda-backtest-kernel`.
- Support `-Mode baseline` and `-Mode candidate`.
- Enforce GPU identity compatibility when a baseline has a GPU name.
- Enforce per-case `mean_ms` regression thresholds.
- Emit a single summary JSON with all cases, not one loose Criterion artifact per case.
- Keep a custom-case mode for focused profiling lanes.

Acceptance criteria:

- Candidate mode fails when a case is slower than baseline by more than the configured threshold.
- Candidate mode fails when a required Criterion estimates file is missing.
- Candidate mode fails when expected route metadata indicates CPU fallback.
- The script can be run independently of the default GPU optimization gate.

### A2. Add Structured Validation Cost Output

Criterion estimates alone do not capture route and CPU-tail details. Add or extend a structured binary that runs deterministic validation workloads and emits JSON.

Candidate owners:

- `src-tauri/src/bin/ta_double_ma_validation_cost.rs`
- or a new `crates/app_core/src/bin/gpu_validation_perf_probe.rs`

Output fields:

- schema version,
- build profile,
- GPU identity,
- workload id,
- candles length,
- candidate count,
- requested backend,
- backend used,
- backend route,
- fallback reason,
- route diagnostics,
- residency summary,
- module mix,
- total wall time,
- in-sample wall time,
- walk-forward wall time,
- reopt wall time,
- CSCV wall time,
- SPA wall time,
- perturb wall time,
- trade-bootstrap wall time,
- aux finalist analytics wall time,
- CPU-tail wall time,
- GPU result materialization wall time,
- tested strategy/profile.

Acceptance criteria:

- The structured probe can run without Criterion.
- The output is stable enough to gate route and CPU-tail invariants.
- The gate stores both Criterion timing and structured route/tail data.

### A3. Bring Validation Into The Workflow Gate

Extend `src-tauri/src/bin/ta_workflow_regression.rs` or add a second workflow gate mode that exercises realistic validation modules.

Candidate modes:

- `basic`: current deterministic workflow.
- `validation-heavy`: optimization plus full validation stack.
- `gpu-validation`: CUDA-required route/residency validation path.

Validation-heavy module mix:

- walk-forward,
- reopt walk-forward,
- CSCV/PBO,
- SPA,
- perturbation,
- trade bootstrap,
- cross-ticker preserved-finalists wrapper.

Acceptance criteria:

- The current `basic` workflow remains fast and stable.
- `validation-heavy` has separate baselines and budgets.
- The gate records stage timings, not only total runtime.
- Stage budget failures point to optimize, validation, meta, or holdout.

## Workstream B: Route And Residency Hardening

### B1. Define A Route Matrix

Create a route matrix that documents and tests expected route behavior for optimization and validation.

Initial matrix:

| Strategy | Config | Optimize Target | Validation Target | Notes |
| --- | --- | --- | --- | --- |
| `DoubleMa` | SMA/SMA | `GpuKernel` | `GpuKernel` | small overhead case |
| `DoubleMa` | ALMA/ALMA | `GpuKernel` | `GpuKernel` | large throughput case |
| `CompositeTrend` | ADX off | `GpuKernel` | `CpuOnly` until parity fixed | known fixed-candidate validation gap |
| `CompositeTrend` | ADX on | `GpuHybridKernel` | `GpuHybridKernel` or documented fallback | hybrid residency must be explicit |
| `BreakoutMomentum` | ADX off | `GpuKernel` | `GpuKernel` | requires OHLCV |
| `BreakoutMomentum` | ADX on | `GpuHybridKernel` | `GpuHybridKernel` or documented fallback | ADX-bearing hybrid |
| `RegimeHybrid` | current form | `GpuHybridKernel` | `GpuHybridKernel` or documented fallback | ADX-bearing hybrid |
| `Asr` | base | measured route | measured route | currently GPU route exists, Auto may prefer CPU |
| `Vcet` | base | measured route | `GpuKernel` | stronger GPU validation support |
| `Mkcs` | base | measured route | `GpuKernel` | full validation coverage exists |
| `Kqsr` | base | measured route | `GpuKernel` | requires OHLC |

Implementation owners:

- `web/src/strategies/compositeProfileGpuPolicy.ts`
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/services/validationRequestBuilder.ts`
- `crates/app_core/src/composite_profile_route.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/double_ma_validate.rs`
- strategy-local `*_gpu_kernel.rs` modules.

Acceptance criteria:

- Frontend route preview matches backend route policy.
- Backend result route matches expected matrix or returns an explicit fallback reason.
- Validation route builder copy matches backend behavior.
- Tests fail if a known GPU route silently becomes CPU.

### B2. Make Residency A First-Class Gate Field

Residency currently exists as result metadata. Promote it to gate and report authority.

Implementation tasks:

- Ensure optimization result artifacts persist `residency`.
- Ensure validation artifacts persist `execution_profile.cpu_tail_wall_ms`.
- Add gate assertions for:
  - `fully_gpu_resident == true` on exact `GpuKernel` workloads,
  - `fully_gpu_resident == false` only when expected and documented,
  - CPU-staged indicator details are present when hybrid is not fully resident.

Acceptance criteria:

- A route can pass as `GpuHybridKernel` but still fail a fully-resident test when that is required.
- Reports distinguish "GPU backend used" from "fully GPU-resident".

### B3. Fix Or Formalize CompositeTrend ADX-Off Validation Fallback

Current state preserves parity by falling back to CPU for generic Double-MA TrendQuality without ADX confirmation, including `CompositeTrend`.

Decision path:

1. Reproduce the parity blocker with a minimal deterministic fixture.
2. Add a failing parity test that compares CPU and GPU fixed-candidate validation for the exact combo.
3. Fix the GPU validation path if the issue is contained.
4. If the issue is not contained, formalize the fallback:
   - route matrix marks it intentional,
   - frontend warning is explicit,
   - report notes it as parity-protecting fallback,
   - gate asserts the fallback reason.

Acceptance criteria:

- No ambiguous `Auto` behavior for `CompositeTrend` ADX-off validation.
- Either GPU validation is parity-safe or CPU fallback is clearly intentional and tested.

## Workstream C: Validation CUDA Performance

### C1. Split Validation Timing By Module

Before moving more work to CUDA, split current validation timing by module.

Target modules:

- candidate normalization,
- indicator/MA preparation,
- in-sample metric evaluation,
- walk-forward,
- reopt walk-forward,
- CSCV/PBO,
- SPA,
- perturbation,
- trade bootstrap,
- finalist analytics,
- result materialization,
- report/artifact serialization when measured at command boundary.

Implementation owners:

- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/validation_core.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `src-tauri/src/commands_validation.rs`

Acceptance criteria:

- Structured validation summaries show which module dominates runtime.
- GPU validation paths always show CPU-tail attribution when CPU-tail work is present.

### C2. Reduce Duplicate Preparation And Recompute

Likely short-term wins:

- Reuse prepared MA/indicator buffers across validation modules.
- Reuse GPU-uploaded candle and derived-series buffers across candidates.
- Avoid recomputing identical MA pairs during reopt and walk-forward when the candidate grid overlaps.
- Materialize only fields needed for active validation modules.
- Keep finalist metric rows compact until report generation needs expanded detail.

Implementation owners:

- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/strategy_gpu_runtime.rs`

Acceptance criteria:

- No change to validation verdicts.
- CPU-only paths remain unchanged or faster.
- GPU validation structured probe shows reduced preparation/materialization time.

### C3. Move The Highest-ROI CPU-Tail Work

Do not automatically port every CPU-tail module to CUDA. Move only modules that dominate measured runtime and have stable semantics.

Priority order:

1. Aux finalist analytics if it is mostly deterministic scans over already available per-bar state.
2. Perturbation when the perturbation shape is candidate-parallel and does not require expensive host-only random state.
3. Trade bootstrap only if measurement shows it dominates and a GPU implementation can preserve statistical semantics.

Design constraints:

- Randomness must remain deterministic by seed.
- GPU and CPU outputs must be statistically equivalent within documented tolerance.
- Exact per-candidate verdict thresholds must not drift due to float precision changes.
- CPU fallback remains the reference path.

Acceptance criteria:

- Each moved module has parity tests.
- Each moved module has a benchmark before and after.
- Each moved module records whether GPU or CPU execution was used.

### C4. Improve Cross-Ticker Validation Cost

Current cross-ticker validation is wrapper-driven and success-only. Performance work should preserve that contract unless the product semantics are deliberately changed.

Implementation tasks:

- Add command-boundary timing around each cross-ticker dataset.
- Emit per-dataset validation route, fallback, and candidate counts.
- Add an optional preserved-finalists benchmark fixture.
- Avoid reloading/reprojecting candles when the dataset is already loaded or cached.

Acceptance criteria:

- Cross-ticker cost is visible separately from single-dataset validation.
- Reports do not imply inline cross-ticker GPU validation when the wrapper path was used.

## Workstream D: Hybrid And Device-Resident Execution

### D1. Classify Hybrid Runtime Shapes

`GpuHybridKernel` can mean materially different execution shapes. Make the difference explicit.

Proposed classification:

- `GpuHybridKernelFullyResident`: all route-critical indicators and metrics stay device-resident.
- `GpuHybridKernelCpuStagedIndicators`: one or more indicators are prepared on CPU then used by GPU evaluation.
- `GpuHybridKernelFallbackGpuOnly`: GPU indicator generation with host metric evaluation.
- `GpuHybridKernelFallbackCpuOnly`: full CPU fallback.

This does not necessarily require changing the public `Backend` enum. It can live in route diagnostics and report metadata first.

Acceptance criteria:

- Hybrid diagnostics identify why a route is not fully resident.
- Perf gates can assert against the classification.

### D2. Push ADX And Regime Pieces Toward Device Residency

High-value hybrid routes:

- `CompositeTrend` with ADX on.
- `BreakoutMomentum` with ADX on.
- `RegimeHybrid`.

Implementation tasks:

- Identify all CPU-staged ADX/regime/filter preparation for these routes.
- Add device-output indicator support when VectorTA or local CUDA kernels already support the primitive.
- For missing primitives, add a small dedicated kernel only after profiling confirms the CPU-staged piece is material.
- Preserve CPU fallback and parity tests.

Acceptance criteria:

- ADX-bearing routes report less CPU-staged work.
- `GpuHybridKernel` runtime either becomes fully resident for selected profiles or reports a smaller CPU-staged segment.
- No regression in no-ADX `GpuKernel` routes.

### D3. Enforce Strict Resident Mode In Tests

Use `GpuExecutionPolicy::StrictGpuResident` as a test tool for routes that should not CPU-stage.

Acceptance criteria:

- Tests fail if supposedly resident routes introduce CPU-staged indicators.
- Product `BestEffort` and `PreferGpuResident` behavior remains user-friendly.

## Workstream E: High-Speedup Strategy Expansion

### E1. Prioritize By Measured Speedup And Product Use

Initial high-priority indicators from `gpu_speedups.csv`:

- `ttm_squeeze`
- `linearreg_angle`
- `zscore`
- `kurtosis`
- `minmax`
- `kaufmanstop`
- `bollinger_bands_width`
- `squeeze_momentum`
- `bollinger_bands`
- `alma`

Candidate strategy families:

- `Asr`
- `Vcc`
- `Vemr`
- `Rocb`
- `Vcet`
- `Mkcs`
- `Kqsr`
- `Carm`
- `Kvdf`
- `Cdmc`
- dynamic Double-MA profiles that compile through `strategy_plan.rs`.

Acceptance criteria:

- Every GPU-preferred route has a benchmark-backed reason.
- `Auto` routing changes only after measured validation and optimization evidence.
- Frontend capability copy distinguishes supported GPU route from recommended GPU route.

### E2. Promote Strategy Routes In Small Batches

For each strategy family:

1. Document current CPU, GPU, and validation behavior.
2. Add a route probe test.
3. Add an optimization benchmark.
4. Add a validation benchmark.
5. Compare CPU and GPU verdict parity.
6. Enable GPU preference only if the real workflow improves.
7. Add route/fallback UI copy.

Acceptance criteria:

- No strategy switches to GPU-preferred `Auto` based only on kernel availability.
- A strategy may keep GPU as explicit opt-in if small workloads are slower or validation remains CPU-heavy.

## Workstream F: Batching, Cold Start, Uploads, And Caches

### F1. Measure Cold And Warm Separately

Separate these timing boundaries:

- CUDA context/device discovery,
- PTX/module load,
- dynamic strategy program compile,
- executable upload,
- candle upload,
- derived-series upload,
- kernel execution,
- result materialization,
- follow-up cache hit.

Implementation owners:

- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/strategy_gpu_runtime.rs`
- `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs`
- `crates/app_core/benches/double_ma_opt.rs`
- `crates/app_core/benches/double_ma_followup_workflow.rs`

Acceptance criteria:

- Cold-start cost is not mixed with steady-state kernel throughput.
- Warm workflow claims explicitly require cache-hit evidence.

### F2. Avoid Regressing Unbatched Fast Paths

The observed `large_batched_1gb` ALMA case is slower than the default large ALMA GPU path. Treat batching as a memory-pressure tool, not automatically as a speed tool.

Implementation tasks:

- Add benchmark labels that distinguish:
  - default unbatched,
  - memory-capped batched,
  - contract-row variants,
  - dynamic preflight warm,
  - dynamic preflight cold.
- Add guardrails so batching is selected only when needed by memory constraints or when measured faster for a case.

Acceptance criteria:

- The default large ALMA path does not regress because of batching changes.
- VRAM-constrained paths have their own baselines and expectations.

### F3. Improve Follow-Up Cache Reliability

Follow-up workflows matter because users often inspect drilldown, local stability, heatmaps, and validation after optimization.

Implementation tasks:

- Gate `double_ma_followup/gpu_kernel/50k` cached and cold cases.
- Validate `followup_cache_key` persistence in artifacts.
- Ensure cache keys include route-relevant strategy and indicator shape.
- Add diagnostics for cache miss reason.

Acceptance criteria:

- Cached follow-up route is faster than cold route on the baseline hardware.
- Cache misses are explainable.
- Artifact restoration does not lose cacheability when the underlying data is still available.

## Workstream G: Frontend, Artifacts, And Reports

### G1. Surface Actual Route And Residency

Frontend should show actual execution, not only requested backend.

Implementation owners:

- `web/src/services/optimizeService.ts`
- `web/src/services/validationService.ts`
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/services/validationRequestBuilder.ts`
- `web/src/utils/strategyOptimizationCapabilities.ts`
- `web/src/components/ValidationWorkspacePanel.vue`
- `web/src/components/TopWorkspacePanel.vue`
- report builders under `src-tauri/src/commands_reports_validation_*`.

Required fields:

- requested backend,
- actual backend used,
- backend route,
- fallback reason,
- residency tier,
- fully GPU-resident flag,
- CPU-tail wall time,
- GPU materialization wall time,
- validation module mix.

Acceptance criteria:

- A user can tell whether a run was fully GPU-resident, hybrid with CPU-staged work, GPU with CPU tail, or CPU fallback.
- Validation reports preserve the same route truth as the UI.

### G2. Expose Composite-Profile Performance Controls

The performance work depends on route-critical composite-profile flags being visible and testable.

Implementation tasks:

- Expose `trendUseAdx`, `breakoutUseAdx`, `trendLrPeriod`, `regimeAdxPeriod`, and `breakoutBbPeriod` in the strategy configuration UI.
- Keep saved-run replay compatibility.
- Add request-builder tests that prove UI-visible settings drive route selection.

Acceptance criteria:

- Users can select the settings that determine `GpuKernel` versus `GpuHybridKernel`.
- UI route preview updates when those settings change.

### G3. Add Recommended Validation Packs

Add named validation presets that map to measured workload profiles.

Candidate presets:

- `fast_smoke`: in-sample plus minimal gates.
- `research_standard`: walk-forward plus CSCV/PBO.
- `gpu_validation_heavy`: walk-forward, reopt walk-forward, CSCV/PBO, SPA.
- `robustness_full`: heavy validation plus perturbation and trade bootstrap.
- `cross_ticker_preserved_finalists`: wrapper-driven cross-ticker path.

Acceptance criteria:

- Preset names appear in artifacts and reports.
- Perf gates can run at least one preset exactly.
- The UI does not require users to manually assemble a complex measured workflow.

## Workstream H: Profiling Workflow

### H1. Nsight Systems

Use Nsight Systems for timeline questions:

- upload overlap,
- kernel launch frequency,
- CPU wait time,
- context initialization,
- result materialization,
- cross-module gaps.

Command surface:

- `powershell -File scripts/profile_gpu_backtest_nsys.ps1`

Notes:

- Run from elevated PowerShell when CUDA attach permissions require it.
- Do not run concurrently with benchmarks or Nsight Compute.

### H2. Nsight Compute

Use Nsight Compute for kernel-level questions:

- occupancy,
- memory throughput,
- branch divergence,
- register pressure,
- shared-memory behavior,
- global load/store efficiency.

Command surface:

- `powershell -File scripts/profile_gpu_backtest_ncu.ps1`

Acceptance criteria:

- Kernel changes include profiler evidence when they target low-level CUDA behavior.
- Workflow changes use Systems first unless the bottleneck is clearly inside a kernel.

## Execution-Ready Details

This section turns the workstreams into concrete implementation contracts. Prefer updating this section whenever a benchmark case, schema field, or threshold changes.

### Proposed Gate Case Registry

The first GPU validation gate should start small and authoritative. Add broader cases after the summary schema and route assertions are stable.

| Case Id | Bench Filter | Estimates Path | Expected Route | Required Fields |
| --- | --- | --- | --- | --- |
| `double_ma_gpu_validation_sparse` | `double_ma_validate/gpu_kernel/sparse_finalists/100k/8candidates/wf_reopt_cscv_spa` | `target/criterion/double_ma_validate_gpu_kernel_sparse_finalists_100k_8candidates/wf_reopt_cscv_spa/new/estimates.json` | `GpuKernel` | `backend_used`, `backend_route`, `cpu_tail_wall_ms`, `gpu_result_materialization_wall_ms` |
| `composite_gpu_validation_asr` | `composite_validate/gpu_kernel/asr/100k/64candidates/wf_cscv` | `target/criterion/composite_validate_gpu_kernel_asr_100k_64candidates/wf_cscv/new/estimates.json` | `GpuKernel` | `backend_used`, `backend_route`, `backend_fallback_reason` |
| `composite_gpu_validation_vcet` | `composite_validate/gpu_kernel/vcet/100000/64candidates/wf_reopt_cscv_spa` | resolved by glob on `target/criterion/composite_validate_gpu_kernel_vcet_100000_64candidates/**/estimates.json` | `GpuKernel` | `backend_used`, `backend_route`, `backend_fallback_reason` |
| `double_ma_gpu_followup_cached` | `double_ma_followup/gpu_kernel/50k/drilldown_then_local_stability_cached` | `target/criterion/double_ma_followup_gpu_kernel_50k/drilldown_then_local_stability_cached/new/estimates.json` | `GpuKernel` | `followup_cache_key`, cache hit/miss diagnostics |

The exact Criterion directory names should be verified on first baseline run. If Criterion sanitizes a path differently, update this table and the gate script together.

### Initial Benchmark Commands

Run one command at a time. Do not run these in parallel with any other benchmark, profiler, or Cargo bench process.

Build and run the Double-MA GPU validation case:

```powershell
cargo bench -p ta_app_core --bench double_ma_validation_workflow --features cuda,cuda-backtest-kernel -- double_ma_validate/gpu_kernel/sparse_finalists/100k/8candidates/wf_reopt_cscv_spa
```

Build and run the composite GPU validation ASR case:

```powershell
cargo bench -p ta_app_core --bench composite_validation_workflow --features cuda,cuda-backtest-kernel -- composite_validate/gpu_kernel/asr/100k/64candidates/wf_cscv
```

Build and run a specific composite strategy validation profile:

```powershell
cargo bench -p ta_app_core --bench composite_validation_workflow --features cuda,cuda-backtest-kernel -- composite_validate/gpu_kernel/vcet
```

Build and run GPU follow-up cache coverage:

```powershell
cargo bench -p ta_app_core --bench double_ma_followup_workflow --features cuda,cuda-backtest-kernel -- double_ma_followup/gpu_kernel/50k
```

Run the default GPU optimization gate:

```powershell
powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate
```

Run the planned GPU validation gate after implementation:

```powershell
powershell -File scripts/run_gpu_validation_perf_gate.ps1 -Mode candidate
```

### Proposed GPU Validation Summary Schema

`scripts/run_gpu_validation_perf_gate.ps1` should emit a stable JSON envelope. Keep additive fields backward-compatible where practical.

```json
{
  "schema_version": 1,
  "case_registry_version": 1,
  "generated_at_utc": "2026-04-23T00:00:00Z",
  "mode": "candidate",
  "environment": {
    "hostname": "LISZKA",
    "os": "windows",
    "architecture": "x64",
    "powershell_version": "5.1"
  },
  "gpu": "NVIDIA GeForce RTX 4090, 595.71",
  "gpu_info": {
    "name": "NVIDIA GeForce RTX 4090",
    "driver_version": "595.71",
    "raw": "NVIDIA GeForce RTX 4090, 595.71"
  },
  "criterion_cases": [
    {
      "id": "double_ma_gpu_validation_sparse",
      "timing_boundary": "Criterion",
      "build_profile": "bench",
      "bench_filter": "double_ma_validate/gpu_kernel/sparse_finalists/100k/8candidates/wf_reopt_cscv_spa",
      "estimates_json": "target/criterion/.../estimates.json",
      "mean_ms": 0.0,
      "mean_ci_lower_ms": 0.0,
      "mean_ci_upper_ms": 0.0,
      "median_ms": 0.0
    }
  ],
  "structured_cases": [
    {
      "id": "double_ma_gpu_validation_sparse",
      "workload_id": "double_ma_validate_gpu_sparse_100k_8",
      "requested_backend": "GpuKernel",
      "backend_used": "Gpu",
      "backend_route": "GpuKernel",
      "backend_fallback_reason": null,
      "residency": {
        "backend_route": "GpuKernel",
        "fully_gpu_resident": true,
        "cpu_staged_indicators": []
      },
      "candles_len": 100000,
      "candidate_count": 8,
      "module_mix": [
        "in_sample",
        "walk_forward",
        "reopt_walk_forward",
        "cscv_pbo",
        "spa"
      ],
      "timing_ms": {
        "total_wall": 0,
        "candidate_normalization": 0,
        "indicator_prepare": 0,
        "in_sample": 0,
        "walk_forward": 0,
        "reopt_walk_forward": 0,
        "cscv_pbo": 0,
        "spa": 0,
        "perturbation": 0,
        "trade_bootstrap": 0,
        "aux_finalist_analytics": 0,
        "gpu_result_materialization": 0,
        "cpu_tail": 0
      }
    }
  ]
}
```

### Regression Threshold Policy

Initial thresholds should be conservative until enough validation baselines exist.

| Metric | Initial Threshold | Failure Mode |
| --- | --- | --- |
| Criterion mean per case | `+10%` versus baseline | fail candidate gate |
| Criterion CI width | warn above `10%` of mean | warn, do not fail initially |
| Structured total wall time | `+15%` versus baseline | fail after two stable baselines exist |
| Validation CPU-tail time | `+20%` versus baseline | fail after CPU-tail baseline exists |
| Expected `GpuKernel` route | exact match required | fail immediately |
| Unexpected fallback reason | must be null unless case explicitly allows fallback | fail immediately |
| Expected intentional fallback | exact fallback reason required | fail immediately |
| GPU identity | GPU name must match baseline when present | fail candidate gate |

Use `+10%` for mature optimization cases and `+15%` for new validation-heavy cases until variance is characterized.

### Dependency Order

Do the work in this dependency order unless a bug fix forces a narrower detour:

1. Add structured validation probe output.
2. Add the GPU validation perf gate around existing benches.
3. Capture baseline data for the initial case registry.
4. Add route/fallback/residency assertions.
5. Persist and surface validation CPU-tail and residency fields.
6. Add validation-heavy workflow mode.
7. Split validation timing by module.
8. Optimize duplicate preparation/materialization.
9. Move measured CPU-tail hotspots to CUDA.
10. Expand strategy GPU preference one family at a time.

Avoid CUDA kernel rewrites before steps 1-7 are complete unless profiling identifies a severe kernel-only bottleneck.

### Definition Of Done

A performance improvement is done only when all applicable items are true:

- The change has a before/after measurement from the relevant local gate or profiling script.
- The measured workload is named and reproducible.
- Route, fallback, and residency are recorded for the measured workload.
- CPU-tail time is recorded for validation workloads.
- CPU-only builds still compile and tests remain valid.
- GPU-required tests are guarded by CUDA feature/runtime availability.
- Frontend route preview and backend result route agree.
- Artifacts and reports preserve the route truth needed to debug the run later.
- Any public performance claim is backed by a fresh local artifact.
- Existing feature coverage is unchanged or deliberately expanded.
- Legacy artifact load/reopen behavior remains compatible.
- Validation verdicts, options validation verdicts, and report outputs have parity or compatibility evidence.
- Explicit CPU and GPU override paths remain available where they were available before the change.

### First Six Tickets

These are the recommended initial tickets if the plan is converted into issue tracker work.

1. **Structured GPU Validation Probe**
   - Add the probe or extend `ta_double_ma_validation_cost.rs`.
   - Emit route, residency, module mix, and timing JSON.
   - Add serialization tests.

2. **GPU Validation Perf Gate Script**
   - Add `run_gpu_validation_perf_gate.ps1`.
   - Run the initial Double-MA and composite GPU validation cases.
   - Emit candidate and baseline summaries.

3. **Route And Residency Gate Assertions**
   - Add exact route checks.
   - Add fallback checks.
   - Add residency checks for exact `GpuKernel` cases.

4. **Validation CPU-Tail Artifact Plumbing**
   - Ensure `cpu_tail_wall_ms` persists from app-core validation result into Tauri artifacts, frontend state, and reports.
   - Add tests around missing legacy fields.

5. **Composite Profile Route Matrix Tests**
   - Add tests for `CompositeTrend`, `BreakoutMomentum`, and `RegimeHybrid`.
   - Explicitly cover ADX-on and ADX-off route expectations.

6. **Validation Timing Split**
   - Add per-module timing fields.
   - Establish the first CPU-tail and module-cost baseline.
   - Identify the first hotspot candidate for optimization.

## Expanded Review Workstreams

These workstreams extend the performance plan beyond conventional backtest optimization while preserving the original first implementation slice.

### X1. Options-Expression Performance Authority

Current options-specific authority already exists through `scripts/run_options_expression_gpu_promotion_gate.ps1`. The expanded plan should make that authority comparable to the conventional GPU perf gates.

Implementation tasks:

- Add a stable options performance summary baseline path, for example `scripts/baselines/options_expression_perf_baseline.json`.
- Preserve the existing promotion gate checks:
  - candidate density,
  - invalid-slot rate,
  - structural cache reuse,
  - summary cache reuse,
  - GPU hybrid-ranking speed,
  - estimated backend preview wall speedup.
- Add explicit timing buckets for:
  - parent artifact validation,
  - signal manifest loading/projection,
  - options dataset manifest validation,
  - candidate tuple compilation,
  - CPU reference scoring,
  - GPU tensor scoring,
  - GPU ranking,
  - exactification,
  - sealed-finalist stage,
  - artifact persistence,
  - compare/reopen summary loading.
- Add backend route fields:
  - requested backend preference,
  - recommended backend,
  - backend used,
  - fallback reason,
  - GPU eligibility,
  - CPU/GPU comparison result.

Acceptance criteria:

- Options performance regressions can fail independently from conventional backtest gates.
- Options GPU promotion decisions are based on local gate artifacts, not ad hoc timings.
- The gate distinguishes compile/prep bottlenecks from GPU scoring bottlenecks.

### X2. Options CUDA Profiling

The options CUDA scorer is separate from the conventional backtest kernel and should be profiled separately.

Primary files:

- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_expression_gpu.rs`
- `crates/app_core/src/kernels/options_expression.cu`
- `scripts/run_options_expression_gpu_promotion_gate.ps1`
- `scripts/profile_options_expression_nsys.ps1`
- `scripts/profile_options_expression_ncu.ps1`
- `scripts/profile_options_expression_compute_sanitizer.ps1`

Profiling questions:

- Is the bottleneck tuple compilation, data marshaling, GPU tensor scoring, ranking, exactification, or sealed-finalist rescoring?
- Are GPU tuple chunks sized well for the target device memory budget?
- Are output buffers reused at the expected rate?
- Are kernel launches too frequent for the tuple count?
- Does the outcome tensor fit in memory, and does the fallback chunk plan degrade throughput?
- Does cache reuse survive repeated preview/run/reopen flows?

Acceptance criteria:

- Nsight Systems evidence exists before optimizing host/device orchestration.
- Nsight Compute evidence exists before changing the options CUDA kernel.
- Compute Sanitizer is used for options CUDA correctness-sensitive kernel changes.

### X3. Full Conventional Validation Coverage

The conventional validation review should become a first-class performance surface, not a side effect of optimization.

Validation feature matrix:

| Feature | Double-MA Family | Composite/Advanced Family | Current Review Need |
| --- | --- | --- | --- |
| in-sample fixed-candidate metrics | CPU/GPU | CPU/GPU for eligible routes | gate route and timing |
| walk-forward | CPU/GPU | CPU/GPU for supported profiles | gate timing and parity |
| reopt walk-forward | CPU/GPU | profile-dependent GPU support | split timing |
| CSCV/PBO | CPU/GPU | CPU/GPU for supported profiles | gate route and cost |
| SPA | CPU/GPU | CPU/GPU for supported profiles | gate route and cost |
| perturbation | GPU route with CPU-tail risk | supported profiles vary | measure CPU-tail |
| trade bootstrap | GPU route with CPU-tail risk | supported profiles vary | measure CPU-tail |
| cross-ticker | wrapper-driven | preserved-finalists wrapper | command-boundary timing |
| aux analytics | broader Double-MA set | narrower composite set | report parity gaps |
| XLSX/report export | shared validation reports | shared validation reports | measure artifact/report overhead |

Acceptance criteria:

- Each validation feature has a benchmark or structured probe owner.
- Each validation feature has route/fallback expectations.
- Full validation stack cost is visible by module and strategy family.

### X4. Cross-Workflow Performance Dashboard Schema

Create one lightweight summary vocabulary that can compare conventional and options workflows without merging their internals.

Shared fields:

- workflow family,
- workload id,
- dataset shape,
- candidate/tuple count,
- requested backend,
- recommended backend,
- backend used,
- route or backend mode,
- fallback reason,
- CPU reference time,
- GPU scoring time,
- validation time,
- CPU-tail time,
- materialization time,
- artifact/report time,
- cache hit/miss counters,
- memory budget counters,
- pass/fail gate checks.

Acceptance criteria:

- Conventional and options performance summaries can be read side by side.
- Shared fields do not erase route-specific details.
- The schema can support a future local dashboard or markdown report generator.

### X5. Options And Conventional Validation Report Truth

Performance work should improve report truthfulness, not only speed.

Implementation tasks:

- Ensure conventional validation reports include actual route, fallback, residency, CPU-tail, and module mix.
- Ensure options artifacts and summaries include backend preference, backend used, fallback reason, GPU speedup, invalid-slot rate, cache reuse, and validation verdict.
- Ensure saved artifact replay shows whether reopened results are using cached summaries or recomputed paths.
- Add tests for legacy artifacts missing new performance fields.

Acceptance criteria:

- Reports explain why a run was fast or slow enough to debug later.
- Replay and compare surfaces do not hide backend fallback or cache miss reasons.

## Implementation Phases

### Phase 0: Baseline Refresh And Evidence Lock

Tasks:

- Run `scripts/run_workflow_gate.ps1 -Mode candidate`.
- Run `scripts/run_cpu_perf_gate.ps1 -Mode candidate`.
- Run `scripts/run_gpu_perf_gate.ps1 -Mode candidate`.
- Run `scripts/run_gpu_alma_contract_rows_perf_gate.ps1 -Mode candidate` only if touching contract-row output.
- Capture current validation Criterion estimates for Double-MA and composite GPU validation benches.
- Capture structured route/residency/CPU-tail output for representative validation workloads.

Exit criteria:

- Fresh baseline/candidate summaries exist for optimization, workflow, and validation.
- The team knows whether validation, CPU-tail, upload, or kernel execution is the current dominant cost.

### Phase 1: Perf Gate Implementation

Tasks:

- Add `run_gpu_validation_perf_gate.ps1`.
- Add `gpu_validation_perf_baseline.json`.
- Add structured validation perf probe output.
- Add route and fallback assertions to the gate.
- Add process serialization checks.

Exit criteria:

- GPU validation regressions can fail locally before handoff.
- Route fallback regressions fail even if raw timing still looks good.

### Phase 2: Route, Residency, And Reporting

Tasks:

- Add route matrix tests.
- Persist residency and CPU-tail metadata in artifacts.
- Surface route/residency/fallback in UI and XLSX reports.
- Expose composite-profile route-critical controls in the UI.

Exit criteria:

- Users and gates see actual execution shape.
- Hybrid routes no longer hide CPU-staged work.

### Phase 3: Validation Performance Reductions

Tasks:

- Split validation timing by module.
- Remove duplicate preparation and recompute.
- Reuse GPU buffers across validation modules where safe.
- Reduce materialization of unused metric fields.
- Move highest-ROI CPU-tail module only after measurement.

Exit criteria:

- Validation-heavy gate improves or has a justified stable baseline.
- CPU-tail attribution is lower or better explained.
- Validation verdict parity is unchanged.

### Phase 4: Hybrid Residency Improvements

Tasks:

- Profile ADX/regime/filter CPU-staged paths.
- Move material ADX/regime primitives to device-resident paths where practical.
- Add strict-resident tests for no-ADX and resident-expected routes.
- Keep ADX-bearing fallback documented where CPU staging remains intentional.

Exit criteria:

- `CompositeTrend`, `BreakoutMomentum`, and `RegimeHybrid` have clearer and faster hybrid execution behavior.
- `GpuHybridKernel` diagnostics identify residency quality.

### Phase 5: Strategy Expansion

Tasks:

- Prioritize strategies using `gpu_speedups.csv`, product usage, and measured validation cost.
- Promote one strategy family at a time.
- Add route probes, optimization benches, validation benches, and parity tests per family.
- Update frontend capability summaries and report copy.

Exit criteria:

- More strategies benefit from GPU execution without degrading `Auto` defaults.
- GPU support claims remain benchmark-backed.

### Phase 6: Release Hardening

Tasks:

- Update `RELEASE.md` if packaging or public GPU claims change.
- Update `README.md` benchmark notes only after fresh faceoff evidence.
- Run release-candidate gates for CUDA changes.
- Capture profiler summaries for substantial CUDA kernel changes.

Exit criteria:

- Performance claims, packaging notes, and release gates reflect the implemented behavior.

## Verification Command Matrix

Use these commands selectively based on changed surface:

| Change Type | Required Commands |
| --- | --- |
| Route policy only | `cargo test -p ta_app_core`, frontend request-builder tests |
| Frontend route UI | `npm --prefix web run test`, `npm --prefix web run typecheck` |
| Optimization GPU path | `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate` |
| Validation GPU path | `powershell -File scripts/run_gpu_validation_perf_gate.ps1 -Mode candidate` |
| Broad workflow timing | `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate` |
| CPU perf-sensitive validation | `powershell -File scripts/run_cpu_perf_gate.ps1 -Mode candidate` |
| ALMA contract rows | `powershell -File scripts/run_gpu_alma_contract_rows_perf_gate.ps1 -Mode candidate` |
| Low-level CUDA kernel change | Nsight Systems or Nsight Compute plus GPU perf gate |
| Report/artifact changes | output parity checks plus validation XLSX tests |
| Options-expression perf change | `powershell -File scripts/run_options_expression_gpu_promotion_gate.ps1` plus CPU/GPU comparison evidence |
| Validation feature coverage change | Double-MA and composite validation tests plus request-builder coverage |
| Legacy artifact compatibility | saved-run load/reopen tests plus report compatibility tests |
| Behavior-preserving refactor | targeted Rust/frontend tests plus artifact/report parity where outputs can change |

Benchmark discipline:

- Never run multiple benchmarks at the same time.
- Before a benchmark or profiler run, verify no benchmark/profiler process is already active.
- Run workflow gates before perf gates when the change may affect staged workflow timing.
- Prefer local perf gates as the authority for timing conclusions.

## Risk Register

| Risk | Impact | Mitigation |
| --- | --- | --- |
| Optimizing kernel throughput while validation dominates workflow cost | Low product impact | Add validation-heavy gate before deep kernel work |
| GPU route silently falls back to CPU | Misleading UX and invalid perf evidence | Gate route, fallback reason, and residency |
| Hybrid route remains CPU-staged but is presented as GPU-resident | Misleading reports | Add hybrid classification and report copy |
| GPU validation changes alter verdicts | Trust loss | CPU/GPU parity tests and deterministic fixtures |
| Randomized robustness modules differ between CPU and GPU | Unstable validation decisions | Seeded tests, statistical tolerance, CPU reference |
| Batching improves memory posture but hurts normal throughput | Slower default path | Separate memory-capped baselines from default baselines |
| CUDA-specific gates block CPU-only contributors | Workflow friction | Keep CUDA gates opt-in and keep CPU-only tests passing |
| Nsight/profiling data is captured under contaminated load | Bad conclusions | Serialize runs and check no perf processes are active |
| Performance shortcut disables a validation module | Reduced decision quality | Gate full validation feature reachability and use additive presets only |
| Faster GPU path changes validation verdicts | Incorrect recommendations | CPU/GPU parity fixtures, seeded randomized modules, and tolerance review |
| New summary fields break saved artifacts | Broken replay/reports | Additive schemas and legacy compatibility tests |
| Auto route promotion removes explicit control | User loses fallback/control | Preserve explicit backend override paths in UI and backend |
| Options GPU optimization reduces candidate coverage | Biased options results | Gate candidate density, invalid-slot rate, and structure-family coverage |
| Report simplification drops diagnostic fields | Harder debugging and audit | Report compatibility tests and required route/fallback/residency fields |

## Open Decisions

1. Should GPU validation become part of the default `run_gpu_perf_gate.ps1`, or stay as a separate `run_gpu_validation_perf_gate.ps1` until stable?
2. Should `GpuHybridKernel` public route labels be refined, or should detailed residency remain report-only metadata?
3. Should `CompositeTrend` ADX-off GPU validation parity be fixed now, or should the CPU fallback be formalized for the next release?
4. Which validation preset should become the recommended default in the UI?
5. What minimum workflow speedup is required before promoting each advanced strategy to GPU-preferred `Auto`?
6. Which legacy artifact/report samples should become mandatory compatibility fixtures for the expanded performance work?
7. Which validation feature matrix entries are release-blocking versus advisory for the next performance slice?

## First Implementation Slice

Recommended first slice:

1. Add the structured validation perf probe.
2. Add `scripts/run_gpu_validation_perf_gate.ps1`.
3. Add route/fallback/residency assertions for one Double-MA GPU validation case and one composite GPU validation case.
4. Persist and surface `cpu_tail_wall_ms` in validation artifacts and reports if not already present end-to-end.
5. Add a small route matrix test for `CompositeTrend`, `BreakoutMomentum`, and `RegimeHybrid`.

This slice creates the evidence loop needed for all later CUDA performance work.

## Implementation Checklist

Use this checklist as the execution tracker. Keep checked items tied to landed code, refreshed evidence, or an explicit decision recorded in this document.

### Baseline And Evidence

- [ ] Confirm no benchmark, profiler, Cargo bench, Nsight, or CUPTI process is already running before collecting performance evidence.
- [ ] Run `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate`.
- [ ] Run `powershell -File scripts/run_cpu_perf_gate.ps1 -Mode candidate`.
- [ ] Run `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`.
- [ ] Capture current `double_ma_validation_workflow` GPU Criterion estimates.
- [ ] Capture current `composite_validation_workflow` GPU Criterion estimates.
- [ ] Capture current `double_ma_followup_workflow` GPU Criterion estimates.
- [ ] Record current Double-MA GPU validation route, fallback, residency, and CPU-tail telemetry.
- [ ] Record current composite/advanced GPU validation route, fallback, residency, and CPU-tail telemetry.
- [ ] Decide whether current validation-heavy timing warrants new baseline files before code changes.

### Regression And Feature Preservation

- [ ] Inventory current conventional optimization strategy coverage before performance changes.
- [ ] Inventory current conventional validation feature coverage before performance changes.
- [ ] Inventory current options-expression structure family coverage before performance changes.
- [ ] Inventory current options-expression backend choices before performance changes.
- [ ] Add or identify legacy saved-run fixtures that must continue to load.
- [ ] Add or identify legacy validation artifact fixtures that must continue to export reports.
- [ ] Add or identify options-expression artifact fixtures that must continue to reopen and compare.
- [ ] Verify CPU-only builds still work after CUDA-related changes.
- [ ] Verify explicit CPU backend override paths still work.
- [ ] Verify explicit GPU backend override paths still work where CUDA is available.
- [ ] Verify frontend request builders still expose all existing validation modules.
- [ ] Verify backend commands still accept all existing validation module fields.
- [ ] Verify named validation presets are additive and do not remove advanced manual configuration.
- [ ] Verify report exports keep existing sheets, columns, verdict fields, and warnings.
- [ ] Verify performance shortcuts do not skip user-selected validation modules.
- [ ] Verify route changes are surfaced in UI summaries, artifacts, and reports.
- [ ] Add rollback notes for any performance change that cannot preserve prior behavior.

### GPU Validation Perf Gate

- [ ] Create `scripts/run_gpu_validation_perf_gate.ps1`.
- [ ] Add `scripts/baselines/gpu_validation_perf_baseline.json`.
- [ ] Add `output/benchmarks/gpu_validation_perf_summary.json` as the default output path.
- [ ] Reuse process-drain checks from `scripts/run_gpu_perf_gate.ps1`.
- [ ] Build validation benches with `--features cuda,cuda-backtest-kernel`.
- [ ] Add `-Mode baseline` support.
- [ ] Add `-Mode candidate` support.
- [ ] Add custom single-case support for focused profiling.
- [ ] Enforce GPU identity compatibility against baseline when available.
- [ ] Enforce per-case regression thresholds.
- [ ] Fail when a required Criterion estimates file is missing.
- [ ] Fail when a required GPU case falls back to CPU unexpectedly.
- [ ] Emit one structured summary containing all validation cases.

### Structured Validation Probe

- [ ] Choose whether to extend `ta_double_ma_validation_cost.rs` or add a new app-core perf probe binary.
- [ ] Emit schema version and build profile.
- [ ] Emit GPU identity and CUDA feature/runtime availability.
- [ ] Emit workload id, strategy/profile, candles length, and candidate count.
- [ ] Emit requested backend, backend used, backend route, and fallback reason.
- [ ] Emit route diagnostics and residency summary.
- [ ] Emit validation module mix.
- [ ] Emit total wall time.
- [ ] Emit in-sample, walk-forward, reopt, CSCV, SPA, perturbation, trade-bootstrap, aux-analytics, materialization, and CPU-tail timings where available.
- [ ] Add tests for summary serialization compatibility.
- [ ] Wire the structured probe into `run_gpu_validation_perf_gate.ps1`.

### Workflow Gate Expansion

- [ ] Decide whether to extend `ta_workflow_regression.rs` with modes or create a separate validation-heavy workflow binary.
- [ ] Preserve the current basic workflow mode and baseline.
- [ ] Add a validation-heavy workflow mode.
- [ ] Add a CUDA-required GPU-validation workflow mode if practical.
- [ ] Include walk-forward in validation-heavy mode.
- [ ] Include reopt walk-forward in validation-heavy mode.
- [ ] Include CSCV/PBO in validation-heavy mode.
- [ ] Include SPA in validation-heavy mode.
- [ ] Include perturbation in validation-heavy mode.
- [ ] Include trade bootstrap in validation-heavy mode.
- [ ] Include cross-ticker preserved-finalists wrapper coverage.
- [ ] Emit separate stage timings for optimize, validation, meta, holdout, and report/artifact work if measured.
- [ ] Add stage-level regression budget checks for validation-heavy mode.

### Route Matrix And Policy Tests

- [ ] Document the route matrix in test fixtures or a small shared table.
- [ ] Add optimization route tests for `DoubleMa` SMA/SMA and ALMA/ALMA.
- [ ] Add validation route tests for `DoubleMa` GPU fixed-candidate validation.
- [ ] Add optimization route tests for `CompositeTrend` ADX off and ADX on.
- [ ] Add validation route tests for `CompositeTrend` ADX-off intentional CPU fallback or fixed GPU parity.
- [ ] Add optimization route tests for `BreakoutMomentum` ADX off and ADX on.
- [ ] Add validation route tests for `BreakoutMomentum` route expectations.
- [ ] Add optimization route tests for `RegimeHybrid`.
- [ ] Add validation route tests for `RegimeHybrid`.
- [ ] Add route probes for at least `Asr`, `Vcet`, `Mkcs`, and `Kqsr`.
- [ ] Ensure frontend route preview tests match backend route policy.
- [ ] Ensure validation request-builder warning copy matches backend behavior.

### Residency And Fallback Hardening

- [ ] Persist optimization `residency` metadata in saved run artifacts.
- [ ] Persist validation `execution_profile.cpu_tail_wall_ms` in validation artifacts.
- [ ] Add gate assertions for `fully_gpu_resident == true` on exact resident `GpuKernel` workloads.
- [ ] Add gate assertions for expected non-resident hybrid cases.
- [ ] Add tests for `StrictGpuResident` rejecting CPU-staged routes.
- [ ] Add diagnostics for CPU-staged indicator names and reasons.
- [ ] Add explicit route notes for `GpuHybridKernel->GpuOnly` and `GpuHybridKernel->CpuOnly`.
- [ ] Make unintended fallback a test and gate failure.

### CompositeTrend ADX-Off Validation Decision

- [ ] Create a minimal deterministic fixture for the current ADX-off TrendQuality validation parity gap.
- [ ] Add CPU versus GPU fixed-candidate validation comparison.
- [ ] Identify the exact mismatch source if parity fails.
- [ ] Fix the GPU validation path if the mismatch is contained.
- [ ] If not fixed, mark CPU fallback as intentional in route matrix, UI copy, artifacts, and reports.
- [ ] Add a test asserting the exact fallback reason.

### Validation Performance Work

- [ ] Split timing for candidate normalization.
- [ ] Split timing for indicator and MA preparation.
- [ ] Split timing for in-sample metric evaluation.
- [ ] Split timing for walk-forward.
- [ ] Split timing for reopt walk-forward.
- [ ] Split timing for CSCV/PBO.
- [ ] Split timing for SPA.
- [ ] Split timing for perturbation.
- [ ] Split timing for trade bootstrap.
- [ ] Split timing for aux finalist analytics.
- [ ] Split timing for result materialization.
- [ ] Identify the top two validation cost centers from measured data.
- [ ] Reuse prepared MA/indicator buffers across validation modules where safe.
- [ ] Reuse uploaded candle and derived-series buffers across candidates where safe.
- [ ] Avoid duplicate MA-pair recompute during walk-forward and reopt where candidate grids overlap.
- [ ] Reduce materialization of metric fields that inactive modules do not consume.
- [ ] Move only the highest-ROI CPU-tail module to CUDA after measurement.
- [ ] Add CPU/GPU parity tests for any moved validation module.

### Cross-Ticker Validation

- [ ] Add command-boundary timing for each cross-ticker dataset.
- [ ] Emit per-dataset route, fallback, candidate count, and pass count.
- [ ] Add a preserved-finalists benchmark fixture.
- [ ] Measure dataset reload/reprojection cost.
- [ ] Avoid reload/reprojection where loaded or cached data can be reused safely.
- [ ] Ensure reports describe wrapper-driven cross-ticker validation accurately.

### Hybrid Device Residency

- [ ] Add hybrid runtime classification metadata.
- [ ] Classify fully resident hybrid routes.
- [ ] Classify CPU-staged-indicator hybrid routes.
- [ ] Classify `GpuHybridKernel->GpuOnly` fallback routes.
- [ ] Classify `GpuHybridKernel->CpuOnly` fallback routes.
- [ ] Profile ADX/regime/filter CPU-staged work for `CompositeTrend`.
- [ ] Profile ADX/regime/filter CPU-staged work for `BreakoutMomentum`.
- [ ] Profile ADX/regime/filter CPU-staged work for `RegimeHybrid`.
- [ ] Move material ADX/regime/filter primitives to device-resident paths where practical.
- [ ] Add strict-resident tests for no-ADX resident routes.

### High-Speedup Strategy Expansion

- [ ] Rank candidate strategy work by `gpu_speedups.csv`, current product use, and validation cost.
- [ ] Pick one strategy family for the first promotion slice.
- [ ] Document current CPU, GPU, and validation behavior for that strategy.
- [ ] Add route probe tests.
- [ ] Add an optimization benchmark.
- [ ] Add a validation benchmark.
- [ ] Add CPU/GPU verdict parity tests.
- [ ] Decide whether `Auto` should prefer GPU or keep GPU explicit opt-in.
- [ ] Update frontend capability summaries.
- [ ] Update report copy for the strategy route.

### Batching, Uploads, Cold Start, And Cache

- [ ] Separate CUDA context/device discovery timing.
- [ ] Separate PTX/module load timing.
- [ ] Separate dynamic strategy program compile timing.
- [ ] Separate executable upload timing.
- [ ] Separate candle upload timing.
- [ ] Separate derived-series upload timing.
- [ ] Separate kernel execution timing.
- [ ] Separate result materialization timing.
- [ ] Separate follow-up cache hit timing.
- [ ] Add benchmark labels for default unbatched paths.
- [ ] Add benchmark labels for memory-capped batched paths.
- [ ] Ensure batching is selected only for memory pressure or measured speed wins.
- [ ] Gate cached and cold `double_ma_followup/gpu_kernel/50k` cases.
- [ ] Add cache miss diagnostics.

### Frontend, Artifacts, And Reports

- [ ] Surface requested backend in optimization result summaries.
- [ ] Surface actual backend used in optimization result summaries.
- [ ] Surface backend route in optimization result summaries.
- [ ] Surface fallback reason in optimization result summaries.
- [ ] Surface residency tier and fully GPU-resident flag in optimization result summaries.
- [ ] Surface requested backend in validation summaries.
- [ ] Surface actual backend used in validation summaries.
- [ ] Surface backend route in validation summaries.
- [ ] Surface fallback reason in validation summaries.
- [ ] Surface CPU-tail wall time in validation summaries.
- [ ] Include route, residency, fallback, and CPU-tail fields in validation XLSX reports.
- [ ] Expose `trendUseAdx` in the strategy configuration UI.
- [ ] Expose `breakoutUseAdx` in the strategy configuration UI.
- [ ] Expose `trendLrPeriod` in the strategy configuration UI.
- [ ] Expose `regimeAdxPeriod` in the strategy configuration UI.
- [ ] Expose `breakoutBbPeriod` in the strategy configuration UI.
- [ ] Add request-builder tests proving these controls affect route selection.
- [ ] Add named validation presets for measured workflows.

### Expanded Options And Full Validation Scope

- [ ] Add an options-expression performance baseline file.
- [ ] Add options-expression performance summary output aligned with conventional perf summaries.
- [ ] Preserve candidate-density and invalid-slot checks in the options GPU promotion gate.
- [ ] Preserve structural-cache and summary-cache reuse checks in the options GPU promotion gate.
- [ ] Preserve GPU speedup and backend preview wall-speed checks in the options GPU promotion gate.
- [ ] Add timing for options parent artifact validation.
- [ ] Add timing for options signal manifest projection/loading.
- [ ] Add timing for options dataset manifest validation.
- [ ] Add timing for options tuple compilation.
- [ ] Add timing for options CPU reference scoring.
- [ ] Add timing for options CUDA tensor scoring.
- [ ] Add timing for options GPU ranking.
- [ ] Add timing for options exactification.
- [ ] Add timing for options sealed-finalist stage.
- [ ] Add timing for options artifact persistence.
- [ ] Add timing for options compare/reopen summary loading.
- [ ] Add options backend fields for requested preference, recommended backend, backend used, fallback reason, GPU eligibility, and CPU/GPU comparison result.
- [ ] Profile options host/device orchestration with Nsight Systems when options gate data points to transfer, launch, or cache overhead.
- [ ] Profile `options_expression.cu` with Nsight Compute before low-level CUDA kernel changes.
- [ ] Run Compute Sanitizer for correctness-sensitive options CUDA changes.
- [ ] Add a conventional validation feature matrix owner for in-sample metrics.
- [ ] Add a conventional validation feature matrix owner for walk-forward.
- [ ] Add a conventional validation feature matrix owner for reopt walk-forward.
- [ ] Add a conventional validation feature matrix owner for CSCV/PBO.
- [ ] Add a conventional validation feature matrix owner for SPA.
- [ ] Add a conventional validation feature matrix owner for perturbation.
- [ ] Add a conventional validation feature matrix owner for trade bootstrap.
- [ ] Add a conventional validation feature matrix owner for cross-ticker preserved-finalists validation.
- [ ] Add a conventional validation feature matrix owner for aux analytics.
- [ ] Add a conventional validation feature matrix owner for XLSX/report generation.
- [ ] Define the cross-workflow performance summary schema.
- [ ] Ensure options artifacts preserve backend/fallback/speedup/cache/validation-verdict fields.
- [ ] Ensure conventional validation reports preserve route/residency/fallback/CPU-tail/module-mix fields.
- [ ] Add legacy artifact/report tests for missing expanded performance fields.

### Profiling And Release Hardening

- [ ] Run Nsight Systems for workflow timeline issues before kernel-level tuning.
- [ ] Run Nsight Compute only when the bottleneck is inside a CUDA kernel.
- [ ] Store profiler notes for substantial CUDA changes.
- [ ] Update `RELEASE.md` if GPU packaging or public CUDA behavior changes.
- [ ] Update `README.md` benchmark claims only after fresh faceoff evidence.
- [ ] Run release-candidate gates for broad CUDA changes.
