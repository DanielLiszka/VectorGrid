# GPU Validation Migration Plan

Date: 2026-03-10

Scope: migrate the validation stage from its current mostly host/CPU execution model toward an end-to-end GPU-capable pipeline that reuses the existing VRAM-resident optimization backend, while preserving the current CPU-only validation path for CPU users and unsupported GPU cases.

This document is a planning and tracking artifact. It is intentionally biased toward extending the existing GPU optimization substrate rather than inventing a separate validation engine with duplicate logic.

March 12, 2026 late-pass implementation note:

- source now carries composite GPU validation execution for `KQSR`, `CARM`, `KVDF`, and `CDMC` across in-sample, `walk_forward`, `CSCV/PBO`, `SPA`, `reopt_walk_forward`, `trade_bootstrap`, `perturb`, and command-layer `cross_ticker` reruns
- representative source tests now exist in:
  - `crates/app_core/src/composite_validate.rs`
  - `src-tauri/src/commands_validation.rs`
- closure evidence was re-run successfully in this session with:
  - `cargo check -p ta_app_core --features cuda,cuda-backtest-kernel`
  - `cargo test -p ta_app_core validation_gpu_kernel_matches_cpu_metrics --features cuda,cuda-backtest-kernel`
  - `cargo test -p ta_app_core advanced_routes_smoke --features cuda,cuda-backtest-kernel`
  - `cargo test -p ta_app_core gpu_smoke_runs_with_ --features cuda,cuda-backtest-kernel`
  - `cargo test -p ta_desktop_demo_app cross_ticker_ --features cuda,cuda-backtest-kernel`
  - `cargo check -p ta_desktop_demo_app --features cuda,cuda-backtest-kernel`
  - `npm --prefix ./web run typecheck`
  - `npm --prefix ./web run test -- validationRequestBuilder.test.ts`

## Goal

Turn validation into a dual-engine system:

- CPU validation remains the compatibility baseline.
- GPU validation becomes the preferred execution path when:
  - CUDA is enabled
  - the request/backend is GPU-capable
  - the specific validation feature set is supported on GPU

The desired product flow is:

1. upload price data once to VRAM
2. run exact brute-force optimization on GPU
3. keep shortlisted candidates on device
4. run validation reruns on the same device-resident data
5. copy back only compact validation summaries and surviving robust candidates
6. keep meta-analysis on CPU

The target state is not "some GPU acceleration during validation". The target state is:

- fewer host/device round-trips
- no redundant CPU reconstruction of candidate series for GPU-capable validations
- consistent validation semantics across CPU and GPU
- materially lower latency for validation on large GPU workloads

## Current Architecture Findings

This section reflects a full parallel scan of the current backend/frontend architecture. It intentionally replaces the earlier simplified “current state” summary with a repo-grounded capability map.

### 1. Double-MA and double-like optimization: actual GPU capability map

The optimization side is not uniformly “GPU”. There are several distinct execution tiers:

- `Backend::GpuKernel`
  - this is the real device-heavy exact optimization path
  - candles/source data are uploaded, MA work is done in VRAM, CUDA backtest kernels run, and top-k can remain on device until compact results are copied back
  - key files:
    - `crates/app_core/src/double_ma.rs`
    - `crates/app_core/src/gpu_backtest_kernel.rs`
- `Backend::GpuHybridKernel`
  - mixed by design
  - some indicator/MA work can still be CPU-staged depending on `ma_policy`
  - not fully VRAM-resident
- `Backend::GpuOnly`
  - not an end-to-end device-resident search path
  - MA sweeps may use CUDA, but pair evaluation is still host/CPU-oriented
- `Backend::CpuOnly`
  - fully CPU

Important current facts:

- `GpuKernel` is the primary exact brute-force GPU route for `DoubleMa`.
- several double-like device-regime variants already exist and can build regime state on device before using the same kernel optimizer
- even `GpuKernel` is not “literally no host involvement”; inputs originate on host and final compact results return to host
- host-heavy behavior still returns when modes like `include_all`, CSV export, or some host-side constraint flows are active

Important backend routing files:

- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/double_ma_backend.rs`
- `crates/app_core/src/ma_policy.rs`

### 2. Validation: current execution reality

Validation is not using the optimization GPU substrate yet in any end-to-end sense.

#### Fully CPU/host-driven today

- shared validation orchestration:
  - `crates/app_core/src/validation_core.rs`
- validation policy / verdict logic:
  - `crates/app_core/src/validation_pipeline.rs`
  - `crates/app_core/src/validation_stats.rs`
  - `crates/app_core/src/sharpe_stats.rs`
  - `crates/app_core/src/cscv_pbo.rs`
- `DoubleMa` validation end to end:
  - candidate series are built with `ma_matrix_cpu_f64`
  - metrics/trades/slice returns are evaluated on CPU
  - key file:
    - `crates/app_core/src/double_ma_validate.rs`

#### Repeated hot-path work in validation

These are the parts that actually dominate a future GPU migration:

- per-candidate series construction
- per-candidate metric evaluation
- walk-forward window reruns
- finalist re-opt walk-forward reruns
- CSCV slice-return generation
- some perturbation and trade-bootstrap related recomputation

The hot path is not verdict assembly. The hot path is repeated candidate evaluation.

#### CPU-safe policy/reduction layers

These are good CPU residents even after a GPU migration:

- hard-gate sanitization
- PSR / DSR / MinTRL math
- verdict assembly
- robust-score folding / final ranking
- final SPA p-value reduction if we keep bootstrap host-side initially
- artifact serialization

#### Important current caveat

- `cross_ticker` exists in the request/result contract and UI flow, but it is not yet a fully built-out repeated-compute backend path in the same sense as main validation, walk-forward, or CSCV
- that means GPU migration should not assume cross-ticker is already a mature compute lane; it likely needs separate implementation work regardless of backend

### 3. Composite and dynamic-strategy architecture: actual GPU boundaries

The composite side is more nuanced than the earlier simplified summary.

#### Truly GPU today

Only four `CompositeCpu` custom-strategy optimization paths are genuinely GPU-capable end to end today:

- `ASR`
- `VCC`
- `VEMR`
- `ROCB`

These use dedicated CUDA-oriented paths in:

- `crates/app_core/src/composite_cpu.rs`

They can prepare device-resident inputs and run dedicated GPU metrics kernels.

#### GPU-assisted only

Composite validation is not end-to-end GPU, but it is not purely CPU either:

- validation prepares CUDA indicator inputs once with:
  - `PreparedCudaIndicatorInputs::from_candles(...)`
- custom strategy candidate builders (`ASR`, `VCC`, `VEMR`, `ROCB`) can switch to `Backend::Auto` when those prepared CUDA inputs exist
- however, downstream validation metric evaluation, slice returns, CSCV, SPA, bootstrap, and verdicts still remain CPU/host-side

Key files:

- `crates/app_core/src/composite_validate.rs`
- `crates/app_core/src/dynamic_indicator_adapter.rs`
- `crates/app_core/src/asr.rs`
- `crates/app_core/src/vcc.rs`
- `crates/app_core/src/vemr.rs`
- `crates/app_core/src/rocb.rs`

#### CPU-bound today

- plain `CompositeCpu` optimization is no longer purely CPU-bound:
  - there is now a bounded explicit `GpuKernel` optimization route for plain `CompositeCpu` when:
    - `signal_mode=HardGate`
    - the request explicitly asks for `GpuKernel`
    - the MA/objective/search shape is kernel-compatible
    - the run does not depend on prescan-subset `allowed_pairs`
  - outside that bounded route, base composite optimization remains on the existing CPU/mixed path
- base composite validation is no longer purely CPU-bound:
  - plain `CompositeCpu` `signal_mode=HardGate` fixed-candidate validation now has a bounded `GpuKernel` route for:
    - in-sample
    - `walk_forward`
    - `CSCV/PBO`
    - `SPA`
    - `reopt_walk_forward`
    - `trade_bootstrap` with host-side bootstrap reduction
    - `perturb` via GPU reruns of perturbed single-candidate requests
  - command-layer `cross_ticker` reruns now preserve the requested base-composite backend by rebuilding per-dataset validation requests with `cross_ticker: None`; representative smoke coverage lives in `src-tauri/src/commands_validation.rs`
- `CompositeMo` remains CPU-bound for actual optimization/evaluation, aside from optional GPU prescan/narrowing

Key files:

- `crates/app_core/src/composite_cpu.rs`
- `crates/app_core/src/composite_mo.rs`

#### Product-facing note: `CompositeTrend`, `RegimeHybrid`, and `BreakoutMomentum`

These three strategy kinds were initially treated as composite-base candidates during planning, but the current product route is now explicit and should be described that way.

- In the current app route, they are sent through:
  - `web/src/services/optimizeService.ts` -> `run_double_ma_optimization`
- Validation already treats them as `DoubleMa`-family validation artifacts via:
  - `src-tauri/src/commands_validation.rs`
- Their active strategy semantics on the optimization path come from `StrategyConfig` overlays handled in:
  - `crates/app_core/src/double_ma.rs`
  - `crates/app_core/src/strategy_presets.rs`

Effective active indicator sets on the product route:

- `CompositeTrend`
  - `fast_ma`
  - `slow_ma`
  - `linearreg_angle`
  - `bollinger_bands_width`
  - optional `roc`
  - optional `adx`
- `RegimeHybrid`
  - `fast_ma`
  - `slow_ma`
  - `linearreg_angle`
  - `bollinger_bands_width`
  - optional `roc`
  - `adx`
- `BreakoutMomentum`
  - `fast_ma`
  - `slow_ma`
  - `bollinger_bands`
  - `macd`
  - `cmo`
  - `adosc`
  - optional `adx`

Important `gpu_speedups.csv` overlap:

- directly present in `gpu_speedups.csv`:
  - `linearreg_angle`
  - `bollinger_bands_width`
  - `roc`
  - `bollinger_bands`
  - `macd`
  - `cmo`
  - `adosc`
  - `di`
  - `adxr`

Current route policy on the product path:

- `CompositeTrend`
  - `ADX off` -> `GpuKernel`
  - `ADX on` -> `GpuHybridKernel`
- `BreakoutMomentum`
  - `ADX off` -> `GpuKernel`
  - `ADX on` -> `GpuHybridKernel`
- `RegimeHybrid`
  - current ADX-bearing form -> `GpuHybridKernel`

So the remaining work for these three is no longer “invent a new composite GPU runtime first.” It is:

- preserve the `double_ma`-family semantics already used by the app
- keep route selection explicit and testable
- later extend the same per-stage GPU model into validation

### 4. Dynamic strategy and GPU-unified infrastructure: what exists vs what it means

The codebase already has meaningful GPU-unified infrastructure, but it is easy to overread what that means.

What exists:

- GPU-aware route planning:
  - `crates/app_core/src/strategy_plan.rs`
  - `crates/app_core/src/ma_policy.rs`
- GPU predicate-program compilation:
  - `crates/app_core/src/strategy_gpu_program.rs`
- reusable CUDA indicator staging / device chaining:
  - `crates/app_core/src/dynamic_indicator_adapter.rs`

What that does **not** mean yet:

- it does not mean validation metrics are GPU-native
- it does not mean walk-forward / CSCV / SPA are GPU-native
- it does not mean the app already has a shared `GpuValidationContext`

Important current clue:

- `strategy_dynamic.rs` is still centered on `PreparedDynamicCpuContext`
- even when some staging uses CUDA, final prepared contexts and metric evaluation remain CPU-oriented

So the repo already has strong GPU building blocks, but not yet the glue that turns them into a shared GPU validation runtime.

### 5. Tauri/frontend workflow and request/result contracts

Validation is driven from the saved workflow artifact, not from raw optimize form state.

Current flow:

1. optimization writes a run artifact
2. validation reads that artifact
3. validation verifies dataset/timeframe/strategy compatibility
4. validation builds a request from persisted top candidates and validation options
5. backend runs validation
6. frontend stores the validation payload back onto the workflow artifact

Key files:

- `web/src/services/validationService.ts`
- `web/src/services/validationRequestBuilder.ts`
- `web/src/stores/validationStore.ts`
- `src-tauri/src/commands_validation.rs`
- `src-tauri/src/validation_common.rs`

Important contract fact:

- optimization already exposes route metadata end to end
- validation does **not** currently expose equivalent route metadata such as:
  - backend used
  - validation execution route
  - fallback reason

That gap must be closed before a GPU validation migration is user-visible or debuggable.

### 6. Meta-analysis, artifacts, and reporting: what should stay CPU

Meta-analysis is safe to keep CPU.

The scan confirms that meta is primarily:

- artifact scanning
- grouping
- quantiles and pass-rate aggregation
- ranking / evidence warnings
- holdout orchestration around validation results

These are not the same kind of repeated per-candidate backtest loops that justify GPU migration.

Key files:

- `crates/app_core/src/meta_select.rs`
- `src-tauri/src/commands_meta.rs`
- `src-tauri/src/commands_reports.rs`
- `src-tauri/src/run_artifacts.rs`

Important migration constraint:

- the validation artifact envelope must remain stable
- meta and reports already consume fields like:
  - `validation.res.candidates`
  - `robust_score`
  - `robust_pareto`
  - `walk_forward`
  - `psr`
  - `dsr`
  - `cscv_pbo`
  - `run_verdict`

So a GPU validation migration should swap execution internals, not invent a new public artifact contract.

### 7. Product constraints that must remain true

1. CPU-only users must keep a fully working validation path.
2. Validation outputs must keep the same schema shape and verdict semantics across CPU and GPU routes.
3. Unsupported GPU validation combinations must fall back cleanly to CPU with explicit route metadata.
4. Meta-analysis can remain CPU-oriented because it is primarily statistical aggregation rather than repeated backtest execution.
5. The migration should reuse the existing optimization substrate and route concepts, not create a second unrelated routing system.

## Architectural Direction

The right model is:

- shared validation policy
- dual execution engines

Shared validation policy means:

- hard gates
- CSCV / PBO semantics
- PSR / DSR
- MinTRL
- walk-forward semantics
- SPA semantics
- run approval / rejection semantics

should remain logically the same regardless of execution backend.

Dual execution engines means:

- CPU engine:
  - current baseline
  - preserves all existing behavior
- GPU engine:
  - reuses the same device-resident optimization primitives where possible
  - moves repeated candidate evaluation onto the GPU
  - returns compact summary statistics to the host

This is a pipeline-extension problem, not a clean-slate rewrite:

- optimization is already the working GPU substrate
- validation should reuse that substrate
- host code should primarily orchestrate and reduce compact stats

## Target GPU Validation Pipeline

Target flow for GPU-capable validation:

1. Data residency
- upload candles and relevant derived source series once
- keep device buffers alive across optimization and validation

2. Candidate residency
- keep shortlisted candidate params on device when validation immediately follows optimization
- allow host-uploaded candidate lists for standalone validation runs

3. Validation evaluation kernels
- run batched validation candidate backtests on GPU
- emit compact per-candidate summary stats, not full reconstructed series

4. GPU-supported validation features
- fixed-candidate in-sample validation
- walk-forward window evaluation
- CSCV slice-return generation
- re-opt walk-forward over finalists
- cross-dataset validation when each dataset can be staged efficiently

5. Host-side reduction
- PSR / DSR reduction if not already integrated into the device summary path
- MinTRL and hard-gate verdict assembly
- SPA p-value reduction if bootstrap remains host-side initially
- final serialization to validation artifacts

6. Meta stage
- stays on CPU
- consumes validation artifacts and holdout summaries only

## Recommended Migration Strategy

Do not attempt a one-shot rewrite.

The safest path is:

1. introduce GPU validation routing and route metadata
2. move the fixed-candidate validation pass onto the GPU first
3. then move walk-forward and CSCV
4. then move re-opt walk-forward
5. only then revisit SPA / bootstrap depth

This keeps each phase measurable and fallback-safe.

## Acceptance Criteria

1. `DoubleMa` validation can execute on GPU without rebuilding candidate series on CPU when the request is GPU-eligible.
2. Validation can reuse device-resident market data from the optimization stage where the workflow is contiguous.
3. GPU validation and CPU validation produce schema-compatible outputs and materially similar verdict outcomes.
4. CPU-only validation remains fully functional and remains the fallback path.
5. Unsupported GPU feature combinations fall back to CPU cleanly, with explicit backend-route metadata.
6. The validation UI does not need to know the implementation details; it only sees route metadata and normal validation outputs.
7. Meta selection remains unchanged except for consuming the improved validation outputs.

## Non-Goals

- Rewriting meta-analysis onto the GPU
- Replacing the CPU validation path
- Building a separate GPU-only validation framework with duplicate business logic
- Chasing Tensor Core usage for validation unless a future surrogate/ML layer justifies it

## Implementation Plan

### Phase 0: Safety, Contracts, and Routing

Objective: define the dual-engine contract before moving heavy logic.

Tasks:

- add explicit validation backend request/result metadata:
  - request carries `backend`
  - result carries `backend_used`
  - result carries `backend_route`
  - result carries `backend_fallback_reason`
- define GPU validation eligibility checks for:
  - backend
  - strategy family
  - objective / metrics support
  - enabled validation features
- keep request/response schemas stable
- define a shared internal summary-stat contract so CPU and GPU engines return comparable candidate-level outputs

Deliverables:

- backend-aware validation request/result contract
- initial validation route/fallback metadata on CPU runtime
- GPU validation route resolver
- shared candidate summary struct contract
- explicit fallback reason fields

### Phase 1: Reusable GPU Validation Context

Objective: create a reusable device-resident context that validation can consume.

Tasks:

- define a `GpuValidationContext` or equivalent internal object containing:
  - device candles
  - source series / returns
  - any reusable MA or indicator staging buffers
  - device/runtime handles
- add context construction for:
  - standalone validation
  - post-optimization validation reuse
- ensure context creation does not break CPU builds
- add lifetime/cleanup rules so VRAM use is explicit and deterministic

Deliverables:

- GPU validation context builder
- contiguous optimize -> validate context reuse path
- CPU-safe no-op equivalent path

### Phase 2: GPU Fixed-Candidate Validation Pass

Objective: move the base candidate validation run onto the GPU first.

Tasks:

- implement GPU evaluation for fixed candidate lists using the existing GPU backtest substrate
- return compact candidate metrics needed by validation:
  - Sharpe
  - PnL
  - max drawdown
  - trades
  - any additional required metrics for robust score / gates
- preserve candidate ordering/ranking semantics
- validate parity vs CPU on representative strategy cases

Initial scope:

- `DoubleMa`
- double-like strategy overlays already supported by the GPU optimization substrate where practical

Deliverables:

- GPU fixed-candidate validation for `DoubleMa`
- route metadata proving the GPU path was used

Current implementation note:

- initial GPU activation now exists for fixed-candidate `DoubleMa` validation
- current support is intentionally narrow:
  - `GpuKernel` / `GpuHybridKernel` / `Auto`
  - uniform candidate MA types
  - uniform strategy / halftrend / MA params across candidates
  - `walk_forward` now runs on `GpuKernel` and kernel-executable `GpuHybridKernel` routes for the fixed-candidate path
  - `CSCV/PBO` now runs on `GpuKernel` and kernel-executable `GpuHybridKernel` routes for the fixed-candidate path
  - `SPA` now reuses GPU-generated slice sums on `GpuKernel` and kernel-executable `GpuHybridKernel` routes while keeping the final SPA bootstrap/reduction on host
  - `perturb` now reuses per-sample `GpuKernel` reruns on the fixed-candidate path
  - `reopt_walk_forward` now runs on `GpuKernel` and kernel-executable `GpuHybridKernel` routes for the fixed-candidate path
  - no trade bootstrap
  - no cross-ticker
  - vectorized validation engine only
- unsupported combinations still fall back explicitly to CPU validation with a route/fallback reason

Parity investigation note:

- The current validation parity concern is broader than `f32` vs `f64`.
- Confirmed:
  - plain fixed-candidate `sma/sma` validation matches on `GpuKernel`
  - `trend_quality` ADX-off fixed-candidate validation now matches on `GpuKernel`
  - `breakout_momentum` ADX-off fixed-candidate validation now matches on `GpuKernel`
  - `RegimeHybrid` fixed-candidate validation now runs on `GpuHybridKernel` and matches CPU within small floating-point tolerance
  - plain fixed-candidate `walk_forward` now runs on `GpuKernel` and matches CPU within expected floating-point tolerance
  - plain fixed-candidate `CSCV/PBO` now runs on `GpuKernel` and matches CPU within expected floating-point tolerance
  - plain fixed-candidate `SPA` now reuses GPU-generated slice sums on `GpuKernel` and matches CPU within expected floating-point tolerance
  - `CompositeTrend` ADX-on fixed-candidate `walk_forward` now runs on `GpuHybridKernel` and matches CPU within expected floating-point tolerance
  - `BreakoutMomentum` ADX-on fixed-candidate `CSCV/PBO` now runs on `GpuHybridKernel` and matches CPU within expected floating-point tolerance
  - `RegimeHybrid` fixed-candidate `walk_forward` and `reopt_walk_forward` now run on `GpuHybridKernel` and match CPU within expected floating-point tolerance
  - `RegimeHybrid` fixed-candidate `CSCV/PBO` and `SPA` now have `GpuHybridKernel` route smoke coverage; parity remains more data-sensitive because slice stats are reconstructed from per-slice metrics before the shared host reducers
- The main identified mismatch classes are:
  - validation GPU currently reuses the optimization kernel path, not the CPU validation evaluator path
  - CPU validation computes `start_t` from actual first finite MA rows; the GPU path needed explicit alignment of MA warmup and `regime_first_valid`
  - hybrid validation still deserves ongoing scrutiny because it reuses the optimization path and therefore remains more sensitive to `f32` reduction drift than the CPU validation evaluator
- Important narrowing result:
  - the original `breakout_momentum` mismatch was not primarily caused by different regime generation code
  - both CPU validation and the GPU kernel path use the same host `compute_combined_regime_filter(...)` output
  - the main confirmed breakout cause was warmup/start-index alignment in the GPU kernel path
  - breakout fixed-candidate `GpuKernel` validation now has parity coverage again after separating `regime_first_valid` from MA warmup in the kernel runtime
  - the same fix also restored parity for `trend_quality` ADX-off fixed-candidate `GpuKernel` validation
  - the remaining `RegimeHybrid` drift after activation was small enough to classify as floating-point tolerance rather than a semantic mismatch

### Phase 3: GPU Walk-Forward

Objective: move rolling window validation reruns onto the GPU.

Tasks:

- implement batched window evaluation over a fixed candidate set
- keep window definitions on host, but keep candidate/data execution on device
- produce compact `WalkForwardStats` summaries directly from GPU-generated window metrics
- verify parity with CPU walk-forward semantics

Deliverables:

- GPU walk-forward engine
- same result schema as current `WalkForwardStats`

### Phase 4: GPU CSCV / Slice Statistics

Objective: move slice-return generation onto the GPU because it maps naturally to batched device evaluation.

Tasks:

- implement GPU generation of per-candidate slice return stats
- preserve the current CSCV/PBO semantics and output shape
- keep final CSCV reduction on host initially if needed
- verify parity on:
  - candidate-level CSCV stats
  - run-level PBO

Deliverables:

- GPU slice-stat generation
- CPU-compatible CSCV outputs

Current status:

- fixed-candidate `GpuKernel` and kernel-executable `GpuHybridKernel` validation now cover both `CSCV/PBO` and `SPA`
- the repeated slice-evaluation work is now on GPU for both features
- final `CSCV/PBO` and `SPA` statistical reduction still remains host-side by design
- hybrid parity evidence currently includes `BreakoutMomentum` ADX-on `CSCV/PBO`; `RegimeHybrid` `CSCV/PBO` / `SPA` currently have route-smoke coverage and remain more data-sensitive because slice stats are reconstructed before host-side final reduction

### Phase 5: GPU Re-Opt Walk-Forward

Objective: move the most multiplicative expensive validation mode onto the GPU.

Tasks:

- reuse the existing GPU optimization backend to re-optimize finalists per window
- keep finalist candidates and reusable data on device across windows where possible
- return compact `ReoptWalkForwardStats`
- confirm deterministic behavior where deterministic optimization behavior is expected

Deliverables:

- GPU finalist re-opt walk-forward for `DoubleMa`
- bounded runtime vs CPU re-opt walk-forward

Current status:

- fixed-candidate `GpuKernel` and kernel-executable `GpuHybridKernel` validation now cover finalist `reopt_walk_forward`
- the GPU path reuses the same windowed metric cache pattern as `walk_forward`, `CSCV/PBO`, and `SPA`
- final `ReoptWalkForwardStats` still uses the shared host reducer for exact semantic parity

### Phase 6: Composite Validation Reuse

Objective: extend the GPU validation engine to composite strategy families without duplicating validation policy.

Current status:

- backend contract is now in place for composite validation:
  - requests carry `backend`
  - results carry `backend_used`, `backend_route`, and `backend_fallback_reason`
- current composite validation is no longer purely CPU-canonical:
  - `ASR` fixed-candidate validation now has a real `GpuKernel` path with CPU-parity coverage for:
    - in-sample metrics
    - `walk_forward`
    - `CSCV/PBO`
    - `perturb`
  - `MKCS` fixed-candidate validation now has a real `GpuKernel` path with CPU-parity coverage for:
    - in-sample metrics
    - `walk_forward`
    - `CSCV/PBO`
    - `SPA`
    - `reopt_walk_forward`
    - `perturb`
    - `trade_bootstrap` with host-side bootstrap reduction
    - `cross_ticker` via command-layer repeated `GpuKernel` reruns
  - `ROCB` fixed-candidate validation now has a real `GpuKernel` path with CPU-parity coverage for:
    - in-sample metrics
    - `walk_forward`
    - `CSCV/PBO`
    - `perturb`
  - `VEMR` fixed-candidate validation now has a real `GpuKernel` path with CPU-parity coverage for:
    - in-sample metrics
    - `walk_forward`
    - `CSCV/PBO`
    - `perturb`
  - `VCC` validation now has a parity-gated mixed `GpuKernel` path
    - active when the requested CPU-canonical momentum rows remain sign-stable after `f32` roundtrip
    - explicit CPU fallback when those rows are not `fp32`-stable
    - stable `GpuKernel` requests now cover:
      - fixed-candidate in-sample metrics
      - `walk_forward`
      - `CSCV/PBO`
      - `SPA`
      - `reopt_walk_forward`
      - `trade_bootstrap` with host-side bootstrap reduction
  - root cause is now clearer:
    - the CUDA `squeeze_momentum` momentum series is not CPU-parity-safe on representative workloads
    - even when momentum is replaced with a CPU-canonical row uploaded to VRAM, some gentle monotonic series still lose trades after `f32` roundtrip because the momentum/acceleration decision boundary is numerically fragile
  - current VCC approach:
    - optimization uses a parity-safe hybrid repair by feeding CPU-canonical momentum rows into the GPU evaluator
    - validation now uses the same CPU-canonical momentum rows, but only activates `GpuKernel` when the requested rows are proven `fp32`-stable
    - `walk_forward` and `reopt_walk_forward` now use a true windowed `GpuKernel` path with canonical-start + metric-start carry-state parity on stable workloads
    - `CSCV/PBO` and `SPA` now reuse GPU-generated VCC slice stats with parity-safe desired-position semantics, while keeping the final statistical reductions host-side
    - the VCC kernel path now accepts canonical start, metric start, and end bounds; that carry-state parity is active for both windowed validation and GPU-generated slice stats
    - `cross_ticker` now rides the same backend-preserving command-layer rerun path as the other composite GPU strategies; each rerun still remains parity-gated by the VCC `fp32`-stability check
    - the checked-in prebuilt PTX now includes the VCC validation kernels so fallback non-NVCC builds keep the same VCC validation surface
  - unsupported composite validation combinations still fall back explicitly to CPU
- GPU composite validation activation is still blocked on strategy-by-strategy parity and runtime support
- shared base composite now has bounded but real `GpuKernel` execution on both stages:
  - optimization has a bounded explicit `GpuKernel` route for plain `CompositeCpu` `signal_mode=HardGate` requests on kernel-compatible full-grid searches
  - that optimization route keeps composite gate preparation host-side, uploads a device regime filter, and uses the shared double-MA `GpuKernel` scorer with a composite warmup override so pair start semantics match the CPU composite contract
  - validation now has a matching bounded fixed-candidate `GpuKernel` route for plain `CompositeCpu` `signal_mode=HardGate`
  - that validation route reuses the same uploaded device regime filter plus precomputed MA rows for:
    - in-sample
    - `walk_forward`
    - `CSCV/PBO`
    - `SPA`
    - `reopt_walk_forward`
    - `trade_bootstrap` with host-side bootstrap reduction
    - `perturb` via GPU reruns
  - base composite `cross_ticker` still falls back explicitly to CPU
- `CompositeTrend`, `RegimeHybrid`, and `BreakoutMomentum` are explicitly parity-gated:
  - their validation execution lives on the `DoubleMa` validation side, not the composite validation runtime
  - parity coverage now exists for the intended routes:
    - `CompositeTrend`
      - `ADX off` -> `GpuKernel` in-sample parity
      - `ADX off` -> `GpuKernel` `walk_forward` parity
      - `ADX on` -> `GpuHybridKernel` in-sample parity
      - `ADX on` -> `GpuHybridKernel` `walk_forward` parity
    - `BreakoutMomentum`
      - `ADX off` -> `GpuKernel` in-sample parity
      - `ADX on` -> `GpuHybridKernel` in-sample parity
      - `ADX on` -> `GpuHybridKernel` `CSCV/PBO` parity
    - `RegimeHybrid`
      - `GpuHybridKernel` in-sample parity is covered
      - `GpuHybridKernel` `walk_forward` parity is covered
      - `GpuHybridKernel` `reopt_walk_forward` parity is covered
      - `GpuHybridKernel` `CSCV/PBO` and `SPA` route smoke is covered

Nearest reusable backend pieces:

- `PreparedCudaIndicatorInputs` and CUDA-backed composite indicator prep
- compiled dynamic/composite plan generation
- the dedicated GPU optimization lanes already present for `ASR`, `VCC`, `VEMR`, and `ROCB`

Nearest blocker:

- there is still no device-side generic composite validation executor; current composite validation still returns to CPU evaluation after preparation for all unsupported profiles

Initial support matrix:

- `ROCB`
  - `GpuKernel`
    - fixed-candidate
    - in-sample metrics
    - `walk_forward`
    - `CSCV/PBO`
    - `SPA`
    - `reopt_walk_forward`
    - `perturb`
    - `trade_bootstrap` with host-side bootstrap reduction
    - `cross_ticker` via command-layer repeated `GpuKernel` reruns
  - CPU fallback
    - `GpuHybridKernel`
- `MKCS`
  - `GpuKernel`
    - fixed-candidate
    - in-sample metrics
    - `walk_forward`
    - `CSCV/PBO`
    - `SPA`
    - `reopt_walk_forward`
    - `perturb`
    - `trade_bootstrap` with host-side bootstrap reduction
    - `cross_ticker` via command-layer repeated `GpuKernel` reruns
  - CPU fallback
    - `GpuHybridKernel`
- `ASR`
  - `GpuKernel`
    - fixed-candidate
    - in-sample metrics
    - `walk_forward`
    - `CSCV/PBO`
    - `SPA`
    - `reopt_walk_forward`
    - `perturb`
    - `trade_bootstrap` with host-side bootstrap reduction
    - `cross_ticker` via command-layer repeated `GpuKernel` reruns
  - CPU fallback
    - `GpuHybridKernel`
- `VCC`
  - `GpuKernel`
    - fixed-candidate
    - in-sample metrics
    - `walk_forward`
    - `CSCV/PBO`
    - `SPA`
    - `reopt_walk_forward`
    - `trade_bootstrap` with host-side bootstrap reduction
    - `cross_ticker` via command-layer repeated backend-preserving reruns, when each rerun remains `fp32`-stable
    - only when requested CPU-canonical momentum rows are `fp32`-stable
    - `walk_forward` and `reopt_walk_forward` are now true windowed GPU executions with carry-state parity
    - `CSCV/PBO` and `SPA` now use GPU-generated slice stats with host-side final reductions
  - CPU fallback
    - fixed-candidate in-sample when momentum rows are not `fp32`-stable
    - `cross_ticker` reruns whose requested momentum rows are not `fp32`-stable
    - `GpuHybridKernel`
- `VEMR`
  - `GpuKernel`
    - fixed-candidate
    - in-sample metrics
    - `walk_forward`
    - `CSCV/PBO`
    - `SPA`
    - `reopt_walk_forward`
    - `perturb`
    - `trade_bootstrap` with host-side bootstrap reduction
    - `cross_ticker` via command-layer repeated `GpuKernel` reruns
  - CPU fallback
    - `GpuHybridKernel`
- base composite
  - optimization
    - bounded explicit `GpuKernel` support for plain `CompositeCpu` with `signal_mode=HardGate`
    - current limitations:
      - explicit `GpuKernel` request only
      - kernel-compatible MA/objective shapes only
      - no prescan-subset `allowed_pairs`
      - composite gate preparation remains host-side
  - validation
    - bounded explicit `GpuKernel` support for plain `CompositeCpu` with `signal_mode=HardGate`
    - current coverage:
      - fixed-candidate in-sample
      - `walk_forward`
      - `CSCV/PBO`
      - `SPA`
      - `reopt_walk_forward`
      - `trade_bootstrap` with host-side bootstrap reduction
      - `perturb` via GPU reruns
    - current limitations:
      - `cross_ticker` still falls back explicitly to CPU
      - composite gate preparation remains host-side
- `CompositeTrend` / `RegimeHybrid` / `BreakoutMomentum`
  - validation execution currently lives on the `DoubleMa` validation side
  - `GpuKernel` / `GpuHybridKernel` coverage follows the fixed-candidate support matrix above

Tasks:

- keep explicit composite validation route metadata honest while GPU runtime support is incomplete
- define the composite GPU validation support matrix by strategy/profile
- add parity-gated activation rules for `CompositeTrend`, `RegimeHybrid`, and `BreakoutMomentum`
- reuse the `PreparedCudaIndicatorInputs` path where already present
- unify composite GPU validation over:
  - base composite
  - ASR
  - MKCS
  - VCC
  - VEMR
  - ROCB
- define which composite variants remain CPU-first initially
- preserve local stability and validation verdict parity

Deliverables:

- GPU validation support matrix for composite strategies
- explicit fallback map for unsupported combinations
- strategy-specific parity checklist for composite activation

### Phase 7: SPA / Bootstrap Strategy

Objective: decide how much of the remaining statistical reduction should move to GPU.

Tasks:

- keep current host SPA reduction initially if GPU candidate slice stats are already available
- evaluate whether trade bootstrap should:
  - remain host-side from compact trade outputs
  - or move partially to GPU later
- evaluate whether full GPU SPA bootstrap is worth the complexity

Recommended stance:

- do not block the migration on full GPU SPA/bootstrap
- first move the repeated candidate backtest work to GPU
- keep the lightweight statistical reduction on CPU unless profiling proves otherwise

Deliverables:

- explicit CPU/GPU split for SPA and bootstrap
- decision record on whether further GPU migration is worth it

## Shared Design Rules

1. CPU path remains canonical for compatibility.
2. GPU path must reuse optimization substrate where possible.
3. Validation policy stays shared; only execution differs.
4. Fallbacks must be explicit and debuggable.
5. Device residency matters more than chasing one-kernel purity.
6. Do not regress exact brute-force optimization performance while extending validation.

## Risks

### 1. Duplicate logic drift between CPU and GPU validation

Mitigation:

- centralize verdict and policy logic
- keep CPU and GPU engines producing the same compact summary contract

### 2. VRAM lifetime / memory pressure regressions

Mitigation:

- introduce explicit GPU validation context ownership
- benchmark both default VRAM and constrained-VRAM paths

### 3. Host/device orchestration overhead erases validation gains

Mitigation:

- reuse post-optimization device context
- batch windows/slices aggressively
- use CUDA Graph replay where practical

### 4. Composite strategy support fragments into special cases

Mitigation:

- migrate `DoubleMa` first
- extend through shared evaluator adapters
- preserve a clear CPU fallback table

### 5. CPU users regress or lose capability

Mitigation:

- CPU validation remains first-class
- all new routing is additive, not replacing the baseline

## Benchmark and Verification Plan

Verification must include both correctness and performance.

### Correctness

- CPU vs GPU parity on:
  - candidate metrics
  - walk-forward summaries
  - CSCV outputs
  - run verdicts
- verify fallback behavior on unsupported GPU cases
- verify CPU-only builds still compile and run

### Performance

- measure standalone validation latency
- measure optimize -> validate contiguous pipeline latency
- measure:
  - `DoubleMa`
  - `alma_alma`
  - representative strategy overlays
  - `15m`, `1h`, and at least one larger real-data case
- compare:
  - CPU validation
  - GPU validation with standalone context creation
  - GPU validation with optimization-context reuse

### Tooling

- existing GPU perf scripts where applicable
- dedicated validation perf harness
- Nsight Systems for end-to-end orchestration
- Nsight Compute only on the actual hot validation kernels

## Tracking Checklist

### Phase 0: Routing and contracts

- [x] Add validation backend-route metadata.
- [x] Add GPU validation eligibility resolver.
- [x] Add explicit GPU-fallback reason fields.
- [x] Add backend field to `DoubleMaValidationRequest`.
- [x] Add `backend_used` / `backend_route` / `backend_fallback_reason` to `DoubleMaValidationResult`.
- [x] Propagate validation backend through Tauri cross-ticker reruns.
- [x] Propagate validation backend from frontend request builder.
- [x] Surface validation backend route/fallback in frontend validation summary.
- [x] Define shared candidate summary contract for CPU/GPU validation engines.

### Phase 1: GPU validation context

- [x] Introduce reusable `GpuValidationContext` or equivalent.
- [x] Support standalone validation context creation.
- [x] Resolve optimize -> validate VRAM reuse path decision in favor of standalone per-stage GPU jobs.
- [x] Keep CPU-only builds compiling cleanly.

### Phase 2: GPU fixed-candidate validation

- [x] Implement initial GPU fixed-candidate validation for `DoubleMa`.
- [x] Return compact candidate metrics required by current validation policy.
- [x] Extend GPU fixed-candidate validation beyond the current narrow support matrix.
- [x] Add CPU vs GPU parity tests for `DoubleMa` validation.
- [x] Add route-metadata tests proving GPU validation activation.

### Phase 3: GPU walk-forward

- [x] Implement GPU walk-forward window evaluation for the fixed-candidate `GpuKernel` path.
- [x] Emit current `WalkForwardStats` schema from GPU-generated results.
- [x] Add CPU vs GPU parity tests for walk-forward summaries.

### Phase 4: GPU CSCV / PBO prep

- [x] Implement GPU slice-return generation.
- [x] Keep current CSCV/PBO reduction semantics.
- [x] Add parity tests for candidate CSCV stats.
- [x] Add parity tests for run-level PBO.

### Phase 5: GPU re-opt walk-forward

- [x] Implement GPU finalist re-opt walk-forward for `DoubleMa`.
- [x] Reuse device-resident optimization context where possible.
- [x] Emit current `ReoptWalkForwardStats` schema.
- [x] Add bounded runtime and parity tests.
- [x] Implement GPU perturb reruns for the fixed-candidate `DoubleMa` path.
- [x] Add CPU vs GPU parity tests for `DoubleMa` perturb summaries.

### Phase 6: Composite validation extension

- [x] Add backend/fallback contract for composite validation results.
- [x] Keep composite validation CPU-canonical with explicit GPU fallback metadata until runtime support exists.
- [x] Define initial composite GPU validation support matrix.
- [x] Add activation gates tied to parity evidence for `CompositeTrend`, `RegimeHybrid`, and `BreakoutMomentum`.
- [x] Add CPU vs `GpuKernel` validation parity tests for `CompositeTrend` with ADX off.
- [x] Add CPU vs `GpuHybridKernel` validation parity tests for `CompositeTrend` with ADX on.
- [x] Add CPU vs `GpuKernel` validation parity tests for `BreakoutMomentum` with ADX off.
- [x] Add CPU vs `GpuHybridKernel` validation parity tests for `BreakoutMomentum` with ADX on.
- [x] Add CPU vs `GpuHybridKernel` validation parity tests for `RegimeHybrid`.
- [x] Extend `GpuHybridKernel` fixed-candidate validation to `walk_forward` on kernel-executable routes.
- [x] Extend `GpuHybridKernel` fixed-candidate validation to `CSCV/PBO` on kernel-executable routes.
- [x] Extend `GpuHybridKernel` fixed-candidate validation to `SPA` on kernel-executable routes with host-side final reduction.
- [x] Extend `GpuHybridKernel` fixed-candidate validation to `reopt_walk_forward` on kernel-executable routes.
- [x] Implement GPU-assisted base composite validation preparation where practical while keeping CPU-canonical evaluation.
- [x] Extend base composite fixed-candidate `GpuKernel` validation to in-sample metrics.
- [x] Extend base composite fixed-candidate `GpuKernel` validation to `walk_forward`.
- [x] Extend base composite fixed-candidate `GpuKernel` validation to `CSCV/PBO`.
- [x] Extend base composite fixed-candidate `GpuKernel` validation to `SPA`.
- [x] Extend base composite fixed-candidate `GpuKernel` validation to `reopt_walk_forward`.
- [x] Extend base composite fixed-candidate `GpuKernel` validation to `trade_bootstrap` with host-side bootstrap reduction.
- [x] Extend base composite fixed-candidate `GpuKernel` validation to `perturb` via GPU reruns.
- [x] Extend GPU validation to ASR fixed-candidate in-sample metrics.
- [x] Extend GPU validation to MKCS fixed-candidate in-sample metrics.
- [x] Extend GPU validation to MKCS fixed-candidate `walk_forward`.
- [x] Extend GPU validation to MKCS fixed-candidate `CSCV/PBO`.
- [x] Extend GPU validation to MKCS fixed-candidate `SPA`.
- [x] Extend GPU validation to MKCS fixed-candidate `reopt_walk_forward`.
- [x] Extend GPU validation to MKCS fixed-candidate `trade_bootstrap` with host-side reduction.
- [x] Preserve the requested backend for MKCS `cross_ticker` reruns instead of forcing `CpuOnly`.
- [x] Extend GPU validation to MKCS fixed-candidate `perturb`.
- [x] Extend GPU validation to ASR fixed-candidate `walk_forward`.
- [x] Extend GPU validation to ASR fixed-candidate `CSCV/PBO`.
- [x] Extend GPU validation to ASR fixed-candidate `SPA`.
- [x] Extend GPU validation to ASR fixed-candidate `reopt_walk_forward`.
- [x] Extend GPU validation to ASR fixed-candidate `trade_bootstrap` with host-side reduction.
- [x] Extend GPU validation to ASR fixed-candidate `perturb`.
- [x] Investigate VCC CPU/CUDA parity deeply enough to identify the real blocker.
- [x] Implement a parity-safe VCC validation route:
  - current route uses CPU-canonical momentum rows plus an `fp32`-stability activation gate
  - unstable momentum rows still fall back explicitly to CPU
- [x] Extend GPU validation to VCC fixed-candidate `walk_forward` on stable `GpuKernel` routes with canonical-start + metric-start carry-state parity.
- [x] Extend GPU validation to VCC fixed-candidate `CSCV/PBO` on stable `GpuKernel` routes with GPU-generated slice stats and host-side reduction parity.
- [x] Extend GPU validation to VCC fixed-candidate `SPA` on stable `GpuKernel` routes with GPU-generated slice stats and host-side reduction.
- [x] Extend GPU validation to VCC fixed-candidate `reopt_walk_forward` on stable `GpuKernel` routes with canonical-start + metric-start carry-state parity.
- [x] Extend GPU validation to VCC fixed-candidate `trade_bootstrap` with host-side bootstrap reduction on stable `GpuKernel` routes.
- [x] Validate VCC `cross_ticker` on the backend-preserving command-layer rerun path, with per-rerun `fp32`-stability gating unchanged.
- [x] Refresh the checked-in VCC-capable prebuilt PTX so fallback builds keep the same validation kernel surface.
- [x] Extend GPU validation to VEMR fixed-candidate in-sample metrics.
- [x] Extend GPU validation to VEMR fixed-candidate `CSCV/PBO`.
- [x] Extend GPU validation to VEMR fixed-candidate `SPA`.
- [x] Extend GPU validation to VEMR fixed-candidate `reopt_walk_forward`.
- [x] Extend GPU validation to VEMR fixed-candidate `trade_bootstrap` with host-side reduction.
- [x] Extend GPU validation to VEMR fixed-candidate `perturb`.
- [x] Extend GPU validation to ROCB fixed-candidate in-sample metrics.
- [x] Extend GPU validation to ROCB fixed-candidate `walk_forward`.
- [x] Extend GPU validation to ROCB fixed-candidate `CSCV/PBO`.
- [x] Extend GPU validation to ROCB fixed-candidate `SPA`.
- [x] Extend GPU validation to ROCB fixed-candidate `reopt_walk_forward`.
- [x] Extend GPU validation to ROCB fixed-candidate `trade_bootstrap` with host-side reduction.
- [x] Extend GPU validation to ROCB fixed-candidate `perturb`.
- [x] Extend GPU validation to VEMR fixed-candidate `walk_forward`.
- [x] Preserve the requested backend for composite `cross_ticker` reruns instead of forcing `CpuOnly`.
- [x] Keep explicit CPU fallbacks for unsupported combinations.

Current practical base-composite note:

- full device-side generic base-composite execution is still not finished; the current route is intentionally bounded to plain `CompositeCpu` `signal_mode=HardGate`
- the practical base-composite optimization slices now implemented are:
  - GPU-assisted indicator preparation:
    - `halftrend`
    - `macz`
    - `ott`
  - backend-aware fast/slow MA generation in the shared CPU-canonical optimization/validation builders
  - a bounded explicit `GpuKernel` optimization route for plain `CompositeCpu` `signal_mode=HardGate`, using:
    - CPU-canonical composite gate construction
    - uploaded device regime filters
    - precomputed MA rows on the shared double-MA kernel path
    - a composite warmup override so the GPU grid uses the same pair start semantics as the CPU composite evaluator
- the practical base-composite validation slices now implemented reuse that same substrate for:
  - fixed-candidate in-sample metrics
  - `walk_forward`
  - `CSCV/PBO`
  - `SPA`
  - `reopt_walk_forward`
  - `trade_bootstrap` with host-side bootstrap reduction
  - `perturb` via GPU reruns
- current validation limitations for base composite:
  - `cross_ticker` still falls back explicitly to CPU
  - composite gate construction still starts from CPU-canonical indicator rows before the device reruns

### Phase 7: Statistical reduction split

- [x] Decide whether SPA stays host-side initially.
- [x] Decide whether trade bootstrap stays host-side initially.
- [x] Document the CPU/GPU split for final statistical reductions.

### Verification

- [x] Add end-to-end optimize -> validate GPU path regression coverage.
- [x] Add CPU-only regression coverage for validation.
- [x] Add VRAM-constrained validation benchmark coverage.
- [x] Run `cargo test --workspace`.
- [x] Run `npm --prefix ./web run typecheck`.
- [x] Run `npm --prefix ./web run test`.
- [x] Run `cargo test -p ta_desktop_demo_app commands_validation -- --nocapture` with `CARGO_INCREMENTAL=0` to avoid nightly ICE during targeted validation runs.
- [x] Run relevant GPU perf validation harnesses.

Latest end-to-end validation artifact note:

- headless optimize -> validate regression now preserves the requested validation backend in
  `src-tauri/src/bin/ta_headless_validate.rs`
- real-data AAPL run with:
  - optimization: `GpuKernel`
  - validation: `GpuKernel`
  - enabled features: `walk_forward`, `CSCV/PBO`
- produced:
  - CPU validation runtime: about `511 ms`
  - GPU validation runtime: about `8947 ms`
  - route metadata: `backend_used=Gpu`, `backend_route=GpuKernel`
  - parity remained tight on the sampled outputs
- current interpretation:
  - the GPU validation path is functionally active and numerically aligned
  - but for a tiny finalist set (`8` candidates, `9` optimization combos) on this workflow, GPU validation is slower than CPU due to orchestration / repeated window launches

Latest validation benchmark note:

- `cargo bench -p ta_app_core --bench composite_validation_workflow --features cuda,cuda-backtest-kernel -- "composite_validate/gpu_kernel/asr/100k/64candidates/in_sample_192candidates" --sample-size 10 --measurement-time 2 --warm-up-time 1`
- benchmark output now records route metadata:
  - `backend_used=Cpu`, `backend_route=CpuOnly` for the CPU workflow group
  - `backend_used=Gpu`, `backend_route=GpuKernel` for the GPU ASR workflow group
- measured GPU validation timing for the VRAM-heavier ASR case:
  - `composite_validate/gpu_kernel/asr/100k/64candidates/in_sample_192candidates`
  - about `339.77 ms / 411.63 ms / 482.29 ms`
  - this does not invalidate the GPU path; it means the next benchmark/profiling focus should be larger finalist sets and/or more multiplicative validation workloads

## Practical Recommendation

Start with `DoubleMa` only.

That is the cleanest path because:

- it already has the strongest GPU optimization substrate
- it is the most obvious current CPU bottleneck in validation
- it will prove the architecture before composite strategy expansion

Recommended execution order:

1. Phase 0
2. Phase 1
3. Phase 2
4. Phase 3
5. Phase 4
6. benchmark and profile
7. Phase 5
8. only then extend to composite strategies

That keeps the migration incremental, measurable, and reversible if any phase underdelivers.

## Higher-Priority Implementation Track: GPU Execution for `CompositeTrend`, `RegimeHybrid`, and `BreakoutMomentum`

This track has higher priority than the GPU validation migration above.

Reason:

- validation should reuse the real production GPU execution path
- these three strategy families are currently a weak point in the product GPU capability map
- forcing GPU validation first would risk building a validation runtime before the corrected production optimization route and capability map were pinned down

So the dependency order is:

1. make these strategy families execute on GPU end to end when their effective indicator set is GPU-capable
2. make them fall back to hybrid GPU execution when some required indicator remains CPU-only
3. only then extend validation to reuse the resulting execution substrate

### Objective

Formalize `CompositeTrend`, `RegimeHybrid`, and `BreakoutMomentum` as `double_ma`-family GPU-capable strategies with an explicit dual-engine model:

- full `GpuKernel` / VRAM-resident path when every required effective indicator is GPU-capable and GPU-preferred
- `GpuHybridKernel` path when some required indicator remains CPU-only or CPU-preferred
- `CpuOnly` fallback when GPU execution is unavailable or unsupported

The required product behavior is:

- the same request shape remains valid
- the same result schema remains valid
- the same strategy semantics remain valid
- only the execution route changes

Important scope clarification:

- the goal is per-stage end-to-end GPU execution, not one continuous device pipeline spanning optimization into validation
- optimization may upload once, run fully GPU/VRAM-resident within that stage, and return compact results
- validation may independently upload once, rerun optimization-like work on GPU within that stage, and return compact validation results
- chaining optimization device state directly into validation is explicitly not required

### Current Ground Truth

These three strategies now route through:

- `web/src/services/optimizeService.ts` -> `run_double_ma_optimization`

and their request shaping now selects:

- `GpuKernel` when the effective strategy config is ADX-free and CUDA is available
- `GpuHybridKernel` when ADX is enabled or structurally required
- `CpuOnly` fallback when GPU cannot be used

The relevant runtime already exists in:

- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/strategy_presets.rs`

### Effective Indicator Sets for the Three Target Profiles

Shared base:

- `fast_ma`
- `slow_ma`

Additional profile-specific components:

- `CompositeTrend`
  - `linearreg_angle`
  - `bollinger_bands_width`
  - optional `roc`
  - optional `adx`
- `RegimeHybrid`
  - `linearreg_angle`
  - `bollinger_bands_width`
  - optional `roc`
  - `adx`
  - regime-switch / reversion thresholds
- `BreakoutMomentum`
  - `bollinger_bands`
  - `macd`
  - `cmo`
  - `adosc`
  - optional `adx`

`gpu_speedups.csv` overlap already confirmed:

- trend-quality-related:
  - `linearreg_angle`
  - `bollinger_bands_width`
  - `roc`
- breakout-related:
  - `bollinger_bands`
  - `macd`
  - `cmo`
  - `adosc`
- related directional entries:
  - `di`
  - `adxr`

Important nuance:

- exact `adx` is not currently represented as its own row in `gpu_speedups.csv`
- so “all indicators are in the speedup table” must be interpreted by effective backend capability, not by a naive string list alone

### Execution Policy to Implement

The execution decision for these three profiles should be formalized as:

#### Full GPU path

Use full `GpuKernel` only when all of the following are true:

- both MA sides are GPU-kernel-capable
- every effective overlay/regime component required by the active strategy config is GPU-capable
- every such component is allowed to stay device-resident through the optimization loop
- no unsupported execution overlays force CPU evaluation

Target policy by profile:

- `CompositeTrend`
  - `trend_use_adx = false` -> `GpuKernel`
  - `trend_use_adx = true` -> not full GPU
- `BreakoutMomentum`
  - `breakout_use_adx = false` -> `GpuKernel`
  - `breakout_use_adx = true` -> not full GPU
- `RegimeHybrid`
  - current strategy definition still carries ADX-style regime logic
  - treat it as non-full-GPU until the effective no-ADX variant is proven and implemented

#### Hybrid GPU path

Use `GpuHybridKernel` when:

- the main backtest loop can still run on GPU
- but one or more required components must be staged or computed on CPU

Typical examples:

- exact missing GPU support for one overlay indicator
- CPU-preferred indicator policy from `gpu_speedups.csv`
- a strategy feature that has GPU staging but not full device-resident integration

Target policy by profile:

- `CompositeTrend`
  - `trend_use_adx = true` -> `GpuHybridKernel`
- `BreakoutMomentum`
  - `breakout_use_adx = true` -> `GpuHybridKernel`
- `RegimeHybrid`
  - current ADX-bearing form -> `GpuHybridKernel`

Important implementation note:

- keep the existing hybrid backend semantics in place
- hybrid should continue to mean CPU staging for the slow pieces and GPU execution for the heavy optimization/backtest path whenever the kernel route remains viable
- do not redesign hybrid around a CPU backtest loop unless later profiling proves that is necessary

#### CPU path

Use `CpuOnly` when:

- CUDA is unavailable
- GPU execution is unsupported for the selected combination
- hybrid mode is also not supportable for the selected combination

For these three profiles, `CpuOnly` should be treated as:

- explicit user choice
- CPU-only build or no CUDA device/runtime
- a defensive fallback only after route resolution proves neither `GpuKernel` nor `GpuHybridKernel` can be honored

### Architectural Direction

Do not force these three profiles back through one generic composite execution layer.

Instead:

- prefer strategy-local backend/frontend organization so one strategy can be changed without reading the whole repo
- keep these three on the `double_ma` runtime unless a real semantic mismatch is proven
- allow strategy-specific GPU/hybrid helpers where that is the shortest path to correct end-to-end GPU execution
- keep only truly cross-cutting primitives shared
- preserve the existing hybrid semantics: CPU stages slow pieces, GPU still handles the heavy optimization/backtest work whenever viable

Recommended code organization target:

- backend strategy-local code should own:
  - request/config normalization
  - route/capability detection
  - CPU execution path
  - GPU execution path
  - hybrid execution path
  - validation adapter
  - tests
- frontend strategy-local code should own:
  - request builder/normalization
  - capability summaries
  - strategy-specific form/config handling
  - tests
- shared infrastructure should remain shared only for:
  - CUDA upload/buffers
  - top-k/reduction
  - common metrics/stat helpers
  - artifact/report plumbing

The intended optimization-stage target pipeline is:

1. upload candles / source data once
2. prepare device-resident shared inputs
3. prepare any CPU-only hybrid side inputs once, only when the effective config requires them
4. run the optimization loop on GPU
5. keep top-k / shortlist reduction on device where practical
6. return only compact results and route metadata

The intended validation-stage target pipeline is:

1. upload candles / source data once for the validation run
2. prepare device-resident shared inputs for that validation run
3. rerun optimization-like candidate evaluation on GPU across windows/slices as needed
4. reduce validation statistics and survivors to compact outputs
5. return only compact validation results and verdict metadata

### ADX-Gated Target State

This is the concrete target behavior for the three profiles.

#### CompositeTrend

- `ADX off` -> `GpuKernel`
- `ADX on` -> `GpuHybridKernel`
- no CUDA or explicit CPU selection -> `CpuOnly`

#### RegimeHybrid

Current target:

- current ADX-bearing form -> `GpuHybridKernel`
- no CUDA or explicit CPU selection -> `CpuOnly`

Future optional target, only if the strategy is given a true no-ADX effective form:

- `ADX off` -> `GpuKernel`
- `ADX on` -> `GpuHybridKernel`

#### BreakoutMomentum

- `ADX off` -> `GpuKernel`
- `ADX on` -> `GpuHybridKernel`
- no CUDA or explicit CPU selection -> `CpuOnly`

### Implementation Phases

### Phase A: capability truth table

Establish a single backend truth table for these three profiles, including ADX-gated routing.

Deliverables:

- explicit effective-indicator inventory per profile
- explicit routeability classification per component:
  - `GpuKernel`
  - `GpuHybridKernel`
  - `CpuOnly`
- explicit unsupported-combination list
- explicit `ADX on` vs `ADX off` route table per profile

This phase should end the current ambiguity where a strategy may use GPU-preferred ingredients but still not have a product GPU route.

### Phase B0: strategy-local layout

Create a clear home for backend and frontend strategy code before deeper runtime work.

Deliverables:

- define the target backend file/module location for:
  - `CompositeTrend`
  - `RegimeHybrid`
  - `BreakoutMomentum`
- define the target frontend file/module location for the same three strategies
- minimize cross-repo hunting for request shaping, capability logic, and execution entrypoints
- document any shared primitives that remain intentionally shared

Acceptance target:

- a contributor changing one of these strategies should primarily read one localized backend area and one localized frontend area
- later GPU validation work can reuse the same strategy-local boundaries

### Phase B: request routing and route metadata

Add product-grade route resolution for these profiles.

Deliverables:

- resolver for `CompositeTrend`
- resolver for `RegimeHybrid`
- resolver for `BreakoutMomentum`
- ADX-aware route selection policy
- route metadata in results:
  - backend used
  - backend route
  - fallback reason
  - effective indicator route plan

The UX should become as debuggable as current `DoubleMa` routing.

### Phase C: full GPU execution path

Use the existing `double_ma` GPU kernel runtime for the subset of these profiles/configs whose effective indicator set is already GPU-capable.

Deliverables:

- keep trend-quality / breakout regime preparation on the `double_ma` path
- route `CompositeTrend` to `GpuKernel` when ADX is off
- route `BreakoutMomentum` to `GpuKernel` when ADX is off
- keep compact result reduction on the existing `double_ma` result path
- preserve `CpuOnly` fallback semantics

Acceptance target:

- no CPU reconstruction inside the optimization inner loop
- no host round-trip per tile

### Phase D: hybrid GPU execution path

Use the existing `double_ma` hybrid route for mixed-capability cases.

Deliverables:

- one-time CPU staging for ADX and any other CPU-preferred components already handled by the hybrid substrate
- stable hybrid input contract into the `double_ma` optimization loop
- no accidental downgrade to pure CPU when hybrid should be possible
- explicit `ADX on` hybrid support for `CompositeTrend`
- explicit `ADX on` hybrid support for `BreakoutMomentum`
- explicit hybrid support for `RegimeHybrid` while ADX remains part of the effective strategy definition

Acceptance target:

- hybrid is a first-class supported route, not an incidental fallback

### Phase E: parity and regression

Lock down correctness against existing CPU behavior.

Deliverables:

- CPU vs `GpuKernel` parity tests
- CPU vs `GpuHybridKernel` parity tests
- route-selection tests
- unsupported-combination tests

### Phase F: benchmarks and route evidence

Prove that the new routes are worth keeping.

Deliverables:

- representative benches for all three profiles
- route metadata captured in benchmark output
- real-data runs showing selected backend route
- Nsight evidence for full GPU and hybrid cases

### Acceptance Criteria

1. `CompositeTrend`, `RegimeHybrid`, and `BreakoutMomentum` can all resolve to a non-CPU route when their effective indicator set permits it.
2. `CompositeTrend` resolves to `GpuKernel` when ADX is inactive.
3. `BreakoutMomentum` resolves to `GpuKernel` when ADX is inactive.
4. `CompositeTrend` resolves to `GpuHybridKernel` when ADX is active.
5. `BreakoutMomentum` resolves to `GpuHybridKernel` when ADX is active.
6. `RegimeHybrid` resolves to `GpuHybridKernel` in its current ADX-bearing form unless and until a true no-ADX effective route is implemented.
7. CPU-only users and CPU-only builds remain fully supported.
8. Result schema remains stable.
9. Backend route metadata is exposed to frontend and artifacts.
10. CPU parity remains bounded and test-covered.
11. Validation migration can later consume the same route metadata and execution substrate.
12. Strategy-specific backend/frontend code becomes easier to locate and modify without repo-wide exploration.

### Risks

1. The effective indicator set is more behavioral than structural.
- Example: some trend/regime behavior is encoded through `StrategyConfig` logic rather than explicit dynamic indicator nodes.
- Mitigation: capability resolution must inspect effective strategy behavior, not only IR node labels.

2. `gpu_speedups.csv` is not enough by itself.
- A positive speedup-table entry does not automatically mean full product-level GPU integration exists.
- Mitigation: use speedup-table membership as one signal, but gate final routing on real backend capability.

3. Hybrid can silently collapse into CPU if not formalized.
- Mitigation: make hybrid a deliberate route with explicit result metadata and tests.

4. Older planning docs may overstate current support.
- Mitigation: treat this section as the authoritative plan for these three strategy families.

5. A generic composite runtime may continue to slow implementation.
- Mitigation: allow strategy-specific GPU/hybrid kernels and backend modules for these three profiles instead of blocking on a universal composite executor.

## Tracking Checklist for the Higher-Priority Strategy GPU Track

### Current progress snapshot

- Confirmed the current product route is the `double_ma` optimizer, not the generic composite CPU optimizer:
  - `CompositeTrend` now routes through `run_double_ma_optimization`
  - `BreakoutMomentum` now routes through `run_double_ma_optimization`
  - `RegimeHybrid` now routes through `run_double_ma_optimization`
- Confirmed the effective strategy semantics on the product route are `StrategyConfig` overlays already supported by:
  - `crates/app_core/src/double_ma.rs`
  - `crates/app_core/src/strategy_presets.rs`
- Confirmed the current route policy is:
  - `CompositeTrend`: `GpuKernel` with ADX off, `GpuHybridKernel` with ADX on
  - `BreakoutMomentum`: `GpuKernel` with ADX off, `GpuHybridKernel` with ADX on
  - `RegimeHybrid`: `GpuHybridKernel` in its current ADX-bearing form
- Verified that runtime optimization results already carry:
  - `backend_used`
  - `backend_route`
  - `backend_fallback_reason`
  - `indicator_route_plan`
- Verified that the frontend already summarizes that route metadata in optimize status handling.
- Clarified architecture target:
  - optimization and validation should each be GPU-executed end to end within their own stage
  - direct chaining of optimization device state into validation is not required
  - strategy-local code organization is preferred over forcing all three profiles through one generic composite runtime
- Added explicit route-policy handling in the product request/command path:
  - frontend request shaping selects `GpuKernel` vs `GpuHybridKernel` vs `CpuOnly`
  - frontend command selection sends all three profiles through `run_double_ma_optimization`
  - runtime route metadata remains surfaced through existing optimize results
- Verified with:
  - `npm --prefix ./web run test -- optimizeRequestBuilder.test.ts optimizeService.test.ts`
  - `npm --prefix ./web run typecheck`
  - `npm --prefix ./web run lint`
- Verified with:
  - `cargo test -p ta_app_core gpu_kernel_supports_double_like_regime_strategies --features cuda,cuda-backtest-kernel -- --nocapture`
  - `cargo test -p ta_app_core cpu_regime_hybrid_small_grid_runs -- --nocapture`
  - `cargo test -p ta_app_core cpu_breakout_momentum_small_grid_runs -- --nocapture`
  - `cargo build -p ta_app_core --release --bin profile_gpu_dynamic_runner --features cuda,cuda-backtest-kernel`
  - live route captures from `target/release/profile_gpu_dynamic_runner.exe` for:
    - `trend_quality --adx-mode off`
    - `trend_quality --adx-mode on`
    - `breakout_momentum --adx-mode off`
    - `breakout_momentum --adx-mode on`
    - `regime_hybrid`
  - Nsight Compute reports:
    - `target/nsight/composite_trend_adx_off.ncu-rep`
    - `target/nsight/composite_trend_adx_on.ncu-rep`
- Current live-run finding:
  - all captured cases stayed on GPU (`backend=Gpu`) with no fallback
  - successful explicit runs now return populated route metadata on the success path:
    - `CompositeTrend` ADX off -> `backend_route=GpuKernel`
    - `CompositeTrend` ADX on -> `backend_route=GpuHybridKernel`
    - `RegimeHybrid` -> `backend_route=GpuHybridKernel`
  - the remaining follow-up is app-level artifact coverage for those same returned route notes

### Phase A: capability map

- [x] Enumerate the exact effective indicator set for `CompositeTrend`.
- [x] Enumerate the exact effective indicator set for `RegimeHybrid`.
- [x] Enumerate the exact effective indicator set for `BreakoutMomentum`.
- [x] Mark each required component as `GpuKernel`, `GpuHybridKernel`, or `CpuOnly`.
- [x] Document exact blockers preventing full GPU execution for each profile.
- [x] Document the exact route when ADX is enabled vs disabled for each profile.
- [x] Confirm that the effective product/backend semantics for these three profiles live on the `double_ma` route.
- [x] Confirm that the current GPU execution goal should build on the existing `double_ma` runtime rather than a new composite runtime.

### Phase B0: strategy-local layout

- [x] Define the backend home for `CompositeTrend`.
- [x] Define the backend home for `RegimeHybrid`.
- [x] Define the backend home for `BreakoutMomentum`.
- [x] Define the frontend home for `CompositeTrend`.
- [x] Define the frontend home for `RegimeHybrid`.
- [x] Define the frontend home for `BreakoutMomentum`.
- [x] Document which shared primitives remain intentionally shared.
- [x] Ensure a contributor can locate request shaping, capability logic, execution entrypoints, and tests for each profile without repo-wide search.

### Phase B: routing contracts

- [x] Add a backend-aware execution seam to the generic composite optimization path.
- [x] Preserve CPU fallback behavior through the existing `double_ma` backend routing.
- [x] Add result metadata for generic composite `backend_route` / `backend_fallback_reason`.
- [x] Add true non-CPU route resolution for the three target profiles.
- [x] Make `CompositeTrend` resolve to a true `GpuKernel` engine when ADX is off.
- [x] Make `CompositeTrend` resolve to a true `GpuHybridKernel` engine when ADX is on.
- [x] Make `BreakoutMomentum` resolve to a true `GpuKernel` engine when ADX is off.
- [x] Make `BreakoutMomentum` resolve to a true `GpuHybridKernel` engine when ADX is on.
- [x] Make `RegimeHybrid` resolve to a true `GpuHybridKernel` engine while ADX remains required.
- [x] Add result metadata for `backend_used`.
- [x] Add result metadata for `backend_route`.
- [x] Add result metadata for `backend_fallback_reason`.
- [x] Add result metadata for effective indicator route plan.
- [x] Surface the routing choice through frontend request building and optimization command selection.
- [x] Surface runtime route metadata through Tauri and frontend results.

### Phase C: full GPU path

- [x] Reuse the existing `double_ma` GPU runtime for full-GPU cases instead of building a new composite runtime.
- [x] Make `CompositeTrend` full-GPU eligible when ADX is off.
- [x] Make `BreakoutMomentum` full-GPU eligible when ADX is off.
- [x] Keep `RegimeHybrid` on the existing non-full-GPU path in its current ADX-bearing form.
- [x] Keep top-k reduction device-heavy where practical by reusing the `double_ma` result path.

### Phase D: hybrid path

- [x] Define explicit hybrid input contract for non-GPU-capable components by reusing the existing `double_ma` hybrid backend.
- [x] Stage ADX on CPU for `CompositeTrend` when enabled.
- [x] Stage ADX on CPU for `BreakoutMomentum` when enabled.
- [x] Stage ADX on CPU for `RegimeHybrid` while preserving GPU optimization/backtest execution.
- [x] Make `CompositeTrend` hybrid-eligible when ADX is on.
- [x] Make `RegimeHybrid` hybrid-eligible for its current ADX-bearing form.
- [x] Make `BreakoutMomentum` hybrid-eligible when ADX is on.
- [x] Prevent accidental downgrade from hybrid to CPU without explicit reason.

### Phase E: correctness and regression

- [x] Add routed-path smoke tests for `CompositeTrend` on a GPU request path.
- [x] Add routed-path smoke tests for `BreakoutMomentum` on a hybrid request path.
- [x] Add routed-path smoke tests for `RegimeHybrid` on a hybrid request path.
- [x] Add CPU vs `GpuKernel` parity tests for `CompositeTrend`.
- [x] Add CPU vs `GpuHybridKernel` parity tests for `RegimeHybrid`.
- [x] Add CPU vs `GpuKernel` parity tests for `BreakoutMomentum`.
- [x] Add CPU vs `GpuHybridKernel` parity tests for mixed-capability cases.
- [x] Add route-selection tests for full GPU and hybrid on the app/request path.
- [x] Add explicit route-selection tests for CPU fallback on these three profiles.
- [x] Add a targeted app-level integration check that records returned backend route metadata for these three profiles.
- [x] Run `cargo test --workspace`.

### Phase F: benchmarks and profiling

- [x] Add representative benchmarks for all three profiles.
- [x] Capture backend route metadata in benchmark outputs.
- [x] Run Nsight Systems on `CompositeTrend` with ADX off.
- [x] Run Nsight Systems on `CompositeTrend` with ADX on.
- [x] Run Nsight Systems on `BreakoutMomentum` with ADX off.
- [x] Run Nsight Systems on `BreakoutMomentum` with ADX on.
- [x] Run Nsight Systems on `RegimeHybrid` in its current ADX-bearing form.
- [x] Capture representative Nsight Compute reports for `CompositeTrend` with ADX off/on.
- [x] Compare CPU vs hybrid vs full GPU runtime on at least one real dataset.

Latest app-level route/runtime artifact:

- `cargo run -p ta_desktop_demo_app --release --features cuda,cuda-backtest-kernel --bin ta_profile_route_matrix -- --csv data/benchmarks/AAPL.csv --timeframe 15m --out target/profile_route_matrix_aapl_15m.json`
- artifact:
  - `target/profile_route_matrix_aapl_15m.json`
- route/runtime highlights on AAPL `15m`:
  - `CompositeTrend` ADX off:
    - `CpuOnly`: about `325 ms`
    - `GpuKernel`: about `914 ms`
  - `CompositeTrend` ADX on:
    - `CpuOnly`: about `326 ms`
    - `GpuHybridKernel`: about `83 ms`
  - `BreakoutMomentum` ADX off:
    - `CpuOnly`: about `530 ms`
    - `GpuKernel`: about `105 ms`
  - `BreakoutMomentum` ADX on:
    - `CpuOnly`: about `460 ms`
    - `GpuHybridKernel`: about `102 ms`
  - `RegimeHybrid`:
    - `CpuOnly`: about `816 ms`
    - `GpuHybridKernel`: about `101 ms`
- current interpretation:
  - the app-level route metadata is now recorded on a real dataset for all three profiles
  - full GPU is not automatically faster for every profile; `CompositeTrend` ADX-off on this dataset is slower on `GpuKernel`
  - hybrid is clearly worthwhile for the ADX-bearing cases on this dataset

Nsight Systems note:

- added repo helper:
  - `scripts/profile_gpu_backtest_nsys.ps1`
- local repair result on this workstation:
  - the newer `2025.x` installs time out after `75 sec` during agent/session startup
  - the older `2022.4.2` install works and is now preferred by the repo helper
  - successful profile artifacts now exist for:
    - `target/nsight/composite_trend_adx_off_nsys_2022.nsys-rep`
    - `target/nsight/composite_trend_adx_on_nsys_2022.nsys-rep`
    - `target/nsight/breakout_momentum_adx_off_nsys_2022.nsys-rep`
    - `target/nsight/breakout_momentum_adx_on_nsys_2022.nsys-rep`
    - `target/nsight/regime_hybrid_nsys_2022.nsys-rep`

## Priority Note

Until this higher-priority track is substantially complete, GPU validation work should be considered downstream.

Practical ordering from here:

1. Phase A
2. Phase B0
3. Phase B
4. Phase C
5. Phase D
6. Phase E
7. Phase F
8. resume GPU validation migration on top of the resulting execution routes

Latest profile benchmark note:

- `cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- double_ma/gpu_kernel_strategy/sma_sma/200k/trend_quality/large --sample-size 10 --measurement-time 2 --warm-up-time 1`
  - route probe: `backend_used=Gpu`, `backend_route=GpuKernel`
  - timing: about `113.10 ms / 113.85 ms / 114.64 ms`
- `cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- double_ma/gpu_kernel_strategy/sma_sma/200k/regime_hybrid/large --sample-size 10 --measurement-time 2 --warm-up-time 1`
  - route probe: `backend_used=Gpu`, `backend_route=GpuKernel`
  - timing: about `106.75 ms / 109.12 ms / 111.55 ms`
- `cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- double_ma/gpu_kernel_strategy/sma_sma/200k/breakout_momentum/large --sample-size 10 --measurement-time 2 --warm-up-time 1`
  - route probe: `backend_used=Gpu`, `backend_route=GpuKernel`
  - timing: about `118.33 ms / 119.56 ms / 120.89 ms`

## Additional Implementation Track: New GPU-Native Strategies From Unused 5x+ Indicators

### Priority and Intent

After the `CompositeTrend` / `RegimeHybrid` / `BreakoutMomentum` routing work, the next strategic expansion should be:

1. use the unused `gpu_speedups.csv` indicators with `>= 5x` measured CUDA speedup
2. create new strategies that are designed from the start for:
   - full `GpuKernel` execution
   - device-resident optimization
   - `CpuOnly` fallback
3. avoid inventing strategies around indicators that are either:
   - utility transforms rather than real signal primitives
   - redundant with better indicators already chosen
   - too opaque or too integration-heavy for the first wave

This track should prefer the same implementation shape as the current custom GPU-native strategies:

- dedicated request/config structs
- explicit signal builders
- dedicated GPU signal staging
- dedicated GPU kernel combo evaluation
- clean CPU fallback

In other words:

- model these more like `ASR`, `VCC`, `VEMR`, and `ROCB`
- do **not** first route them through the generic CPU composite path

### Source Inventory

The source list comes from:

- [STRATEGY_INDICATOR_GPU_COVERAGE_INVENTORY_2026-03-10.md](/C:/Rust%20Projects/ta_desktop_demo/research/STRATEGY_INDICATOR_GPU_COVERAGE_INVENTORY_2026-03-10.md)

Specifically:

- `Indicators With At Least 5x Speedup That Are Not Used By Any Existing Strategy`

### Curated Use / Don't-Use Classification

The point of this classification is to avoid building weak or incoherent strategies just to consume indicator names.

#### Use

These indicators are worth turning into strategy primitives:

- `kurtosis`
- `mass`
- `correl_hl`
- `kdj`
- `srsi`
- `coppock`
- `dpo`
- `ao`
- `rocr`
- `keltner`
- `vwap`
- `correlation_cycle`
- `ift_rsi`
- `mama`
- `prb`
- `qqe`
- `apo`
- `fvg_trailing_stop`
- `stddev`
- `devstop`
- `dti`
- `decycler`
- `otto`

#### Don't use

These should be explicitly skipped in the first strategy wave:

- `minmax` - don't use
- `buff_averages` - don't use
- `var` - don't use
- `deviation` - don't use
- `medprice` - don't use
- `wclprice` - don't use
- `uma` - don't use
- `lpc` - don't use
- `emd` - don't use

Rationale for the `don't use` set:

- `minmax`, `buff_averages`
  - better treated as utility/batch primitives than standalone strategy signals
- `var`, `deviation`
  - redundant in first-wave strategy design once `stddev` is already selected
- `medprice`, `wclprice`
  - price transforms, not strong standalone edge carriers
- `uma`
  - better thought of as an MA primitive than a dedicated strategy identity
- `lpc`, `emd`
  - too opaque / heavy / difficult to productize safely in the first wave

### New Strategy Set To Cover Every `Use` Indicator

The first-wave proposal is five new GPU-native strategies.

Each chosen indicator from the `Use` list is assigned to at least one strategy below.

#### 1. `KQSR` - KDJ QQE Stochastic Reversion

Core intent:

- short-horizon mean reversion using layered oscillator exhaustion and rank-based confirmation

Indicators:

- `kdj`
- `srsi`
- `qqe`
- `ift_rsi`
- `prb`

Execution target:

- `GpuKernel` + `CpuOnly` fallback

Why this is coherent:

- all five components are oscillator / rank / exhaustion primitives
- it avoids mixing trend-following logic into a pure reversion design

#### 2. `CARM` - Coppock AO Relative Momentum

Core intent:

- medium-horizon momentum continuation with cross-horizon confirmation

Indicators:

- `coppock`
- `ao`
- `rocr`
- `apo`
- `otto`

Execution target:

- `GpuKernel` + `CpuOnly` fallback

Why this is coherent:

- these are all momentum / slope / acceleration style signals
- this gives a clean “momentum cluster” strategy family that is distinct from `MTS`

#### 3. `KVDF` - Keltner VWAP Directional Flow

Core intent:

- intraday trend / breakout participation with anchored flow and risk-aware exits

Indicators:

- `keltner`
- `vwap`
- `dti`
- `devstop`
- `fvg_trailing_stop`

Execution target:

- `GpuKernel` + `CpuOnly` fallback

Why this is coherent:

- `keltner` and `vwap` define price location and breakout context
- `dti` acts as directional confirmation
- `devstop` and `fvg_trailing_stop` provide explicit stop / trend-protection logic

#### 4. `MKCS` - Mass Kurtosis Correlation Snapback

Core intent:

- volatility-shape reversal and post-expansion snapback

Implementation status note:

- `MKCS` v1 is now implemented as a parity-safe GPU-native optimization slice, but the implemented signal stack is currently:
  - `stddev`
  - `donchian`
- the original expansion to:
  - `mass`
  - `kurtosis`
  - `correl_hl`
  is still deferred
- checklist items marked complete for `MKCS` below refer to this implemented v1 optimization backend, not the full original distribution-shape extension

Indicators:

- `stddev`
- `donchian` (implementation support indicator)
- planned extension:
  - `mass`
  - `kurtosis`
  - `correl_hl`

Execution target:

- `GpuKernel` + `CpuOnly` fallback

Why this is coherent:

- this is the cleanest way to use the “distribution / shape / expansion” indicators without forcing them into generic trend logic

#### 5. `CDMC` - Cycle Decycler MAMA Composite

Core intent:

- cycle regime detection and rotation between trend and reversion behavior

Indicators:

- `dpo`
- `correlation_cycle`
- `decycler`
- `mama`

Execution target:

- `GpuKernel` + `CpuOnly` fallback

Why this is coherent:

- these are all cycle / adaptive-trend / de-noising primitives
- together they form a distinct regime-sensitive strategy family rather than a generic oscillator mashup

### Coverage Check

All `Use` indicators are covered:

- `kurtosis` -> `MKCS`
- `mass` -> `MKCS`
- `correl_hl` -> `MKCS`
- `kdj` -> `KQSR`
- `srsi` -> `KQSR`
- `coppock` -> `CARM`
- `dpo` -> `CDMC`
- `ao` -> `CARM`
- `rocr` -> `CARM`
- `keltner` -> `KVDF`
- `vwap` -> `KVDF`
- `correlation_cycle` -> `CDMC`
- `ift_rsi` -> `KQSR`
- `mama` -> `CDMC`
- `prb` -> `KQSR`
- `qqe` -> `KQSR`
- `apo` -> `CARM`
- `fvg_trailing_stop` -> `KVDF`
- `stddev` -> `MKCS`
- `devstop` -> `KVDF`
- `dti` -> `KVDF`
- `decycler` -> `CDMC`
- `otto` -> `CARM`

Current implementation note:

- the intended coverage map above remains the longer-term target
- the shipped `MKCS` backend for the current GPU validation completion pass is explicitly the narrower v1 slice:
  - `stddev`
  - `donchian`
- `mass`, `kurtosis`, and `correl_hl` remain unimplemented and are now explicitly deferred out of the current execution-completion scope unless product scope is widened later

### Architectural Direction

These new strategies should be implemented as dedicated GPU-native families, not as generic composite overlays.

Recommended shape:

1. define one request/config struct per strategy
2. define one signal-combo grid per strategy
3. build dedicated CPU signal generators
4. build dedicated GPU signal sweep generators
5. build dedicated GPU kernel combo evaluators
6. return the standard optimization result contract

This should mirror the current successful pattern used by:

- `ASR`
- `VCC`
- `VEMR`
- `ROCB`

### Implementation Phases

### Phase S1: indicator and strategy formalization

Deliverables:

- finalize the `Use` / `don't use` classification
- define each new strategy's entry / exit logic
- define the tunable parameter surface for each strategy
- define candle-field requirements for each strategy

Finalized first-wave classification:

- `Use`
  - `kdj`
  - `srsi`
  - `ift_rsi`
  - `prb`
  - `qqe`
  - `coppock`
  - `ao`
  - `rocr`
  - `apo`
  - `otto`
  - `keltner`
  - `vwap`
  - `fvg_trailing_stop`
  - `devstop`
  - `dti`
  - `stddev`
  - `dpo`
  - `correlation_cycle`
  - `mama`
  - `decycler`
- `don't use`
  - none in the first-wave selected set; any remaining unused `>= 5x` indicators not listed above stay out of scope until the first five families land

Finalized strategy definitions:

- `KQSR` = KDJ + QQE Stochastic Reversal
  - indicator stack:
    - `kdj`
    - `srsi`
    - `ift_rsi`
    - `prb`
    - `qqe`
  - long entry:
    - `KDJ J` crosses up through oversold threshold
    - `sRSI` is rising from lower band
    - `IFT RSI` is positive
    - `QQE` trend state is bullish
    - optional `PRB` filter confirms lower-percentile reversal
  - short entry:
    - mirror of long conditions through overbought thresholds
  - exit:
    - opposite `QQE` state
    - `KDJ J` mean reversion back through neutral band
  - tunable surface:
    - `kdj_period`
    - `srsi_period`
    - `srsi_smooth`
    - `ift_rsi_period`
    - `qqe_period`
    - `oversold`
    - `overbought`
    - `prb_period`
    - `prb_threshold`
  - candle fields:
    - `open/high/low/close`

- `CARM` = Coppock AO ROCR Momentum
  - indicator stack:
    - `coppock`
    - `ao`
    - `rocr`
    - `apo`
    - `otto`
  - long entry:
    - `coppock` positive and rising
    - `AO` above zero
    - `ROCR` above one
    - `APO` above zero
    - `OTTO` fast/slow state confirms momentum regime
  - short entry:
    - mirror of long conditions
  - exit:
    - any two momentum components flip against the trade
  - tunable surface:
    - `coppock_long_period`
    - `coppock_short_period`
    - `coppock_wma_period`
    - `ao_fast_period`
    - `ao_slow_period`
    - `rocr_period`
    - `apo_fast_period`
    - `apo_slow_period`
    - `otto_period`
    - `otto_threshold`
  - candle fields:
    - `high/low/close`

- `KVDF` = Keltner VWAP Dislocation Flow
  - indicator stack:
    - `keltner`
    - `vwap`
    - `fvg_trailing_stop`
    - `devstop`
    - `dti`
  - long entry:
    - price reclaims `VWAP`
    - price closes back inside upper `Keltner` expansion after a displacement
    - `DTI` trend is positive
    - `FVG trailing stop` and `DevStop` remain below price
  - short entry:
    - mirror of long conditions
  - exit:
    - close through trailing stop / devstop
    - or `VWAP` recross against the trade
  - tunable surface:
    - `keltner_period`
    - `keltner_mult`
    - `vwap_anchor`
    - `fvg_lookback`
    - `devstop_period`
    - `devstop_mult`
    - `dti_period`
    - `dti_threshold`
  - candle fields:
    - `high/low/close/volume`

- `MKCS` = Market Kurtosis Compression Setup
  - indicator stack:
    - `stddev`
  - long entry:
    - rolling `stddev` compresses below threshold
    - breakout close above recent compression range
  - short entry:
    - mirror of long conditions
  - exit:
    - breakout fails back into range
    - volatility re-expands beyond exit threshold
  - tunable surface:
    - `stddev_period`
    - `compression_threshold`
    - `breakout_lookback`
    - `reexpansion_threshold`
  - candle fields:
    - `high/low/close`

- `CDMC` = Cycle Decoupling MAMA Cycle
  - indicator stack:
    - `dpo`
    - `correlation_cycle`
    - `mama`
    - `decycler`
  - long entry:
    - `MAMA` leads `FAMA` upward
    - `DPO` positive
    - `Correlation Cycle` indicates in-phase bullish regime
    - `Decycler` trend filter is positive
  - short entry:
    - mirror of long conditions
  - exit:
    - `MAMA/FAMA` recross
    - cycle phase flips against trade
  - tunable surface:
    - `dpo_period`
    - `correlation_cycle_period`
    - `mama_fast_limit`
    - `mama_slow_limit`
    - `decycler_period`
    - `phase_threshold`
  - candle fields:
    - `close`

### Phase S2: request and frontend surface

Deliverables:

- add the new strategy kinds to the frontend strategy selector
- add request builders and normalization
- add capability summaries showing:
  - `GpuKernel`
  - `CpuOnly`
- keep global backend selection only where it is actually supported

### Phase S3: CPU baseline implementations

Deliverables:

- implement CPU signal builders for:
  - `KQSR`
  - `CARM`
  - `KVDF`
  - `MKCS`
  - `CDMC`
- implement CPU optimization baselines for parity

### Phase S4: GPU signal staging

- [x] Implement GPU signal staging for `KQSR`.
- [x] Implement GPU signal staging for `CARM`.
- [x] Implement GPU signal staging for `KVDF`.
- [x] Implement GPU signal staging for `MKCS`.
- [x] Implement GPU signal staging for `CDMC`.
- [ ] Verify all selected first-wave indicators remain GPU-executable end to end.
  - `MKCS` v1 currently stages GPU-computed host rows into device matrices for parity-safe optimization.
  - current implementation note on March 11, 2026:
    - after the workspace bump to `vector-ta 0.1.9`, the generic CUDA dispatch surface now exposes `QQE` `fast` + `slow` outputs, so the existing mixed staging path in `kqsr.rs` / `composite_cpu.rs` can keep `KDJ`, `sRSI`, `IFT RSI`, `QQE`, and `PRB` on CUDA-backed staging when available, and those staged rows now feed the verified strategy-local `GpuKernel` optimization path.
    - `CARM` now attempts CUDA signal staging on non-CPU backends for `Coppock`, `AO`, `ROCR`, `APO`, and `OTTO` through `carm.rs` / `composite_cpu.rs`, and those staged rows now feed the verified strategy-local `GpuKernel` optimization path.
    - `KVDF` now attempts CUDA signal staging on non-CPU backends for `Keltner`, `VWAP`, `DTI`, `DevStop`, and `FVG Trailing Stop` through `kvdf.rs` / `composite_cpu.rs`, and those staged rows now feed the verified strategy-local `GpuKernel` optimization path.
    - after the workspace bump to `vector-ta 0.1.9`, `CDMC` now attempts CUDA signal staging on non-CPU backends for `DPO`, `Correlation Cycle`, `MAMA/FAMA`, and `Decycler` through `cdmc.rs` / `composite_cpu.rs`, and those staged rows now feed the verified strategy-local `GpuKernel` optimization path.

### Phase S5: GPU optimization path

- [x] Implement `GpuKernel` optimization for `KQSR`.
  - current implementation note on March 12, 2026:
    - `KQSR` now has a strategy-local `GpuKernel` optimization path through `kqsr_gpu_kernel.rs`, `composite_cpu.rs`, and `double_crossover.cu`.
    - the current S5 route keeps metrics evaluation on GPU and batches compact row-major signal matrices for `KDJ.J`, `sRSI.K/D`, `IFT RSI`, `QQE fast/slow`, and `PRB upper/lower` into device buffers before device-side scoring.
    - this is intentionally not claiming a fully direct device-sweep path for every indicator family yet: `sRSI`, `IFT RSI`, and `PRB` still use host-assembled row-major uploads in the current route because their lockstep multi-parameter surfaces are not yet exposed as a simple one-axis device sweep in the generic adapter path.
    - follow-up backend optimization note on March 12, 2026:
      - the `KQSR` GPU route no longer computes the full eight-output bundle twice during staging.
      - `kqsr.rs` now exposes `compute_kqsr_fast_signals(...)` and `compute_kqsr_slow_signals(...)`, and `composite_cpu.rs` uses them in the fast/slow staging loops so the route stops materializing discarded seed-side outputs before upload.
      - the profiling follow-up pass now also keeps those fast/slow staging buffers in `f32` form end to end for the GPU route.
      - `kqsr.rs` now exposes `compute_kqsr_fast_signals_f32(...)` and `compute_kqsr_slow_signals_f32(...)`, and the optimization / validation staging loops in `composite_cpu.rs` and `composite_validate.rs` no longer allocate intermediate `Vec<f64>` rows only to cast them back to `f32` before upload.
- [x] Implement `GpuKernel` optimization for `CARM`.
  - current implementation note on March 12, 2026:
    - `CARM` now has a strategy-local `GpuKernel` optimization path through `carm.rs`, `carm_gpu_kernel.rs`, `composite_cpu.rs`, and `double_crossover.cu`.
    - the route keeps `Coppock`, `AO`, `ROCR`, `APO`, and `OTTO HOTT/LOTT` staged as compact row-major matrices, uploads them to device buffers once, and evaluates combo metrics through `carm_backtest_metrics_f32` before returning compact host results.
    - focused verification passed with `cargo check -p ta_app_core --features cuda,cuda-backtest-kernel --target-dir codex_tmp_carm_s5`, `cargo test -p ta_app_core carm_ --features cuda,cuda-backtest-kernel --target-dir codex_tmp_carm_s5 -- --nocapture`, and `cargo check -p ta_desktop_demo_app --features cuda,cuda-backtest-kernel --target-dir codex_tmp_carm_s5`.
    - follow-up backend optimization note on March 12, 2026:
      - Nsight Compute inspection on `target/nsight/carm_gpu_small_ncu.ncu-rep` did not point to local spilling; it reinforced that the route was still staging more row-major data than necessary.
      - `CARM` now factors `ROCR` into a fast-only row matrix, keeps the mixed `Coppock` / `AO` / `APO` rows per combo, and threads `combo_fast_row_index` into `carm_gpu_kernel.rs` / `double_crossover.cu` so the device kernel reuses the fast-side `ROCR` rows instead of uploading them per combo.
      - the profiling follow-up pass now also dedupes identical `OTTO` parameter tuples before upload and threads a combo-side `otto_row_index` into `carm_gpu_kernel.rs` / `double_crossover.cu`, so repeated `OTTO HOTT/LOTT` rows are staged once and reused across combos.
- [x] Implement `GpuKernel` optimization for `KVDF`.
  - current implementation note on March 12, 2026:
    - `KVDF` now has a strategy-local `GpuKernel` optimization path through `kvdf.rs`, `kvdf_gpu_kernel.rs`, `composite_cpu.rs`, and `double_crossover.cu`.
    - the route keeps `Keltner`, `VWAP`, `DTI`, `DevStop`, and `FVG Trailing Stop` staged as compact row-major matrices, uploads them to device buffers once, and evaluates combo metrics through `kvdf_backtest_metrics_f32` before returning compact host results.
    - focused verification passed with `cargo check -p ta_app_core --features cuda,cuda-backtest-kernel --target-dir codex_tmp_kvdf_s5`, `cargo test -p ta_app_core kvdf_ --features cuda,cuda-backtest-kernel --target-dir codex_tmp_kvdf_s5 -- --nocapture`, and `cargo check -p ta_desktop_demo_app --features cuda,cuda-backtest-kernel --target-dir codex_tmp_kvdf_s5`.
    - follow-up backend optimization note on March 12, 2026:
      - `KVDF` no longer stages every matrix per combo. The route now treats `VWAP` as shared, `FVG Trailing Stop` as fast-only, `Keltner` + `DevStop` as slow-only, and leaves only `DTI` as combo-shaped.
      - `kvdf.rs` now exposes strategy-local split builders for shared / fast / slow / mixed staging, and `kvdf_gpu_kernel.rs` / `double_crossover.cu` now take explicit fast-row and slow-row combo indices so the kernel reuses those factored matrices instead of consuming one full row per combo for every signal family.
      - profiling follow-up pass on March 12, 2026:
        - `DTI` is no longer forced to remain combo-shaped internally.
        - `kvdf_gpu_kernel.rs` / `double_crossover.cu` now take an explicit combo-side `DTI` row index, and the optimization / validation routes in `composite_cpu.rs` and `composite_validate.rs` dedupe identical `(dti_r, dti_s, dti_u)` triples before computing and uploading `DTI` rows.
        - on the current benchmark-grid shape used in `crates/app_core/benches/gpu_native_expansion_opt.rs` (`fast 5..20`, `slow 20..80`), that reduces the `DTI` staging set from `975` combo rows to `540` unique `DTI` rows, or about `44.6%` fewer `DTI` rows to compute/upload.
        - targeted regression coverage passed with `cargo check -p ta_app_core --features cuda,cuda-backtest-kernel`, `cargo test -p ta_app_core kvdf_ --features cuda,cuda-backtest-kernel`, and `cargo test -p ta_desktop_demo_app cross_ticker_kvdf_gpu_kernel_smoke_applies_stats --features cuda,cuda-backtest-kernel`.
- [x] Implement `GpuKernel` optimization for `MKCS`.
- [x] Implement `GpuKernel` optimization for `CDMC`.
  - current implementation note on March 12, 2026:
    - `CDMC` now has a strategy-local `GpuKernel` optimization path through `cdmc.rs`, `cdmc_gpu_kernel.rs`, `composite_cpu.rs`, and `double_crossover.cu`.
    - the route keeps `DPO`, `Correlation Cycle`, `MAMA/FAMA`, and `Decycler` staged as device-resident matrices/buffers, evaluates combo metrics through `cdmc_backtest_metrics_f32`, and returns compact host results after device-side scoring.
    - focused verification passed with `cargo check -p ta_app_core --features cuda,cuda-backtest-kernel`, `cargo test -p ta_app_core cdmc_ --features cuda,cuda-backtest-kernel -- --nocapture`, `cargo test -p ta_app_core dynamic_indicator_adapter --features cuda,cuda-backtest-kernel -- --nocapture`, and `cargo check -p ta_desktop_demo_app --features cuda,cuda-backtest-kernel`.
    - follow-up backend optimization note on March 12, 2026:
      - the repo-local hotspot was duplicated `MAMA/FAMA` generation rather than the final scorer kernel.
      - `cdmc.rs` now computes the `MAMA/FAMA` pair in one `CudaMama` pass for optimization / validation staging and reuses both device buffers.
      - the shared `compute_cdmc_signals(...)` helper now computes the `MAMA/FAMA` pair once per request instead of running two identical passes for the two outputs.
- [x] Wire clean `CpuOnly` fallback for all five.
  - `MKCS` fallback is implemented.
  - `KQSR` now falls back cleanly to `CpuOnly` when CUDA is unavailable or the build omits `cuda-backtest-kernel`; otherwise the new strategy-local `GpuKernel` route runs directly.
  - `CARM` now falls back cleanly to `CpuOnly` when CUDA is unavailable or the build omits `cuda-backtest-kernel`; otherwise the strategy-local `GpuKernel` route runs directly.
  - `KVDF` now falls back cleanly to `CpuOnly` when CUDA is unavailable or the build omits `cuda-backtest-kernel`; otherwise the strategy-local `GpuKernel` route runs directly.
  - `CDMC` now falls back cleanly to `CpuOnly` when CUDA is unavailable or the build omits `cuda-backtest-kernel`; otherwise the strategy-local `GpuKernel` route runs directly.

### Phase S6: parity and evidence

- current implementation note on March 12, 2026:
  - for these GPU-native S5 routes, Phase S6 parity is treated as a tolerance-gated major-difference check, not bitwise `fp64` equality.
  - device-side `fp32` metric reduction is allowed when the trade path remains aligned and the resulting `pnl`, `sharpe`, `volatility`, `max_dd`, exposure, and trade-count outputs stay within explicit guardrails.
- [x] Add CPU vs GPU parity tests for `KQSR`.
- [x] Add CPU vs GPU parity tests for `CARM`.
- [x] Add CPU vs GPU parity tests for `KVDF`.
- [x] Add CPU vs GPU parity tests for `MKCS`.
- [x] Add CPU vs GPU parity tests for `CDMC`.
- [x] Add representative benchmarks for all five strategies.
  - `MKCS` benchmark coverage is implemented in `crates/app_core/benches/mkcs_opt.rs`.
  - `KQSR`, `CARM`, `KVDF`, and `CDMC` benchmark coverage is implemented in `crates/app_core/benches/gpu_native_expansion_opt.rs`.
  - current local benchmark evidence on March 11, 2026:
    - `mkcs/cpu/200k/small`: about `1.73 s`
    - `mkcs/gpu_kernel/200k/small/975pairs`: about `5.05 s`
  - interpretation:
    - the implemented parity-safe GPU staging is real, but it is not yet a speed win on the small benchmark case
- [x] Run Nsight Systems on at least one benchmark per strategy.
  - benchmark-mode Nsight Systems reports were captured on March 12, 2026 for:
    - `target/nsight/kqsr_gpu_small_2023.nsys-rep`
    - `target/nsight/carm_gpu_small_2023.nsys-rep`
    - `target/nsight/kvdf_gpu_small_2023.nsys-rep`
    - `target/nsight/cdmc_gpu_small_2023.nsys-rep`
    - `target/nsight/mkcs_gpu_small_2023.nsys-rep`
  - supporting script/runtime fixes landed alongside the captures:
    - `scripts/profile_gpu_backtest_nsys.ps1` now prefers the stable 2023 Nsight Systems CLI in this environment, preserves a `BenchStrategyOnly` selector via `VECTORBT_GPU_NATIVE_BENCH_ONLY`, and treats Windows nonzero-exit-but-report-generated runs as usable captures.
    - `scripts/profile_gpu_backtest_ncu.ps1` now supports the same `BenchStrategyOnly` selector so compute-level follow-up can isolate a single strategy without replaying unrelated benchmark probes.
    - `crates/app_core/benches/gpu_native_expansion_opt.rs` now honors `VECTORBT_GPU_NATIVE_BENCH_ONLY`, which prevents unrelated strategy route probes from contaminating per-strategy benchmark traces.
  - hotspot summary from the benchmark-mode Nsight Systems captures:
    - `KQSR`: the trace is dominated by indicator staging (`prb_batch_f32`, `srsi_build_rsi_f32`, `qqe_batch_f32`), with `kqsr_backtest_metrics_f32` only about `0.4%` of GPU kernel time and about `264 MB` host-to-device plus `246 MB` device-to-host transfer volume on the small case.
    - `CARM`: the trace is overwhelmingly dominated by `otto_batch_f32` with the strategy backtest kernel only about `0.1%` of GPU kernel time; the small case still moves about `3.9 GB` in each direction and pays heavy module-load/unload churn, so any next optimization should target `OTTO` staging rather than the final scorer.
    - `KVDF`: the trace is dominated by `dti_batch_f32`, `devstop_batch_grouped_f32`, and `fvg_trailing_stop_batch_small_shmem_f32`, while `kvdf_backtest_metrics_f32` is about `1.6%` of GPU kernel time; this reinforces the earlier Nsight Compute result that the next wins are upstream indicator-side, not another pass on the scorer loop.
    - `CDMC`: the trace is dominated by `mama_inv_dp_f32`, with `cdmc_backtest_metrics_f32` about `5.2%` of GPU kernel time and very small transfer volume; the remaining cost is clearly in `MAMA/FAMA` generation.
    - profiling follow-up implementation state on March 12, 2026:
      - `KQSR`: repo-local host-side staging now stays split and `f32`-native before upload.
      - `CARM`: repo-local `OTTO` staging now dedupes repeated tuples and reuses them by combo-side row index.
      - `KVDF`: repo-local `DTI` staging now dedupes repeated `(dti_r, dti_s, dti_u)` tuples and reuses them by combo-side row index.
      - `CDMC`: repo-local staging now computes `MAMA/FAMA` once per request or per device staging pass instead of triggering the same CUDA indicator work twice.
    - post-change profiling closeout on March 12, 2026 after the U6 repo-local reductions:
      - refreshed Nsight Systems reports were captured at:
        - `target/nsight/kqsr_gpu_small_post_u6.nsys-rep`
        - `target/nsight/carm_gpu_small_post_u6.nsys-rep`
        - `target/nsight/kvdf_gpu_small_post_u6.nsys-rep`
        - `target/nsight/cdmc_gpu_small_post_u6.nsys-rep`
      - environment note:
        - `nsys stats --report cuda_gpu_kern_sum` was not usable on those post-U6 `.nsys-rep` captures in this environment because the CLI reported no CUDA kernel data, so the post-change hotspot summaries below come from Nsight Compute imports instead.
      - refreshed Nsight Compute reports were captured at:
        - `target/nsight/kqsr_gpu_small_post_u6_ncu.ncu-rep`
        - `target/nsight/carm_gpu_small_post_u6_ncu.ncu-rep`
        - `target/nsight/kvdf_gpu_small_post_u6_ncu.ncu-rep`
        - `target/nsight/cdmc_gpu_small_post_u6_ncu2.ncu-rep`
      - post-change hotspot summary from those Nsight Compute captures:
        - `KQSR`: `srsi_build_rsi_f32` remains dominant at about `71.11%` of captured device duration, followed by `willr_build_sparse_level_f32` at about `17.10%`; this is consistent with the implemented win being host-side `f32` staging reduction rather than a device-kernel rewrite.
        - `CARM`: `otto_batch_f32` still dominates at about `62.31%` of captured device duration after the repo-local dedupe, which suggests the next ceiling is upstream `OTTO` indicator work rather than another scorer-path change.
        - `KVDF`: `DTI` no longer appears in the captured top-duration kernel set; `fvg_trailing_stop_batch_small_shmem_f32` now dominates at about `76.07%`, followed by `devstop_batch_grouped_f32` at about `16.20%`.
        - `CDMC`: `mama_inv_dp_f32` is no longer the dominant device-side hotspot; `correlation_cycle_batch_f32_ria` now leads at about `62.76%`, while `mama_inv_dp_f32` is about `14.95%`, which is consistent with the duplicated repo-local `MAMA/FAMA` computation having been removed.
      - focused regression verification for the U6 implementation and profiling pass remained green with:
        - `cargo check -p ta_app_core --features cuda,cuda-backtest-kernel`
        - `cargo test -p ta_app_core kvdf_ --features cuda,cuda-backtest-kernel`
        - `cargo test -p ta_app_core carm_ --features cuda,cuda-backtest-kernel`
        - `cargo test -p ta_app_core kqsr_ --features cuda,cuda-backtest-kernel`
        - `cargo test -p ta_app_core cdmc_ --features cuda,cuda-backtest-kernel`
        - `cargo test -p ta_desktop_demo_app cross_ticker_ --features cuda,cuda-backtest-kernel`
    - `MKCS`: the trace is dominated by `donchian_batch_from_rmq_f32`, with `mkcs_backtest_metrics_f32` about `10.4%` of GPU kernel time; the strategy is no longer bottlenecked primarily by orchestration.
- post-profile perf-gate caveat on March 12, 2026:
  - `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate` failed against the stored baseline and wrote `target/gpu_perf_summary.json`.
  - that gate measures `double_ma/gpu_kernel/sma_sma/200k/small` through `scripts/run_gpu_perf_gate.ps1`, not the `gpu_native_expansion_opt` harness used by `KQSR`, `CARM`, `KVDF`, and `CDMC`, so it should be treated as a separate `DoubleMa` performance investigation instead of evidence that the U6 strategy-local changes regressed.
- [x] Record route metadata in benchmark outputs.
  - `mkcs_opt.rs` and `gpu_native_expansion_opt.rs` now print a route probe before the benchmark groups, including `backend_used`, `backend_route`, and `backend_fallback_reason`.
  - focused verification in this session compiled both benchmark harnesses with `cargo bench -p ta_app_core --bench mkcs_opt --features cuda,cuda-backtest-kernel --no-run` and `cargo bench -p ta_app_core --bench gpu_native_expansion_opt --features cuda,cuda-backtest-kernel --no-run`, and the `mkcs_opt` release benchmark executable printed route metadata on `--list`.

## Ordering Note

This strategy-expansion track should begin only after the ADX-gated routing work for:

- `CompositeTrend`
- `RegimeHybrid`
- `BreakoutMomentum`

is at least implementation-ready, because that work establishes the route-metadata and GPU-execution conventions that these new strategies should reuse.
