# Review Packet: `crates/app_core/src/iterative_sim.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/iterative_sim.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Useful and internally exercised, but not fully public-review ready because invalid input collapses into a successful zero-metrics result, several documented `Metrics` fields are emitted as structural zeros rather than computed values, and the file duplicates execution semantics that already exist elsewhere without a shared contract`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Public-surface`, `Validation-hot-path`, `Semantics-sensitive`, `Low-allocation`

## What This File Owns

`iterative_sim.rs` is the repo's exact-bar, iterative validation engine for signal-driven backtests.

It owns:

- public validation-engine and intrabar-policy enums:
  - `ValidationEngine`: `crates/app_core/src/iterative_sim.rs:7-17`
  - `IntrabarRule`: `20-31`
- the public iterative validation config:
  - `IterativeSimConfig`: `34-54`
- the delta DTO used to compare vectorized and iterative outputs:
  - `IterativeMetricsDelta`: `57-76`
- the public OHLC view used by the simulator:
  - `OhlcSlice`: `79-84`
- internal helpers for percent clamping, trade-cost accounting, intrabar fill resolution, and execution overlays:
  - `clamp_pct`: `86-93`
  - `trade_log_cost`: `95-104`
  - `fill_price_from_intrabar`: `106-168`
  - overlay helpers: `170-253`
- the public iterative engine entrypoint:
  - `simulate_iterative_from_signals`: `255-515`
- two module-local tests:
  - `517-596`

This file does not own request routing or validation orchestration. Those live in:

- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/composite_validate.rs`
- `crates/app_core/src/ml_signal.rs`

This file also does not own the canonical `Metrics` type; it populates `ta_strategies::double_ma::Metrics` from:

- `crates/strategies/src/double_ma/mod.rs:39-107`

## Classification

- Public surface: yes
  - the module is public from `crates/app_core/src/lib.rs:33`
  - `ValidationEngine`, `IntrabarRule`, `IterativeSimConfig`, `OhlcSlice`, and `simulate_iterative_from_signals` are publicly reachable through the public module surface
- Hot path: yes
  - not optimizer-hot, but it is a validation-side per-bar execution loop that can be run repeatedly across many candidates and windows
- CUDA/runtime-critical: no
- Strategy-local: no
  - shared validation support used by Double-MA, composite validation flows, and ML-signal validation
- Low-risk support code: no
  - the code is compact, but it directly affects reported validation metrics and iterative/vectorized parity

## Direct Interactions

### Backend interaction map

- module/export surface:
  - `crates/app_core/src/lib.rs:33`
  - `crates/app_core/src/lib.rs:160-162`
- validation orchestration wrapper:
  - `crates/app_core/src/validation_pipeline.rs:1343-1364`
- direct ML-signal caller:
  - `crates/app_core/src/ml_signal.rs:2002-2053`
- validation request/config DTOs that carry `ValidationEngine` / `IterativeSimConfig`:
  - `crates/app_core/src/double_ma_validate.rs:1520-1544`
  - `crates/app_core/src/composite_validate.rs` via iterative validation flows
  - `crates/app_core/src/meta_select.rs:15`, `161-163`
- canonical metrics model and shared risk-metric helper:
  - `crates/strategies/src/double_ma/mod.rs:39-107`
  - `crates/strategies/src/double_ma/mod.rs:141-323`
- duplicated helper logic in another execution file:
  - `crates/app_core/src/asr.rs:2092-2157`

### Downstream verification impact if changed

- changes to input validation or return-shape semantics affect:
  - `validation_pipeline.rs`
  - `double_ma_validate.rs`
  - `composite_validate.rs`
  - `ml_signal.rs`
- changes to metric population affect:
  - any consumer that compares iterative vs vectorized results
  - any scoring or artifact code that reads `Metrics`
- changes to intrabar or overlay semantics affect:
  - parity between exact-validation backtests and any other exact simulation codepaths reusing similar logic

## Standards And Research Log

This packet was written after targeted web research focused on public Rust API contract clarity, documentation expectations for public enums/config DTOs, error-surface design for invalid inputs, serde-backed configuration defaults, and test posture for public execution helpers.

### Sources consulted

1. Rust API Guidelines Checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this file exposes public enums, config DTOs, and a public execution entrypoint
   - Standard informed:
     - public APIs should make invariants and behavior explicit, especially when failure or normalization is otherwise implicit

2. Rust API Guidelines, Documentation
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why it was consulted:
     - the public surface here has several behavioral defaults and clamping rules that are currently implicit
   - Standard informed:
     - public items should document defaults, error behavior, and important semantics rather than relying on callers to infer them from code

3. Rust Book, Error Handling
   - Source: <https://doc.rust-lang.org/book/ch09-00-error-handling.html>
   - Why it was consulted:
     - the main entrypoint currently handles invalid input by returning `Metrics::default()` instead of a fallible result
   - Standard informed:
     - invalid input should generally travel through an explicit error path unless silent fallback is a deliberate, documented compatibility choice

4. Rust Book, Test Organization
   - Source: <https://doc.rust-lang.org/book/ch11-03-test-organization.html>
   - Why it was consulted:
     - this file exposes public behavior but only has two module-local tests
   - Standard informed:
     - public behavior should be pinned both locally and through higher-level integration-style tests where cross-module behavior matters

5. Serde Field Attributes
   - Source: <https://serde.rs/field-attrs.html>
   - Why it was consulted:
     - `IterativeSimConfig` uses serde defaults and is clearly intended to cross request/config boundaries
   - Standard informed:
     - defaulted public config should make its normalization and omitted-field behavior explicit

## Findings

## Correctness And Safety

### 1. Invalid input shape or `start_t` currently returns a successful zero-metrics payload

- Evidence:
  - the public engine checks length mismatches and invalid `start_t`, then returns `Metrics::default()`:
    - `crates/app_core/src/iterative_sim.rs:263-272`
  - the function is part of a public module surface:
    - `crates/app_core/src/lib.rs:33`
  - higher-level orchestration treats the return value as a real iterative result and derives a delta from it:
    - `crates/app_core/src/validation_pipeline.rs:1360-1364`
- Why it matters:
  - a programmer error or upstream shape bug is indistinguishable from a real backtest that legitimately produced zero-ish metrics
  - this weakens diagnostics and can silently pollute iterative-vs-vectorized comparisons
- Smallest justified fix:
  - safest low-churn option:
    - add a fallible sibling such as `try_simulate_iterative_from_signals(...) -> Result<Metrics, String>`
    - migrate internal callers to the fallible path
    - keep the current function only as a compatibility wrapper if needed
  - at minimum:
    - document the current fallback and add a debug assertion path for internal misuse
- Review judgment:
  - medium-to-high public correctness issue
  - semantics-sensitive if the current silent fallback is already depended on externally

### 2. The returned `Metrics` object leaves several documented fields as structural zeros rather than computed values

- Evidence:
  - this file computes only a subset of the full `Metrics` contract and hard-codes other fields to `0`:
    - `crates/app_core/src/iterative_sim.rs:488-513`
  - fields forced to zero include:
    - `downside_frequency`: `493`
    - `downside_severity`: `494`
    - `kappa_tau`: `497`
    - `long_mean_return`: `505`
    - `short_mean_return`: `506`
    - `closed_trades`: `507`
    - `winning_trades`: `508`
    - `losing_trades`: `509`
    - `gross_profit`: `510`
    - `gross_loss`: `511`
    - `underwater_fraction`: `512`
    - `max_underwater_duration_bars`: `513`
  - the canonical `Metrics` type documents those fields as meaningful metrics, not as “unavailable in iterative mode” placeholders:
    - `crates/strategies/src/double_ma/mod.rs:39-107`
  - the same strategies crate already has shared risk-metric finalization logic that computes several of these fields:
    - `crates/strategies/src/double_ma/mod.rs:141-323`
- Why it matters:
  - consumers receive a structurally complete `Metrics` value that looks authoritative even when substantial fields were never computed
  - this is especially risky because iterative results are later compared against vectorized metrics or may feed downstream scoring/reporting
- Smallest justified fix:
  - safe-now:
    - document exactly which fields are intentionally unavailable today
    - add tests pinning that contract until a fuller implementation lands
  - better bounded follow-up:
    - reuse the shared risk-metric finalizer for the fields this loop already has enough information to compute
    - separately decide whether trade-distribution fields should be accumulated here or intentionally omitted behind a different result type
- Review judgment:
  - medium-to-high correctness / contract issue
  - semantics-sensitive because filling these fields later may change observed outputs

## API And Naming

### 1. `IterativeSimConfig` silently clamps or ignores invalid values, but the public config contract does not say so

- Evidence:
  - `clamp_pct` turns non-finite or non-positive stop/take values into `None`, and caps larger values at `0.95`:
    - `crates/app_core/src/iterative_sim.rs:86-93`
  - `trade_log_cost` floors negative slippage multiplier effects at `0.0` and clamps the final cost to `0.95`:
    - `95-104`
  - `IterativeSimConfig` is public and serde-backed:
    - `34-54`
- Why it matters:
  - callers can provide invalid or extreme config and receive silently normalized behavior instead of an explicit error or clearly documented normalization rule
  - for a config that likely crosses Tauri/API boundaries, that should be deliberate and visible
- Smallest justified fix:
  - add rustdoc and, ideally, a `normalize()` / `validate()` helper so the normalization policy is explicit and testable
- Review judgment:
  - medium API clarity issue
  - safe-now as docs and optional normalization helper

## Error Handling

### 1. The file has no explicit error vocabulary for invalid public input

- Evidence:
  - `simulate_iterative_from_signals` is infallible at the type level and returns `Metrics` unconditionally:
    - `crates/app_core/src/iterative_sim.rs:255-515`
  - invalid series shape, mismatched OHLC lengths, and invalid `start_t` are all collapsed into `Metrics::default()`:
    - `263-272`
- Why it matters:
  - this prevents higher-level callers from distinguishing:
    - “the engine ran and produced real values”
    - “the engine did not run because the input contract was violated”
- Smallest justified fix:
  - same as above:
    - add a fallible sibling and migrate internal callers
  - if compatibility blocks that:
    - at minimum emit a stronger contract comment and add targeted negative tests
- Review judgment:
  - real error-handling gap
  - best fixed via additive API, not a broad redesign

## Module Boundaries And Technical Debt

### 1. Execution semantics are duplicated in `asr.rs`, which creates drift risk for exact-bar behavior

- Evidence:
  - this file defines `trade_log_cost` and `fill_price_from_intrabar`:
    - `crates/app_core/src/iterative_sim.rs:95-168`
  - `asr.rs` contains duplicated helpers with the same roles:
    - `crates/app_core/src/asr.rs:2092-2157`
- Why it matters:
  - intrabar stop/take resolution and trade-cost accounting are semantics-bearing, not cosmetic helpers
  - duplicated logic means one exact execution path can drift from the other during later fixes or feature additions
- Smallest justified fix:
  - safe-now:
    - add parity-focused tests that pin shared expectations across both paths
  - bounded refactor:
    - extract the shared helpers into a small internal module once both call sites are audited
- Review judgment:
  - medium tech-debt issue
  - coordination required if extracted, but the risk is objective today

### 2. The file reimplements a subset of shared metric-finalization logic instead of reusing the existing helper

- Evidence:
  - this file performs its own return-stat and drawdown finalization:
    - `crates/app_core/src/iterative_sim.rs:421-487`
  - the strategies crate already provides `finalize_risk_metrics(...)` for shared `Metrics` fields:
    - `crates/strategies/src/double_ma/mod.rs:141-323`
- Why it matters:
  - this duplication is one reason the iterative result drifts toward a partial `Metrics` population
  - it also raises the maintenance burden when risk-metric definitions evolve elsewhere
- Smallest justified fix:
  - evaluate whether the shared helper can be reused directly for the fields this loop already tracks
  - if not, document why the divergence is intentional
- Review judgment:
  - moderate boundary / tech-debt issue
  - safe-now as documentation; small bounded refactor possible later

## Docs And Examples

### 1. Public behavior is under-documented for a semantics-heavy execution helper

- Evidence:
  - the file currently lacks rustdoc explaining:
    - what `ValidationEngine::Iterative` actually changes
    - how `IntrabarRule` resolves stop/take ambiguity
    - how `IterativeSimConfig` defaults and clamps values
    - what shape contract `OhlcSlice` and `desired_pos` must satisfy
    - that the returned `Metrics` is currently only partially populated for some fields
- Smallest justified fix:
  - add rustdoc to:
    - `ValidationEngine`
    - `IntrabarRule`
    - `IterativeSimConfig`
    - `OhlcSlice`
    - `simulate_iterative_from_signals`
  - include `Defaults`, `Input contract`, and `Output caveats` sections

## Tests And Missing Verification

### 1. Existing coverage is useful, but it does not yet pin the sharpest public contracts

- Evidence:
  - local verification run:
    - `cargo test -p ta_app_core iterative_ -- --nocapture`
      - passed
      - covered:
        - `iterative_sim::tests::iterative_uses_strategy_level_stop_loss_when_sim_cfg_is_none`
        - `iterative_sim::tests::iterative_slippage_penalizes_pnl`
        - `double_ma_validate::tests::double_ma_validation_iterative_engine_emits_delta_fields`
        - `composite_validate::tests::composite_validation_iterative_engine_emits_delta_fields`
        - `double_ma_validate::tests::double_ma_validation_iterative_engine_matches_optimizer_top_candidates`
- Missing tests I would want before changing this file:
  - invalid-shape / invalid-`start_t` behavior
  - tie cases for every `IntrabarRule` variant
  - interaction between cooldown/persistence overlays and intrabar exits
  - explicit assertions around the currently zeroed `Metrics` fields
  - parity tests against duplicated `asr.rs` cost/intrabar helpers if the duplication remains

## Performance, Allocation, And Runtime Concerns

### 1. The loop is already allocation-light; performance work here should be measured, not guessed

- Evidence:
  - the simulation loop itself is scalar and allocation-free after setup:
    - `crates/app_core/src/iterative_sim.rs:298-419`
  - the direct ML-signal caller already reuses working buffers outside this function:
    - `crates/app_core/src/ml_signal.rs:2021-2026`
- Review judgment:
  - there is no obvious “free” micro-optimization worth prioritizing in this file ahead of contract hardening
  - branchy intrabar and overlay semantics are not good candidates for speculative cleanup

### 2. This file is not the right place for semantics-changing performance simplification

- Why it matters:
  - the loop encodes stop-loss, take-profit, flip, cooldown, and persistence behavior
  - attempts to “simplify” branch structure or state transitions without measurement and parity checks would be high-risk relative to likely gains
- Review judgment:
  - benchmark-sensitive only if the per-bar loop or overlay semantics are changed
  - otherwise, prioritize correctness/documentation over performance work

## Code-Quality Improvement Register

- Safe now:
  - add rustdoc for public enums/config/input/output contracts
  - introduce a fallible sibling for invalid input handling
  - document the currently partial `Metrics` population
  - add targeted negative and semantics tests
- Coordination required:
  - reusing shared metric finalization
  - extracting duplicated helpers shared with `asr.rs`
- Defer until later pass:
  - redesigning the result shape if the project wants “partial metrics” represented more explicitly than `Metrics`

## Technical-Debt Reduction Register

- Safe to reduce now:
  - silent invalid-input fallback
  - missing contract docs
  - thin test coverage on public semantics
- Coordination required:
  - shared helper extraction with `asr.rs`
  - shared risk-metric finalization reuse with `ta_strategies::double_ma`
- Useful debt that should not be “cleaned up” casually:
  - the explicit state-machine style in the per-bar loop; it is verbose, but it makes semantics easier to audit than a more compressed form

## Performance Opportunity Register

- Safe structural improvements:
  - none urgent beyond avoiding future divergence and better documenting semantics
- Benchmark-sensitive candidates:
  - any change to per-bar branching, overlay logic, or intrabar fill handling
- Not the right place to chase performance without evidence:
  - cosmetic refactors to the simulation loop
  - branch-collapsing changes that would make state transitions harder to verify

## Observability And Diagnostics Gaps

- invalid input currently looks the same as a legitimate zero-metrics simulation
- the public API does not tell callers that some `Metrics` fields are intentionally left at zero
- config normalization/clamping behavior is implicit rather than documented

## Implementation Staging Guidance

- Safe now:
  - docs
  - targeted tests
  - additive fallible API
- Coordination required:
  - helper extraction with `asr.rs`
  - shared risk-metric finalization reuse
- Defer until later pass:
  - result-type redesign for partially populated iterative metrics

## Change Blast Radius

- Local only:
  - rustdoc
  - targeted tests
- Same subsystem:
  - fallible input handling
  - shared metric finalization reuse
  - intrabar/cost helper extraction
- Cross-backend / frontend-backend contract:
  - any change to serialized config behavior
  - any change to the observed `Metrics` field values

## Open Questions Or Assumptions

- assumption:
  - the current zero-filled `Metrics` fields are legacy placeholders, not intentionally authoritative values
- open question:
  - whether any external consumer already depends on `simulate_iterative_from_signals` returning `Metrics::default()` for invalid input instead of an error
- open question:
  - whether iterative validation is expected to converge toward full `Metrics` parity with the vectorized path or remain a knowingly partial view

## Recommended Action

`Small surgical refactor`

The right next pass is:

- document the public contract
- add a fallible sibling for invalid input
- pin the current partial-metrics behavior with tests or start filling the fields that are already derivable without changing trade semantics

## Exact Local Verification Required If Changed

- `cargo test -p ta_app_core iterative_ -- --nocapture`
- `cargo test -p ta_app_core double_ma_validation_iterative_engine_emits_delta_fields -- --nocapture`
- `cargo test -p ta_app_core double_ma_validation_iterative_engine_matches_optimizer_top_candidates -- --nocapture`
- `cargo test -p ta_app_core composite_validation_iterative_engine_emits_delta_fields -- --nocapture`
- add and run focused tests for:
  - invalid input-shape handling
  - all `IntrabarRule` tie cases
  - overlay + intrabar interaction cases
  - currently zero-filled `Metrics` field expectations

## Regression Risk And Feature-Preservation Constraints

- do not casually change intrabar semantics, overlay rules, or flip/close accounting under the banner of cleanup
- if invalid input handling changes, preserve compatibility deliberately rather than by accident
- if additional `Metrics` fields start being computed, treat that as an observable semantics change and verify downstream scoring/reporting behavior
