# Review Packet: `crates/app_core/src/validation_pipeline.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/validation_pipeline.rs`
- Packet status: `Reviewed`
- Public-review readiness: `High-value shared validation engine, but not fully review-ready because multiple statistical subsystems, clamped defaults, and verdict-compression rules are only partially documented`
- Recommended action bucket: `Larger refactor but defer`
- Sensitivity tags: `Validation-engine`, `Statistics`, `Parallelism`, `Semantics-sensitive`, `Shared-infrastructure`

## What This File Owns

`validation_pipeline.rs` is the shared statistical and verdict engine used by multiple validation flows. It owns:

- hard-gate config and verdict/result types:
  - `26-95`
- the `ValidationPostCandidate` trait:
  - `97-119`
- PSR/DSR integration, robust-score computation, Pareto marking, CSCV folding, and hard-gate application:
  - `121-436`
- run-verdict construction:
  - `437-580`
- parallel candidate collection:
  - `581-616`
- walk-forward and re-optimization statistics:
  - `618-1063`
- perturbation, trade-bootstrap, iterative-validation, CSCV/PBO finalization, and SPA:
  - `1065-1629`
- file-local tests:
  - `1630-2318`

## Classification

- Public surface: effectively yes
  - used by `double_ma_validate.rs`, `composite_validate.rs`, `ml_signal.rs`, and Tauri validation commands
- Hot path: partially
  - validation is batch work, but this file contains nontrivial statistical loops and parallel collection
- Contract-critical: yes
- Shared-infrastructure: yes
- Benchmark-sensitive: yes
  - changes here alter validation rankings, gating, and statistical verdicts

## Direct Interactions

### Backend interaction map

- major callers:
  - `crates/app_core/src/double_ma_validate.rs`
  - `crates/app_core/src/composite_validate.rs`
  - `crates/app_core/src/ml_signal.rs`
  - `src-tauri/src/commands_validation.rs`
- statistical dependencies:
  - `crates/app_core/src/sharpe_stats.rs`
  - `crates/app_core/src/cscv_pbo.rs`
  - `crates/app_core/src/validation_stats.rs`
  - `crates/app_core/src/validation_shared.rs`

### Downstream verification impact if changed

- changing scoring or gating changes candidate ranking and approval
- changing run-verdict mapping changes product-facing validation recommendations
- changing bootstrap or SPA defaults changes statistical reproducibility
- changing parallel collection or cancel semantics changes batch runtime behavior

## Standards And Research Log

This packet was written after targeted web research focused on the primary statistical methods and the parallel-iterator contract used in collection.

### Sources consulted

1. David H. Bailey and Marcos Lopez de Prado, *The Deflated Sharpe Ratio*
   - Source: <https://www.davidhbailey.com/dhbpapers/deflated-sharpe.pdf>
   - Why it was consulted:
     - this file computes PSR/DSR-derived gating and approval signals through `validation_stats`
   - Standard informed:
     - probability-based ranking/gating should preserve its current statistical assumptions unless a deliberate methodology change is approved

2. David H. Bailey et al., *The Probability of Backtest Overfitting*
   - Source: <https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2326253>
   - Why it was consulted:
     - this file integrates CSCV/PBO into candidate scoring and hard-gate handling
   - Standard informed:
     - CSCV/PBO behavior is methodology-sensitive and should not be casually simplified

3. Peter R. Hansen, *A Test for Superior Predictive Ability*
   - Source: <http://math.ku.dk/~rolf/teaching/thesis/The%20Model%20Confidence%20Set/Hansen.TestSuperiorPredictiveAbility.JBES05.pdf>
   - Why it was consulted:
     - the file implements SPA-based validation statistics
   - Standard informed:
     - bootstrap/sample-count defaults and significance interpretation need documentation

4. `rayon::iter::ParallelIterator`
   - Source: <https://docs.rs/rayon/latest/rayon/iter/trait.ParallelIterator.html>
   - Why it was consulted:
     - candidate collection uses Rayon parallel iteration with cancellation and progress updates
   - Standard informed:
     - parallel collection contracts should make side effects and ordering assumptions explicit

## Findings

## Module Boundaries

### 1. This file is doing too many statistically distinct jobs at once

- Evidence:
  - scoring/gating and verdict logic: `121-580`
  - parallel collection: `581-616`
  - walk-forward / reopt: `618-1063`
  - perturb and trade bootstrap: `1065-1342`
  - CSCV/PBO finalization and SPA: `1367-1629`
- Why this matters:
  - a maintainer touching one methodology must read a wide unrelated surface
  - the file is shared infrastructure, so broad edits here have a large blast radius
- Smallest justified fix:
  - do not split the module immediately
  - first document the subsystem boundaries, then defer a bounded extraction by statistical family

## Statistical Defaults And Guardrails

### 2. Several clamped defaults and minimums are important methodology choices, but they are largely implicit

- Evidence:
  - walk-forward minimum window/step handling: `668-736`
  - CSCV slices sanitized to even values between `4` and `32`: `1449-1457`
  - SPA bootstrap samples are floored to `100`: `1572`
  - trade bootstrap and stationary bootstrap use local minimum/fallback rules: `1202-1499`
- Why this matters:
  - these are not mere implementation details; they affect whether validation outputs are statistically comparable across runs
  - “cleanup” edits could silently change methodology
- Smallest justified fix:
  - add local comments/rustdoc explaining which defaults are methodological guardrails versus incidental implementation choices

## Verdict Semantics

### 3. `build_run_verdict` compresses multiple failure modes into a coarse final recommendation

- Evidence:
  - if `fail_reasons` is non-empty, `final_recommendation` becomes `NoCandidateSurvived`: `517-520`
  - that includes cases like:
    - `pbo_above_max:*`
    - `pbo_gate_requested_but_cscv_unavailable`
    - and other post-candidate run-level failures
- Why this matters:
  - the verdict still carries detailed flags, but the top-level recommendation can under-describe why the run failed
  - product-facing consumers may overinterpret `NoCandidateSurvived` as “all candidates failed hard gates” even when the real issue was a later statistical gate
- Smallest justified fix:
  - document the current compression rule
  - defer any recommendation-enum expansion until downstream UI/report consumers are reviewed together

## Trait Design

### 4. `ValidationPostCandidate` is broad and setter-heavy, which tightly couples the shared engine to caller-owned candidate structs

- Evidence:
  - the trait spans in-sample metrics, walk-forward stats, CSCV fields, track-record fields, robust score, Pareto state, and verdict state: `97-119`
- Why this matters:
  - the trait works, but it requires every caller type to expose a large mutation surface just to participate in the shared engine
  - this raises the cost of introducing new validation result types
- Smallest justified fix:
  - document the required candidate lifecycle now
  - defer trait decomposition until the shared validation surface changes again

## Tests And Missing Verification

### 5. The test coverage is solid and deterministic, which is a major strength of the file

- Evidence:
  - tests cover scoring, quantiles, walk-forward-related helpers, trade bootstrap determinism, and SPA determinism: `1630-2318`
- Residual gap:
  - the tests do not explicitly explain the methodology-driven default floors and clamps listed above
- Smallest justified fix:
  - preserve the tests and add comments/docs before touching defaults

## Code-Quality Improvement Register

- Safe now:
  - document subsystem boundaries
  - document methodology-sensitive floors/clamps
  - document run-verdict compression rules
  - document `ValidationPostCandidate` lifecycle expectations
- Defer:
  - splitting the file by subsystem
  - changing validation math or bootstrap defaults
  - changing recommendation semantics

## Technical-Debt Reduction Register

- Safe now:
  - documentation hardening
  - maybe tiny helper comments around statistical guardrails
- Defer:
  - module extraction
  - trait redesign
  - verdict enum expansion
- Do not casually remove:
  - existing deterministic test coverage
  - current CSCV/PBO and SPA integration

## Performance Opportunity Register

- Safe structural work:
  - none beyond documentation and very local comments
- Benchmark-sensitive:
  - changes to parallel collection, bootstrap loops, or statistical defaults

## Recommended Action

`Larger refactor but defer`

This file is shared validation infrastructure and already has good tests. The immediate value is to make the methodology and verdict contracts explicit, not to reshuffle the math engine under active consumers.

## Exact Local Verification Required If Changed

1. File-local tests:
   - `cargo test -p ta_app_core validation_pipeline::tests:: -- --nocapture`
2. Representative Double-MA validation surface:
   - `cargo test -p ta_app_core double_ma_validate -- --nocapture`
3. Representative composite validation surface:
   - `cargo test -p ta_app_core composite_validate -- --nocapture`

### Verification run for this packet

- Passed:
  - `cargo test -p ta_app_core validation_pipeline::tests:: -- --nocapture`

## Regression Risk And Feature-Preservation Constraints

- Do not alter statistical floors, clamps, or recommendation mapping without reviewing reports/UI consumers and reproducibility expectations together
- Do not split this file until subsystem boundaries are documented; otherwise the risk is accidental methodology drift during extraction
- Treat the current deterministic tests as contract locks, not optional examples
