# Review Packet: `crates/app_core/src/composite_cpu.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/composite_cpu.rs`
- Packet status: `Reviewed`
- Public-review readiness: `High-value backend orchestrator with strong feature coverage, but not yet accept-as-is because backend-contract semantics, route diagnostics, and indicator fallback reporting are inconsistent enough to obscure what actually ran`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Public-surface`, `Hot-path`, `Route-orchestration-critical`, `CUDA-runtime-critical`, `Benchmark-sensitive`

## What This File Owns

`composite_cpu.rs` is the main composite-strategy orchestration file inside `ta_app_core`. It owns the public request and result types for base composite optimization, drilldown, and local-stability runs; the route-selection logic that decides between CPU, GPU-assisted indicator execution, and GPU-kernel optimization; strategy-specific composite optimization entry points; the base composite tiled CPU optimizer; and a large in-file test surface.

This file owns:

- public request/result surface for base composite workflows:
  - `CompositeCpuRequest`: `crates/app_core/src/composite_cpu.rs:371-431`
  - `CompositeCpuDrilldownRequest`: `442-480`
  - `CompositeCpuDrilldownResult`: `499-504`
  - `CompositeCpuLocalStabilityRequest`: `507-548`
- route and backend helpers:
  - backend labels and device selection: `1822-1905`
  - strategy route-diagnostics helpers: `1832-1865`
  - base GPU-kernel route: `1963-2109`
  - dynamic indicator output binding: `2113-2148`
- strategy and base optimization entry points:
  - `run_composite_cpu_optimization_blocking_with_candles`: `10873-11576`
  - strategy-specific optimizers earlier in the file for ASR/VCC/VEMR/ROCB/VCET/MKCS/KQSR/CARM/KVDF/CDMC
- base composite drilldown and local stability:
  - `compute_composite_cpu_drilldown_blocking_with_candles`: `11578-11761`
  - `compute_composite_cpu_local_stability_blocking_with_candles`: `11854-12122`
- backend-aware indicator helpers:
  - `compute_halftrend_trend_with_backend`: `12148-12174`
  - `compute_macz_hist_with_backend`: `12210-12238`
  - `compute_ott_line_with_backend`: `12272-12300`
- the base tiled CPU search engine:
  - `optimize_composite_cpu_tiled`: `12311-12863`
- a large regression/smoke test surface:
  - tests start around `12919`

This file does not own:

- the core Double-MA engine and shared backend types
  - `crates/app_core/src/double_ma.rs`
- strategy-local GPU kernels and device wrappers
  - for example `asr_gpu_kernel.rs`, `carm_gpu_kernel.rs`, `cdmc_gpu_kernel.rs`
- Tauri command transport
  - `src-tauri/src/commands*.rs`

## Classification

- Public surface: yes
  - request types and blocking entry points define backend behavior visible to command callers
- Hot path: yes
  - this file owns optimization loops, route selection, and search-space narrowing
- Runtime-critical: high
  - route labeling, fallback messaging, and indicator/backend selection affect user-visible behavior
- CUDA/runtime-critical: medium to high
  - this file prepares CUDA indicator inputs, chooses GPU-kernel routes, and coordinates fallback to CPU
- Strategy-local: mixed
  - it hosts both base composite logic and multiple strategy-specific routes
- Low-risk support code: no

## Direct Interactions

### Backend interaction map

- shared backend types and optimization result plumbing:
  - `crates/app_core/src/double_ma.rs`
  - `crates/app_core/src/double_ma_backend.rs`
  - `crates/app_core/src/request_normalization.rs`
- GPU-kernel base optimization and resident-device uploads:
  - `crates/app_core/src/gpu_backtest_kernel.rs`
- dynamic indicator dispatch and CUDA indicator execution:
  - `crates/app_core/src/dynamic_indicator_adapter.rs`
  - `crates/app_core/src/dynamic_indicator_catalog.rs`
- strategy-local GPU and CPU helpers:
  - `crates/app_core/src/asr.rs`
  - `crates/app_core/src/carm.rs`
  - `crates/app_core/src/cdmc.rs`
  - `crates/app_core/src/kqsr.rs`
  - `crates/app_core/src/kvdf.rs`
  - `crates/app_core/src/mkcs.rs`
  - `crates/app_core/src/rocb.rs`
  - `crates/app_core/src/vcet.rs`
  - `crates/app_core/src/vcc.rs`
  - `crates/app_core/src/vemr.rs`
- shared strategy evaluation and metrics:
  - `crates/strategies/src/composite_cpu/mod.rs`
  - `crates/optimizer/src/lib.rs`
  - `crates/optimizer/src/stream.rs`
- validation and follow-on verification paths:
  - `crates/app_core/src/composite_validate.rs`
  - `crates/app_core/src/double_ma_validate.rs`
- command-facing consumers:
  - `src-tauri/src/commands.rs`
  - `src-tauri/src/commands_validation.rs`

### Downstream verification impact if changed

- any change to route-diagnostic behavior affects:
  - command responses that expose `backend_route`, `backend_fallback_reason`, and `route_diagnostics`
- any change to base GPU-kernel routing affects:
  - GPU parity expectations
  - prescan behavior
  - include-all behavior
- any change to drilldown/local-stability backend semantics affects:
  - consumers that believe `backend` on those requests is meaningful today
- any change to `resolve_output_id` or indicator backend helpers affects:
  - HalfTrend/MACZ/OTT dynamic-dispatch correctness
  - CUDA-vs-CPU parity for base composite gating

### Test anchors

- local tests in this file:
  - `crates/app_core/src/composite_cpu.rs:12919-19350`
- representative validation consumers:
  - `crates/app_core/src/composite_validate.rs`
- command boundary integration:
  - `src-tauri/src/commands_validation.rs`

## Standards And Research Log

This packet was written after targeted external review focused on public Rust API contract clarity, recoverable-error guidance for route/fallback reporting, Rayon parallel reduction semantics for the tiled CPU optimizer, and CUDA best-practices guidance relevant to host/device transfer and benchmark-sensitive optimization work.

### Sources consulted

1. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this file exposes public request types and backend-facing behavior that callers need to understand predictably
   - Standard informed:
     - API surfaces should be clear, unsurprising, and dependable

2. Rust API Guidelines naming guidance
   - Source: <https://rust-lang.github.io/api-guidelines/naming.html>
   - Why it was consulted:
     - this file uses public request fields such as `backend`, plus route labels that imply specific execution semantics
   - Standard informed:
     - names should communicate semantics accurately and avoid overstating behavior

3. Rust Book, Error Handling
   - Source: <https://doc.rust-lang.org/book/ch09-00-error-handling.html>
   - Why it was consulted:
     - this file returns many recoverable `Result<_, String>` errors and fallback reasons to higher layers
   - Standard informed:
     - recoverable failures should remain explicit rather than silently altering behavior

4. Rayon `ParallelIterator` docs
   - Source: <https://docs.rs/rayon/latest/rayon/iter/trait.ParallelIterator.html>
   - Why it was consulted:
     - the base tiled optimizer uses `into_par_iter()`, `try_fold`, and `try_reduce` in its non-`include_all` path
   - Standard informed:
     - fold/reduce semantics and grouping behavior matter when reasoning about aggregation, ordering, and performance changes

5. NVIDIA CUDA C++ Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
   - Why it was consulted:
     - this file decides when to prepare CUDA inputs, when to fall back, and whether to keep work host-side before upload
   - Standard informed:
     - minimize unnecessary host/device movement and treat performance changes as measurement-sensitive

## Findings

## Correctness And Safety

### 1. Base composite indicator routing can silently fall back from CUDA to CPU without surfacing that downgrade precisely

- Evidence:
  - once CUDA input preparation succeeds, the orchestrator records a GPU-targeted route:
    - `crates/app_core/src/composite_cpu.rs:10982-11003`
  - the file then keeps `indicator_backend` on the non-CPU path:
    - `11036-11040`
  - but each backend-aware indicator helper silently retries on CPU with `if let Ok(...)` and no diagnostic:
    - HalfTrend: `12156-12167`
    - MACZ: `12220-12231`
    - OTT: `12282-12293`
- Why it matters:
  - output values may still be numerically valid, but the execution contract is not faithfully reported
  - that makes backend-route diagnostics less trustworthy during review, benchmarking, and bug triage
- Smallest justified fix:
  - surface the first helper-level CUDA failure into `backend_fallback_reason` or `route_diagnostics`
  - only report a pure GPU-oriented indicator route when all helper-level GPU computations actually stayed on the GPU path
- Review judgment:
  - safe-now observability and contract fix

### 2. Cheap request-shape validation happens after expensive CUDA input preparation on the base path

- Evidence:
  - CUDA input preparation is attempted first:
    - `10982-11021`
  - only afterwards does the function reject obvious request-shape problems such as:
    - wrong MA-type count: `11061-11063`
    - empty MA ids: `11065-11068`
    - invalid MA kind: `11070-11071`
- Why it matters:
  - malformed requests can pay GPU setup cost before being rejected for simple, local validation failures
  - this is not a correctness bug, but it does worsen error-path cost and trace clarity
- Smallest justified fix:
  - move the cheap request-shape checks ahead of CUDA input preparation
- Review judgment:
  - safe-now local improvement

## API And Naming

### 1. The public `backend` field on base drilldown and local-stability requests is effectively ignored

- Evidence:
  - public request fields exist:
    - drilldown request: `442-480`
    - local-stability request: `507-548`
  - the base drilldown path computes HalfTrend/MACZ/OTT with CPU-only helpers:
    - `11667-11689`
  - the base local-stability path does the same:
    - `11946-11968`
  - strategy-specific unsupported modes currently fail only with runtime strings:
    - drilldown: `11597-11625`
    - local stability: `11880-11901`
- Why it matters:
  - a public caller can reasonably infer that setting `backend` changes execution policy for these base flows, but for the base composite path it currently does not
  - that is an API-contract ambiguity rather than a computation bug
- Smallest justified fix:
  - either reject non-`CpuOnly` backends explicitly for the base drilldown/local-stability path
  - or route those base flows through the existing backend-aware indicator helpers
- Review judgment:
  - safe-now API clarification or guardrail

### 2. The `GpuKernel` label overstates how much of the base composite route is actually device-resident

- Evidence:
  - the base GPU-kernel path still builds both MA matrices on CPU:
    - `2009-2026`
  - converts those matrices on the host:
    - `2030-2037`
  - builds the regime on the host and only then uploads it:
    - `2048-2057`
- Why it matters:
  - the route is real, but it is not an end-to-end device-resident pipeline
  - names and route labels should not imply more than the implementation currently does
- Smallest justified fix:
  - document this route as a GPU-kernel optimization path with host-precomputed inputs, not as a fully device-resident pipeline
- Review judgment:
  - document now; broader naming changes are compatibility-sensitive

## Error Handling

### 1. Dynamic indicator output binding fails open and can silently pick the wrong output if the catalog drifts

- Evidence:
  - `resolve_output_id` tries exact preferred ids:
    - `2123-2131`
  - then substring matches:
    - `2133-2145`
  - and finally returns the first declared output:
    - `2148`
  - those ids feed HalfTrend, MACZ, and OTT backend-aware helpers:
    - `12156`
    - `12220`
    - `12282`
- Why it matters:
  - this is convenient for compatibility, but too permissive for a hot-path gating file
  - if the dynamic indicator catalog changes, the file can silently bind the wrong output rather than failing loudly
- Smallest justified fix:
  - tighten base composite output binding to exact known ids for these three indicators and fail fast otherwise
- Review judgment:
  - safe-now correctness hardening

### 2. Route-diagnostic quality is inconsistent across strategy orchestrators

- Evidence:
  - centralized route-diagnostic plumbing only covers:
    - ASR, VCC, VEMR, VCET, KQSR, KVDF
    - `1832-1865`
  - several result constructors default to no route diagnostics:
    - `2819-2822`
    - `3165-3168`
    - `10318-10321`
    - `10861-10864`
  - CARM and CDMC GPU fallbacks only populate `backend_fallback_reason`:
    - CARM: `8481-8497`, `8502-8518`
    - CDMC: `9860-9876`, `9881-9898`
- Why it matters:
  - diagnostic fidelity depends on which strategy family is active, not on a consistent backend contract
  - that weakens reviewability and makes backend results less reusable as engineering evidence
- Smallest justified fix:
  - standardize route-diagnostic capture across all strategy families that support GPU or hybrid routes
- Review judgment:
  - safe-now observability improvement

## Module Boundaries And Technical Debt

### 1. This file is doing too much route-policy work inline, which makes cross-file contracts harder to reason about

- Evidence:
  - it owns request decoding, route normalization, prescan decisions, CPU optimization, strategy-specific orchestration, drilldown, local stability, and extensive tests
  - the file is very large and mixes base composite concerns with many strategy-specific policy branches
- Why it matters:
  - the breadth is already making route semantics inconsistent across profiles
  - changes in one family can silently diverge from others because there is no single contract boundary for fallback and diagnostics
- Smallest justified fix:
  - keep current structure for now, but extract shared route-diagnostic policy and backend-reporting helpers before attempting broader modular decomposition
- Deferred larger fix:
  - split base composite orchestration from strategy-specific route owners
- Review judgment:
  - larger refactor exists, but should be deferred

### 2. Prescan behavior materially changes route selection and should stay explicit

- Evidence:
  - base composite GPU-kernel path rejects `allowed_pairs` subsets:
    - `1983-1987`
  - auto-prescan is skipped for explicit plain `GpuKernel` requests:
    - `11106-11112`
  - the file records an explicit warning that prescan may force CPU fallback for the base `GpuKernel` path:
    - `11150-11152`
  - successful prescan narrows `allowed_pairs`:
    - `11295-11300`
- Why it matters:
  - route choice is not determined by `backend` alone; prescan policy also affects the eventual route
  - that is important product behavior and should not be “simplified” casually
- Review judgment:
  - semantics-sensitive; document and test rather than rewrite

## Docs And Examples

### 1. Public request types do not document backend limitations, prescan interactions, or `include_all` cost semantics

- Missing documentation topics:
  - base drilldown/local-stability currently behave as CPU-only for the base path
  - explicit `GpuKernel` plus prescan-subset narrowing is not supported on the base path
  - `include_all=true` changes execution strategy, memory pressure, and collection behavior
- Evidence:
  - `include_all` is explicitly recorded as a serial-path forcing condition:
    - `11054-11058`
  - the optimizer uses serial full-row collection for `include_all`:
    - `12499-12577`
  - the non-`include_all` path uses Rayon `try_fold`/`try_reduce`:
    - `12601-12720`
- Smallest justified fix:
  - add Rustdoc or packet-linked documentation for these request fields and route constraints

## Tests And Missing Verification

### 1. Existing smoke coverage is useful, but the most important route-contract gaps are not pinned directly

- Current targeted verification observed during this review:
  - `cargo test -p ta_app_core composite_cpu_drilldown_smoke_has_bins_and_values -- --nocapture`
  - `cargo test -p ta_app_core composite_cpu_local_stability_smoke_has_base_point_and_summary -- --nocapture`
  - `cargo test -p ta_app_core composite_cpu_gpu_kernel_request_skips_implicit_prescan -- --nocapture`
- Missing or under-specified coverage:
  - explicit test that base composite indicator helper CUDA failure is surfaced in route diagnostics rather than silently falling back
  - explicit test that non-CPU backend on base drilldown/local-stability is either honored or rejected clearly
  - explicit test that dynamic output binding fails fast if HalfTrend/MACZ/OTT catalogs drift
  - route-diagnostic consistency tests across CARM and CDMC, not just fallback reason assertions

## Performance, Allocation, And CUDA Concerns

### 1. `include_all=true` is a real execution-mode switch, not just an output-shape option

- Evidence:
  - route diagnostics record that `include_all=true` forces serial collection:
    - `11054-11058`
  - the tiled optimizer uses nested serial loops for `include_all`:
    - `12499-12577`
  - the non-`include_all` path uses parallel `try_fold` / `try_reduce`:
    - `12601-12720`
- Why it matters:
  - any “optimization” here changes concurrency, memory shape, and aggregation behavior
- Review judgment:
  - benchmark-sensitive; do not casually rewrite

### 2. The tiled CPU path intentionally exactifies finalists in a second pass when the fast metric path was used

- Evidence:
  - second-pass recomputation is gated by:
    - `12737-12738`
  - recompute caches are built and then used to exactify best/top rows:
    - `12743-12833`
- Why it matters:
  - this is a deliberate accuracy-versus-throughput trade
  - changing it could perturb ranking stability or reported best/top metrics
- Review judgment:
  - benchmark-sensitive and semantics-sensitive

### 3. CARM and CDMC CPU routes still recompute full strategy signals per pair and again during rerank

- Evidence:
  - CARM CPU per-pair recompute:
    - `9001-9019`
  - CDMC CPU per-pair recompute:
    - `10416-10427`
  - contrasted with clearer reuse patterns elsewhere:
    - ASR fast-signal cache: `2905-2920`
    - ROCB breakout/risk caches: `6455-6457`
    - CARM GPU row-major ROCR reuse: `8592-8602`
    - CDMC GPU key precomputation: `9966-9975`
- Why it matters:
  - this is the strongest performance-improvement surface visible in the file
  - but it directly affects ranking inputs and first-valid calculations, so it is not safe-now cleanup
- Review judgment:
  - benchmark-sensitive larger refactor; defer until measured

### 4. Base composite GPU-kernel routing still pays host-precompute and upload cost

- Evidence:
  - MA matrices are computed and converted on host before the GPU-kernel optimizer call:
    - `2009-2037`
  - regime is also host-built before upload:
    - `2048-2057`
- Why it matters:
  - this is a legitimate optimization path, but not the final device-residency target implied by the repo’s broader CUDA posture
- Review judgment:
  - performance opportunity exists, but it is not a small local change

## Code-Quality Improvement Register

- Safe now:
  - surface helper-level CUDA indicator fallback into backend diagnostics
  - reject or explicitly honor non-CPU backend on base drilldown/local-stability
  - move cheap request-shape validation ahead of CUDA input preparation
  - tighten `resolve_output_id` for HalfTrend/MACZ/OTT to fail fast on catalog drift
  - document `include_all`, prescan, and base `GpuKernel` route semantics
- Defer:
  - splitting this file into smaller route owners
  - renaming route labels or backend surface without a broader compatibility review

## Technical-Debt Reduction Register

- Safe-now debt to reduce:
  - inconsistent route-diagnostic policy across strategy families
  - fail-open dynamic output binding for critical gate indicators
  - public backend field ambiguity in base drilldown/local-stability
- Defer-later debt:
  - collapsing repeated strategy-route scaffolding into a shared abstraction
  - turning base composite `GpuKernel` into a more device-resident route
- Useful debt that should not be “cleaned up” casually:
  - explicit prescan-versus-backend tradeoff logic
  - finalist exactify pass
  - `include_all` serial path for full row materialization

## Performance Opportunity Register

- Safe structural opportunities:
  - move low-cost validation ahead of CUDA input prep
  - improve backend/fallback reporting without changing computation
- Benchmark-sensitive opportunities:
  - reduce CARM/CDMC CPU recomputation with strategy-local caches
  - revisit host-side precompute in the base GPU-kernel route
  - experiment with `include_all` collection strategies only behind careful measurement
  - revisit exactify work only if ranking stability and parity are revalidated

## Recommended Action

- `Small surgical refactor`

Reasoning:

- there are several objectively useful improvements that can be made without changing core scoring semantics:
  - make route reporting honest
  - clarify or guard backend request semantics
  - fail fast when indicator output binding becomes ambiguous
  - avoid expensive CUDA prep for trivially invalid requests
- the larger throughput opportunities are real, but they sit in benchmark-sensitive code and should not be bundled into the same pass

## Exact Local Verification Required If Changed

If route reporting, drilldown/local-stability backend behavior, or base GPU-kernel routing changes:

- `cargo test -p ta_app_core composite_cpu_drilldown_smoke_has_bins_and_values -- --nocapture`
- `cargo test -p ta_app_core composite_cpu_local_stability_smoke_has_base_point_and_summary -- --nocapture`
- `cargo test -p ta_app_core composite_cpu_gpu_kernel_request_skips_implicit_prescan -- --nocapture`
- `cargo test -p ta_app_core base_composite_hard_gate_gpu_kernel_include_all_smoke_preserves_grid_shape -- --nocapture`

If indicator routing or output-id binding changes:

- run the targeted `composite_cpu` GPU/CPU parity and fallback tests in this file
- recheck any strategy-local tests that rely on HalfTrend, MACZ, or OTT dynamic dispatch

If hot-path performance logic changes:

- `powershell -File scripts/run_cpu_perf_gate.ps1 -Mode candidate`
- if GPU path behavior changes and GPU is available:
  - `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`

## Regression Risk And Feature-Preservation Constraints

- do not remove prescan behavior, shortlist behavior, or fallback behavior as “cleanup”
- do not narrow supported strategy routes without explicit product approval
- do not change `include_all` semantics or final exactify behavior without parity and benchmark evidence
- do not relabel routes in a way that hides mixed CPU/GPU execution or fallback conditions
- preserve current feature coverage even if the packet recommends stricter diagnostics or clearer request validation
