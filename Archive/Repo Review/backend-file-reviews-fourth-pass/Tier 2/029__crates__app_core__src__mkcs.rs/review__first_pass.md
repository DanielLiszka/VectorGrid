# Review Packet: `crates/app_core/src/mkcs.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/mkcs.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Operationally useful and strategy-local, but not fully public-review ready because invalid desired-position inputs silently degrade to flat output, the GPU-preferred signal path silently falls back to CPU on CUDA failure, dynamic output resolution can mask indicator-catalog drift, and the public CUDA sweep types expose raw device-pointer handles with little contract documentation`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Public-surface`, `Strategy-local`, `CUDA-adjacent`, `Benchmark-sensitive`, `Semantics-sensitive`

## What This File Owns

`mkcs.rs` owns MKCS strategy configuration, validation, signal generation, CUDA sweep construction, and desired-position generation for the MKCS strategy family.

It owns:

- public MKCS strategy configuration and defaults:
  - `MkcsStrategyParams`: `crates/app_core/src/mkcs.rs:22-65`
- resolved-period and signal carrier types:
  - `MkcsResolvedPeriods`: `71-74`
  - `MkcsCompressionSignals`: `77-80`
  - `MkcsBreakoutSignals`: `83-87`
- CUDA/device sweep carrier types:
  - `MkcsDonchianSweepsDevice`: `90-93`
  - `MkcsDeviceMatrix`: `113-140`
  - `MkcsPeriodSweepDevice`: `144-176`
- config validation and period resolution:
  - `validate_mkcs_params`: `180-197`
  - `resolve_mkcs_periods`: `199-216`
- dynamic-indicator descriptor/output/parameter resolution:
  - `resolve_output_id`: `254-290`
  - `resolve_param_key`: `292-340`
  - cached indicator-id map:
    - `mkcs_indicator_ids`: `342-376`
- CUDA sweep builders:
  - `compute_indicator_period_sweep_cuda_device`: `384-429`
  - `compute_mkcs_stddev_sweep_cuda_device`: `434-451`
  - `compute_mkcs_donchian_sweeps_cuda_device`: `455-484`
  - `compute_mkcs_donchian_output_sweep_cuda_device`: `489-517`
- CPU/GPU single-indicator signal generation:
  - `compute_indicator_f64`: `520-569`
  - `compute_mkcs_compression_signals`: `571-599`
  - `compute_mkcs_breakout_signals`: `601-648`
- strategy start index and desired-position logic:
  - `compute_mkcs_start_t`: `650-668`
  - `build_mkcs_desired_positions`: `679-777`
- file-local tests:
  - `779-875`

This file does not own:

- the MKCS GPU-kernel evaluator itself:
  - `crates/app_core/src/mkcs_gpu_kernel.rs`
- composite optimization orchestration:
  - `crates/app_core/src/composite_cpu.rs`
- composite validation orchestration:
  - `crates/app_core/src/composite_validate.rs`

## Classification

- Public surface: yes
  - the module is public at `crates/app_core/src/lib.rs:43`
  - `MkcsStrategyParams` is re-exported at `crates/app_core/src/lib.rs:125`
- Hot path: yes, partially
  - the signal builders and desired-position loop sit on optimization and validation execution paths
- CUDA/runtime-critical: indirectly yes
  - this file is not the kernel wrapper, but it owns CUDA sweep construction and device-pointer transport used by the MKCS GPU path
- Strategy-local: yes
  - this file is specific to MKCS
- Low-risk support code: no
  - it is strategy-local infrastructure with contract and performance consequences

## Direct Interactions

### Backend interaction map

- crate export surface:
  - `crates/app_core/src/lib.rs:43`
  - `125`
- composite optimization imports and usage:
  - imports: `crates/app_core/src/composite_cpu.rs:114-119`
  - Trade22 evaluation: `954-1007`
  - main MKCS optimization validation gate: `7454`
  - CUDA sweep usage: `7515`
  - `7544`
  - MKCS evaluation path in optimization loop: `10595`
  - signal generation and desired positions: `10691-10733`
- composite validation imports and usage:
  - imports: `crates/app_core/src/composite_validate.rs:82-88`
  - CUDA sweep usage: `2436-2437`
  - `2622-2628`
  - `2850-2851`
  - validation gate for MKCS params: `10694`
  - signal generation and desired positions: `10727-10755`
- MKCS GPU-kernel evaluator consumes MKCS types and config:
  - `crates/app_core/src/mkcs_gpu_kernel.rs:10`
  - `352`
- desktop-level smoke wiring reaches MKCS defaults through validation commands:
  - `src-tauri/src/commands_validation.rs:2808`

### Downstream verification impact if changed

- changing `MkcsStrategyParams` fields or defaults affects:
  - composite optimization requests
  - validation requests
  - any Tauri/default-config surface using MKCS defaults
- changing CUDA sweep or raw-pointer carrier types affects:
  - `mkcs_gpu_kernel.rs`
  - composite CPU GPU-route callers
  - composite validation GPU-route callers
- changing desired-position behavior affects:
  - optimization metrics
  - validation metrics
  - benchmark parity against the GPU kernel route

## Standards And Research Log

This packet was written after targeted web research focused on public Rust API design, Serde request evolution, `OnceLock` initialization behavior, recoverable-error conventions, unsafe/raw-handle boundary guidance, and CUDA guidance around keeping intermediate data resident on device.

### Sources consulted

1. Rust API Guidelines Checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this file exposes public config types and public CUDA-adjacent transport types
   - Standard informed:
     - public types should encode clear contracts and avoid broader exposure than necessary

2. Serde field attributes
   - Source: <https://serde.rs/field-attrs.html>
   - Why it was consulted:
     - `MkcsStrategyParams` is a serialized config surface with defaults
   - Standard informed:
     - defaults and omitted fields are part of the public contract and should remain intentional

3. `OnceLock` documentation
   - Source: <https://doc.rust-lang.org/std/sync/struct.OnceLock.html>
   - Why it was consulted:
     - the file caches indicator descriptors/keys in `OnceLock<Result<...>>`
   - Standard informed:
     - one-time initialization semantics and failure behavior are persistent contract decisions, not just implementation details

4. Rust Book, Recoverable Errors With `Result`
   - Source: <https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html>
   - Why it was consulted:
     - this file mixes explicit `Result`-based validation with one path that silently returns flat output
   - Standard informed:
     - recoverable input/shape problems should not silently collapse into apparently valid business results unless that is a deliberate documented policy

5. Rustonomicon
   - Source: <https://doc.rust-lang.org/nomicon/>
   - Why it was consulted:
     - the module exposes raw device-pointer-style handles through safe public types
   - Standard informed:
     - unsafe-adjacent contracts should be explicit, and safe wrappers should not hide invariants they cannot actually enforce

6. `NonZeroU64` documentation
   - Source: <https://doc.rust-lang.org/std/num/struct.NonZeroU64.html>
   - Why it was consulted:
     - pointer-like `u64` fields may be candidates for stronger handle typing if the public surface remains exposed
   - Standard informed:
     - opaque/non-zero handle types can encode at least some invariants without changing wire size dramatically

7. NVIDIA CUDA Best Practices Guide, data transfer guidance
   - Source: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html#data-transfer-between-host-and-device>
   - Why it was consulted:
     - this file builds device-resident sweeps and hands row pointers to GPU callers
   - Standard informed:
     - keeping reusable intermediate data resident on device is the correct performance posture and should not be regressed casually

## Findings

## Correctness And Safety

### 1. `build_mkcs_desired_positions` silently degrades invalid inputs into an all-flat position vector

- Evidence:
  - the function allocates a zeroed result immediately:
    - `crates/app_core/src/mkcs.rs:687-688`
  - it returns that flat vector when `start_t >= n`:
    - `689-690`
  - it also returns that flat vector on compression/breakout length mismatch:
    - `692-694`
- Why it matters:
  - invalid warmup/index wiring or mismatched signal lengths become indistinguishable from a legitimate “stay flat” trading result
  - that is materially different from the rest of the file, which uses `Result<_, String>` for most configuration and signal-building failures
- Smallest justified fix:
  - keep the existing function for compatibility if needed
  - add a checked variant that returns `Result<Vec<i32>, String>` and have higher-risk callers use it
  - at minimum, add `debug_assert!` coverage and tests around the fallback behavior if it is intentionally kept
- Review judgment:
  - medium-high correctness/contract issue
  - semantics-sensitive because some callers may implicitly rely on the current flat-output fallback

### 2. The GPU-preferred single-indicator path silently falls back to CPU when CUDA execution fails

- Evidence:
  - GPU preference is chosen when backend and descriptor metadata allow it:
    - `crates/app_core/src/mkcs.rs:539-541`
  - the CUDA path is attempted:
    - `544-551`
  - if CUDA execution returns `Err`, the error is ignored and the function falls through to CPU execution:
    - `552-556`
  - CPU execution is then used as if no GPU issue occurred:
    - `559-569`
- Why it matters:
  - this may be an intentional resiliency policy, but it also hides route failures from callers and makes performance/regression interpretation ambiguous
  - a nominally GPU-preferred run can become CPU-backed without explicit diagnostic signal
- Smallest justified fix:
  - keep the CPU fallback if product policy wants resiliency
  - add a surfaced warning/diagnostic path when a GPU-preferred attempt fails and CPU fallback is taken
  - avoid turning this into a hard error without checking downstream route expectations
- Review judgment:
  - medium correctness/observability issue

## API And Naming

### 1. Public CUDA sweep types expose raw device-pointer handles from a safe API boundary

- Evidence:
  - `MkcsDeviceMatrix` is public and exposes:
    - `device_ptr`
    - `rows`
    - `cols`
    - `device_id`
    - `crates/app_core/src/mkcs.rs:113-118`
  - `MkcsPeriodSweepDevice` is public and exposes the raw matrix plus range metadata:
    - `144-149`
  - `row_ptr_for_period` returns raw row pointers as `u64`:
    - `165-176`
  - the module itself is publicly exported:
    - `crates/app_core/src/lib.rs:43`
  - in-repo callers appear to be internal GPU-route code:
    - `crates/app_core/src/composite_cpu.rs`
    - `crates/app_core/src/composite_validate.rs`
    - `crates/app_core/src/mkcs_gpu_kernel.rs`
- Why it matters:
  - the type system does not express lifetime, device-affinity, layout, or ownership constraints for these pointers
  - if the surface is truly internal, the current public exposure is broader than justified
- Smallest justified fix:
  - audit whether any crate-external caller truly needs these types
  - if not, narrow them to `pub(crate)`
  - if public exposure must remain, document invariants explicitly and consider a stronger handle wrapper than bare `u64`
- Review judgment:
  - high public-surface/API issue

### 2. `resolve_output_id` is much more permissive than `resolve_param_key`, which can hide indicator-catalog drift

- Evidence:
  - `resolve_output_id` falls back to the first declared output if preferred-name matching fails:
    - `crates/app_core/src/mkcs.rs:254-289`
  - `resolve_param_key` does not do the equivalent for ambiguous multi-param descriptors and errors instead:
    - `292-340`
- Why it matters:
  - output-resolution drift can silently choose a different output series than intended, while parameter-resolution drift is treated as an explicit error
  - the asymmetry makes catalog compatibility issues harder to notice
- Smallest justified fix:
  - if the fallback is intentional, document it and add tests for the exact intended output-selection policy
  - otherwise, tighten `resolve_output_id` to fail on ambiguous descriptors in the same way `resolve_param_key` already does
- Review judgment:
  - medium API / contract issue

## Error Handling

### 1. `OnceLock<Result<...>>` permanently caches indicator-id initialization failures

- Evidence:
  - `mkcs_indicator_ids` uses:
    - `static IDS: OnceLock<Result<MkcsIndicatorIds, String>> = OnceLock::new();`
    - `crates/app_core/src/mkcs.rs:342-343`
  - initialization is performed via `get_or_init(...)`:
    - `345-373`
  - the cached `Result` is then cloned back out:
    - `374-375`
- Why it matters:
  - the first failure becomes sticky for the process lifetime, even if the underlying registry state or feature availability would later be corrected
  - that may be acceptable, but it is a real error-handling policy choice that should be explicit
- Smallest justified fix:
  - if startup-time failure should be permanent, document that contract
  - otherwise, cache successful resolution only, or add a controlled reset/test seam rather than baking failure persistence into the steady-state path
- Review judgment:
  - medium error-handling / initialization-policy issue

## Module Boundaries And Technical Debt

### 1. This file provides explicit parameter validation, but one downstream optimization path bypasses it

- Evidence:
  - the file provides a dedicated validator:
    - `crates/app_core/src/mkcs.rs:180-197`
  - main optimization and validation paths call it:
    - `crates/app_core/src/composite_cpu.rs:7454`
    - `10595`
    - `crates/app_core/src/composite_validate.rs:2372`
    - `2592`
    - `2774`
    - `10694`
  - `evaluate_mkcs_trade22_metrics` consumes MKCS config directly but does not call `validate_mkcs_params`:
    - `crates/app_core/src/composite_cpu.rs:954-1004`
- Why it matters:
  - the file’s validation contract is not enforced uniformly
  - that weakens confidence that all MKCS callers see the same invariants before signal generation and desired-position construction
- Smallest justified fix:
  - add validation at the Trade22 caller before using MKCS config
  - keep the validation logic centralized in this file rather than duplicating rules downstream
- Review judgment:
  - medium module-boundary / contract-consistency issue

### 2. Feature-gated CUDA metadata currently leaves warning-level cleanup debt in non-CUDA builds

- Evidence:
  - top-level import includes `IndicatorInputKind` unconditionally:
    - `crates/app_core/src/mkcs.rs:10-11`
  - CPU-only verification emits:
    - unused import warning for `IndicatorInputKind`
    - unused local warnings for `stddev_descriptor` and `donchian_descriptor`
  - those locals are only materially used for `#[cfg(feature = "cuda")]` fields:
    - `346-371`
- Why it matters:
  - this is low severity, but it adds noise to warning surfaces and makes real issues harder to spot
- Smallest justified fix:
  - gate the import and descriptor locals more tightly or underscore-prefix CPU-only-unused locals
- Review judgment:
  - low code-quality / warning-debt issue

## Docs And Examples

### 1. The public MKCS types and CUDA-adjacent helpers lack rustdoc for invariants and fallback policy

- Evidence:
  - public items include:
    - `MkcsStrategyParams`: `crates/app_core/src/mkcs.rs:22`
    - `MkcsResolvedPeriods`: `71`
    - `MkcsCompressionSignals`: `77`
    - `MkcsBreakoutSignals`: `83`
    - `MkcsDonchianSweepsDevice`: `90`
    - `MkcsDeviceMatrix`: `113`
    - `MkcsPeriodSweepDevice`: `144`
    - `validate_mkcs_params`: `180`
    - `resolve_mkcs_periods`: `199`
    - `compute_mkcs_stddev_sweep_cuda_device`: `434`
    - `compute_mkcs_donchian_sweeps_cuda_device`: `455`
    - `compute_mkcs_donchian_output_sweep_cuda_device`: `489`
    - `compute_mkcs_compression_signals`: `571`
    - `compute_mkcs_breakout_signals`: `601`
    - `compute_mkcs_start_t`: `650`
    - `build_mkcs_desired_positions`: `679`
  - there is no rustdoc explaining:
    - public device-pointer invariants
    - when CPU fallback is expected
    - whether flat-output fallback is deliberate behavior or defensive degradation
    - whether descriptor fallback to first output is intentional compatibility policy
- Smallest justified fix:
  - add rustdoc to the public types and helpers
  - document invariants and fallback behavior explicitly rather than leaving them discoverable only by reading the implementation

## Tests And Missing Verification

### 1. Local functional coverage is present, but important fallback and perf surfaces are either untested or stale

- Evidence:
  - local CPU verification run:
    - `cargo test -p ta_app_core mkcs::tests:: -- --nocapture`
      - passed
      - included:
        - `validate_mkcs_params_rejects_invalid_values`
        - `build_mkcs_desired_positions_enters_and_exits_on_breakout_rules`
  - optimization smoke verification run:
    - `cargo test -p ta_app_core mkcs_optimization_smoke_runs -- --nocapture`
      - passed
  - CUDA signal smoke verification run:
    - `cargo test -p ta_app_core mkcs::tests::compute_mkcs_signals_gpu_smoke_runs_with_stddev_and_donchian_surfaces --features cuda -- --nocapture`
      - passed
  - attempted benchmark compile verification:
    - `cargo bench -p ta_app_core --bench mkcs_opt --no-run`
      - failed
      - concrete failure:
        - `crates/app_core/benches/mkcs_opt.rs:47`
        - `CompositeCpuRequest` initializer is missing `include_trade_metrics` and `metric_contract`
- Missing tests I would want before changing this file:
  - a test that pins the intended behavior of `build_mkcs_desired_positions` on invalid `start_t` and series-length mismatch
  - a test that verifies GPU-preferred failures produce whatever diagnostics the project intends
  - a descriptor-resolution test that proves `resolve_output_id` fallback is correct if it remains permissive
  - a repaired MKCS bench surface that can actually be used for regression measurement

## Performance, Allocation, And CUDA Concerns

### 1. The current CUDA sweep design is aligned with the right performance posture and should not be regressed casually

- Evidence:
  - period sweeps are built as device-resident matrices:
    - `crates/app_core/src/mkcs.rs:384-429`
  - MKCS exports reusable sweep carriers rather than forcing host-materialized rows:
    - `90-176`
  - MKCS GPU-route callers consume row pointers from those sweeps:
    - `crates/app_core/src/composite_cpu.rs:7515`
    - `7544`
    - `crates/app_core/src/composite_validate.rs:2436-2437`
    - `2622-2628`
    - `2850-2851`
- Why it matters:
  - this matches the CUDA best-practice direction of keeping reusable intermediate data resident on device instead of bouncing back to host
- Review judgment:
  - no structural performance regression should be introduced here in the name of simplification

### 2. Public raw-pointer API changes, GPU-fallback policy changes, and sweep-ownership changes are benchmark-sensitive

- Evidence:
  - row-pointer access is central to downstream GPU routes:
    - `crates/app_core/src/mkcs.rs:165-176`
  - GPU fallback policy is part of the live signal-building path:
    - `539-556`
  - the dedicated MKCS bench surface is intended to exist even though it is currently stale:
    - `crates/app_core/benches/mkcs_opt.rs`
    - `crates/app_core/benches/gpu_native_expansion_opt.rs:500-589`
- Review judgment:
  - do not change pointer-carrier shape, fallback policy, or transfer behavior without re-enabling and running the MKCS perf benches

## Code-Quality Improvement Register

- Safe now:
  - add rustdoc for public types and fallback behavior
  - tighten feature gating to remove warning noise
  - add tests around invalid-input desired-position behavior
  - repair the stale `mkcs_opt` bench initializer
- Coordination required:
  - narrowing public CUDA transport visibility
  - surfacing GPU fallback diagnostics if downstream route consumers depend on silent resiliency today
  - enforcing validation consistently in the Trade22 path
- Defer until later pass:
  - any broader redesign of raw-pointer carrier types or desired-position API shape

## Technical-Debt Reduction Register

- Safe to reduce now:
  - missing rustdoc
  - CPU-build warning noise
  - stale benchmark harness
- Coordination required:
  - public raw-handle exposure
  - validation-consistency across all MKCS call sites
  - output-resolution strictness policy
- Useful debt that should not be “cleaned up” casually:
  - device-resident sweep ownership, which is performance-relevant
  - strategy-local separation of config validation, signal generation, and desired-position logic

## Performance Opportunity Register

- Safe structural improvements:
  - restore the dedicated MKCS bench surface so changes can be measured
- Benchmark-sensitive candidates:
  - altering CPU fallback policy for GPU failures
  - redesigning pointer carriers or sweep ownership
  - host-staging any currently device-resident sweep path
- Not the right place to chase performance without evidence:
  - refactoring the sweep/device types purely for aesthetics if it increases transfers or weakens GPU reuse

## Observability And Diagnostics Gaps

- GPU-preferred failure currently does not surface a warning when CPU fallback is taken
- `resolve_output_id` fallback does not report when it had to settle for the first declared output
- flat-output fallback in `build_mkcs_desired_positions` has no explicit diagnostic path

## Implementation Staging Guidance

- Safe now:
  - docs
  - warning cleanup
  - benchmark harness repair
  - additive tests
- Coordination required:
  - public API narrowing for device-pointer types
  - route diagnostics for GPU fallback
  - downstream validation-consistency fixes
- Defer until later pass:
  - redesigning desired-position error semantics or raw-handle wrappers

## Change Blast Radius

- Local only:
  - docs
  - warning cleanup
  - additive tests
- Same subsystem:
  - validation-consistency changes affect MKCS callers in `composite_cpu.rs` and `composite_validate.rs`
  - device-sweep API changes affect `mkcs_gpu_kernel.rs`
- Cross-backend/runtime:
  - fallback policy or pointer-carrier changes affect CPU/GPU route behavior and performance interpretation

## Open Questions Or Assumptions

- assumption:
  - the public CUDA sweep/device types are intended for internal crate/workspace use rather than a broad external API
- open question:
  - whether silent flat-output fallback is a deliberate product policy or just defensive coding convenience
- open question:
  - whether silent CPU fallback on GPU failure is expected UX behavior or simply an implementation shortcut

## Recommended Action

`Small surgical refactor`

The right next pass is:

- document the current contracts and fallback behavior
- repair the stale benchmark surface
- tighten the lowest-risk API and warning debt
- defer semantics-sensitive changes such as raw-handle redesign or fallback-policy changes until the downstream MKCS GPU packet and caller packets are reviewed together

## Exact Local Verification Required If Changed

- `cargo test -p ta_app_core mkcs::tests:: -- --nocapture`
- `cargo test -p ta_app_core mkcs_optimization_smoke_runs -- --nocapture`
- `cargo test -p ta_app_core mkcs::tests::compute_mkcs_signals_gpu_smoke_runs_with_stddev_and_donchian_surfaces --features cuda -- --nocapture`
- if CUDA sweep/device types or GPU fallback policy change:
  - `cargo test -p ta_app_core mkcs_optimization_gpu_kernel_matches_cpu_small_grid --features cuda,cuda-backtest-kernel -- --nocapture`
  - `cargo test -p ta_app_core mkcs_validation_gpu_kernel_matches_cpu_metrics --features cuda,cuda-backtest-kernel -- --nocapture`
- repair and re-run:
  - `cargo bench -p ta_app_core --bench mkcs_opt --no-run`

## Regression Risk And Feature-Preservation Constraints

- do not remove or narrow public MKCS config fields without checking all composite request surfaces
- do not change sweep/device ownership in a way that reintroduces host staging for GPU routes
- do not change flat-output or GPU-fallback semantics silently; if behavior changes, treat it as a deliberate contract change and verify downstream optimization/validation parity
