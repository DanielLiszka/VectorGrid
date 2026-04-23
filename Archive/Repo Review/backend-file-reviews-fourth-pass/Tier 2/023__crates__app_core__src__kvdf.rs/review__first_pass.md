# Review Packet: `crates/app_core/src/kvdf.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/kvdf.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Useful and moderately well exercised, but not fully public-review ready because one live metrics path bypasses the file’s own validator, the desired-position builder tolerates malformed signal vectors by degrading through `NaN` defaults, and several internal helper carriers/compute functions are exposed publicly without a clear external contract`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Public-surface`, `Hot path`, `Strategy-local`, `Semantics-sensitive`, `Benchmark-sensitive`

## What This File Owns

`kvdf.rs` is the strategy-local KVDF signal-construction and desired-position engine.

It owns:

- the public request/config DTO and defaults:
  - `KvdfStrategyParams`: `crates/app_core/src/kvdf.rs:22-161`
- public config validation and normalization helpers:
  - `validate_kvdf_params`: `163-190`
  - `normalized_kvdf_dti_threshold`: `192-194`
  - `normalize_kvdf_vwap_anchor`: `196-202`
- public period resolution:
  - `resolve_kvdf_periods`: `204-219`
- public signal carriers:
  - `KvdfResolvedPeriods`: `43-51`
  - `KvdfSignals`: `54-65`
  - `KvdfSlowSignals`: `68-75`
  - `KvdfFastSignals`: `78-82`
  - `KvdfSharedSignals`: `85-88`
  - CUDA `f32` carriers: `91-118`
- dynamic-indicator output resolution and CPU/CUDA indicator dispatch helpers:
  - `resolve_output_id`: `336-372`
  - `compute_indicator_f64`: `374-433`
  - `compute_indicator_f32`: `436-488`
- assembled KVDF signal builders:
  - `compute_kvdf_signals`: `490-551`
  - `compute_kvdf_slow_signals`: `553-693`
  - `compute_kvdf_slow_signals_f32`: `695-829`
  - `compute_kvdf_shared_signals`: `831-857`
  - `compute_kvdf_shared_signals_f32`: `859-883`
  - `compute_kvdf_dti_signal`: `885-918`
  - `compute_kvdf_dti_signal_f32`: `920-951`
  - `compute_kvdf_fast_signals`: `953-1006`
  - `compute_kvdf_fast_signals_f32`: `1008-1058`
- warmup/start-index logic:
  - `compute_kvdf_start_t`: `1060-1071`
- final desired-position assembly:
  - `build_kvdf_desired_positions`: `1073-1200`
- route-diagnostic helpers for CUDA fallback:
  - `clear_kvdf_route_diagnostics`: `297-319`
  - `take_kvdf_route_diagnostics`: `302-319`

This file does not own:

- composite-family request/response shells:
  - `crates/app_core/src/composite_cpu.rs`
  - `crates/app_core/src/composite_validate.rs`
- raw GPU metric evaluation:
  - `crates/app_core/src/kvdf_gpu_kernel.rs`
- dynamic-indicator implementation details:
  - `crates/app_core/src/dynamic_indicator_adapter.rs`
- indicator metadata registration:
  - `crates/app_core/src/indicator_catalog.rs`

## Classification

- Public surface: yes
  - the module is public from `crates/app_core/src/lib.rs`
  - `KvdfStrategyParams` is a real request/API type and the other helpers are still publicly reachable through the module surface
- Hot path: yes
  - the signal builders and desired-position builder run inside optimization and validation loops
- CUDA/runtime-critical: no
  - this file chooses CPU/CUDA helper dispatch, but raw GPU kernel/runtime ownership lives in `kvdf_gpu_kernel.rs`
- Strategy-local: yes
  - all behavior is KVDF-specific
- Low-risk support code: no

## Direct Interactions

### Backend interaction map

- optimization callers:
  - trade22 metric path:
    - `crates/app_core/src/composite_cpu.rs:1072-1100`
  - CPU/GPU optimization:
    - `crates/app_core/src/composite_cpu.rs:9129-9778`
- validation callers:
  - fixed-candidate validation:
    - `crates/app_core/src/composite_validate.rs:10894-10944`
  - GPU-staged validation/reopt preparation:
    - `crates/app_core/src/composite_validate.rs:7965-8589`
- dynamic indicator backend dependency:
  - `crates/app_core/src/dynamic_indicator_adapter.rs`
- output metadata dependency:
  - `crates/app_core/src/indicator_catalog.rs`
- GPU evaluator dependency:
  - `crates/app_core/src/kvdf_gpu_kernel.rs`

### Downstream verification impact if changed

- validation or normalization changes affect:
  - optimization
  - validation
  - trade22 metric evaluation
- desired-position changes affect:
  - CPU optimization
  - fixed-candidate validation
  - GPU-staged validation, because CPU desired-position semantics are the parity target
- visibility changes affect:
  - any out-of-repo use of public helper types/functions

## Standards And Research Log

This packet was written after targeted web research focused on public API minimality, documentation for serde-backed strategy config, explicit error handling over silent degradation, and test posture for public execution helpers.

### Sources consulted

1. Rust API Guidelines Checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this file exposes a public strategy config and many public helper types/functions
   - Standard informed:
     - public APIs should expose only the intended contract and should make invariants explicit

2. Rust API Guidelines, Documentation
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why it was consulted:
     - the file has meaningful defaults, normalization, and output-selection behavior that is currently implicit
   - Standard informed:
     - public docs should cover defaults, failure conditions, and semantics-bearing caveats

3. Rust API Guidelines, Naming
   - Source: <https://rust-lang.github.io/api-guidelines/naming.html>
   - Why it was consulted:
     - the file exposes many helper carriers whose public visibility may not reflect intended scope
   - Standard informed:
     - names and visibility should match real usage and responsibility

4. Rust Book, Error Handling
   - Source: <https://doc.rust-lang.org/book/ch09-00-error-handling.html>
   - Why it was consulted:
     - this file mixes strong validation in some paths with permissive degradation in others
   - Standard informed:
     - invalid inputs should generally travel through explicit errors rather than hidden fallback behavior

5. Rust Book, Test Organization
   - Source: <https://doc.rust-lang.org/book/ch11-03-test-organization.html>
   - Why it was consulted:
     - KVDF correctness depends on both file-local semantics and integration into composite optimization/validation
   - Standard informed:
     - both local and cross-module behavior should be pinned

6. Serde Field Attributes
   - Source: <https://serde.rs/field-attrs.html>
   - Why it was consulted:
     - `KvdfStrategyParams` is serde-backed and defaulted
   - Standard informed:
     - defaulted public config should make omitted-field behavior and validation expectations explicit

## Findings

## Correctness And Safety

### 1. One live KVDF metrics path bypasses `validate_kvdf_params`

- Evidence:
  - `evaluate_kvdf_trade22_metrics` reads `req.kvdf`, resolves periods, computes signals, and builds desired positions without calling `validate_kvdf_params`:
    - `crates/app_core/src/composite_cpu.rs:1072-1100`
  - the validator exists and is used in the main optimization and validation paths:
    - `crates/app_core/src/kvdf.rs:163-190`
    - `crates/app_core/src/composite_cpu.rs:9218-9219`
    - `crates/app_core/src/composite_cpu.rs:9639-9640`
    - `crates/app_core/src/composite_validate.rs:10903-10904`
- Why it matters:
  - invalid KVDF configuration can reach live metric evaluation through this path even though the strategy defines a validator
  - that creates inconsistent contract enforcement across KVDF entrypoints
- Smallest justified fix:
  - add `validate_kvdf_params(&kvdf_cfg)?;` in `evaluate_kvdf_trade22_metrics`
- Review judgment:
  - medium correctness issue
  - safe-now

### 2. `build_kvdf_desired_positions` tolerates malformed signal vectors by substituting `NaN`, which can change state-machine behavior instead of surfacing an error

- Evidence:
  - the function only rejects `start_t >= n`:
    - `crates/app_core/src/kvdf.rs:1080-1084`
  - every signal access uses `.get(...).unwrap_or(f64::NAN)`:
    - `1096-1121`
  - missing stop/fvg values are treated permissively:
    - `1136-1139`
  - missing breakout/DTI inputs suppress entries because they require finite values:
    - `1123-1135`
- Why it matters:
  - malformed signal vectors do not fail cleanly
  - they can suppress new entries while also weakening hard-exit logic, which may leave an existing position live longer than intended
  - that is more dangerous than a simple flat fallback because it changes the strategy state machine under invalid input
- Smallest justified fix:
  - add explicit length validation up front for every signal vector
  - return `Result<Vec<i32>, String>` or add a fallible sibling and migrate internal callers
- Review judgment:
  - high-severity correctness / error-handling issue
  - semantics-sensitive only because some caller may already rely on permissive behavior

### 3. `resolve_output_id` silently falls back to the first declared output when preferred IDs miss

- Evidence:
  - exact preferred-ID match:
    - `crates/app_core/src/kvdf.rs:346-353`
  - relaxed contains-based match:
    - `355-368`
  - final fallback:
    - `371`
- Why it matters:
  - KVDF semantics depend on specific outputs such as Keltner upper/middle/lower, DevStop value, VWAP value, DTI value, and FVG trailing-stop sides
  - indicator-catalog drift can silently route the strategy onto the wrong output instead of producing a hard failure
- Smallest justified fix:
  - make “no preferred output matched” an error for the multi-output indicators KVDF depends on
- Review judgment:
  - medium correctness / module-boundary issue
  - safe-now if the intended contract is already “exact outputs required”

## API And Naming

### 1. The public surface is broader than the repo-local usage justifies

- Evidence:
  - many helper carriers and compute functions are public:
    - `KvdfResolvedPeriods`: `43-51`
    - signal carriers: `54-118`
    - compute helpers: `192-1200`
  - current repo-local callers are internal app-core modules:
    - `composite_cpu.rs`
    - `composite_validate.rs`
- Why it matters:
  - this widens the compatibility surface for helpers that look internal to KVDF execution plumbing
- Smallest justified fix:
  - keep `KvdfStrategyParams` public
  - narrow the rest to `pub(crate)` unless an external consumer is known
- Review judgment:
  - medium API-surface issue

### 2. The VWAP-anchor contract is under-validated and under-documented

- Evidence:
  - validator only checks that `vwap_anchor` is non-empty after trimming:
    - `crates/app_core/src/kvdf.rs:170-172`
  - normalization maps `""` or `"session"` to `"1d"` and passes any other lowercased value through unchanged:
    - `196-202`
- Why it matters:
  - this config behaves more like an enum/string contract than a free-form string, but validation does not enforce a known set
  - because the trade22 path bypasses validation, even an empty anchor can be silently normalized to `"1d"` there
- Smallest justified fix:
  - document supported anchor values explicitly
  - if the underlying indicator accepts only a bounded set, validate against that set here
- Review judgment:
  - medium API/config-contract issue
  - safe-now as docs; stricter validation may be semantics-sensitive if arbitrary anchors are intentionally supported

## Error Handling

### 1. The file mixes explicit validation with permissive degradation, weakening the overall contract

- Evidence:
  - explicit validation in `validate_kvdf_params`:
    - `163-190`
  - explicit errors for missing indicators/outputs or all-non-finite signals:
    - `336-372`
    - `545-551`
    - `819-829`
  - permissive degradation in `build_kvdf_desired_positions` through `unwrap_or(NaN)`:
    - `1096-1121`
- Why it matters:
  - strict validation in one part of the file suggests callers can rely on explicit failure for contract violations
  - the desired-position builder breaks that expectation
- Smallest justified fix:
  - align the desired-position builder with the file’s otherwise explicit-error posture
- Review judgment:
  - real error-handling gap

## Module Boundaries And Technical Debt

### 1. Output-resolution logic is strategy-local but brittle against indicator-catalog drift

- Evidence:
  - KVDF depends on `find_dynamic_indicator_descriptor` from the shared catalog:
    - `crates/app_core/src/kvdf.rs:10-12`
    - `336-372`
  - output IDs are resolved repeatedly inside hot signal builders:
    - slow `f64`: `561-565`
    - slow `f32`: `703-707`
    - shared `f64`/`f32`: `838`, `866`
    - DTI `f64`/`f32`: `892`, `927`
    - fast `f64`/`f32`: `960-961`, `1015-1016`
- Why it matters:
  - the strategy is tightly coupled to shared output naming
  - silent fallback-to-first-output makes that coupling unsafe rather than explicit
- Smallest justified fix:
  - keep output lookup local, but make required-output mismatches fail closed
- Review judgment:
  - moderate boundary issue

### 2. Hot-path signal builders repeat small allocations and output lookup, but the value of caching needs measurement

- Evidence:
  - repeated `Vec<IndicatorParamKV>` construction:
    - slow signals: `567-620`, `709-762`
    - shared signals: `839-841`, `867-869`
    - DTI: `893-904`, `928-939`
    - fast signals: `962-973`, `1017-1028`
  - repeated lookup across optimization loops:
    - `composite_cpu.rs:9296-9317`
    - `9713-9730`
- Why it matters:
  - there is real overhead here
  - but the indicator computations are heavier than the allocation/lookup work, so caching should be justified by profiling
- Smallest justified fix:
  - do not prioritize this ahead of correctness work
  - if profiling says it matters, cache output IDs and parameter templates
- Review judgment:
  - low-to-medium performance / tech-debt issue
  - benchmark-sensitive

## Docs And Examples

### 1. Public KVDF helpers need contract documentation

- Evidence:
  - missing docs include:
    - config defaults and valid ranges
    - supported VWAP anchors
    - desired-position input-shape requirements
    - whether public helper carriers are stable external contract or internal plumbing
- Smallest justified fix:
  - add rustdoc to:
    - `KvdfStrategyParams`
    - `validate_kvdf_params`
    - `normalize_kvdf_vwap_anchor`
    - `compute_kvdf_start_t`
    - `build_kvdf_desired_positions`

## Tests And Missing Verification

### 1. Local tests are useful, and higher-level KVDF optimization/GPU parity coverage is broad, but important edge cases are still unpinned

- Evidence:
  - local verification run:
    - `cargo test -p ta_app_core kvdf::tests:: -- --nocapture`
      - passed
    - `cargo test -p ta_app_core kvdf_optimization_smoke_runs -- --nocapture`
      - passed
  - file-local tests cover:
    - invalid param rejection
    - one breakout/stop desired-position scenario
    - `crates/app_core/src/kvdf.rs:1229-1277`
  - broader repo-level coverage exists in:
    - optimization smoke:
      - `crates/app_core/src/composite_cpu.rs:17557`
    - GPU optimization parity:
      - `14877`
      - `17830`
      - `17881`
    - validation GPU parity / diagnostics:
      - `crates/app_core/src/composite_validate.rs:23637-24086`
- Missing tests I would want before changing this file:
  - malformed/short signal vector handling in `build_kvdf_desired_positions`
  - `trade_on_next_bar` start-index behavior
  - validator enforcement in `evaluate_kvdf_trade22_metrics`
  - strict output-resolution failure cases
  - supported/unsupported VWAP-anchor cases

## Performance, Allocation, And Runtime Concerns

### 1. Performance work here should follow measurement, not precede correctness fixes

- Why it matters:
  - there are real repeated small allocations/lookups
  - but the larger cost remains in indicator compute and downstream evaluation
- Review judgment:
  - fix contract issues first
  - only then consider caching if profiling shows benefit

### 2. The desired-position state machine is not a good target for speculative simplification

- Why it matters:
  - it encodes the actual KVDF entry/exit behavior
  - missing-signal behavior is already subtle; collapsing branches would increase risk
- Review judgment:
  - do not chase readability/perf changes here without parity tests

## Code-Quality Improvement Register

- Safe now:
  - add validator call in trade22 path
  - harden desired-position input validation
  - make required output mismatches fail closed
  - document supported VWAP-anchor behavior
  - narrow helper visibility if external use is not intended
- Coordination required:
  - stricter anchor validation if arbitrary anchors may already be relied on
  - any caching/memoization work
- Defer until later pass:
  - broader refactor of dynamic-indicator lookup shared across strategy-local files

## Technical-Debt Reduction Register

- Safe to reduce now:
  - inconsistent contract enforcement
  - malformed-signal permissiveness
  - missing docs on config and helper scope
- Coordination required:
  - visibility narrowing
  - stronger anchor validation
  - lookup/param caching
- Useful debt that should not be “cleaned up” casually:
  - separation between slow/shared/DTI/fast signals, which is operationally useful for staged GPU paths

## Performance Opportunity Register

- Safe structural improvements:
  - none urgent beyond later caching if profiling justifies it
- Benchmark-sensitive candidates:
  - caching output IDs
  - reusing `IndicatorParamKV` templates
- Not the right place to chase performance without evidence:
  - compressing the desired-position state machine

## Observability And Diagnostics Gaps

- malformed signal vectors are not reported explicitly by `build_kvdf_desired_positions`
- VWAP-anchor support is not documented
- output-ID fallback can hide indicator-catalog drift instead of surfacing it

## Implementation Staging Guidance

- Safe now:
  - validator call in trade22 path
  - desired-position input validation
  - docs
  - stricter output resolution
- Coordination required:
  - helper visibility narrowing
  - stronger anchor validation
  - caching work
- Defer until later pass:
  - broader shared-lookup abstraction work

## Change Blast Radius

- Local only:
  - docs
  - output-resolution error messaging
- Same subsystem:
  - desired-position return-shape changes
  - validator-call enforcement
  - visibility narrowing
  - anchor validation tightening
- Cross-backend / frontend-backend contract:
  - any serialized config validation change if external callers exist

## Open Questions Or Assumptions

- assumption:
  - `KvdfStrategyParams` is the intended stable public type here; most helper carriers are not
- open question:
  - whether arbitrary VWAP anchor strings are intentionally supported by the underlying indicator or whether this should be an explicit bounded enum-like contract
- open question:
  - whether any out-of-repo caller already depends on public helper types or permissive desired-position behavior

## Recommended Action

`Small surgical refactor`

The right next pass is:

- enforce validation consistently
- stop tolerating malformed signal vectors in the desired-position builder
- make required output selection explicit
- then decide whether anchor validation and visibility narrowing can be tightened safely

## Exact Local Verification Required If Changed

- `cargo test -p ta_app_core kvdf::tests:: -- --nocapture`
- `cargo test -p ta_app_core kvdf_optimization_smoke_runs -- --nocapture`
- if CUDA paths are touched:
  - `cargo test -p ta_app_core kvdf_optimization_gpu_kernel_sanity_matches_cpu_small_grid --features cuda,cuda-backtest-kernel -- --nocapture`
  - `cargo test -p ta_app_core kvdf_validation_gpu_kernel_matches_cpu_metrics --features cuda,cuda-backtest-kernel -- --nocapture`
  - `cargo test -p ta_app_core kvdf_validation_gpu_kernel_reports_route_diagnostics --features cuda,cuda-backtest-kernel -- --nocapture`
- add and run focused tests for:
  - malformed signal lengths in `build_kvdf_desired_positions`
  - `trade_on_next_bar` start-index behavior
  - validator enforcement in trade22 evaluation
  - strict output-resolution failure cases

## Regression Risk And Feature-Preservation Constraints

- do not change KVDF entry/exit semantics under the banner of cleanup
- if the desired-position builder becomes fallible, update internal callers deliberately
- if anchor validation is tightened, confirm whether any existing workflow depends on today’s permissive string handling
