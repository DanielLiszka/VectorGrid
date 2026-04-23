# Review Packet: `crates/app_core/src/kqsr.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/kqsr.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Substantively useful and already covered by smoke/parity tests, but not fully public-review ready because one live metrics path bypasses the file’s own validator, the desired-position builder fails open on bad input, and several internal-only signal/helper types are exposed as public surface without a clear external contract`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Public-surface`, `Hot path`, `Strategy-local`, `Semantics-sensitive`, `Benchmark-sensitive`

## What This File Owns

`kqsr.rs` is the strategy-local KQSR signal-construction and desired-position engine.

It owns:

- the public request/config DTO and defaults:
  - `KqsrStrategyParams`: `crates/app_core/src/kqsr.rs:22-154`
- public param validation:
  - `validate_kqsr_params`: `156-191`
- public period resolution:
  - `resolve_kqsr_periods`: `193-205`
- signal-carrier structs:
  - `KqsrResolvedPeriods`: `44-51`
  - `KqsrSignals`: `53-64`
  - `KqsrFastSignals`: `66-73`
  - `KqsrSlowSignals`: `75-82`
  - CUDA `f32` carriers: `84-102`
- dynamic-indicator output resolution and CPU/CUDA indicator dispatch helpers:
  - `resolve_output_id`: `335-371`
  - `compute_indicator_f64`: `373-432`
  - `compute_indicator_f32`: `435-487`
- assembled KQSR signal builders:
  - `compute_kqsr_signals`: `489-527`
  - `compute_kqsr_fast_signals`: `529-648`
  - `compute_kqsr_slow_signals`: `650-763`
  - CUDA `f32` variants: `766-993`
- warmup/start-index logic:
  - `compute_kqsr_start_t`: `995-1006`
- final desired-position assembly:
  - `build_kqsr_desired_positions`: `1019-1158`
- CUDA route-diagnostic helpers:
  - `clear_kqsr_route_diagnostics`: `295-317`
  - `take_kqsr_route_diagnostics`: `300-317`

This file does not own:

- the strategy-family request/response shell:
  - `crates/app_core/src/composite_cpu.rs`
  - `crates/app_core/src/composite_validate.rs`
- raw GPU metric evaluation:
  - `crates/app_core/src/kqsr_gpu_kernel.rs`
- dynamic-indicator implementation details:
  - `crates/app_core/src/dynamic_indicator_adapter.rs`
- indicator metadata registration:
  - `crates/app_core/src/indicator_catalog.rs`

## Classification

- Public surface: yes
  - the module is public from `crates/app_core/src/lib.rs:34`
  - `KqsrStrategyParams` is explicitly re-exported from `crates/app_core/src/lib.rs:123`
  - other helpers/types are still public through the module surface even though they are not re-exported
- Hot path: yes
  - `compute_kqsr_signals`, `compute_kqsr_fast_signals`, `compute_kqsr_slow_signals`, and `build_kqsr_desired_positions` sit directly in optimization and validation loops
- CUDA/runtime-critical: no
  - CUDA helper dispatch exists, but raw kernel/runtime ownership lives elsewhere
- Strategy-local: yes
  - all behavior is specifically about KQSR signal construction and entry/exit policy
- Low-risk support code: no
  - the file directly shapes backtest decisions and route diagnostics

## Direct Interactions

### Backend interaction map

- public module / re-export:
  - `crates/app_core/src/lib.rs:34`
  - `crates/app_core/src/lib.rs:123`
- optimization callers:
  - `crates/app_core/src/composite_cpu.rs:7799-8402`
  - representative live metric evaluator:
    - `crates/app_core/src/composite_cpu.rs:1010-1038`
- validation callers:
  - `crates/app_core/src/composite_validate.rs:10774-10824`
  - GPU-staged validation/reopt signal-row preparation:
    - `crates/app_core/src/composite_validate.rs:6389-6452`
    - `crates/app_core/src/composite_validate.rs:6722-6757`
- dynamic indicator backend dependency:
  - `crates/app_core/src/dynamic_indicator_adapter.rs`
- indicator descriptor/output dependency:
  - `crates/app_core/src/indicator_catalog.rs`
- GPU evaluator that depends on signal builders and period resolution:
  - `crates/app_core/src/kqsr_gpu_kernel.rs`

### Downstream verification impact if changed

- param-validation changes affect:
  - optimization
  - validation
  - trade22 metric evaluation in `composite_cpu.rs`
- signal-builder or desired-position changes affect:
  - CPU optimization results
  - GPU-staged optimization row prep
  - fixed-candidate validation
  - walk-forward / CSCV / reopt validation that uses KQSR series
- visibility changes affect:
  - any out-of-repo caller using public-but-not-re-exported KQSR helpers

## Standards And Research Log

This packet was written after targeted web research focused on public Rust API minimality and clarity, documentation expectations for serde-backed strategy config, explicit error handling over silent fallbacks, and test posture for public execution helpers.

### Sources consulted

1. Rust API Guidelines Checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this file exposes a public strategy config type and multiple public helper types/functions
   - Standard informed:
     - public APIs should expose only what needs to be public and should make contracts explicit

2. Rust API Guidelines, Documentation
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why it was consulted:
     - the public surface here has important defaults, output-selection semantics, and route-fallback behavior that are currently implicit
   - Standard informed:
     - public types and functions should document defaults, assumptions, and edge-case behavior

3. Rust API Guidelines, Naming
   - Source: <https://rust-lang.github.io/api-guidelines/naming.html>
   - Why it was consulted:
     - this file exposes several helper carriers and compute functions that look public even though they appear intended for in-crate use
   - Standard informed:
     - naming and visibility should reflect actual intended use and scope

4. Rust Book, Error Handling
   - Source: <https://doc.rust-lang.org/book/ch09-00-error-handling.html>
   - Why it was consulted:
     - the file mixes strong validation in some paths with silent fallback in others
   - Standard informed:
     - invalid input should generally move through explicit errors rather than fail-open behavior

5. Rust Book, Test Organization
   - Source: <https://doc.rust-lang.org/book/ch11-03-test-organization.html>
   - Why it was consulted:
     - the file has useful unit tests, but KQSR correctness also depends on cross-module optimization/validation flows
   - Standard informed:
     - both local and integrated behavior should be pinned when public helpers feed larger workflows

6. Serde Field Attributes
   - Source: <https://serde.rs/field-attrs.html>
   - Why it was consulted:
     - `KqsrStrategyParams` is serde-backed and clearly intended to travel across request boundaries
   - Standard informed:
     - defaulted public config should make omitted-field behavior and validation expectations explicit

## Findings

## Correctness And Safety

### 1. One live KQSR metrics path bypasses `validate_kqsr_params`

- Evidence:
  - `evaluate_kqsr_trade22_metrics` reads `req.kqsr`, resolves periods, computes signals, and builds desired positions without calling `validate_kqsr_params`:
    - `crates/app_core/src/composite_cpu.rs:1010-1038`
  - the file’s validator exists and is used in the main optimization and validation paths:
    - `crates/app_core/src/kqsr.rs:156-191`
    - `crates/app_core/src/composite_cpu.rs:7888-7889`
    - `crates/app_core/src/composite_cpu.rs:8261-8262`
    - `crates/app_core/src/composite_validate.rs:10783-10784`
- Why it matters:
  - invalid KQSR thresholds or non-finite values can reach live metric evaluation on this route even though the strategy has a defined validator
  - that creates inconsistent contract enforcement across KQSR entrypoints
- Smallest justified fix:
  - add `validate_kqsr_params(&kqsr_cfg)?;` inside `evaluate_kqsr_trade22_metrics`
- Review judgment:
  - medium correctness issue
  - safe-now, low regression risk

### 2. `build_kqsr_desired_positions` fails open to an all-flat position vector on invalid input

- Evidence:
  - the builder allocates a zeroed `desired` vector:
    - `crates/app_core/src/kqsr.rs:1026-1027`
  - if `start_t >= n` or any signal vector is short, it returns that zeroed vector instead of an error:
    - `1028-1039`
- Why it matters:
  - upstream contract violations become indistinguishable from a legitimate “stay flat” strategy outcome
  - this can hide data-shape or warmup bugs during optimization and validation
- Smallest justified fix:
  - preferred:
    - change the function to `Result<Vec<i32>, String>`
    - propagate the error from callers that already use `Result`
  - if compatibility blocks that:
    - add a new fallible sibling and migrate internal callers
- Review judgment:
  - medium-to-high correctness / error-handling issue
  - semantics-sensitive only because some caller may currently rely on silent flat output

### 3. `resolve_output_id` silently falls back to the first declared output when preferred IDs miss

- Evidence:
  - explicit preferred-ID match:
    - `crates/app_core/src/kqsr.rs:345-352`
  - relaxed contains-based match:
    - `355-367`
  - final fallback:
    - `370`
- Why it matters:
  - KQSR semantics depend on specific outputs such as `j`, `k`, `d`, `fast`, `slow`, `upper`, and `lower`
  - if the indicator catalog changes or an output is renamed, this helper can silently select the wrong output instead of failing
- Smallest justified fix:
  - for multi-output indicators used by KQSR, treat “no preferred output match” as an error
  - keep the current behavior only if there is a proven backward-compatibility need
- Review judgment:
  - medium correctness / boundary issue
  - safe-now if the intended contract is already “exact output names required”

## API And Naming

### 1. The public surface is broader than the repo-local usage justifies

- Evidence:
  - publicly exposed helper carriers and compute functions include:
    - `KqsrResolvedPeriods`: `44-51`
    - `KqsrSignals`: `53-64`
    - `KqsrFastSignals`: `66-73`
    - `KqsrSlowSignals`: `75-82`
    - CUDA `f32` carriers: `84-102`
    - compute helpers: `193-205`, `489-527`, `529-648`, `650-763`, `766-993`, `995-1158`
  - the only clearly intentional crate re-export is:
    - `crates/app_core/src/lib.rs:123`
  - in-repo call sites are all internal app-core modules:
    - `composite_cpu.rs`
    - `composite_validate.rs`
- Why it matters:
  - exposing internal helper carriers widens the compatibility surface without clear product value
  - it also makes future cleanup or signature hardening harder than necessary
- Smallest justified fix:
  - narrow non-request helper types/functions to `pub(crate)` unless an external consumer is known
- Review judgment:
  - medium API-surface issue
  - safe-now if no external callers depend on the broader visibility

### 2. The strategy config has meaningful defaults and invariants, but they are not documented on the public type

- Evidence:
  - serde-backed defaults:
    - `crates/app_core/src/kqsr.rs:24-41`
  - default-value helpers:
    - `104-137`
  - validator:
    - `156-191`
- Why it matters:
  - callers can construct `KqsrStrategyParams` through defaults or deserialization, but the acceptable ranges and inter-field relationship (`oversold < overbought`) are only visible in code
- Smallest justified fix:
  - add rustdoc covering:
    - default values
    - allowed ranges
    - the expectation that callers should validate before execution
- Review judgment:
  - medium docs/API issue
  - safe-now

## Error Handling

### 1. The file mixes strict validation with fail-open behavior, which weakens the overall contract

- Evidence:
  - strict validation exists in `validate_kqsr_params`:
    - `156-191`
  - strict errors are used for missing indicators / missing outputs / all-non-finite signal sets:
    - `335-371`
    - `633-639`
    - `748-754`
    - `866-872`
    - `978-984`
  - but `build_kqsr_desired_positions` quietly returns flat output on invalid input:
    - `1028-1039`
- Why it matters:
  - this inconsistency makes KQSR harder to reason about under failure
  - strict paths encourage callers to handle contract violations explicitly; the fail-open path hides them
- Smallest justified fix:
  - align desired-position construction with the rest of the file’s explicit-error posture
- Review judgment:
  - real error-handling gap
  - bounded fix

## Module Boundaries And Technical Debt

### 1. Output-resolution logic is strategy-local but brittle against indicator-catalog drift

- Evidence:
  - KQSR depends on `find_dynamic_indicator_descriptor` from the shared indicator catalog:
    - `crates/app_core/src/kqsr.rs:10-12`
    - `335-371`
  - signal builders repeatedly resolve output IDs at runtime:
    - fast `f64`: `537-540`
    - slow `f64`: `657-660`
    - fast `f32`: `774-777`
    - slow `f32`: `891-894`
- Why it matters:
  - the strategy-local code is coupled to shared output naming
  - the fallback-to-first-output rule makes that coupling unsafe rather than explicit
- Smallest justified fix:
  - keep output lookup here, but make required-output mismatches hard errors
- Review judgment:
  - moderate boundary issue
  - safe-now

### 2. The hot-path signal builders repeatedly allocate small parameter vectors and redo output lookup

- Evidence:
  - each builder allocates fresh `Vec<IndicatorParamKV>`:
    - fast `f64`: `542-591`
    - slow `f64`: `662-705`
    - fast `f32`: `779-828`
    - slow `f32`: `896-939`
  - output IDs are resolved on each call:
    - `537-540`, `657-660`, `774-777`, `891-894`
  - these helpers are called inside optimization loops:
    - GPU staging rows: `crates/app_core/src/composite_cpu.rs:7962-7989`
    - CPU optimization loop: `crates/app_core/src/composite_cpu.rs:8335-8352`
- Why it matters:
  - this is real repeated work in a hot path
  - however, the indicator computations themselves are much heavier than the lookup/allocation overhead, so the value of caching is not obvious without measurement
- Smallest justified fix:
  - do not change this by default
  - if profiling shows it matters, cache resolved output IDs and reuse parameter templates
- Review judgment:
  - low-to-medium performance / tech-debt issue
  - benchmark-sensitive; do not prioritize ahead of correctness work

## Docs And Examples

### 1. The public KQSR helpers need explicit contract docs

- Evidence:
  - missing public-facing docs include:
    - config defaults and validation rules
    - output-selection assumptions
    - GPU-route diagnostic behavior
    - `trade_on_next_bar` interaction with `compute_kqsr_start_t`
    - invalid-input behavior for `build_kqsr_desired_positions`
- Smallest justified fix:
  - add rustdoc to:
    - `KqsrStrategyParams`
    - `validate_kqsr_params`
    - `resolve_kqsr_periods`
    - `compute_kqsr_start_t`
    - `build_kqsr_desired_positions`

## Tests And Missing Verification

### 1. Local tests exist, and higher-level KQSR smoke/parity coverage is good, but important edge cases are still unpinned

- Evidence:
  - local verification run:
    - `cargo test -p ta_app_core kqsr::tests:: -- --nocapture`
      - passed
    - `cargo test -p ta_app_core kqsr_optimization_smoke_runs -- --nocapture`
      - passed
  - existing file-local tests cover:
    - invalid param rejection
    - one reversal-rule desired-position scenario
    - `crates/app_core/src/kqsr.rs:1187-1219`
  - existing higher-level KQSR suites include:
    - optimization smoke: `crates/app_core/src/composite_cpu.rs:16477`
    - GPU-kernel optimization sanity/parity: `16824`, `16875`
    - validation GPU parity and route diagnostics: `crates/app_core/src/composite_validate.rs:22072-22521`
- Missing tests I would want before changing this file:
  - fail-open desired-position invalid-shape branch
  - `trade_on_next_bar` start-index behavior
  - output-resolution failure when preferred IDs are missing
  - param-validation enforcement for `evaluate_kqsr_trade22_metrics`

## Performance, Allocation, And Runtime Concerns

### 1. This file has some repeated small allocations, but they are not the first thing to fix

- Why it matters:
  - there is real repeated `Vec<IndicatorParamKV>` construction and output lookup in hot loops
  - but the strategy still spends much more time in indicator computation and downstream evaluation
- Review judgment:
  - correctness and contract hardening should land before caching/memoization work
  - if performance work is attempted here, benchmark both CPU optimization and GPU-staged prep loops first

### 2. Desired-position semantics are more important than micro-optimizations

- Why it matters:
  - `build_kqsr_desired_positions` contains the strategy’s actual entry/exit logic
  - branch-collapsing or “cleanup” changes here are more likely to change behavior than to create meaningful speedups
- Review judgment:
  - do not touch this loop for performance without parity tests and measured evidence

## Code-Quality Improvement Register

- Safe now:
  - add missing validator call in `evaluate_kqsr_trade22_metrics`
  - make desired-position invalid input explicit instead of fail-open
  - make required output-ID mismatches fail closed
  - narrow helper visibility where no external consumer is known
  - add rustdoc on config defaults and helper contracts
- Coordination required:
  - any visibility narrowing if there are out-of-repo consumers
  - any caching/memoization work in optimization loops
- Defer until later pass:
  - larger redesign of indicator-output lookup or route-diagnostic plumbing

## Technical-Debt Reduction Register

- Safe to reduce now:
  - inconsistent contract enforcement
  - fail-open desired-position builder
  - undocumented public helper surface
- Coordination required:
  - visibility narrowing
  - caching of output IDs / param templates
- Useful debt that should not be “cleaned up” casually:
  - strategy-local separation between fast and slow signal builders; this is operationally useful for GPU staging and validation reuse

## Performance Opportunity Register

- Safe structural improvements:
  - none urgent beyond possible later caching of output IDs and parameter templates
- Benchmark-sensitive candidates:
  - caching output-ID resolution
  - reusing `IndicatorParamKV` templates
- Not the right place to chase performance without evidence:
  - compressing the desired-position state machine
  - broad refactors of the builder split between fast and slow signals

## Observability And Diagnostics Gaps

- `build_kqsr_desired_positions` hides contract violations behind a flat signal vector
- output-ID fallback can mask indicator-catalog drift instead of surfacing it
- the public API does not say which helper types are stable public contract and which are internal implementation plumbing

## Implementation Staging Guidance

- Safe now:
  - validator call in trade22 path
  - desired-position error handling hardening
  - strict output-ID mismatch errors
  - docs
- Coordination required:
  - visibility narrowing
  - any caching work
- Defer until later pass:
  - broader helper extraction or redesign

## Change Blast Radius

- Local only:
  - docs
  - output-resolution error messaging
- Same subsystem:
  - desired-position return-shape changes
  - validator-call enforcement
  - visibility narrowing for internal helpers
- Cross-backend / frontend-backend contract:
  - any change to serialized/publicly reachable helper types if external callers exist

## Open Questions Or Assumptions

- assumption:
  - `KqsrStrategyParams` is the intended durable public API here; most helper carriers are not
- open question:
  - whether any out-of-repo consumer already uses public helper types like `KqsrSignals` or `KqsrResolvedPeriods`
- open question:
  - whether the fallback-to-first-output behavior was ever intentionally relied on for indicator catalog compatibility

## Recommended Action

`Small surgical refactor`

The right next pass is:

- enforce the validator consistently
- stop fail-open desired-position construction on bad inputs
- make required output selection strict
- then consider visibility narrowing and only later consider perf caching if profiling justifies it

## Exact Local Verification Required If Changed

- `cargo test -p ta_app_core kqsr::tests:: -- --nocapture`
- `cargo test -p ta_app_core kqsr_optimization_smoke_runs -- --nocapture`
- if CUDA is available and KQSR GPU paths are touched:
  - `cargo test -p ta_app_core kqsr_optimization_gpu_kernel_sanity_matches_cpu_small_grid --features cuda,cuda-backtest-kernel -- --nocapture`
  - `cargo test -p ta_app_core kqsr_validation_gpu_kernel_matches_cpu_metrics --features cuda,cuda-backtest-kernel -- --nocapture`
  - `cargo test -p ta_app_core kqsr_validation_gpu_kernel_reports_route_diagnostics --features cuda,cuda-backtest-kernel -- --nocapture`
- add and run focused tests for:
  - invalid input handling in `build_kqsr_desired_positions`
  - `trade_on_next_bar` start-index behavior
  - strict output-ID failure cases
  - validator enforcement in trade22 evaluation

## Regression Risk And Feature-Preservation Constraints

- do not change KQSR entry/exit semantics under the banner of cleanup
- if desired-position construction becomes fallible, update all internal callers deliberately rather than preserving old behavior accidentally
- if visibility is narrowed, confirm there is no external consumer depending on the currently public helpers
