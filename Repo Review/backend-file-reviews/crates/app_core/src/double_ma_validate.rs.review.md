# Review Packet: `crates/app_core/src/double_ma_validate.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/double_ma_validate.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Powerful and heavily tested validation engine, but not public-review ready as-is because its exported app-core request/result DTOs currently advertise cross_ticker and validation_context semantics that are only actually enforced or populated in the Tauri/headless orchestration layer`
- Recommended action bucket: `Larger refactor but defer`
- Sensitivity tags: `Public-surface`, `Validation-critical`, `CUDA/runtime-critical`, `Cross-file-contract-sensitive`, `Semantics-sensitive`

## What This File Owns

`double_ma_validate.rs` is the main app-core validation engine for the Double-MA family. It owns the public validation request/result DTOs, the core blocking validation entrypoints, CPU/GPU fixed-candidate validation routing, and the in-core computation of validation metrics, robustness stats, and verdicts.

Major owned surfaces include:

- public DTO surface:
  - `ValidationExecutionProfile`: `crates/app_core/src/double_ma_validate.rs:234+`
  - `DoubleMaValidationParams`: `1476-1489`
  - `DoubleMaValidationRequest`: `1492-1535`
  - `DoubleMaValidationCandidateResult`: `1538-1580`
  - `DoubleMaValidationResult`: `1661-1681`
- backend-runtime routing helpers:
  - `resolve_validation_backend_runtime`: `59-93`
  - `validation_supports_gpu_fixed_candidates`: `108-119`
  - `gpu_validation_feature_blocker`: `130-142`
- core public entrypoints:
  - `run_double_ma_validation_blocking_with_candles`: `3406-3419`
  - `run_double_ma_validation_blocking_with_candles_with_aux_mask`: `3421-4055`
- very large in-file test surface:
  - `4176+`

This file does not fully own:

- `validation_context` enforcement against loaded dataset identity/timeframe:
  - `src-tauri/src/commands_validation.rs:131-166`
  - `587-602`
- cross-ticker orchestration and summary population:
  - `src-tauri/src/commands_validation.rs:632-668`
  - `675-911`
  - `src-tauri/src/bin/ta_headless_validate.rs:577-621`

That ownership split is the main reason this packet is not just a local cleanup review.

## Classification

- Public surface: yes
  - public request/result DTOs are re-exported from app-core
- Hot path: yes
  - validation execution and route selection
- CUDA/runtime-critical: high
  - GPU fixed-candidate route/fallback behavior is decided here
- Strategy-local: yes
  - Double-MA-family validation engine
- Low-risk support code: no

## Direct Interactions

### Backend interaction map

- app-core public export surface:
  - `crates/app_core/src/lib.rs:144-148`
- Tauri validation command wrapper:
  - `src-tauri/src/commands_validation.rs:578-673`
- headless validation wrapper:
  - `src-tauri/src/bin/ta_headless_validate.rs:572-628`
- direct/headless app-core consumers:
  - `crates/app_core/src/bin/real_data_fidelity_audit.rs:760`
  - `src-tauri/src/bin/ta_double_ma_validation_cost.rs:936`
  - `src-tauri/src/commands_meta.rs:2873`
  - `src-tauri/src/bin/ta_refactor_snapshots.rs:203`
- shared strategy validation gate:
  - `crates/app_core/src/double_ma_strategy.rs:3-381`

### Downstream verification impact if changed

- DTO changes affect:
  - app-core library callers
  - Tauri command serialization/deserialization
  - headless utilities
  - report and artifact consumers
- GPU blocker changes affect:
  - CPU/GPU route selection
  - validation perf posture
  - benchmark comparability
- runtime measurement changes affect:
  - CLI and desktop result comparability
  - downstream artifact interpretation

## Standards And Research Log

This packet was written after targeted web research focused on public Rust API clarity, documentation for contract-bearing DTOs, recoverable error handling, test organization for large validation modules, Tauri command-boundary guidance, and CUDA best-practices framing for GPU route/fallback behavior.

### Sources consulted

1. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this file exposes public DTOs and public validation entrypoints
   - Standard informed:
     - public API surfaces should not claim semantics that only happen in an outer wrapper

2. Rust API Guidelines, Documentation
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why it was consulted:
     - the layer split between app-core and Tauri/headless orchestration is currently implicit
   - Standard informed:
     - contract-bearing DTOs and entrypoints should document omitted or wrapper-owned behavior

3. Rust Book, Error Handling
   - Source: <https://doc.rust-lang.org/book/ch09-00-error-handling.html>
   - Why it was consulted:
     - validation routing and fallback here are surfaced through `Result<_, String>` plus result metadata
   - Standard informed:
     - unsupported combinations should fail or document themselves clearly, rather than appearing silently supported

4. Rust Book, Test Organization
   - Source: <https://doc.rust-lang.org/book/ch11-03-test-organization.html>
   - Why it was consulted:
     - this file has extensive tests, so the key question is which contract lanes are still untested
   - Standard informed:
     - boundary semantics need direct tests, especially when multiple layers share one DTO

5. Tauri v2, Calling Rust from the frontend
   - Source: <https://v2.tauri.app/develop/calling-rust/>
   - Why it was consulted:
     - part of this file’s effective behavior is completed in the Tauri command layer
   - Standard informed:
     - command-boundary responsibilities should be explicit rather than silently split across layers

6. NVIDIA CUDA C++ Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
   - Why it was consulted:
     - GPU validation fallback and route selection should distinguish true unsupported features from avoidable guard-path pessimism
   - Standard informed:
     - route gating should be deliberate and should avoid disabling GPU work for effectively inactive features

## Findings

## Correctness And Safety

### 1. The public app-core request/result contract includes `cross_ticker`, but the app-core validator never executes or returns cross-ticker results itself

- Evidence:
  - the public request DTO includes:
    - `pub cross_ticker: Option<CrossTickerConfig>`: `crates/app_core/src/double_ma_validate.rs:1512`
  - candidate and top-level public result DTOs also expose cross-ticker fields:
    - candidate: `1571-1573`
    - result: `1675`
  - the app-core entrypoint is re-exported publicly:
    - `crates/app_core/src/lib.rs:144-148`
  - the core result returned from this file hard-codes:
    - `cross_ticker: None`: `crates/app_core/src/double_ma_validate.rs:4049`
  - actual cross-ticker execution happens only after the core call in wrapper layers:
    - Tauri: `src-tauri/src/commands_validation.rs:632-668`
    - headless: `src-tauri/src/bin/ta_headless_validate.rs:577-621`
- Why it matters:
  - direct library callers can submit a request containing `cross_ticker` and receive a result shape that looks complete but is missing the advertised cross-ticker population
  - this is a public DTO contract problem, not a mere implementation detail
- Smallest justified fix:
  - short-term:
    - document that app-core validation does not execute cross-ticker work directly
    - reject active `cross_ticker` on the core public entrypoint unless a wrapper is responsible for finishing it
  - longer-term:
    - either move cross-ticker ownership into app-core
    - or move the field out of the app-core public DTO and keep it in the Tauri/headless layer
- Review judgment:
  - highest-priority contract issue in this file
  - cross-file and semantics-sensitive

### 2. `validation_context` is exposed on the public request DTO but is ignored by the app-core validator

- Evidence:
  - the request includes:
    - `pub validation_context: Option<ValidationArtifactContext>`: `crates/app_core/src/double_ma_validate.rs:1534`
  - the core public entrypoint accepts the request unchanged:
    - `3406-3419`
    - `3421-4055`
  - no use of `req.validation_context` was found in the core execution body before return
  - Tauri independently enforces it:
    - `src-tauri/src/commands_validation.rs:131-166`
    - `587-602`
  - direct/headless callers still pass the field:
    - `src-tauri/src/bin/ta_headless_validate.rs:569`
    - `crates/app_core/src/bin/real_data_fidelity_audit.rs:753`
- Why it matters:
  - the same public request type means two different things depending on whether the caller goes through Tauri or calls app-core directly
  - that is especially risky because `validation_context` sounds like a boundary invariant, not optional metadata
- Smallest justified fix:
  - document the layer split immediately
  - longer-term, either:
    - validate it consistently in the wrapper-owned API only, using a different DTO
    - or add an explicit app-core preflight contract that can validate what app-core actually knows
- Review judgment:
  - high-severity public-contract inconsistency
  - coordination required

## API And Naming

### 1. The public DTO surface implies a single validation contract, but the real contract is split across app-core and wrapper layers

- Evidence:
  - app-core publicly exports:
    - `DoubleMaValidationRequest`
    - `DoubleMaValidationResult`
    - `run_double_ma_validation_blocking_with_candles`
    - `crates/app_core/src/lib.rs:144-148`
  - wrapper layers then add behavior not implemented by the core function:
    - `validation_context` enforcement
    - cross-ticker execution
    - in the Tauri path, final `runtime_ms` overwrite
- Why it matters:
  - public DTOs should make ownership boundaries obvious
  - today a direct caller can reasonably assume the core function fully honors all request fields because the fields live on the same exported type
- Smallest justified fix:
  - add rustdoc or module documentation that separates:
    - app-core validation
    - wrapper-owned cross-ticker/context enforcement
  - longer-term, consider separate request/result types for core versus orchestrated validation

## Error Handling

### 1. An effectively disabled `cross_ticker` payload still changes backend routing

- Evidence:
  - GPU blocker is based on presence alone:
    - `crates/app_core/src/double_ma_validate.rs:130-142`
  - the blocker feeds CPU fallback:
    - `3478-3490`
  - the wrapper layer sanitizes `cross_ticker` and exits early when it is inactive:
    - `src-tauri/src/commands_validation.rs:243-245`
  - existing feature-gated test shows fallback to CPU even with:
    - `datasets: Vec::new()`
    - `max_datasets: 0`
    - `crates/app_core/src/double_ma_validate.rs:5234+`
- Why it matters:
  - a no-op or effectively disabled cross-ticker config should not pessimistically disable GPU validation just because the option is `Some(...)`
- Smallest justified fix:
  - sanitize or normalize `cross_ticker` before deciding the GPU blocker
  - preserve the CPU fallback only for actually active cross-ticker work
- Review judgment:
  - medium-severity route-correctness issue
  - benchmark-sensitive only insofar as it changes CPU/GPU routing

## Module Boundaries And Technical Debt

### 1. `runtime_ms` does not have a stable meaning across app-core, Tauri, and headless consumers

- Evidence:
  - app-core measures runtime before any wrapper cross-ticker work:
    - `crates/app_core/src/double_ma_validate.rs:4018`
  - Tauri overwrites it after wrapper cross-ticker work:
    - `src-tauri/src/commands_validation.rs:668`
  - headless applies cross-ticker work but keeps the unchanged pre-wrapper runtime:
    - `src-tauri/src/commands_validation.rs:886-911`
    - `src-tauri/src/bin/ta_headless_validate.rs:624-627`
- Why it matters:
  - consumers comparing `runtime_ms` across these paths are comparing different measurements
  - that makes performance interpretation and artifact comparison less trustworthy
- Smallest justified fix:
  - define what `runtime_ms` is supposed to mean:
    - core validation only
    - fully orchestrated validation
  - then make all layers honor the same definition or expose both timings explicitly
- Review judgment:
  - medium-severity observability and contract issue
  - coordination required

## Docs And Examples

### 1. The file needs an explicit ownership note for wrapper-owned behavior

- Why it matters:
  - the contract split is not obvious from the DTO definitions alone
- Smallest justified fix:
  - document that app-core owns core fixed-candidate validation
  - document that Tauri/headless layers currently own:
    - loaded-data-context enforcement
    - cross-ticker orchestration
    - possibly final end-to-end runtime measurement

## Tests And Missing Verification

### 1. The module has extensive coverage, but the public contract split is not pinned directly at the app-core boundary

- Evidence:
  - direct core smoke test run succeeded:
    - `cargo test -p ta_app_core double_ma_validation_smoke_runs_and_returns_stats -- --nocapture`
    - result: `1` test passed
  - Tauri-side validation-context test run succeeded:
    - `cargo test -p ta_desktop_demo_app validation_context_rejects_dataset_mismatch -- --nocapture`
    - result: `1` test passed
  - headless cross-ticker test run succeeded:
    - `cargo test -p ta_desktop_demo_app cross_ticker_headless_double_ma_smoke_applies_stats -- --nocapture`
    - result: `1` test passed
  - feature-gated in-file GPU fallback test exists:
    - `crates/app_core/src/double_ma_validate.rs:5234+`
  - I did not find an app-core boundary test asserting either:
    - that direct library callers get populated cross-ticker results
    - or that direct library callers are explicitly told those fields are wrapper-owned
- Smallest justified fix:
  - add boundary tests that lock whichever contract is chosen:
    - explicit rejection/documented no-op for core `cross_ticker`
    - explicit wrapper-only enforcement for `validation_context`, or a split DTO

## Performance, Allocation, And CUDA Concerns

### 1. GPU fallback should key off active unsupported features, not merely non-`None` optional payloads

- Evidence:
  - `gpu_validation_feature_blocker` currently checks only `req.cross_ticker.is_some()`: `130-142`
  - wrapper-layer sanitization already distinguishes inactive configs: `src-tauri/src/commands_validation.rs:243-245`
- Why it matters:
  - avoidable fallback widens CPU usage and reduces route predictability
- Smallest justified fix:
  - normalize the feature config before GPU blocking, or move the blocker later in the orchestration path once inactivity is known
- Review judgment:
  - modest but real routing/perf issue

## Code-Quality Improvement Register

- Safe now:
  - document wrapper-owned versus app-core-owned validation behavior
  - add boundary tests for direct app-core caller semantics
  - normalize inactive `cross_ticker` config before GPU blocking
- Coordination required:
  - DTO split or contract realignment for `cross_ticker` and `validation_context`
  - `runtime_ms` definition unification
- Defer until later pass:
  - broader public API redesign if the repo wants a clean separation between core validation and orchestrated validation

## Technical-Debt Reduction Register

- Safe to reduce now:
  - hidden layer split
  - missing direct boundary tests
  - no-op `cross_ticker` route pessimism
- Coordination required:
  - request/result DTO reshaping
  - runtime measurement unification
- Useful debt that should not be “cleaned up” casually:
  - the extensive in-file validation test matrix
  - current backend-route diagnostics and execution-profile output

## Performance Opportunity Register

- Safe structural improvements:
  - avoid GPU fallback for semantically inactive `cross_ticker` configs
- Benchmark-sensitive candidates:
  - any change that shifts work between app-core and wrapper layers
  - any change that redefines `runtime_ms`
- Not the right place to chase performance casually:
  - broad refactors of validation orchestration without preserving current metric and verdict behavior

## Observability And Diagnostics Gaps

- direct library callers are not clearly told that `cross_ticker` is wrapper-owned
- `validation_context` looks enforced but is not on the core path
- `runtime_ms` does not identify whether it measures core-only or end-to-end orchestrated work

## Implementation Staging Guidance

- Safe now:
  - docs
  - boundary tests
  - no-op `cross_ticker` normalization before GPU blocking
- Coordination required:
  - public DTO contract cleanup
  - runtime timing semantics alignment
- Defer until later pass:
  - full API split between app-core validation and Tauri/headless orchestration

## Change Blast Radius

- Local only:
  - docs and some direct boundary tests
- Same subsystem:
  - validation routing and result semantics
- Cross-backend/runtime:
  - Tauri commands
  - headless tools
  - app-core library callers
  - report/artifact consumers

## Open Questions Or Assumptions

- assumption:
  - the current DTO shape is partly historical convenience rather than an intentionally final public app-core contract
- open question:
  - whether the repo wants app-core to become the true end-to-end validation owner, or whether the cleaner design is to move orchestration-only fields out of app-core DTOs entirely

## Recommended Action

`Larger refactor but defer`

The highest-value issues are not just local bugs. They are boundary inconsistencies between app-core and the wrapper/orchestration layer.

The right next pass is:

- immediately document and test the current layer split
- make the GPU blocker ignore semantically inactive `cross_ticker` configs
- then plan a coordinated contract cleanup so app-core DTOs only advertise semantics that app-core actually owns

## Exact Local Verification Required If Changed

- `cargo test -p ta_app_core double_ma_validation_smoke_runs_and_returns_stats -- --nocapture`
- `cargo test -p ta_desktop_demo_app validation_context_rejects_dataset_mismatch -- --nocapture`
- `cargo test -p ta_desktop_demo_app cross_ticker_headless_double_ma_smoke_applies_stats -- --nocapture`
- if changing GPU blocker behavior under CUDA features:
  - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel double_ma_validation_gpu_kernel_direct_cross_ticker_request_falls_back_to_cpu -- --nocapture`
- if changing runtime timing semantics:
  - rerun both Tauri and headless cross-ticker validation tests and compare reported `runtime_ms`

## Regression Risk And Feature-Preservation Constraints

The intended outcome is clearer validation ownership and better route correctness with no feature loss.

- do not remove cross-ticker capability from the product; only clarify which layer owns it
- do not narrow supported validation modes or verdict behavior as a side effect of DTO cleanup
- if runtime timing is redefined, preserve backwards compatibility where artifacts or UI depend on the current field
- if direct app-core callers begin receiving explicit errors for wrapper-owned fields, document that as a correctness and contract-hardening change, not a feature removal
