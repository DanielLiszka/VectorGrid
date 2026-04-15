# Review Packet: `crates/app_core/src/double_ma_strategy.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/double_ma_strategy.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Important invariant gate with a real correctness weakness: many “must be > 0” checks are currently dead because the validator calls accessor methods that already clamp invalid raw values into valid defaults before validation can reject them`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Internal-contract-surface`, `Validation-critical`, `Cross-file-contract-sensitive`, `Low-throughput-risk`

## What This File Owns

`double_ma_strategy.rs` owns the crate-local validation and optimization-preparation shim for `ta_strategies::double_ma::StrategyConfig`.

It owns:

- central strategy validation for Double-MA-family flows:
  - `validate_strategy_cfg`: `crates/app_core/src/double_ma_strategy.rs:3-376`
- optimization-specific wrapper entrypoint:
  - `validate_strategy_cfg_for_optimization`: `379-381`
- optimization-stage strategy shaping shim:
  - `strategy_cfg_for_optimization`: `383-384`

This file does not own:

- the underlying `StrategyConfig` data shape and accessor normalization logic:
  - `crates/strategies/src/double_ma/mod.rs:624-1337`
- preset builders for representative strategy modes:
  - `crates/app_core/src/strategy_presets.rs`
- optimization or validation execution:
  - `crates/app_core/src/double_ma.rs`
  - `crates/app_core/src/double_ma_validate.rs`
  - `crates/app_core/src/composite_cpu.rs`
  - `crates/app_core/src/composite_validate.rs`

## Classification

- Public surface: no
  - `pub(crate)` only
- Hot path: low direct throughput impact
  - validation gate, not execution loop
- CUDA/runtime-critical: indirect
  - this module controls whether strategy configs are admitted into many CPU/GPU flows
- Strategy-local: yes
  - Double-MA-family strategy invariant owner
- Low-risk support code: no
  - mistakes here widen or weaken validation across several backend surfaces

## Direct Interactions

### Backend interaction map

- main optimization surface:
  - `crates/app_core/src/double_ma.rs:29`
  - `4507`
  - `4682`
  - `5039-5040`
- validation surface:
  - `crates/app_core/src/double_ma_validate.rs:1854`
- dynamic strategy compiler/runtime:
  - `crates/app_core/src/strategy_dynamic.rs:6`
  - `359`
  - `668-698`
  - `2654`
  - `2905`
  - `3009`
- composite-family optimization routes:
  - `crates/app_core/src/composite_cpu.rs:73`
  - `2340-2341`
  - many repeated uses through `11045`
- composite MO:
  - `crates/app_core/src/composite_mo.rs:8`
  - `1148-1149`
  - `2267`
- composite validation:
  - repeated `validate_strategy_cfg(...)` use throughout `crates/app_core/src/composite_validate.rs`

### Downstream verification impact if changed

- strategy validation semantics affect:
  - Double-MA optimization
  - Double-MA validation
  - composite-profile optimization/validation routes that reuse `StrategyConfig`
  - dynamic strategy compilation and runtime planning
- any tightening of validation can change whether previously accepted malformed configs are rejected at request time instead of being silently normalized

## Standards And Research Log

This packet was written after targeted web research focused on Rust API clarity for validation boundaries, documentation expectations for contract-bearing helpers, recoverable error guidance, and Rust test-organization guidance for keeping invariants pinned near the owning module.

### Sources consulted

1. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this file is a central contract gate used across multiple backend subsystems
   - Standard informed:
     - invariant-checking helpers should behave predictably and should not silently rewrite caller input

2. Rust API Guidelines, Documentation
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why it was consulted:
     - this file encodes important validation semantics, but has no module-level explanation
   - Standard informed:
     - contract helpers should document what they validate and what they intentionally leave to lower layers

3. Rust API Guidelines, Naming
   - Source: <https://rust-lang.github.io/api-guidelines/naming.html>
   - Why it was consulted:
     - this file exposes wrapper names that imply optimization-specific normalization or policy
   - Standard informed:
     - names should not imply extra semantics that the implementation does not provide

4. Rust Book, Error Handling
   - Source: <https://doc.rust-lang.org/book/ch09-00-error-handling.html>
   - Why it was consulted:
     - the module uses `Result<(), String>` to communicate invariant failures across many call sites
   - Standard informed:
     - validation failures should surface as intentional, specific boundary errors rather than silent fallback

5. Rust Book, Test Organization
   - Source: <https://doc.rust-lang.org/book/ch11-03-test-organization.html>
   - Why it was consulted:
     - this file is the invariant owner, but its tests currently live elsewhere
   - Standard informed:
     - boundary invariants are easier to maintain when the owning module has direct negative-path coverage

## Findings

## Correctness And Safety

### 1. Many `> 0` validation checks are dead because they call `StrategyConfig` accessors that already clamp invalid raw values to valid defaults

- Evidence:
  - representative validator checks:
    - `kaufman_stop_period() == 0`: `crates/app_core/src/double_ma_strategy.rs:52-63`
    - `trend_quality_*_period() == 0`: `65-103`
    - breakout period checks: `106-155`
    - volume-flow period checks: `157-180`
    - PIT, SET, MTS, ORC, and regime-hybrid period checks: `182-289`
  - representative `StrategyConfig` accessors clamp raw invalid values before the validator sees them:
    - `kaufman_stop_period`: `crates/strategies/src/double_ma/mod.rs:946-948`
    - `trend_quality_linearreg_period`: `977-978`
    - `trend_quality_bbw_period`: `992-993`
    - `trend_quality_roc_period`: `1017-1018`
    - `trend_quality_adx_period`: `1032-1033`
    - `breakout_bbands_period`: `1102-1103`
    - `breakout_confirm_bars`: `1117-1118`
    - `breakout_macd_fast_period`: `1122-1123`
    - `breakout_macd_slow_period`: `1127-1128`
    - `volume_flow_confirm_bars`: `1217-1218`
    - `pit_vosc_long_period`: `1300-1302`
    - `orc_willr_period`: `1334-1335`
- Why it matters:
  - malformed deserialized configs with zero-valued periods or counts can pass validation even though the validator’s error messages claim they are rejected
  - execution then proceeds on silently rewritten values rather than on the caller’s submitted values
  - this is especially important because this validator is reused by optimization, validation, dynamic strategy planning, and composite-profile flows
- Smallest justified fix:
  - validate raw struct fields for “must be > 0” requirements before calling the normalizing accessors
  - keep the accessors for execution-time sanitization if that is still desired as a secondary safety net
  - add focused tests for zero raw inputs across at least one representative field from each strategy family
- Review judgment:
  - highest-priority finding in this file
  - semantics-sensitive, not benchmark-sensitive

## API And Naming

### 1. The optimization-specific wrapper API currently implies extra semantics that do not exist

- Evidence:
  - `validate_strategy_cfg_for_optimization` is a direct pass-through:
    - `crates/app_core/src/double_ma_strategy.rs:379-381`
  - `strategy_cfg_for_optimization` is also a direct identity function:
    - `383-384`
  - these wrappers are used widely enough that their names matter:
    - `crates/app_core/src/double_ma.rs:5039-5040`
    - `crates/app_core/src/composite_cpu.rs:2340-2341`
    - `crates/app_core/src/composite_mo.rs:1148-1149`
    - `crates/app_core/src/strategy_dynamic.rs:668-698`
- Why it matters:
  - the names suggest an optimization-only validation mode or optimization-stage normalization step
  - future maintainers can easily assume optimization-specific policy already exists here when it does not
- Smallest justified fix:
  - document that these wrappers are compatibility shims today
  - if the repo no longer needs the extra naming layer, remove or consolidate it only after checking all call sites
- Review judgment:
  - real API clarity issue, but lower severity than the dead-validation bug

## Error Handling

### 1. This validator is intentionally first-failure-only and string-based; if it stays that way, the contract should be explicit

- Evidence:
  - all failures are returned as `Result<(), String>` from a single sequential validator:
    - `crates/app_core/src/double_ma_strategy.rs:3-376`
- Why it matters:
  - because this gate is reused across many backend surfaces, the error text becomes de facto contract and debugging surface
  - once raw-field validation is fixed, the messages should continue to identify the actual invalid input rather than the normalized fallback
- Smallest justified fix:
  - keep the string surface if desired, but ensure messages remain specific and aligned with raw-field checks
- Review judgment:
  - secondary observability concern, not a reason for typed-error churn by itself

## Module Boundaries And Technical Debt

### 1. Validation responsibility is concentrated here, but the current tests are not colocated with the invariant owner

- Evidence:
  - `double_ma_strategy.rs` has no local tests
  - the current validator coverage lives in `crates/app_core/src/double_ma.rs`:
    - `22032-22088`
  - current targeted run:
    - `cargo test -p ta_app_core validate_strategy_cfg -- --nocapture`
    - result: `6` tests passed
- Why it matters:
  - invariant drift is easier when the tests are anchored to a very large neighboring module instead of the owning validator module
  - the missing zero-input tests likely contributed to the dead-check issue above surviving
- Smallest justified fix:
  - add a small local `#[cfg(test)]` block in this module for negative validation cases
  - keep high-level integration coverage in `double_ma.rs`, but move the sharpest invariant tests here

## Docs And Examples

### 1. The module needs a short ownership note explaining the difference between validation and accessor normalization

- Why it matters:
  - the bug above exists partly because the contract split between this module and `StrategyConfig` accessors is implicit
- Smallest justified fix:
  - add a module-level comment explaining:
    - this file validates user-submitted/raw config invariants
    - `StrategyConfig` accessors may still normalize values for execution-time safety, but should not replace boundary rejection where strict validation is intended

## Tests And Missing Verification

### 1. Existing coverage misses the zero-input paths that the validator claims to reject

- Evidence:
  - existing tests cover:
    - same-bar execution rejection
    - mixed commission-model rejection
    - negative bps rejection
    - incompatible hybrid/breakout modes
    - valid bps-derived costs
    - invalid regime-hybrid thresholds
  - current targeted run:
    - `cargo test -p ta_app_core validate_strategy_cfg -- --nocapture`
    - result: `6` tests passed
  - no nearby test was found for:
    - zero `kaufman_stop_period`
    - zero `breakout_confirm_bars`
    - zero `volume_flow_confirm_bars`
    - zero `pit_vosc_short_period`
    - zero `orc_rsi_period`
- Smallest justified fix:
  - add representative zero-input tests that would have failed under the intended contract and currently pass

## Performance, Allocation, And CUDA Concerns

### 1. No meaningful performance issue was identified in this file

- Review judgment:
  - this is not a throughput hotspot
  - do not spend perf budget here before fixing validation correctness

## Code-Quality Improvement Register

- Safe now:
  - validate raw fields instead of normalized accessors for strict `> 0` checks
  - add local negative tests for zero-input paths
  - add a short module comment explaining ownership
- Coordination required:
  - any change to wrapper naming or removal because these helpers are used by several backend modules
- Defer until later pass:
  - broader typed-error redesign, if ever needed

## Technical-Debt Reduction Register

- Safe to reduce now:
  - dead validation branches
  - missing colocated tests
  - misleading lack of ownership docs
- Coordination required:
  - wrapper consolidation if other modules rely on the current helper names
- Useful debt that should not be “cleaned up” casually:
  - centralized validation ownership itself; the problem is not centralization, it is the raw-vs-normalized boundary

## Performance Opportunity Register

- Safe structural improvements:
  - none identified
- Benchmark-sensitive candidates:
  - none identified
- Not the right place to chase performance:
  - this file should prioritize contract correctness and test coverage

## Observability And Diagnostics Gaps

- invalid zero-valued inputs are silently normalized rather than clearly rejected
- callers currently cannot tell whether execution used their raw values or sanitized fallbacks

## Implementation Staging Guidance

- Safe now:
  - raw-field validation fix
  - local invariant tests
  - module-level documentation
- Coordination required:
  - any wrapper rename or removal
- Defer until later pass:
  - typed-error redesign or broad validator decomposition

## Change Blast Radius

- Local only:
  - docs and local tests
- Same subsystem:
  - strategy validation semantics across Double-MA-family flows
- Cross-backend/runtime:
  - composite-profile and dynamic-strategy surfaces that reuse `StrategyConfig`

## Open Questions Or Assumptions

- assumption:
  - the repo intends raw invalid strategy inputs to be rejected, not silently coerced, because the validator’s current error messages explicitly say those inputs are invalid
- open question:
  - whether the optimization wrapper names are intentionally reserved for future policy divergence or are now pure compatibility ballast

## Recommended Action

`Small surgical refactor`

The central task is narrow and high-value:

- fix the dead `> 0` validation branches by checking raw fields
- add representative zero-input tests close to the validator
- document the raw-validation versus accessor-normalization split

This does not require a redesign of the strategy-config system.

## Exact Local Verification Required If Changed

- `cargo test -p ta_app_core validate_strategy_cfg -- --nocapture`
- add and run new representative zero-input tests for at least:
  - Kaufman stop
  - breakout
  - volume flow
  - PIT or ORC
- rerun at least one caller-side test path that admits `StrategyConfig` through optimization or validation after tightening the contract

## Regression Risk And Feature-Preservation Constraints

The intended outcome is stricter and more truthful validation with no feature loss.

- do not remove supported strategy modes or compatibility combinations unless a separate product decision approves that change
- tightening validation will change behavior only for malformed inputs that are currently being silently normalized
- preserve valid existing strategy configs and the current non-zero defaults for legitimate callers that omit fields entirely and rely on `serde(default)`
