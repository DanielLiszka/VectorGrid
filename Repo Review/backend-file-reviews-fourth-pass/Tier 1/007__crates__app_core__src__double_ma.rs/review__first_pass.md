# Review Packet: `crates/app_core/src/double_ma.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/double_ma.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Critical optimization/runtime hub with extensive coverage, but not public-review ready as-is because one public backend helper appears to overwrite explicit GPU device selections, follow-up caching can reuse stale dataset-derived series, and several dynamic GPU cache/materialization choices are benchmark-sensitive and under-bounded`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Public-surface`, `Hot-path`, `CUDA/runtime-critical`, `Benchmark-sensitive`, `Cross-file-contract-sensitive`

## What This File Owns

`double_ma.rs` is one of the core backend execution hubs of the repo. It owns the public optimization request/backend surface for Double-MA optimization, public drilldown and local-stability entrypoints, backend route normalization, GPU residency policy enforcement, dynamic-strategy GPU preflight logic, follow-up caching for derived series, and large portions of CPU/GPU/hybrid execution dispatch.

Major owned surfaces include:

- public backend and request surface:
  - `Backend`: `crates/app_core/src/double_ma.rs:105-149`
  - `DoubleMaRequest`: `273-327`
- public drilldown/local-stability APIs:
  - `compute_double_ma_drilldown_blocking_with_candles`: `4482-4485`
  - `compute_double_ma_local_stability_blocking_with_candles`: `4654-4657`
- public optimization entrypoints:
  - `run_double_ma_optimization_blocking_with_candles`: `4925-4930`
  - `run_double_ma_optimization_blocking_with_candles_and_eval_range`: `4975-4981`
- backend route normalization and fallback policy:
  - `4989-5517`
- follow-up cache and follow-up key construction:
  - `63-73`
  - `9851-9991`
- GPU residency policy summary/enforcement:
  - `9994-10096`
- dynamic GPU preflight and dynamic GPU batch cache:
  - `43-61`
  - `11564-11764`
  - `13322-13357`
  - `13449+`
- very large in-file test suite:
  - `14121+`

This file does not own:

- backend label helpers and indicator route-plan primitives
  - `crates/app_core/src/double_ma_backend.rs`
- result-shaping helpers
  - `crates/app_core/src/double_ma_result.rs`
- strategy validation and strategy-shaping helpers
  - `crates/app_core/src/double_ma_strategy.rs`
- downstream validation workflows
  - `crates/app_core/src/double_ma_validate.rs`

## Classification

- Public surface: yes
  - multiple public enums, structs, and entrypoints
- Hot path: yes
  - this file sits directly on optimization execution and route selection
- CUDA/runtime-critical: high
  - GPU, hybrid, kernel, and residency policy decisions flow through this file
- Strategy-local: mixed
  - Double-MA specific, but it also acts as a shared runtime-policy hub
- Low-risk support code: no

## Direct Interactions

### Backend interaction map

- device-selection dependency:
  - `crates/app_core/src/cuda_device.rs`
  - consumed through `with_preferred_cuda_device`: `147-149`
- backend/indicator route helpers:
  - `crates/app_core/src/double_ma_backend.rs:19-22`
- optimization-mode selection:
  - `crates/app_core/src/double_ma_mode.rs:24-26`
- result assembly:
  - `crates/app_core/src/double_ma_result.rs:27`
- strategy plan/dynamic runtime selection:
  - `crates/app_core/src/strategy_dynamic.rs`
  - `crates/app_core/src/strategy_plan.rs`
- GPU runtime/cache ownership:
  - `crates/app_core/src/gpu_backtest_kernel.rs`
- validation consumers and DTO parity surfaces:
  - `crates/app_core/src/double_ma_validate.rs`
  - composite-family callers also rely on the backend contract shape from this file

### Downstream verification impact if changed

- backend helper changes affect:
  - Double-MA optimization
  - composite CPU/composite MO code paths that normalize backend/device selection
- follow-up cache changes affect:
  - drilldown/local-stability latency and correctness
- dynamic GPU cache/materialization changes affect:
  - long desktop sessions
  - GPU benchmarks
  - dynamic-strategy optimization throughput and VRAM usage

## Standards And Research Log

This packet was written after targeted web research focused on public Rust API clarity, documentation expectations for large public modules, recoverable error handling, and CUDA optimization guidance for cache and residency-sensitive GPU paths.

### Sources consulted

1. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this file exposes a large public request/backend surface
   - Standard informed:
     - public behavior should be predictable and not silently rewrite caller intent

2. Rust API Guidelines, Documentation
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why it was consulted:
     - many public enums/structs/functions in this file rely on test knowledge rather than rustdoc
   - Standard informed:
     - public contract surfaces should document route policy, fallback semantics, and caveats

3. Rust Book, Error Handling
   - Source: <https://doc.rust-lang.org/book/ch09-00-error-handling.html>
   - Why it was consulted:
     - this file is a `Result<_, String>`-heavy runtime surface that encodes many fallback and route-error conditions
   - Standard informed:
     - important route and fallback behavior should surface as clear, intentional contract behavior

4. NVIDIA CUDA C++ Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
   - Why it was consulted:
     - this file owns several GPU/hybrid execution and cache paths where memory growth and unnecessary host materialization matter
   - Standard informed:
     - cache growth, host-device transfer patterns, and materialization choices should be benchmarked and bounded

## Findings

## Correctness And Safety

### 1. `with_preferred_cuda_device` appears to overwrite explicit GPU `device_id` requests instead of only filling `Auto`

- Evidence:
  - `Backend::with_resolved_preferred_cuda_device` ignores the existing `device_id` payload for every GPU-capable variant:
    - `crates/app_core/src/double_ma.rs:125-144`
  - for `GpuOnly`, `GpuHybridKernel`, and `GpuKernel`, it always substitutes:
    - `preferred_device_id.unwrap_or(0)`: `135-143`
  - the helper is then used during request normalization:
    - `4989`
  - the same `Backend` type is used as a public contract, so callers can reasonably expect explicit `device_id` values to be preserved
- Why it matters:
  - a caller that deliberately requested `GpuKernel { device_id: 7 }` appears to be silently redirected to the preferred device instead
  - this is a public contract issue, not just an internal convenience
- Smallest justified fix:
  - preserve explicit `device_id` for `GpuOnly`, `GpuHybridKernel`, and `GpuKernel`
  - only inject the preferred device for `Auto`
  - add a focused test that locks this behavior
- Review judgment:
  - highest-priority finding in this file
  - semantics-sensitive, not benchmark-sensitive

### 2. Drilldown accepts `fast_len >= slow_len` even though the wider Double-MA surface treats that pair as invalid

- Evidence:
  - drilldown checks only positivity and bounds:
    - `crates/app_core/src/double_ma.rs:4496-4501`
  - local stability explicitly rejects `fast_len >= slow_len`:
    - `4678`
  - the shared Double-MA strategy/grid invariant is `fast < slow`:
    - `crates/strategies/src/double_ma/mod.rs:4255`
- Why it matters:
  - drilldown can compute curves for a parameter pair that cannot come from a valid optimization result
  - that makes the public follow-up surface wider and less coherent than the optimization surface it is supposed to inspect
- Smallest justified fix:
  - add the same `fast_len < slow_len` guard to drilldown
  - add a focused unit test for the invalid pair
- Review judgment:
  - medium-severity public-contract bug
  - semantics-sensitive

### 2. Follow-up cache reuse is keyed too weakly to prove candle identity

- Evidence:
  - `FollowupCacheSignature` includes:
    - `data_id`
    - `num_candles`
    - `ma_source`
    - MA types
    - strategy
    - halftrend filter
    - `crates/app_core/src/double_ma.rs:9840-9849`
  - it does not include any direct candle identity such as timestamp range or content fingerprint
  - the key is persisted and reused later for drilldown/local-stability follow-up work:
    - `7525-7536`
    - `9872-9912`
- Why it matters:
  - if candle contents change while `data_id` and `num_candles` stay the same, cached log returns/regime filters/Kaufman-stop series can be reused against the wrong dataset
  - this is stale-data risk, not just a cache-hit optimization detail
- Smallest justified fix:
  - extend the signature with a cheap candle identity signal such as first/last timestamp plus last close, or a dataset-version token from the owning runtime
- Review judgment:
  - real correctness/contract issue
  - semantics-sensitive

## API And Naming

### 1. The public surface is broad, but key route/fallback semantics are encoded mostly in tests and strings

- Evidence:
  - core public backend and request types live here:
    - `Backend`: `105-149`
    - `DoubleMaRequest`: `273-327`
  - route and fallback strings carry important semantic meaning:
    - `5485-5495`
    - `5958`
    - `6113-6114`
    - `12860-12929`
  - many public behaviors are locked by tests rather than rustdoc:
    - `14121+`
- Why it matters:
  - consumers should not have to reverse-engineer route semantics from fallback strings or test names
- Smallest justified fix:
  - add rustdoc for the public backend enum, request type, GPU execution policy, and main entrypoints
- Review judgment:
  - safe-now docs improvement

### 2. Follow-up requests duplicate MA params in two places, but the implementation only honors one of them

- Evidence:
  - `DoubleMaParamsResolved` already carries `fast_ma_params` / `slow_ma_params`:
    - `crates/app_core/src/double_ma.rs:441-451`
  - both follow-up request types repeat those fields at top level:
    - drilldown request: `453-463`
    - local-stability request: `471-481`
  - follow-up implementations read only the top-level copies for MA generation and ALMA resolution:
    - drilldown: `4591`, `4611`
    - local stability: `4797`, `4816`
  - `LocalStabilityResult.base` also echoes the top-level copies back out:
    - `4908`
- Why it matters:
  - a caller that naturally round-trips `best_params` or `top[*].0` into a follow-up request can lose MA params unless it knows to duplicate them manually
  - that is an API-shape footgun caused by duplicated state
- Smallest justified fix:
  - if the top-level follow-up fields are absent, fall back to `req.params.fast_ma_params` / `req.params.slow_ma_params`
  - add a round-trip test that builds follow-up requests directly from `DoubleMaParamsResolved`
- Review judgment:
  - medium-severity public-contract issue
  - semantics-sensitive

## Error Handling

### 1. Follow-up cache poisoning is silently collapsed into cache miss/no-op behavior

- Evidence:
  - cache reads use:
    - `.lock().ok().and_then(...)`: `crates/app_core/src/double_ma.rs:9977-9980`
  - cache writes are dropped silently if the lock is poisoned:
    - `9983-9990`
- Why it matters:
  - cache corruption or lock poisoning becomes invisible operationally
  - the result is not a crash, but it removes signal when the cache stops behaving as expected
- Smallest justified fix:
  - at minimum, log or propagate a cache-poison note in debug/test paths
  - if silent degradation is intentional, document that the cache is best-effort only
- Review judgment:
  - low-severity observability issue

## Module Boundaries And Technical Debt

### 1. The bounded follow-up cache does not actually evict the oldest entry

- Evidence:
  - eviction logic names `oldest_key`:
    - `crates/app_core/src/double_ma.rs:9985-9988`
  - but it uses `HashMap::keys().next()`, which is not insertion-ordered
- Why it matters:
  - once more than 16 entries are present, cache retention becomes arbitrary rather than chronological
  - that makes follow-up cache reuse less predictable and harder to reason about
- Smallest justified fix:
  - either rename the variable/comment to reflect arbitrary eviction
  - or use an ordered/LRU structure if “oldest” is really the intended policy
- Review judgment:
  - small but real debt
  - benchmark-sensitive for follow-up-heavy workflows

## Docs And Examples

### 1. The key public surfaces need rustdoc coverage

- Evidence:
  - important public items with no rustdoc include:
    - `Backend`: `105`
    - `DoubleMaRequest`: `273`
    - drilldown/local-stability entrypoints: `4482`, `4654`
    - main optimization entrypoints: `4925`, `4975`
- Why it matters:
  - this file defines many product-level backend semantics:
    - route normalization
    - fallback behavior
    - export restrictions
    - GPU execution policy semantics
    - follow-up cache persistence
- Smallest justified fix:
  - document the public surfaces that are meant to be stable inputs/outputs
- Review judgment:
  - safe-now docs improvement

## Tests And Missing Verification

### 1. Coverage is extensive, but the explicit-device-preservation contract is not obviously locked

- Evidence:
  - there is a test for propagated resolver errors:
    - `crates/app_core/src/double_ma.rs:14745-14749`
  - there is no nearby test proving that explicit `device_id` values survive `with_preferred_cuda_device`
- Why it matters:
  - the public contract bug above can persist unnoticed because route tests mostly run with device `0`
- Smallest justified fix:
  - add a unit test that passes `GpuKernel { device_id: 7 }` plus `Ok(Some(3))` and asserts the result stays on `7` unless the backend was `Auto`

### 2. Cache correctness/perf contracts are only partially tested

- Evidence:
  - there is coverage for dynamic GPU preflight cache reuse:
    - `17600-17633`
  - but no direct tests were found for:
    - follow-up cache stale-data identity
    - follow-up cache eviction order
    - dynamic GPU batch cache boundedness
- Why it matters:
  - these are long-session or repeat-workflow behaviors that can drift without affecting smoke tests
- Review judgment:
  - add focused cache-contract tests where feasible

## Performance, Allocation, And CUDA Concerns

### 1. The dynamic GPU batch cache is unbounded and retains uploaded device buffers across requests

- Evidence:
  - cache is a thread-local `HashMap<String, Rc<UploadedDoubleMaDynamicProgramBatch>>`:
    - `crates/app_core/src/double_ma.rs:45-49`
  - entries are inserted without eviction:
    - `13322-13357`
  - cache clear is manual:
    - `51-60`
  - uploaded batches own GPU-resident buffers in `gpu_backtest_kernel.rs`
- Why it matters:
  - VRAM and host memory can grow monotonically over long desktop sessions or benchmark sweeps
  - this is not a stylistic issue; it is a lifetime/boundedness issue
- Smallest justified fix:
  - add an entry cap or session/request-scoped eviction policy
  - alternatively clear the cache after each dynamic GPU optimization run if reuse is not critical
- Review judgment:
  - high-priority benchmark/runtime finding
  - benchmark-sensitive

### 2. `optimize_double_ma_dynamic_gpu_kernel_tiled` defeats the streaming aggregator model by materializing all passing rows

- Evidence:
  - full filtered materialization:
    - `crates/app_core/src/double_ma.rs:13577-13581`
  - second pass clones the whole vector back through `StreamAggregator`:
    - `13583-13601`
  - the file otherwise uses stream-based aggregation patterns elsewhere
- Why it matters:
  - host memory and copy cost scale with all passing pairs even when the caller only needs top-k/heatmap outputs
  - this is exactly the kind of benchmark-sensitive shape change that can matter on large dynamic GPU runs
- Smallest justified fix:
  - materialize full rows only when extra shortlist/confidence/slice reranking requires them
  - otherwise push directly into the aggregator
- Review judgment:
  - real benchmark-sensitive performance opportunity

### 3. Follow-up cache hits still clone full series payloads

- Evidence:
  - `FollowupCacheEntry` owns `Vec<f64>` payloads:
    - `crates/app_core/src/double_ma.rs:63-70`
  - lookups clone the entire entry:
    - `9972-9980`
- Why it matters:
  - drilldown/local-stability skip recomputation, but still pay O(n) copy/allocation cost on every cache hit
- Smallest justified fix:
  - move cached heavy payloads to shared ownership such as `Arc<[f64]>` or similar
- Review judgment:
  - medium-severity perf debt
  - benchmark-sensitive

## Code-Quality Improvement Register

- Safe now:
  - add rustdoc to public request/backend/entrypoint surfaces
  - add explicit test coverage for device-ID preservation and cache contracts
  - fix or rename the misleading `oldest_key` eviction logic
- Coordination required:
  - preserve explicit GPU device IDs instead of overwriting them with the preferred device
  - strengthen follow-up cache identity
- Defer until later pass:
  - broader decomposition of this file into smaller runtime and cache modules

## Technical-Debt Reduction Register

- Safe to reduce now:
  - misleading eviction naming/behavior
  - missing rustdoc
  - missing direct tests for key cache semantics
- Coordination required:
  - stale-data cache key strengthening
  - any change to device-selection semantics
- Useful debt that should not be “cleaned up” casually:
  - rich route/fallback diagnostics
  - extensive in-file test coverage that currently protects many GPU/hybrid policies

## Performance Opportunity Register

- Safe structural improvements:
  - bound the dynamic GPU batch cache
  - avoid unnecessary full-row materialization when shortlist reranking is not requested
  - reduce clone cost on follow-up cache hits
- Benchmark-sensitive candidates:
  - dynamic GPU cache lifetime
  - aggregator materialization strategy
  - follow-up cache ownership model
- Not the right place to chase performance casually:
  - broad route/fallback refactors without perf gates or parity verification

## Observability And Diagnostics Gaps

- explicit-device overwrite is not surfaced to callers
- follow-up cache poison degrades silently
- stale-data reuse risk is not visible in the follow-up cache token

## Implementation Staging Guidance

- Safe now:
  - docs
  - tests
  - eviction-logic cleanup
- Coordination required:
  - explicit-device preservation
  - stronger follow-up cache identity
- Defer until later pass:
  - larger file decomposition or sweeping route-logic consolidation

## Change Blast Radius

- Local only:
  - docs and some tests
- Same subsystem:
  - follow-up cache semantics
  - dynamic GPU cache/materialization changes
- Cross-backend/runtime:
  - `Backend` device-ID preservation affects other callers that share this type and helper

## Recommended Action

`Small surgical refactor`

This file does not need a rewrite-first response. The right next pass is:

- fix the explicit-device contract bug
- harden cache identity and cache boundedness
- keep benchmark-sensitive GPU materialization changes narrow and measured
- document the public route/fallback surface

## Exact Local Verification Required If Changed

If only docs or small local tests change:

- `cargo test -p ta_app_core preferred_cuda_device_resolution_error_is_not_masked -- --nocapture`

If `Backend` device-resolution semantics change:

- add and run a dedicated unit test for explicit device-ID preservation
- rerun route-selection tests around Auto/GpuOnly/GpuHybridKernel/GpuKernel

If follow-up cache semantics change:

- `cargo test -p ta_app_core drilldown_smoke_has_bins_and_values -- --nocapture`
- `cargo test -p ta_app_core local_stability_smoke_has_base_point_and_summary -- --nocapture`

If follow-up request contract handling or drilldown invariants change:

- add and run a round-trip test that constructs follow-up requests from `DoubleMaParamsResolved`
- add and run a drilldown invalid-pair test for `fast_len >= slow_len`

If dynamic GPU cache/materialization changes:

- `cargo test -p ta_app_core dynamic_gpu_preflight_cache_reuses_identical_request_signature -- --nocapture`
- targeted GPU perf gate or representative dynamic GPU benchmark on a CUDA-enabled machine

## Regression Risk And Feature-Preservation Constraints

The intended outcome is better backend correctness and safer long-session GPU behavior with no feature loss.

- do not remove rich backend route/fallback diagnostics
- do not remove follow-up cache support outright unless a replacement is provided
- do not narrow supported GPU/hybrid routes without parity and benchmark verification
- if cache behavior is tightened, preserve correctness and reuse for valid repeated follow-up workflows
