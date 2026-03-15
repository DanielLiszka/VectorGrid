# Validation XLSX Report Modernization Plan

Date: 2026-03-13
Status: Implementation in progress; XLSX modernization is functionally implemented and verified, Double-MA CPU/GPU parity is materially improved, preserved-finalist cross-ticker performance is now materially reduced, verdict-relevant rerun policy is in place, pre-run route-capability surfacing is now implemented in the validation UI and XLSX dashboard, and broader strategy-family support plus the remaining GPU-native parity decisions remain the main active gaps
Scope: `ta_desktop_demo` validation-stage XLSX output for all implemented strategies

Purpose: define a purpose-built plan to bring XLSX analysis/output up to the current validation model, replace outdated shortlist logic, and provide a progress checklist that can be updated during implementation.

This document is intentionally about the Rust/Tauri app only. The external Python `SERENITY` workbook and legacy `__init__.py` are inspiration for information design, not implementation dependencies.

## Goal

Produce an XLSX report that communicates validation outcomes instantly and accurately for all implemented strategies.

The target is not "export a spreadsheet". The target is:

- reflect the actual end-of-validation approval semantics
- summarize the best evidence without hiding raw detail
- surface current validation-stage signals across all supported strategies
- preserve CPU fallback behavior and current artifact persistence flows
- make the report useful both interactively and in headless/automated runs

## Why This Work Is Needed

Current XLSX export exists, but it is behind the current state of validation.

Current issues confirmed in source:

- report filtering still keys primarily off `robust_pareto`, `cross_ticker_pareto`, and optional CSCV thresholds
- report builders do not currently use the newer final approval semantics such as candidate-level `validation_verdict` and run-level `run_verdict`
- the validation UI exposes XLSX export, but the "auto-export after validation" toggle appears not to be wired into `runValidation()`
- current workbook structure is still largely generic and MA-shaped rather than truly aligned to the current strategy-agnostic validation model
- XLSX test coverage is smoke-level only and does not validate workbook semantics deeply
- normal validation export still has legacy-compatibility concerns because saved artifacts may exist without the latest verdict fields
- `exportReportXlsx()` currently has a Stage 3 meta-report fallback path when no validation artifact exists, and that behavior must remain explicit while this validation-focused work is scoped separately

## Product Direction

The right direction is:

- keep the "instant-read" clarity of the old `SERENITY` workbook
- do not copy the old giant hard-coded single-sheet implementation model
- use the current Rust builder as the technical foundation
- redesign the workbook around current validation semantics and strategy-agnostic result fields

The report should answer these questions immediately:

1. Did the run approve any candidate?
2. What is the final recommendation?
3. How many candidates were approved and why?
4. Which candidate is currently best and what are its parameters?
5. How did it perform across walk-forward, CSCV/PBO, PSR/DSR, cross-ticker, bootstrap, perturbation, and any other active validation layers?
6. What does the broader shortlist look like?
7. What raw evidence is available for deeper inspection?

## Current Code Surfaces

Primary backend export surfaces:

- `src-tauri/src/commands_reports.rs`
- `src-tauri/src/commands_reports_csv.rs`
- `src-tauri/src/commands_reports_api.rs`
- `src-tauri/src/commands_reports_shared.rs`

Primary validation/result surfaces:

- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/composite_validate.rs`
- `src-tauri/src/commands_validation.rs`
- `src-tauri/src/run_artifacts.rs`

Primary frontend surfaces:

- `web/src/services/validationService.ts`
- `web/src/stores/validationStore.ts`
- `web/src/components/ValidationWorkspacePanel.vue`
- `web/src/services/historyService.ts`
- `web/src/stores/workflowArtifactsStore.ts`

Related but separate XLSX surface:

- `src-tauri/src/commands_reports_meta_xlsx.rs`

## Scope Rules

In scope:

- validation-stage report XLSX for all currently implemented strategies
- manual export from the validation workspace
- optional auto-export after validation
- headless validate-to-XLSX flow
- workbook semantics, color rules, chart set, and test coverage

Out of scope for this document:

- rewriting meta-analysis XLSX output
- importing code from the external Python/Jesse project
- optimization-stage XLSX export unless it is needed for shared report primitives

## Report Design Principles

1. The first sheet must communicate the run verdict immediately.
2. Candidate selection must reflect current backend approval semantics, not older shortlist heuristics alone.
3. The workbook must remain readable for non-MA strategies.
4. Strategy-specific parameter fields should be shown without making the workbook strategy-specific.
5. Color must encode meaning consistently.
6. The workbook must separate "summary", "ranked shortlist", "distribution view", and "raw audit data".
7. Hidden or secondary staging sheets are acceptable if they make charts and formulas cleaner.

## Target Workbook Architecture

### Sheet 1: `Dashboard`

Purpose: the "open once, understand instantly" page.

Must include:

- run timestamp
- strategy kind
- dataset identity
- validation backend used / route metadata when available
- run-level `run_verdict`
- `final_recommendation`
- `approved_candidate_count`
- first approved rank
- explicit "no candidate approved" state when applicable
- best approved candidate summary
- compact gate summary cards for:
  - walk-forward
  - CSCV/PBO
  - PSR
  - DSR
  - cross-ticker
  - perturbation
  - trade bootstrap
- compact top-approved table with color-coded pass/fail indicators
- charts for the most decision-relevant evidence

### Sheet 2: `Approved Candidates`

Purpose: ranked table of final approved candidates only.

Must include:

- rank
- approval/pass flag
- candidate verdict summary
- parameter summary
- in-sample metrics
- walk-forward metrics
- CSCV/PBO metrics
- PSR / DSR
- cross-ticker metrics
- bootstrap / perturb metrics when present
- route/backend metadata when relevant

Behavior:

- if zero candidates are approved, the sheet should still exist and clearly state that no candidates passed final approval

### Sheet 3: `Shortlist`

Purpose: broader ranked candidate table for context and comparison.

Should include:

- all reportable validation candidates after current shortlist policy is applied
- explicit columns showing why a candidate was not finally approved
- stable sort order and row cap controls

### Sheet 4: `Distributions`

Purpose: chart-oriented and aggregate evidence.

Candidate content:

- distribution of approved vs non-approved candidates
- robust score distribution
- walk-forward Sharpe / drawdown distributions
- CSCV OOS metric distribution
- PSR / DSR distribution
- per-strategy parameter-shape charts where meaningful
- cross-ticker score distribution when present

### Sheet 5: `Audit Data`

Purpose: raw and machine-friendly evidence.

Should include:

- one row per candidate
- stable columns intended for downstream analysis or copy/paste
- exact numeric values used by dashboard and charts

This sheet may be hidden by default if that improves workbook UX.

## Row-Cap and Data-Volume Policy

The workbook should distinguish between:

- visible presentation tables
- chart staging data
- raw audit export data

Recommended policy:

- row caps may apply to visible tables such as `Approved Candidates` and `Shortlist`
- `Distributions` should be computed from the full intended candidate population, not only the visible capped rows
- `Audit Data` should preserve the full exported evidence set unless an explicit export-size limit is introduced later
- if row caps are used, the workbook should state that visible rows are capped while aggregate charts may use the full underlying set

## Candidate Selection Policy for the XLSX

The report should distinguish clearly between:

- approved candidates
- shortlist candidates
- all validation candidates

Recommended policy:

1. `Dashboard` and `Approved Candidates` should key off the final backend approval semantics.
2. `Shortlist` should show the broader ranked context, including near-miss candidates.
3. `Audit Data` should preserve the full exportable evidence set.

Important rule:

- older shortlist heuristics such as `robust_pareto` and `cross_ticker_pareto` can still be useful as explanatory columns, but they should not remain the sole definition of what the report considers "final".

## Validation Data Contract to Surface

The report should surface, when available:

- run-level verdict fields
- candidate-level verdict fields
- final recommendation
- approved candidate count
- first approved rank
- strategy kind
- backend used / backend route / fallback reason
- parameter payload or normalized parameter summary
- in-sample metrics
- walk-forward metrics
- CSCV/PBO metrics
- PSR / DSR
- cross-ticker metrics
- perturbation metrics
- trade bootstrap metrics
- local stability / related hard-gate evidence when present
- any strategy-family-specific fields already carried in validation artifacts

Missing data policy:

- absent validation layers should render as "not run" rather than zero
- charts should omit unavailable series cleanly
- workbook generation must not fail merely because an optional validation layer was disabled
- legacy artifacts that predate `run_verdict` or candidate `validation_verdict` should either:
  - render under a clearly documented compatibility mode, or
  - fail with an explicit user-facing explanation

Compatibility policy must be chosen intentionally before implementation is marked complete.

## Color and Visual System

Color must be semantic and reusable, not ad hoc.

Recommended baseline:

- green: approved / good / above threshold
- red: rejected / failed / below threshold
- amber: caution / borderline / advisory warning
- blue: contextual or descriptive metadata
- neutral gray: not run / not applicable

Formatting policy:

- prefer reusable formats and conditional formatting rules over hundreds of one-off style variants
- use icons/badges sparingly and consistently
- keep chart palette aligned with the table semantics

## Chart Set

Minimum recommended chart set for the first implementation pass:

- approved candidate score by rank
- risk/return scatter for finalists
- walk-forward vs in-sample comparison
- CSCV or PBO summary view
- parameter-shape distribution relevant to the strategy family
- approved vs rejected counts by reason category if feasible

Charts should be driven from clean staging data, not directly from the visible formatted tables.

## Strategy Coverage Requirements

The workbook must support all currently implemented strategies without assuming MA-only parameters.

Implementation guidance:

- keep shared workbook/report plumbing shared
- keep strategy-specific parameter-summary extraction strategy-local where necessary
- avoid hard-coding only `fast_ma_type`, `fast_len`, `slow_ma_type`, `slow_len` as the primary candidate identity

## Validation Feature Support Audit

Date: 2026-03-13

This section tracks current validation-feature coverage across existing strategies and backend routes so XLSX modernization does not assume broader support than the runtime actually provides.

### Strategy Families and Validation Routes

Current validation command routing is intentionally split into two families:

- `DoubleMa`, `Pit`, `Set`, `Mts`, `Orc`, `Marrt`, `Vfct`, `CompositeTrend`, `RegimeHybrid`, `BreakoutMomentum`
  - route through the double-MA validation path in `src-tauri/src/commands_validation.rs`
- `Asr`, `Vcc`, `Vemr`, `Rocb`, `Mkcs`, `Kqsr`, `Carm`, `Kvdf`, `Cdmc`, `CompositeCpu`, `CompositeMo`
  - route through the composite validation path in `src-tauri/src/commands_validation.rs`

At the request-contract level, both families expose the same major validation modules:

- `walk_forward`
- `reopt_walk_forward`
- `spa`
- `perturb`
- `trade_bootstrap`
- `cross_ticker`
- `cscv_pbo`
- hard gates:
  - `min_psr`
  - `min_dsr`
  - `min_cscv_oos_rank_frac`
  - `min_cscv_oos_metric`
  - `max_pbo`

This means the broad feature model is shared. The real gaps are mostly runtime-route gaps, not request-schema gaps.

### Current Support Summary

Broadly supported across both validation families:

- candidate-level `validation_verdict`
- run-level `run_verdict`
- PSR / DSR computation
- minimum track-record-length checks
- robust score / pareto ranking
- optional walk-forward candidate stats
- optional reopt walk-forward summary
- optional SPA summary
- optional perturbation stats
- optional trade bootstrap stats
- optional CSCV / PBO
- optional cross-ticker validation pass

Important note about approval semantics:

- final candidate approval currently depends most directly on:
  - PSR / DSR gates
  - CSCV OOS rank / metric gates
  - minimum track-record-length checks
- run-level approval then adds:
  - PBO gate logic
  - SPA significance handling
- perturbation and trade bootstrap are implemented and exported, but they are not currently part of the final hard-gate / run-verdict approval rule

That appears to be current design, not necessarily a bug.

### Missing or Partial Support by Route

#### 1. `DoubleMa`-family fixed-candidate GPU validation is not feature-complete

Affected strategies:

- `DoubleMa`
- `Pit`
- `Set`
- `Mts`
- `Orc`
- `Marrt`
- `Vfct`
- `CompositeTrend`
- `RegimeHybrid`
- `BreakoutMomentum`

Current limitations:

- does not support `trade_bootstrap`
- does not support `cross_ticker`
- requires the vectorized validation engine
- requires uniform MA types / strategy config / halftrend filter / MA params across the fixed-candidate batch

Why this appears intentional:

- the blockers are explicitly encoded in `crates/app_core/src/double_ma_validate.rs`
- the error strings clearly describe current runtime constraints rather than accidental omissions

Assessment:

- intentional current limitation
- still a product gap if the goal is full feature parity across CPU and GPU validation routes

#### 1a. Measured Double-MA validation cost evidence changes the parity priority

Measured on 2026-03-13 with:

- `src-tauri/src/bin/ta_double_ma_validation_cost.rs`
- repo-local datasets:
  - `data/AAPL.csv`
  - `data/SPY.csv`
  - `data/QQQ.csv`
  - `data/NVDA.csv`
- output artifact:
  - `_write_test/perf/double_ma_validation_cost_aapl_cpu.json`

Representative CPU run results for a real ALMA Double-MA workflow:

- optimization baseline: about `122 ms`
- base validation: about `54 ms`
- walk-forward: about `71 ms`
- trade bootstrap: about `97 ms`
- reopt walk-forward: about `268 ms`
- CSCV/PBO: about `414 ms`
- SPA: about `424 ms`
- perturb: about `1265 ms`
- cross-ticker rerun on 3 extra datasets: about `3331 ms`
- full stack without cross-ticker: about `3145 ms`
- full stack plus cross-ticker: about `6515 ms`

Practical reading:

- only the tiny host-side reduction logic is clearly cheap enough to remain CPU-only without meaningful opportunity cost:
  - PSR / DSR computation
  - candidate hard-gate application
  - run-verdict assembly
- `trade_bootstrap` is not negligible relative to optimization on this representative run; it is comparable in cost, not effectively free
- `cross_ticker`, `perturb`, `CSCV/PBO`, `SPA`, and `reopt_walk_forward` are materially more expensive than the optimization baseline and therefore should be treated as real GPU-parity targets for the Double-MA golden-standard roadmap
- base validation and walk-forward are also not free, but those paths already have real GPU support and therefore do not argue for accepting CPU-only behavior

Resulting policy guidance:

- keep final semantic reduction on CPU
- prioritize GPU parity work for:
  - `cross_ticker`
  - `perturb`
  - `trade_bootstrap`
  - `CSCV/PBO`
  - `SPA`
  - `reopt_walk_forward`
- do not justify current CPU-only Double-MA gaps by calling those modules cheap without fresh measurement evidence on the target route

Additional artifact evidence collected on 2026-03-13:

- broad-stack CPU validation artifact with surviving candidates:
  - `_write_test/headless_reports/aapl_alma_validation_full_stack_no_cross.xlsx`
  - `_write_test/headless_reports/aapl_alma_validation_full_stack_no_cross.json`
- broad-stack CPU validation artifact with cross-ticker enabled:
  - `_write_test/headless_reports/aapl_alma_validation_full_stack.xlsx`
  - `_write_test/headless_reports/aapl_alma_validation_full_stack.json`
- smaller preserved-stack GPU cross-ticker validation artifact:
  - `_write_test/headless_reports/aapl_alma_validation_cross_parity_gpu_small.xlsx`
  - `_write_test/headless_reports/aapl_alma_validation_cross_parity_gpu_small.json`

Observed XLSX behavior from those artifacts:

- the no-cross artifact surfaces `SPA`, `Reopt WF`, `CSCV / PBO`, `Perturb`, `Bootstrap`, and `Cross-Ticker` sections and still emits chart parts
- the cross-enabled artifact can legitimately finish with zero surviving candidates after post-validation cross-ticker filtering
- the preserved-stack GPU cross artifact also demonstrates the zero-survivor case with:
  - `backend_route = GpuKernel`
  - `approved_candidate_count = 0`
  - `final_recommendation = no_candidate_survived`
- the XLSX builder now emits run-verdict fallback charts for that zero-survivor case instead of dropping to zero chart parts entirely

That zero-survivor chart behavior was a real XLSX UX edge case and is now explicitly handled by run-verdict fallback charts.

Additional GPU cost evidence collected on 2026-03-13:

- measurement artifact:
  - `_write_test/perf/double_ma_validation_cost_aapl_gpu.json`
- build path:
  - `cargo run -p ta_desktop_demo_app --features cuda,cuda-backtest-kernel --bin ta_double_ma_validation_cost ...`

Representative GPU-route results on the same ALMA AAPL workload:

- optimization baseline on `GpuKernel`: about `1165 ms` wall / `1045 ms` reported runtime
- base validation on `GpuKernel`: about `71 ms`
- walk-forward on `GpuKernel`: about `603 ms`
- CSCV/PBO on `GpuKernel`: about `137 ms`
- SPA on `GpuKernel`: about `141 ms`
- reopt walk-forward on `GpuKernel`: about `417 ms`
- perturb on `GpuKernel`: about `53515 ms`
- trade bootstrap request:
  - route downgraded to `GpuKernel->CpuOnly`
  - fallback reason: `GPU fixed-candidate validation does not support trade bootstrap yet`
  - isolated runtime: about `133 ms`
- cross-ticker rerun on 3 extra datasets while preserving `GpuKernel` on the validation pass: about `3694 ms`

GPU-side interpretation:

- `CSCV/PBO` and `SPA` already look good on the GPU route relative to GPU optimization cost
- `walk_forward` and `reopt_walk_forward` are materially heavier than base validation but still in a plausible range for GPU-backed validation
- isolated `trade_bootstrap` is cheap enough to tolerate on CPU if it can be kept module-local; the real problem is that it currently forces the entire validation route down to `CpuOnly`
- `cross_ticker` remains expensive because it multiplies reruns by dataset count, even when the underlying validation route stays GPU-backed
- the current GPU `perturb` path is a major red flag:
  - it stays on `GpuKernel`
  - it does not report a fallback
  - but it is dramatically slower than the GPU optimization baseline
  - this should be treated as a performance bug or design issue, not as evidence that perturb parity is complete

Refined Double-MA GPU-priority guidance:

1. Highest priority:
   - decide whether `cross_ticker` should remain lightweight or become full-stack while keeping GPU execution for the heavy reruns
   - add a true batched GPU `perturb` implementation if CPU-auxiliary perturb remains a material bottleneck
2. Medium priority:
   - verify broader parity and stability for GPU `walk_forward` and `reopt_walk_forward`
3. Lower priority:
   - keep host-side hard-gate and run-verdict reduction on CPU unless future profiling proves otherwise

Implementation update applied after the first GPU measurement:

- `crates/app_core/src/double_ma_validate.rs` now preserves the GPU main validation route when `trade_bootstrap` is enabled by computing bootstrap as an auxiliary CPU post-pass per candidate instead of forcing a whole-route downgrade
- the same file now avoids the catastrophic per-iteration GPU perturb setup loop by using a CPU auxiliary perturb pass while still keeping the main validation route on `GpuKernel`

Measured post-change GPU evidence:

- updated measurement artifact:
  - `_write_test/perf/double_ma_validation_cost_aapl_gpu_after_parity_patch.json`
- broad-stack GPU validation artifact:
  - `_write_test/headless_reports/aapl_alma_validation_full_stack_gpu.json`
  - `_write_test/headless_reports/aapl_alma_validation_full_stack_gpu.xlsx`

Important deltas versus the earlier GPU measurement:

- `trade_bootstrap` now stays on `GpuKernel` with no `CpuOnly` fallback
- `validation_full_stack_no_cross` now stays on `GpuKernel` with no `CpuOnly` fallback
- GPU `perturb` dropped from about `53515 ms` to about `1927 ms` on the representative workload by using the auxiliary CPU perturb tail instead of the old per-iteration GPU-prep loop

Current broad-stack CPU/GPU parity checkpoint:

- CPU reference artifact:
  - `_write_test/headless_reports/aapl_alma_validation_full_stack_no_cross.json`
  - `_write_test/headless_reports/aapl_alma_validation_full_stack_no_cross.xlsx`
- GPU reference artifact:
  - `_write_test/headless_reports/aapl_alma_validation_full_stack_gpu.json`
  - `_write_test/headless_reports/aapl_alma_validation_full_stack_gpu.xlsx`
- observed parity evidence:
  - candidate count: `8` on both
  - approved candidate count: `8` on both
  - final recommendation: `single_candidate_supported` on both
  - top-ranked candidate: `9/30` on both
  - ranked candidate ordering for the top 8 rows matched between CPU and GPU
  - workbook sections and validation-feature labels were present on both artifacts

Cross-ticker preserved-stack update applied after the initial parity checkpoint:

- Double-MA cross-ticker reruns in `src-tauri/src/commands_validation.rs` now preserve the verdict-relevant stack:
  - `cscv_pbo`
  - hard gates:
    - `min_psr`
    - `min_dsr`
    - `min_cscv_oos_rank_frac`
    - `min_cscv_oos_metric`
    - `max_pbo`
- non-gating evidence modules are intentionally omitted on auxiliary cross-ticker reruns because they do not feed candidate `validation_verdict` on the extra datasets:
  - `walk_forward`
  - `reopt_walk_forward`
  - `spa`
  - `perturb`
  - `trade_bootstrap`
- Double-MA cross-ticker aggregation now only counts per-dataset reruns where the candidate passed that dataset rerun’s `validation_verdict`
- Double-MA results now recompute `run_verdict` after post-cross-ticker candidate filtering, preventing stale run-level approval/recommendation

Initial preserved-stack cross-ticker measurement artifacts:

- CPU:
  - `_write_test/perf/double_ma_validation_cost_aapl_cpu_cross_parity.json`
- GPU:
  - `_write_test/perf/double_ma_validation_cost_aapl_gpu_cross_parity.json`

Representative initial preserved-stack cross-ticker runtimes on the same ALMA workload:

- CPU preserved-stack cross only: about `3511 ms`
- CPU full stack plus preserved-stack cross: about `132696 ms`
- GPU preserved-stack cross only: about `17142 ms`
- GPU full stack plus preserved-stack cross: about `345461 ms`

Interpretation at that checkpoint:

- Double-MA cross-ticker now has much stronger semantic parity across CPU and GPU
- preserved-stack cross-ticker is not lightweight and should not be described that way anymore
- this likely warrants an explicit policy/mode decision:
  - lightweight cross-ticker
  - preserved-stack cross-ticker

Finalist-bounded preserved-stack update applied after the first preserved-stack measurements:

- Double-MA cross-ticker now reruns a bounded finalist set instead of every candidate:
  - approved finalists first
  - otherwise hard-gate survivors
  - otherwise top-ranked remaining context
  - current finalist cap: `4`
- implementation surface:
  - `src-tauri/src/commands_validation.rs`

Reasoning:

- this keeps full validation-feature coverage for the selected Double-MA finalists
- it reduces repeated cross-dataset reruns substantially without dropping the parity-preserving stack for those finalists

Finalist-bounded measurement artifacts:

- CPU:
  - `_write_test/perf/double_ma_validation_cost_aapl_cpu_cross_finalists.json`
- GPU:
  - `_write_test/perf/double_ma_validation_cost_aapl_gpu_cross_finalists.json`

Representative finalist-bounded runtimes on the same ALMA workload:

- CPU preserved-stack cross only: about `3166 ms`
- CPU full stack plus preserved-stack cross: about `89131 ms`
- GPU preserved-stack cross only: about `3218 ms`
- GPU full stack plus preserved-stack cross: about `94494 ms`

Compared with the earlier preserved-stack version:

- CPU full-stack-plus-cross dropped from about `132696 ms` to about `89131 ms`
- GPU full-stack-plus-cross dropped from about `345461 ms` to about `94494 ms`

Interpretation after finalist bounding:

- finalist-bounded preserved-stack cross-ticker is still expensive, but now plausibly usable on the GPU route for Double-MA
- the finalist-selection policy is now an important part of the Double-MA performance design and should eventually be surfaced/documented rather than remaining implicit

Aligned-horizon preserved-stack update applied after the finalist-bounded checkpoint:

- Double-MA cross-ticker reruns in `src-tauri/src/commands_validation.rs` now align each extra dataset to the base validation horizon before rerunning finalists.
- the same preserved-finalist path continues to keep only verdict-relevant rerun modules:
  - `cscv_pbo`
  - hard gates:
    - `min_psr`
    - `min_dsr`
    - `min_cscv_oos_rank_frac`
    - `min_cscv_oos_metric`
    - `max_pbo`
- non-gating evidence modules remain intentionally omitted on auxiliary cross reruns:
  - `walk_forward`
  - `reopt_walk_forward`
  - `spa`
  - `perturb`
  - `trade_bootstrap`

Aligned-horizon measurement artifacts:

- CPU:
  - `_write_test/perf/double_ma_validation_cost_aapl_cpu_cross_finalists_aligned.json`
- GPU:
  - `_write_test/perf/double_ma_validation_cost_aapl_gpu_cross_finalists_aligned.json`

Representative aligned-horizon runtimes on the same ALMA workload:

- CPU preserved-stack cross only: about `406 ms`
- CPU full stack plus preserved-stack cross: about `3454 ms`
- CPU full stack without cross: about `4253 ms`
- GPU preserved-stack cross only: about `331 ms`
- GPU full stack plus preserved-stack cross: about `4374 ms`
- GPU full stack without cross: about `4027 ms`

Compared with the earlier finalist-bounded preserved-stack version:

- CPU preserved-stack cross only dropped from about `3166 ms` to about `406 ms`
- CPU full-stack-plus-cross dropped from about `89131 ms` to about `3454 ms`
- GPU preserved-stack cross only dropped from about `3218 ms` to about `331 ms`
- GPU full-stack-plus-cross dropped from about `94494 ms` to about `4374 ms`

Current interpretation:

- preserved-finalist Double-MA cross-ticker is now in a practical runtime range on both CPU and GPU
- on the representative GPU workload, full-stack-plus-cross is now only modestly above full-stack-without-cross
- the main remaining work is not basic feasibility; it is ensuring the route capability/fallback story is explicit and deciding whether any further GPU-native replacement work is worth the complexity for `perturb` and `trade_bootstrap`

#### 2. Composite fixed-candidate GPU validation is only partially feature-complete

Affected strategies:

- `Asr`
- `Vcc`
- `Vemr`
- `Rocb`
- `Mkcs`
- `Kqsr`
- `Carm`
- `Kvdf`
- `Cdmc`
- `CompositeCpu`
- `CompositeMo`

Current limitations:

- does not support `cross_ticker`
- conditionally blocks some modules by candidate/profile family:
  - `walk_forward`
  - `reopt_walk_forward`
  - `spa`
  - `trade_bootstrap`
  - `cscv_pbo`
- requires the vectorized validation engine

Why this appears intentional:

- the blockers are explicitly encoded in `crates/app_core/src/composite_validate.rs`
- the code checks profile support before allowing those modules on the fixed-candidate GPU route

Assessment:

- intentional current limitation
- still a parity gap versus CPU validation

#### 3. Cross-ticker reruns currently differ by family and policy

Affected strategies:

- all strategies that use cross-ticker through `commands_validation.rs`

Current behavior:

- composite-family cross-ticker rerun requests are still rebuilt in a lightweight form with:
  - `walk_forward: None`
  - `reopt_walk_forward: None`
  - `spa: None`
  - `perturb: None`
  - `trade_bootstrap: None`
  - `cscv_pbo: None`
  - hard gates cleared
- Double-MA-family cross-ticker reruns now preserve the verdict-relevant validation stack and hard gates
- verdict-irrelevant evidence modules (`walk_forward`, `reopt_walk_forward`, `spa`, `perturb`, `trade_bootstrap`) are no longer rerun during preserved-finalist cross validation because they do not feed cross aggregation or candidate-level pass/fail semantics

Why this appears intentional:

- the command layer constructs lightweight cross-ticker follow-up requests on purpose
- likely rationale:
  - keep cross-ticker runtime bounded
  - avoid recursively nesting expensive validation modules on every extra dataset
  - treat cross-ticker as a separate secondary robustness filter rather than a full revalidation pass

Assessment:

- now intentionally split by family
- Double-MA now uses a verdict-preserving finalist rerun policy for cross-ticker evaluation
- composite family still uses the older lightweight model
- this product difference should be documented explicitly because it materially affects runtime and semantics

#### 4. Headless validation artifact context was incomplete and is now partially resolved

Affected path:

- `src-tauri/src/bin/ta_headless_validate.rs`

Current behavior:

- headless validation now propagates `validation_context`
- headless-generated artifacts preserve strategy identity and hard-gate request context

Residual follow-up:

- add a regression test specifically proving headless-generated workbooks do not regress to `Strategy = Unknown`

#### 5. Validation capability surfacing is weaker than backend reality

Affected surface:

- validation UI / request-building / capability messaging

Current behavior:

- the request model is broadly shared across strategies
- route-specific runtime blockers are mostly backend-only knowledge
- users can request a broad validation stack without being clearly warned that a given strategy/backend route will skip, block, or downshift some modules

Assessment:

- likely a product/UX gap rather than a core backend bug
- should be addressed so users can tell in advance which validation modules will actually execute

### Current Cross-Ticker Test Data Already Available

Repo-local datasets already present under `data/` and suitable for cross-ticker validation testing include:

- `AAPL.csv`
- `NVDA.csv`
- `QQQ.csv`
- `SPY.csv`
- `SQQQ.csv`
- `TQQQ.csv`
- `TSLA.csv`
- `UVXY.csv`

These should be preferred for support-validation smoke runs before adding any new fixture data.

## Validation Feature Broadening Checklist

This checklist is separate from the XLSX workbook checklist. Its purpose is to broaden actual validation feature support across existing strategies and routes, except where current limitations remain intentional by product decision.

### Documentation and policy

- [ ] Document the current strategy-family split between double-MA validation and composite validation in product-facing or engineering-facing docs.
- [ ] Document which validation modules are optional evidence surfaces versus true approval gates.
- [ ] Document the current meaning of cross-ticker validation:
  - secondary lightweight rerun filter
  - or full-stack revalidation across additional datasets
- [ ] Decide explicitly whether perturbation and trade bootstrap should remain non-gating or become optional gate inputs later.

### Route-level parity audit

- [ ] Build a strategy-by-strategy parity matrix covering CPU, Auto, GpuKernel, and GpuHybridKernel validation routes.
- [ ] Mark each strategy/route pair for support status of:
  - walk-forward
  - reopt walk-forward
  - SPA
  - perturb
  - trade bootstrap
  - cross-ticker
  - CSCV/PBO
  - PSR/DSR gates
  - min-track-record checks
  - run verdict
- [ ] Mark each unsupported cell as one of:
  - intentional
  - deferred
  - unknown / needs investigation

### Double-MA family gaps

- [ ] Add parameter-surface stability / plateau analysis for the Double-MA family as the highest-ROI next robustness feature.
  - goal: prefer broad stable neighborhoods over narrow isolated spikes
  - initial scope: analyze local robustness around the final Double-MA shortlist using nearby optimization-surface neighbors from `run.res.top` / `run.res.all`
  - first implementation target:
    - [x] compute local neighborhood summaries around each shortlisted candidate using nearby fast/slow-length combinations
    - [x] derive a simple plateau score or plateau label
    - [x] surface plateau provenance (`run.res.all` vs `run.res.top`) so top-slice fallbacks are visible
    - [x] fall back to `optimization.res.top` / `optimization.res.all` for headless artifacts that do not carry `run.res`
    - [ ] report richer neighborhood robust-score / PSR / DSR statistics where available
  - first UX/report target:
    - [x] surface plateau score / plateau label in the XLSX shortlist/dashboard
    - [x] surface neighborhood median Sharpe / worst MaxDD plus plateau provenance in the workbook
    - [x] add a compact plateau chart or heatmap-style neighborhood summary for the top finalists
  - first selection-policy target:
    - [ ] use plateau stability as a secondary ranking/selection signal for Double-MA finalists
    - [x] do not immediately make it a hard gate until evidence quality is reviewed
- [ ] Add finalist ranking-stability checks for the Double-MA family.
  - goal: verify that the same top candidates remain near the top across validation evidence views instead of only winning on one metric slice
  - initial scope:
    - compare ranking by in-sample robust score
    - walk-forward-adjusted score
    - CSCV-informed score
    - cross-ticker-informed score when cross validation is enabled
  - first implementation target:
    - [x] compute simple top-k overlap / rank-drift summaries for finalists
    - [x] export those summaries into the XLSX dashboard and audit sheet
  - first selection-policy target:
    - [ ] use ranking stability as an explanatory and tie-break signal before considering any hard-gate role
    - [x] do not change approval semantics until the stability summaries have been reviewed on real artifacts

- [ ] Add literature-backed practical robustness additions for the Double-MA family.
  - implementation policy:
    - prioritize low-churn additions that reuse existing Double-MA validation outputs first
    - add new request/result modules only where the current payload is insufficient
    - keep all new signals explanatory first; do not change approval semantics until real-artifact review is complete
  - priority 1: walk-forward efficiency / generalization ratio
    - research basis:
      - walk-forward efficiency and parameter-plateau selection literature explicitly prefer parameter sets that retain out-of-sample performance rather than maximizing in-sample fit
      - recent anti-overfitting objective-function work also treats generalization ratio (`validation / training`) as a core robustness diagnostic
    - best implementation path:
      - [x] phase 1: derive candidate-level `wf_pnl_efficiency` and `wf_sharpe_efficiency` from existing Double-MA candidate outputs
        - `walk_forward.pnl_median / in_sample.pnl`
        - `walk_forward.sharpe_median / in_sample.sharpe`
      - [x] phase 2: extend Double-MA reopt walk-forward stats to retain per-window train/OOS metric pairs so a true WFE distribution can be computed instead of only median/worst OOS summaries
        - note: the current plain `walk_forward` payload is summary-only, while `reopt_walk_forward` already has the train-window context needed for a low-churn first implementation
      - [x] phase 3: expose run-level summaries such as median efficiency, worst efficiency, and dispersion across finalist windows
      - [x] phase 4: regenerate and review a real artifact with the new reopt-WFE distribution fields once local cargo file-lock issues stop blocking fresh headless runs
    - first UX/report target:
      - [x] dashboard block for `WF Efficiency`
      - [x] shortlist/audit columns for candidate-level efficiency
      - [x] compact distribution chart for efficiency by robust rank
      - [x] dashboard `Reopt WF` summary lines for WFE median / worst / dispersion
    - first selection-policy target:
      - [x] use efficiency as an explanatory and tie-break signal first
      - [x] do not make it a hard gate pending later evidence review
  - priority 2: regime-slice robustness
    - research basis:
      - recent rigorous walk-forward work shows regime dependence can completely change the apparent viability of a strategy
      - intended-regime robustness is more useful than aggregate profitability when the strategy only works in certain volatility/activity states
    - best implementation path:
      - [x] phase 1: evaluate top Double-MA finalists automatically after validation to contain cost
      - [x] initial slice taxonomy:
        - realized-volatility tertiles
        - trend vs range state
      - [x] initial metrics per slice:
        - Sharpe
        - PnL
        - MaxDD
        - trade count
      - [x] initial finalist summary fields:
        - worst-slice label
        - worst-slice Sharpe
        - worst-slice MaxDD
        - stable-slice bucket count
      - [ ] phase 2: decide whether `regime_slice_robustness` needs to become an explicit request module instead of an always-on finalist analysis
      - [x] phase 3: add deeper slice-by-slice audit rows and intended-regime pass summaries
    - first UX/report target:
      - [x] dashboard `Regime Robustness` block
      - [x] shortlist / audit columns for worst bucket, worst Sharpe, worst MaxDD, and stable bucket count
      - [x] shortlist flags for regime fragility / intended-regime support
      - [x] audit rows for slice-by-slice metrics
    - first selection-policy target:
      - explanatory first
      - later optional soft gate on intended-regime pass fraction and worst-slice drawdown
  - priority 3: dependence-aware bootstrap
    - research basis:
      - stationary / block bootstrap is preferred over iid resampling when dependence and autocorrelation matter
      - this is a better fit for trading PnL/returns than purely iid trade resampling
    - best implementation path:
      - [x] phase 1: extend the existing bootstrap output with a stationary-bootstrap comparison path while preserving the current iid fields
      - [x] first implementation operates on the existing closed-trade return series, not yet per-bar return or equity-delta paths
      - [x] add a heuristic `stationary_mean_block_len` and emit dependence-aware comparison quantiles
      - [x] phase 2a: add the first explicit bootstrap mode options:
        - `compare` (default; iid + stationary)
        - `iid_only`
      - [ ] phase 2b: evaluate whether later explicit modes should add:
        - `stationary_only`
        - `moving_block`
      - [x] explain the current first-pass semantics clearly:
        - current output is an `iid` plus `stationary` comparison
        - it operates on closed-trade returns
        - `moving_block` remains future work
      - [ ] phase 3: evaluate whether the dependence-aware path should migrate from closed-trade returns to per-bar return / equity-delta series for finalists
    - first outputs:
      - [x] lower confidence bound for PnL via stationary `pnl_p05`
      - [x] upper-tail drawdown quantiles via stationary `max_dd_p95`
      - [x] comparison against the current iid bootstrap result
    - first UX/report target:
      - [x] dashboard bootstrap comparison / block-length summary
      - [x] shortlist / audit columns for dependence-aware downside diagnostics
      - [x] explain in workbook notes whether bootstrap is iid-only, compare mode, or later user-selected
    - first selection-policy target:
      - explanatory first
      - later optional soft gate on lower confidence bound staying above threshold
  - priority 4: friction-stability stress
    - research basis:
      - realistic costs and slippage frequently reorder parameter winners
      - robustness should prefer candidates whose rank is stable under harsher frictions
    - best implementation path:
      - [x] phase 1: reuse the current Double-MA finalist rerun path
      - [x] add a small scenario matrix over:
        - fee stress
        - slippage stress
        - spread stress
      - [x] compute finalist rank drift and pass fraction under stressed frictions
      - [x] derive stress severity per finalist instead of reusing the first finalist's baseline costs
      - [x] phase 2: decide whether the first-pass pass/fail rule should stay absolute (`pnl >= 0 && sharpe >= 0`) or shift to a relative degradation rule
    - first UX/report target:
      - [x] dashboard `Friction Stability` summary
      - [x] shortlist flag for cost-fragile candidates
      - [x] workbook note explaining the stress matrix assumptions and finalist-only scope
    - first selection-policy target:
      - explanatory first
      - later optional soft gate if rank collapses under modest stress
  - second-tier backlog after the four priorities above:
    - purged / embargoed split stability
    - combinatorial path stability
    - broader multiple-testing controls beyond SPA
    - nested outer-loop evaluation of the selection rule itself

- [ ] Treat the double-MA family as the validation golden standard for the application.
- [ ] Define the parity target explicitly:
  - the same validation-stage feature set should be available for double-MA-family runs on CPU and GPU validation routes
  - CPU and GPU runs may differ internally in execution strategy, but not in user-visible feature availability or final semantics
- [ ] Freeze the double-MA parity scope across:
  - `DoubleMa`
  - `Pit`
  - `Set`
  - `Mts`
  - `Orc`
  - `Marrt`
  - `Vfct`
  - `CompositeTrend`
  - `RegimeHybrid`
  - `BreakoutMomentum`
- [ ] Build a double-MA-family CPU vs GPU validation parity matrix covering:
  - walk-forward
  - reopt walk-forward
  - SPA
  - perturb
  - trade bootstrap
  - cross-ticker
  - CSCV/PBO
  - PSR / DSR gates
  - min-track-record checks
  - candidate `validation_verdict`
  - run `run_verdict`
- [ ] Record, for each double-MA-family route, whether the current limitation is:
  - intentional short-term
  - accidental / incomplete
  - blocked by CUDA/runtime architecture
- [x] Add a real-run Double-MA validation cost profiler comparing optimization versus optional validation modules:
  - `src-tauri/src/bin/ta_double_ma_validation_cost.rs`
- [x] Add a Double-MA validation workflow benchmark for regression tracking:
  - `crates/app_core/benches/double_ma_validation_workflow.rs`
- [x] Capture initial CPU evidence for Double-MA validation module costs on repo-local data:
  - `_write_test/perf/double_ma_validation_cost_aapl_cpu.json`
- [x] Capture equivalent GPU evidence on the same dataset family and compare module cost ratios:
  - `_write_test/perf/double_ma_validation_cost_aapl_gpu.json`
- [ ] Keep final hard-gate and run-verdict semantics identical across CPU and GPU validation routes, even if the expensive reruns are GPU-executed and the cheap reductions remain CPU-side.
- [ ] Ensure the XLSX/report output for double-MA-family runs is semantically identical across CPU and GPU validation routes:
  - same sections
  - same feature visibility
  - same “not run” behavior
  - same approval/rejection reasoning
- [x] Add at least one broad-stack double-MA CPU reference artifact that exercises the full intended validation surface:
  - `_write_test/headless_reports/aapl_alma_validation_full_stack_no_cross.xlsx`
  - `_write_test/headless_reports/aapl_alma_validation_full_stack_no_cross.json`
- [x] Add at least one broad-stack double-MA GPU reference artifact that exercises the same validation surface with parity expectations:
  - `_write_test/headless_reports/aapl_alma_validation_full_stack_gpu.xlsx`
  - `_write_test/headless_reports/aapl_alma_validation_full_stack_gpu.json`
- [x] Compare CPU and GPU reference artifacts for:
  - candidate ordering
  - approval counts
  - run recommendation
  - rejection reasons
  - surfaced validation modules
- [x] Document residual CPU-only fallback behavior that still exists while parity work is in progress:
  - direct app-core `GpuKernel + cross_ticker` validation requests still fall back to `CpuOnly`
  - `GpuKernel + iterative validation engine` still falls back to `CpuOnly`
  - `Auto + iterative validation engine` still falls back to `CpuOnly`
  - preserved-finalists cross-ticker keeps GPU route parity only through the Tauri orchestration layer, not through the direct app-core `cross_ticker` request field
- [x] Decide whether the current auxiliary-CPU `trade_bootstrap` implementation is sufficient, or whether full GPU-native bootstrap support is still required.
  - current decision: keep `trade_bootstrap` as a CPU tail on top of the GPU main validation route
  - rationale: fresh AAPL recheck still shows `trade_bootstrap` in the `comparable` band, so backend redesign is not justified right now
- [x] Decide whether to pursue a true GPU-native `perturb` variant right now.
  - current decision: keep `perturb` as a CPU tail for now rather than opening a deeper GPU validation backend project
  - rationale: although `perturb` remains expensive, the current fixed-candidate GPU path cannot represent faithful perturb batches without broader backend changes
- [x] Document why faithful GPU-native variants are not currently viable without deeper backend changes:
  - `perturb`: current fixed-candidate GPU batching requires uniform `StrategyConfig`, while perturb currently mutates `StrategyConfig`
  - `trade_bootstrap`: current fixed-candidate GPU path does not expose the trade sequence needed by bootstrap resampling
- [x] If the measured CPU-tail implementation is already near-parity with any realistic GPU-native attempt, document that explicitly and keep the CPU path.
  - `trade_bootstrap` is now intentionally retained as a CPU tail
  - `perturb` is also retained as a CPU tail for now, with the remaining issue being cost rather than an immediate implementation plan
- [x] Refresh auxiliary-tail cost evidence with the upgraded feasibility-report harness:
  - CPU recheck: `_write_test/perf/double_ma_validation_cost_aapl_cpu_tail_recheck.json`
  - GPU recheck: `_write_test/perf/double_ma_validation_cost_aapl_gpu_tail_recheck.json`
  - `trade_bootstrap` remains in the `comparable` band on both runs:
    - CPU: about `100 ms`, ratio about `0.81x` versus optimization
    - GPU main route + CPU tail: about `163 ms`, ratio about `0.34x` versus optimization
  - `perturb` remains the expensive auxiliary tail:
    - CPU: about `1709 ms`, ratio about `13.89x` versus optimization
    - GPU main route + CPU tail: about `1753 ms`, ratio about `3.68x` versus optimization
  - current interpretation:
    - `trade_bootstrap` looks increasingly acceptable as a CPU tail unless a stronger GPU-native path becomes easy to implement
    - `perturb` still looks expensive enough to justify deeper architectural work if full GPU-native parity remains a product goal
- [x] Double-MA-family `cross_ticker` now preserves verdict-relevant candidate approval semantics and keeps GPU route parity through the Tauri cross-ticker orchestration layer.
- [x] Avoid rerunning verdict-irrelevant evidence modules during preserved-finalist cross validation:
  - `walk_forward`
  - `reopt_walk_forward`
  - `spa`
  - `perturb`
  - `trade_bootstrap`
  - rationale: they do not feed cross aggregation or candidate-level pass/fail semantics
- [ ] If full GPU parity is not planned, document forced CPU fallback behavior clearly in UX/status text.
- [x] Add pre-run and post-run route-capability surfacing for the Double-MA golden path:
  - validation UI now shows `Expected Route`, `Expected Cross`, `Expected Modules`, and `Expected Notes`
  - XLSX dashboard now mirrors the same expected-execution summary block alongside actual execution notes
  - post-run validation summaries and the XLSX `Execution Notes` block now explicitly surface `Aux CPU Tails` when a GPU main route still uses auxiliary CPU work for `perturb` and/or `trade_bootstrap`
  - service tests cover preserved-finalists GPU-main-route expectations and unsupported `GpuOnly` fixed-candidate requests
- [x] Fix validation service strategy-kind routing drift so request shaping and command dispatch stay aligned for:
  - `Kqsr`
  - `Carm`
  - `Kvdf`
  - `Cdmc`
- [ ] Add GPU parity for `reopt_walk_forward` if any route still falls back or degrades there.
- [ ] Add GPU parity for `SPA` if any route still depends on a CPU-only prep path.
- [ ] Add a true GPU-native batched `perturb` implementation if the current auxiliary CPU perturb tail remains too expensive for target workloads.
- [x] Investigate and fix the current GPU `perturb` performance pathology before treating that module as parity-complete:
  - immediate fix applied by removing the catastrophic per-iteration GPU-prep loop and preserving `GpuKernel` for the main route
- [x] Preserve GPU main-route execution when `trade_bootstrap` is enabled, even if the bootstrap statistics themselves remain CPU-computed.
- [ ] Add GPU parity for `trade_bootstrap` or explicitly decide that CPU fallback remains acceptable for that module.
- [x] Double-MA-family `cross_ticker` now supports a finalist-bounded verdict-preserving revalidation pass.
- [x] Make the preserved-finalist cross mode an explicit selectable policy because of runtime cost:
  - headless now supports `--cross-mode`
  - frontend validation UI exposes cross mode
  - request shaping defaults to `auto by strategy`
- [x] Improve XLSX behavior for the zero-survivor cross-ticker edge case so the workbook still provides meaningful charts or explicit chart placeholders when post-cross filtering removes every candidate.
- [x] Improve XLSX behavior for the zero-survivor cross-ticker edge case so the workbook still provides meaningful fallback charts based on the run verdict.
- [ ] Review whether the uniform-candidate constraints on MA types / params / strategy config are acceptable long-term or should be relaxed.
- [x] Add explicit tests proving fallback behavior when unsupported modules are requested on GPU validation routes:
  - `GpuKernel + cross_ticker` direct request falls back to `CpuOnly`
  - `GpuKernel + iterative validation engine` falls back to `CpuOnly`
  - `Auto + iterative validation engine` falls back to `CpuOnly`
  - orchestration-level preserved-finalists cross still preserves `GpuKernel` on the final Double-MA validation result

### Composite-family gaps

- [x] Build a profile-by-profile support table for:
  - `Asr`
  - `Vcc`
  - `Vemr`
  - `Rocb`
  - `Mkcs`
  - `Kqsr`
  - `Carm`
  - `Kvdf`
  - `Cdmc`
  - `CompositeCpu`
  - `CompositeMo`
  - current audit summary from `crates/app_core/src/composite_validate.rs`:
    - shared fixed-candidate GPU rules:
      - `cross_ticker` is not supported on the main composite GPU validation route
      - composite cross reruns are still rebuilt as lightweight requests in `src-tauri/src/commands_validation.rs`
      - fixed-candidate GPU validation requires the vectorized validation engine
      - unsupported GPU requests fall back to `CpuOnly`
    - `CompositeCpu`:
      - GPU fixed-candidate base validation: implemented
      - GPU `walk_forward`: implemented
      - GPU `cscv_pbo`: implemented
      - GPU `spa`: implemented
      - GPU `reopt_walk_forward`: implemented with extra restrictions
      - GPU `trade_bootstrap`: implemented
      - GPU `perturb`: not audited to the same parity depth yet
      - restrictions:
        - plain `CompositeCpu` only
        - `signal_mode=HardGate`
        - uniform strategy / HalfTrend / MACZ / OTT / MA-type configuration
    - `CompositeMo`:
      - CPU-preferred today for fixed-candidate validation
      - current GPU fixed-candidate path still requires plain `CompositeCpu`, so `CompositeMo` should be treated as a fallback-to-CPU route in previews
      - post-run validation summaries now also state that `CompositeMo` used the CPU validation route because fixed-candidate GPU validation currently supports plain `CompositeCpu` only
    - `Asr`, `Vcc`, `Vemr`, `Rocb`, `Mkcs`:
      - GPU fixed-candidate base validation: implemented
      - GPU `walk_forward`: implemented
      - GPU `cscv_pbo`: implemented
      - GPU `spa`: implemented
      - GPU `reopt_walk_forward`: implemented
      - GPU `trade_bootstrap`: implemented
      - dedicated CPU-vs-GPU parity tests are present in `composite_validate.rs`
    - `Kqsr`, `Carm`, `Kvdf`, `Cdmc`:
      - GPU fixed-candidate base validation: implemented
      - GPU `walk_forward`: implemented in code
      - GPU `reopt_walk_forward`: implemented in code
      - GPU `cscv_pbo`: implemented in the shared composite GPU route
      - GPU `spa`: implemented in the shared composite GPU route
      - GPU `trade_bootstrap`: implemented in the shared composite GPU route
      - current gap is evidence depth:
        - `Kqsr` now has a dedicated GPU-vs-CPU walk-forward parity test in addition to the existing advanced-route smoke
        - `Carm` now has stronger fixed-candidate prep coverage:
          - the fixed-candidate GPU validation path now uses CPU-reference `CARM` signal surfaces for `ROCR`, mixed momentum, and `OTTO`, then uploads those arrays for GPU metric evaluation
          - this mitigates the prior drift caused by threshold-sensitive `HostF32` signal outputs while keeping the GPU metrics route intact
          - remaining gap is deeper evidence for strict full in-sample parity and standalone direct GPU indicator parity
        - `Kvdf` now also has a dedicated GPU-vs-CPU walk-forward parity test in addition to the existing advanced-route smoke
        - `Cdmc` now has stronger fixed-candidate prep coverage:
          - the fixed-candidate GPU prep path now uses CPU-reference `CDMC` signal surfaces and the tested `start_t` mismatch is closed
          - the GPU `reopt_walk_forward` prep path now uses the same CPU-reference `CDMC` signal construction policy instead of the older hybrid sweep path
          - dedicated `start_t` parity coverage is now present for the current fixture
          - a strict full in-sample GPU-vs-CPU metrics parity assertion still does not hold on the noisy fixture, so evidence depth is improved but not fully closed yet
- [x] Make composite-family cross-ticker semantics explicit in frontend capability surfacing:
  - pre-run summaries now distinguish composite GPU main-route expectations from composite cross-ticker behavior
  - composite `cross_ticker` no longer incorrectly implies a full CPU fallback in the preview summary when the main validation route is otherwise GPU-capable
  - pre-run summaries now state that composite cross reruns remain lightweight and omit:
    - `walk_forward`
    - `reopt_walk_forward`
    - `spa`
    - `perturb`
    - `trade_bootstrap`
    - `cscv_pbo`
    - hard gates
  - post-run summaries now state `effective lightweight reruns` for composite cross mode even when the requested mode string is `preserved_finalists`
- [ ] Decide whether composite fixed-candidate GPU validation should gain `cross_ticker` support.
- [ ] Decide whether currently profile-conditional support for walk-forward, reopt walk-forward, SPA, trade bootstrap, and CSCV/PBO should be broadened.
- [ ] If some profiles will remain CPU-preferred for validation, document that explicitly rather than treating it as implicit backend behavior.

### Cross-ticker broadening

- [x] Double-MA cross-ticker reruns now preserve verdict-relevant candidate approval semantics and hard gates.
- [x] Double-MA cross-ticker now uses a bounded finalist policy to preserve full-stack coverage for selected finalists while improving runtime.
- [ ] Decide whether composite-family cross-ticker reruns should also preserve any of the original optional validation modules:
  - walk-forward
  - reopt walk-forward
  - SPA
  - perturb
  - trade bootstrap
  - CSCV/PBO
- [ ] If full preservation is too expensive, define an intentional reduced cross-ticker mode and expose that policy in the UI/XLSX.
- [ ] Add smoke tests using existing repo-local datasets such as:
  - `AAPL.csv`
  - `NVDA.csv`
  - `SPY.csv`
  - `QQQ.csv`
- [ ] Verify that cross-ticker filtering semantics are clearly represented in the final workbook for both approved and rejected candidates.

### Context propagation and artifact quality

- [x] Populate `validation_context` in headless validation flows so exported artifacts preserve strategy identity and other context.
- [ ] Audit other artifact-generation paths for missing validation context or strategy metadata.
- [ ] Add regression tests ensuring headless-generated XLSX artifacts do not show `Strategy = Unknown` for known strategies.

### Frontend capability surfacing

- [ ] Add validation-capability summaries to the UI comparable to existing optimization capability messaging.
- [ ] Warn users when a requested validation module will not run on the selected strategy/backend route.
- [ ] Surface when the backend falls back from requested GPU validation to CPU validation.
- [ ] Surface when a validation module was skipped intentionally versus disabled by the user.

### Verification

- [ ] Add end-to-end validation support tests for at least one strategy in each family using the broadest supported stack.
- [ ] Add route-specific tests for unsupported-module behavior on GPU validation paths.
- [ ] Add cross-ticker tests that use multiple repo-local datasets from `data/`.
- [ ] Extend XLSX verification so workbook dashboards clearly indicate which validation layers actually ran versus which were unavailable or disabled.

## Frontend and Workflow Requirements

Frontend behavior should support:

- manual `Export report XLSX`
- working `Auto-export report XLSX after validation`
- persisted `report_xlsx_path` in run artifacts/history
- graceful open/reveal behavior after save
- explicit preserved behavior for the current Stage 3 meta-report fallback when no validation result exists
- clear status messages for:
  - export succeeded
  - export skipped because no validation exists
  - export generated a report with zero approved candidates

## Headless and Automation Requirements

The headless validate-to-XLSX path must remain supported.

Requirements:

- headless and UI-triggered workbooks should use the same core builder path
- `save_report_xlsx` and `build_report_xlsx` fallback output should remain semantically aligned
- row-cap behavior should be explicit and testable
- output path logic should remain deterministic and sanitized

## Testing and Verification Requirements

Rust/backend:

- add targeted tests for candidate selection semantics used by XLSX
- add workbook-structure assertions beyond "bytes start with PK"
- verify sheet names and expected chart/drawing parts
- add tests for zero-approved-candidate runs
- add tests for multiple strategy families

Frontend:

- verify auto-export is actually invoked when enabled
- verify artifact path persistence after XLSX save
- verify UI status lines for success/failure/no-validation cases

Artifact/parity:

- update or replace shallow XLSX parity checks with workbook-semantic checks
- refresh sample artifacts after the new workbook shape is stable

Current preview artifact for manual inspection:

- `research/validation_report_preview.xlsx`

## Verification Evidence

Verification completed on 2026-03-13:

- Rust integration test target `src-tauri/tests/validation_xlsx_report_tests.rs` passes and covers:
  - expected sheet set
  - active Dashboard tab
  - chart presence
  - run-level rejection overriding candidate-level approval
  - legacy compatibility mode
  - strategy-family coverage across `ASR`, `VCC`, `VEMR`, `ROCB`, `MKCS`, `KQSR`, `CARM`, `KVDF`, and `CDMC`
- frontend workflow test target `web/src/services/validationService.test.ts` passes and covers:
  - auto-export after validation
  - persisted `report_xlsx_path`
  - non-fatal auto-export failure handling
  - preserved Stage 3 meta-report fallback
- generated preview workbook inspection confirmed:
  - workbook opens to `Dashboard`
  - workbook contains `Approved Candidates`, `Shortlist`, `Distributions`, `Audit Data`, `Dashboard`
  - workbook contains 7 chart parts across `Dashboard` and `Distributions`
  - dashboard/shared strings include run verdict, CSCV/PBO, SPA, reopt walk-forward, cross-ticker, row-cap notes, and fallback notes
- follow-up readability pass implemented on 2026-03-13:
  - candidate sheets and dashboard now render friendlier approval labels
  - distributions now surface approval-state counts plus top non-approval reasons for mixed approved/rejected runs
  - shortlist note now states the ordering rule: approved -> hard-gate survivors -> remaining context
  - dashboard `Top Visible Rows` now has explicit column headers and fills from approved rows first, then shortlist/context rows
  - preview/example source and Rust test expectations were updated for those surfaces
- refreshed execution verification completed on 2026-03-13:
  - `cargo test -q -p ta_desktop_demo_app --test validation_xlsx_report_tests` passes after the readability pass
  - `cargo run -q -p ta_desktop_demo_app --example generate_validation_report_preview` regenerated `research/validation_report_preview.xlsx`
  - real gated `DoubleMa` ALMA optimize -> validate -> XLSX flow reran against `data/AAPL.csv`
  - regenerated `_write_test/headless_reports/aapl_alma_validation_gated.xlsx` opens to `Dashboard`, contains 7 chart parts, and exposes gate-config, approval-state, top-reason, and dashboard-header surfaces in workbook XML

## Verification Checklist

Shared backend evidence:

- [x] report builder uses final validation approval semantics
- [x] zero-approved-candidate runs produce a valid workbook
- [x] all implemented strategies can export without MA-only assumptions
- [x] headless and UI-triggered exports share the intended builder path
- [x] compatibility behavior for legacy validation artifacts is explicit and tested
- [x] `save_report_xlsx` and `build_report_xlsx` fallback paths remain semantically aligned

Shared frontend evidence:

- [x] manual export works from the validation workspace
- [x] auto-export after validation works when enabled
- [x] `report_xlsx_path` persists for normal validation artifacts
- [x] history/open/reveal flows work for validation-generated XLSX files
- [x] the existing Stage 3 meta-report fallback behavior remains explicit and tested

Shared workbook evidence:

- [x] expected sheet set exists
- [x] expected charts/drawings exist
- [x] dashboard communicates run verdict and recommendation immediately
- [x] optional validation layers render as "not run" rather than zero

Shared regression evidence:

- [x] Rust tests cover workbook structure and selection semantics
- [x] frontend tests cover workflow wiring
- [x] sample artifacts refreshed
- [x] parity checks updated
- [ ] telemetry checks updated

## Acceptance Criteria

The work is complete only when all of the following are true:

1. XLSX export reflects final validation approval semantics rather than older shortlist-only heuristics.
2. The workbook includes a true dashboard-grade first sheet.
3. The workbook supports all implemented strategies without MA-only assumptions.
4. Manual export works from the validation workspace.
5. Auto-export after validation works when enabled.
6. Headless validate-to-XLSX still works.
7. Run artifact/history persistence includes the XLSX path for normal validation exports.
8. Rust tests validate workbook structure and report-selection semantics.
9. Frontend tests cover auto-export wiring and artifact persistence.
10. Sample/baseline artifacts and tracker docs are updated.
11. Compatibility behavior for older validation artifacts is intentional and documented.
12. The current no-validation Stage 3 fallback behavior is either preserved explicitly or deliberately replaced with documented rationale.

## Proposed Implementation Phases

## Phase 0: Planning and Contract Freeze

- confirm target workbook architecture
- define final candidate-selection semantics for XLSX
- define the minimum strategy-agnostic parameter summary contract
- define missing-data rendering policy

## Phase 1: Backend Data Contract Alignment

- audit current validation result fields used by the XLSX builder
- align report builder inputs with final approval semantics
- add helpers to normalize candidate summaries across strategies
- choose and implement the compatibility policy for older validation artifacts
- keep legacy fallback behavior only if it is still justified

## Phase 2: Workbook Redesign

- implement `Dashboard`
- implement `Approved Candidates`
- rework `Shortlist`
- add `Distributions`
- add `Audit Data`
- move charts onto staged data ranges

## Phase 3: Workflow Wiring

- wire auto-export after validation
- ensure normal validation exports persist `report_xlsx_path`
- verify reveal/open/history flows
- keep `build_report_xlsx` fallback and `save_report_xlsx` behavior aligned
- preserve or deliberately replace the current Stage 3 meta-report fallback behavior
- align headless export with the same builder path

## Phase 4: Tests and Sample Artifacts

- add Rust workbook-structure tests
- add selection-policy tests
- add frontend workflow tests
- refresh sample artifacts
- strengthen parity/telemetry checks

## Implementation Checklist

### Planning

- [x] Create this purpose-built planning document.
- [x] Confirm the target workbook architecture and keep it stable during implementation.
- [x] Confirm the final XLSX candidate-selection rule with current validation semantics.
- [x] Confirm the compatibility policy for older validation artifacts.
- [x] Confirm whether the current Stage 3 meta-report fallback remains unchanged.

### Backend: report selection semantics

- [x] Inventory all current validation result fields that the workbook should consume.
- [x] Stop treating `robust_pareto` and related shortlist heuristics as the sole final selection rule.
- [x] Surface run-level approval semantics in the builder.
- [x] Surface candidate-level approval semantics in the builder.
- [x] Add explicit handling for zero-approved-candidate runs.
- [x] Add explicit compatibility handling for artifacts that lack `run_verdict` and/or candidate `validation_verdict`.

### Backend: strategy coverage

- [x] Replace MA-only candidate-summary assumptions with strategy-agnostic summaries.
- [x] Keep strategy-local parameter formatting where required.
- [x] Verify all implemented profile families can produce a workbook without missing required columns.

### Backend: workbook redesign

- [x] Implement `Dashboard` sheet.
- [x] Implement `Approved Candidates` sheet.
- [x] Rework `Shortlist` sheet.
- [x] Implement `Distributions` sheet.
- [x] Implement `Audit Data` sheet.
- [x] Add staged chart-data ranges.
- [x] Apply a reusable semantic color system.
- [x] Surface approval-state and rejection-reason distributions for mixed approved/rejected runs.
- [x] Add explicit dashboard headers for the top visible-row summary table.
- [x] Make row-cap behavior explicit for visible tables versus full-set distributions/audit data.
- [ ] Review whether the legacy fallback builder should remain.

### Frontend and workflow

- [x] Wire `Auto-export report XLSX after validation` into the actual validation flow.
- [x] Preserve `report_xlsx_path` for normal validation artifacts.
- [x] Verify history/open/reveal flows for validation-generated XLSX files.
- [x] Preserve or deliberately replace the current Stage 3 meta-report fallback behavior in `exportReportXlsx()`.
- [x] Ensure status text is accurate for success, failure, and no-validation cases.
- [x] Make cross-ticker mode explicit in validation request shaping and visible in the validation workspace/UI.

### Headless and automation

- [x] Keep headless validate-to-XLSX working through the same core builder path.
- [x] Keep `build_report_xlsx` fallback output semantically aligned with saved-file output.
- [x] Verify row-cap behavior for headless and headless-preview flows.
- [x] Support explicit cross-ticker mode selection in headless validation runs.
- [ ] Verify deterministic output-path behavior.

### Tests

- [x] Add Rust tests for workbook sheet names and core parts.
- [x] Add Rust tests for selection semantics.
- [x] Add Rust tests for zero-approved-candidate scenarios.
- [x] Add Rust tests for at least one MA-family and one non-MA-family strategy.
- [x] Add frontend tests for auto-export behavior.
- [x] Add frontend tests for artifact persistence/open flows.
- [x] Strengthen artifact/parity checks beyond basic ZIP-shape validation.

### Samples and docs

- [x] Refresh sample XLSX artifacts after workbook redesign stabilizes.
- [ ] Update any telemetry/parity manifests that track XLSX outputs.
- [x] Update architecture/status docs to match the new XLSX state.

### VectorGrid naming cleanup

- [x] Audit all remaining user-visible `vectorbt_*` strings, artifact kinds, download filenames, and storage keys that should now be branded as `VectorGrid`.
- [ ] Separate true external `VectorBT` faceoff/benchmark references from internal app naming debt so only the benchmark/comparison surfaces retain `vectorbt` wording.
- [x] Rename user-visible validation/report/export download filenames from `vectorbt_*` to `vectorgrid_*` or neutral app-specific names:
  - validation exports now use `vectorgrid_*`
  - analysis top/run downloads now use `vectorgrid_*`
- [x] Rename user-visible artifact `kind` values such as `vectorbt_report` / `vectorbt_run_artifact` where feasible, while preserving backward compatibility for older saved artifacts:
  - new report payload writes now use `vectorgrid_report`
  - new run artifact writes now use `vectorgrid_run_artifact`
  - preset exports now use `vectorgrid_preset`
  - batch summary writes now use `vectorgrid_batch_summary`
- [x] Add compatibility readers so existing `vectorbt_*` artifact kinds and filenames continue to load after the rename:
  - run-artifact readers now accept both `vectorbt_run_artifact` and `vectorgrid_run_artifact`
- [x] Audit `run_artifacts.rs` path prefixes and decide whether on-disk artifact filenames should migrate immediately or remain backward-compatible aliases for one transition period:
  - batch summaries now write `vectorgrid_batch_*`
  - new run artifacts now write `vectorgrid_run_*`
  - pruning remains backward-compatible across both `vectorgrid_run_*` and `vectorbt_run_*`
  - meta selection artifacts now write `vectorgrid_meta_*`
  - Stage 5 / ML policy artifacts now write `vectorgrid_stage5_*` and `vectorgrid_ml_confidence_eval_*`
  - meta/stage listing and pruning now accept both legacy and renamed prefixes
- [x] Update frontend storage keys and history/preset naming with backward-compatible migration:
  - primary local-storage keys now use `vectorgrid_*` names with legacy `vectorbt_*` aliases
  - schema migration copies legacy values forward to the new primaries
  - preset exports now use `vectorgrid_preset_*`
- [x] Add regression tests covering mixed old/new artifact naming compatibility.
- [ ] Update research/docs so the app is consistently referred to as `VectorGrid`, with `VectorBT` mentioned only for explicit external comparisons.

## Risks and Guardrails

Risks:

- the workbook regresses into MA-only assumptions and breaks non-MA strategies
- the builder confuses shortlist heuristics with final approval semantics
- optional validation layers cause sparse-data layout failures
- UI and headless flows diverge into separate workbook behaviors
- chart/layout ambition outpaces test coverage and becomes fragile

Guardrails:

- keep candidate-summary normalization explicit and strategy-aware
- separate approved-candidate logic from broader shortlist logic
- treat missing optional validation layers as first-class expected cases
- keep chart data staged and testable
- prefer reusable formats and conditional formatting over brittle one-off styling

## Current Next Execution Order

1. Treat the Double-MA robustness-feature checkpoint as complete for the current first pass and keep any follow-up changes policy-driven rather than ad hoc.
   - `WFE / generalization`:
     - policy is frozen as explanatory + tie-break ordering only; no approval gate
   - `Regime-slice robustness`:
     - fragility/support flags and slice-by-slice audit rows are in the workbook path
   - `Dependence-aware bootstrap`:
     - first-pass semantics are frozen as a `compare` default (`iid + stationary`) plus an explicit `iid_only` option, both still operating on closed-trade returns
   - `Friction-stability`:
     - first-pass semantics are frozen as an absolute pass rule (`pnl >= 0 && sharpe >= 0`)
2. Continue improving user-facing validation route capability and fallback/skipped-module surfacing only where that work directly supports the now-frozen Double-MA robustness semantics.
   - post-run UI now exposes separate actual `Route`, `Route Detail`, `Aux CPU`, `Cross`, `Skipped`, and `Top Warning` summaries
   - post-run UI now also surfaces top-finalist `Bootstrap`, `Regime`, and `Friction` summaries so the workbook-only robustness signals are visible in the live validation panel
   - preserved-finalists post-run summaries now explicitly note that auxiliary cross reruns omit walk-forward, reopt WF, SPA, perturb, and trade bootstrap
3. Keep the optimization-side dynamic-strategy GPU perf gate on the separate steady-state measurement path rather than mixing it back into validation work.
   - `scripts/run_gpu_perf_gate.ps1` now uses `profile_gpu_dynamic_runner` steady-state iterations instead of the earlier criterion-on/criterion-off comparison that was overstating the dynamic ratio
   - current candidate gate passes again after dropping first-iteration warmup noise from the dynamic-strategy comparison
4. Continue the `VectorGrid` naming-cleanup pass for remaining docs/research wording and any residual non-product `vectorbt_*` references, while preserving backward compatibility.
5. Continue the composite-family validation support audit from the current surfacing baseline before attempting broader backend parity work.
   - the strategy/profile support matrix is now documented
   - `Kqsr` and `Kvdf` now have added walk-forward parity coverage
   - `Carm` fixed-candidate and reopt prep now use CPU-reference signal uploads where direct GPU signal parity was fragile
   - `Cdmc` fixed-candidate and reopt prep now use CPU-reference signal uploads where the earlier hybrid prep path was producing warmup drift
   - the next useful step is deeper strict parity evidence for `Carm` / `Cdmc` plus any explicit user-facing note for profiles that remain CPU-preferred such as `CompositeMo`

## Working Notes

Plateau / ranking-stability real-artifact review note:

- a real full-stack GPU Double-MA report regenerated through `generate_report_xlsx_from_json` now surfaces:
  - plateau score / label / source
  - plateau neighborhood Sharpe / MaxDD evidence
  - ranking-stability dashboard summary
  - rank-drift audit columns
  - compact plateau / max-drift charts on `Distributions`
- the reviewed artifact was:
  - `_write_test/headless_reports/aapl_alma_validation_full_stack_gpu.json`
  - regenerated as `_write_test/headless_reports/aapl_alma_validation_full_stack_gpu_plateau_review.xlsx`
- a stricter preserved-finalists cross artifact also regenerated correctly:
  - `_write_test/headless_reports/aapl_alma_validation_cross_mode_gpu.json`
  - regenerated as `_write_test/headless_reports/aapl_alma_validation_cross_mode_gpu_plateau_review.xlsx`
- in that harsher zero-survivor path, fallback charts still render correctly, but the richer ranking-stability lines are naturally absent because no finalist rows remain after filtering
- implication:
  - the new plateau / ranking-stability surfaces are now proven on a real positive-path artifact
  - the zero-survivor cross path still behaves acceptably, but it is not an informative review case for finalist-rank stability semantics

Regime-slice robustness real-artifact review note:

- a fresh post-change headless Double-MA validation run now emits `regime_slice_robustness` directly in the JSON artifact and surfaces the corresponding workbook fields:
  - `_write_test/headless_reports/aapl_alma_validation_regime_fresh.json`
  - `_write_test/headless_reports/aapl_alma_validation_regime_fresh.xlsx`
- verified JSON fields include:
  - `regime_slice_robustness`
  - `worst_bucket_label`
  - `stable_bucket_count`
  - `stable_bucket_frac`
  - `support_label`
- verified workbook text includes:
  - `Regime Worst`
  - `Regime Sharpe`
  - `Regime MaxDD`
  - `Regime Stable`
  - `Stable Frac`
  - `Regime Support`
  - `Regime Robustness`
  - `Worst Bucket`
- implication:
  - regime-slice robustness is now implemented end to end for Double-MA finalists
  - backend results now carry explicit regime-support semantics instead of deriving them only in the export layer
  - shortlist/export semantics are now frozen for the first pass:
    - workbook columns include `Regime Flag` and `Regime Support`
    - the dashboard links directly to `Audit Data -> Regime Slice Audit`
    - the audit sheet now materializes per-bucket rows for `low_vol`, `mid_vol`, `high_vol`, `trend`, and `range`

Checkpoint completion note:

- WFE is now frozen as an explanatory plus shortlist tie-break signal for the Double-MA family; it is still not a hard approval gate.
- Regime-slice robustness now includes shortlist-level fragility/support labels plus slice-by-slice audit rows in the workbook.
- Dependence-aware bootstrap now explicitly reports the current first-pass semantics as an IID plus stationary closed-trade comparison; moving-block remains future work.
- Friction-stability now freezes its first-pass semantics around an absolute stress pass rule (`pnl >= 0` and `sharpe >= 0`) and remains explanatory rather than a hard approval gate.

Dependence-aware bootstrap first-pass note:

- the current bootstrap path was extended to emit a stationary-bootstrap comparison alongside the existing iid trade bootstrap
- this first pass is intentionally low churn:
  - the existing iid fields remain unchanged
  - dependence-aware fields are emitted as additional comparison diagnostics
  - the bootstrap still operates on closed-trade return series rather than a per-bar return path
  - `stationary_mean_block_len` can now be supplied explicitly instead of always using the heuristic
- a fresh post-change artifact confirms the new JSON and workbook surfaces:
  - `_write_test/headless_reports/aapl_alma_validation_regime_fresh.json`
  - `_write_test/headless_reports/aapl_alma_validation_regime_fresh.xlsx`
- verified JSON fields include:
  - `stationary_mean_block_len`
  - `stationary_pnl_p05`
  - `stationary_max_dd_p95`
- verified workbook text includes:
  - `Bootstrap P05`
  - `Bootstrap DD95`
  - `Stat Block`
  - `Stat P05`
  - `Stat DD95`
  - `Bootstrap Compare`
- validation note:
  - `Trade.pnl` is already encoded as a fractional trade return in `ta_strategies::double_ma::Trade`, so the bootstrap is not compounding absolute-dollar PnL as if it were a return multiplier
  - the large bootstrap quantiles seen on the fresh AAPL artifact are consistent with the strategy's own very large compounded in-sample `pnl`, not a stationary-bootstrap-only bug
- semantics note:
  - the current first-pass bootstrap presentation is explicitly frozen as:
    - `compare` (default): `iid + stationary closed-trade comparison`
    - `iid_only`: iid-only closed-trade bootstrap
  - the workbook dashboard, XLSX execution notes, request-preview summaries, and validation UI now state the selected mode directly instead of leaving compare-mode implicit
  - the headless validation surfaces now expose the same first-pass mode split via `bootstrap_mode`, keeping automated validation runs aligned with the interactive path
  - the heavy-holdout/meta request builders now preserve the same bootstrap-mode semantics instead of collapsing back to iid-only behavior implicitly

WFE / friction policy note:
- `WF Efficiency` now carries an explicit workbook policy line: explanatory + tie-break ordering only; no approval gate
- `Friction Stability` now carries an explicit workbook rule line: absolute pass uses `pnl >= 0 && sharpe >= 0`
- fresh real-artifact re-review was completed by regenerating:
  - `_write_test/headless_reports/aapl_alma_validation_friction_wfe_no_cross.json`
  - `_write_test/headless_reports/aapl_alma_validation_friction_wfe_completed.xlsx`
- verified workbook text includes:
  - `WF Efficiency`
  - `explanatory + tie-break ordering only; no approval gate`
  - `Regime Flag`
  - `Regime Support`
  - `Regime Slice Audit`
  - `Compare (IID + Stationary)`
  - `absolute pass: pnl >= 0 and sharpe >= 0`

Optimization perf gate audit note:

- current `target/gpu_perf_summary.json` shows the `double_ma/gpu_kernel/sma_sma/200k/small` benchmark at about `462.7 ms`, versus the older `scripts/baselines/gpu_perf_baseline.json` baseline at about `155.3 ms`
- the dynamic preflight ratio is now effectively neutral (`on/off ~= 1.006`), so the current gate failure is no longer explained by `VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL`
- local source inspection indicates the benchmark still targets the same Criterion path in `scripts/run_gpu_perf_gate.ps1` and `crates/app_core/benches/double_ma_opt.rs`
- most plausible remaining causes are:
  - the benchmark workload or optimization runtime path has materially changed since the older baseline was captured
  - the older baseline is now stale relative to current code and/or driver state
  - the current slowdown is in the broader GPU optimization path rather than in dynamic-preflight-only logic
- next concrete checks when command execution is available again:
  - rerun the single Criterion benchmark directly and capture route metadata from a matching ad hoc optimization call
  - compare the current benchmark timing against a small direct `run_double_ma_optimization_blocking_with_candles()` smoke/profiler run on the same request shape
  - inspect whether kernel-route output still reports plain `GpuKernel` with no hidden fallback or extra staging

Real gated validation artifact used for mixed-run sanity checks:

- `_write_test/headless_reports/aapl_alma_validation_gated.json`
- run recommendation: `single_candidate_supported`
- approved candidates: 2
- rejected candidates: 6
- dominant rejection reason: `dsr_below_min:0.30`
- double-MA headless report now emits:
  - `run.req.strategy_kind = DoubleMa`
  - `validation.req.strategy_kind = DoubleMa`
  - `validation.req.validation_context`
  - hard-gate request fields such as `min_dsr`

Real explicit cross-mode GPU artifacts used to verify preserved-finalist reporting:

- earlier cross-mode reference:
  - `_write_test/headless_reports/aapl_alma_validation_cross_mode_gpu.json`
  - `_write_test/headless_reports/aapl_alma_validation_cross_mode_gpu.xlsx`
- optimized preserved-finalists reference:
  - `_write_test/headless_reports/aapl_alma_validation_cross_optimized_gpu.json`
  - `_write_test/headless_reports/aapl_alma_validation_cross_optimized_gpu.xlsx`
- verified:
  - full-stack GPU validation runtime on the optimized artifact: `7550 ms`
  - `backend_used = Gpu`
  - `backend_route = GpuKernel`
  - `cross_ticker.mode = preserved_finalists`
  - workbook text contains `Preserved Finalists`
  - workbook text contains `Cross-Ticker`
  - workbook text contains `GpuKernel`
  - workbook opens to `Dashboard`
  - zero-survivor fallback still emits charts (`ChartCount = 2`)
  - fallback chart titles are `Approval state counts` and `Top rejection / review reasons`
  - dashboard now includes an `Execution Notes` block for:
    - route detail / route note text
    - skipped-module summary
    - top warning or fail reason
  - frontend validation panel now includes structured execution summaries for:
    - route
    - cross mode / dataset count
    - skipped modules
    - top warning

Gate results after the aligned-horizon preserved-finalists optimization:

- workflow gate:
  - command: `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate`
  - status: pass
  - summary: `target/workflow_regression_summary.json`
- GPU perf gate:
  - command: `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`
  - core kernel benchmark status: pass
  - ratio gate status: fail
  - summary: `target/gpu_perf_summary.json`
  - gate script now snapshots separate `OFF` and `ON` estimate files:
    - `target/gpu_perf/off_estimates.json`
    - `target/gpu_perf/on_estimates.json`
  - current failing check:
    - latest rerun shows the problem is no longer only the dynamic ratio
    - current candidate run measured about `462.72 ms` off vs `465.29 ms` on
    - interpretation: the gate output is now less ambiguous, but there is still a separate optimization benchmark slowdown to audit
  - interpretation:
    - likely unrelated to the validation/XLSX changes because the failing benchmark targets the optimization path rather than the validation route
  - dashboard cross section now labels count-based verdict summaries as `final post-cross shortlist`

Reference inspiration only:

- external `SERENITY` workbook design language
- external Python `__init__.py` formatter

Key design decision to preserve from that inspiration:

- "open once, understand instantly"
- mixed validation runs must show both approved candidates and the dominant rejection reasons immediately

Key implementation decision not to preserve:

- giant single-sheet, hard-coded coordinate logic tied to one dataframe shape
