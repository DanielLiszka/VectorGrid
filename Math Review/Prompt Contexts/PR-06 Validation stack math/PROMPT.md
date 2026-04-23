# PR-06 Validation stack math

- package type: `core`
- copied context files: `14`
- missing context files: `0`

## Prompt Scope

## `PR-06` Validation stack math

Owned problem:
the full validation ranking and verdict stack, including single-candidate
statistics, reopt/regime summaries, and cross-ticker aggregation

Source packets:
`MR-08`, `MR-09`, `MR-10`, `MR-13`

Primary ownership files:

- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/validation_shared.rs`
- `crates/app_core/src/sharpe_stats.rs`
- `crates/app_core/src/cscv_pbo.rs`
- `crates/app_core/src/validation_stats.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/composite_validate.rs`
- `src-tauri/src/validation_common.rs`
- `src-tauri/src/commands_validation.rs`

Context files:

- `src-tauri/src/commands_reports_validation_common.rs`
- `src-tauri/src/bin/ta_double_ma_validation_cost.rs`
- `src-tauri/src/bin/ta_headless_validate.rs`
- `src-tauri/src/state.rs`
- `crates/app_core/src/bin/real_data_fidelity_audit.rs`

Own these symbols or decisions:

- robust-score and verdict contract
- PSR or DSR semantics
- bootstrap, CSCV/PBO, and superiority-test math
- surrogate slice-return assumptions
- reopt walk-forward summaries
- regime or bucket summaries
- cross-ticker aggregation and verdict rebuild math

Open problem:

- characterize a validation stack whose verdicts remain mathematically coherent
  under dependence, reruns, regime slicing, and cross-ticker aggregation

Why this is hard here:

- single-candidate statistics, walk-forward summaries, cross-ticker aggregation,
  and final verdict logic are coupled rather than separable post-processors
- dependence, overlapping windows, and surrogate slice-return constructions
  challenge naive exactness assumptions

Mathematical object:

- a layered validation functional that maps path data and rerun evidence into
  intermediate statistics, robust scores, and final verdict classes

Notation seed:

- let `E(c)` denote the full validation evidence collected for candidate `c`
- let `T(E(c))` be the vector of intermediate statistics
  `(psr, dsr, spa, cscv, bootstrap, reopt, regime, cross)`
- let `R(c) = \rho(T(E(c)))` be the robust-score map
- let `V(c) = \nu(T(E(c)), R(c))` be the final verdict class
- coherence requires that equivalent evidence implies equivalent verdicts:
  `T(E(c_1)) = T(E(c_2)) \Rightarrow V(c_1) = V(c_2)`

Primary questions:

- what sufficient statistics are needed for coherent verdicts across the full
  validation stack?
- when are current surrogate slice-return constructions exact, and when do they
  introduce identifiable bias?
- what dependence-aware aggregation rule yields stable verdicts across reruns,
  regimes, and tickers?

Required deliverables:

- formal verdict-stack model
- theorem or lemma candidates for verdict coherence
- one counterexample family for false confidence or incoherent verdicts
- explicit bias, regret, or instability bounds for any approximation

Acceptance bar:

- every non-exact validation shortcut must expose a measurable approximation
  contract

Verification hooks:

- verdict parity under rerun perturbations
- cross-ticker consistency checks
- adversarial dependence cases

Non-goals:

- do not redesign meta ranking or ML-confidence policy; that belongs to `PR-07`
- do not redesign ML-signal policy; that belongs to `PR-10`
- do not redesign report workbook structure; that is context only

Files to provide as context:

- all primary ownership files
- all context files

Prompt-specific addenda:

- explicitly assess the coupling between `robust_score`, `final_score`, and
  cross-ticker aggregation
- explicitly compare family differences in regime definitions

## First-Pass Prompt

Treat `PR-06` as an open mathematical problem, not a code-review task. Stay inside the owned problem and respect the non-goals. Use the primary ownership files as the redesign surface and the context files as constraints only. Do not propose implementation yet.

Use the following prompt scope block exactly:

```md
## `PR-06` Validation stack math

Owned problem:
the full validation ranking and verdict stack, including single-candidate
statistics, reopt/regime summaries, and cross-ticker aggregation

Source packets:
`MR-08`, `MR-09`, `MR-10`, `MR-13`

Primary ownership files:

- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/validation_shared.rs`
- `crates/app_core/src/sharpe_stats.rs`
- `crates/app_core/src/cscv_pbo.rs`
- `crates/app_core/src/validation_stats.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/composite_validate.rs`
- `src-tauri/src/validation_common.rs`
- `src-tauri/src/commands_validation.rs`

Context files:

- `src-tauri/src/commands_reports_validation_common.rs`
- `src-tauri/src/bin/ta_double_ma_validation_cost.rs`
- `src-tauri/src/bin/ta_headless_validate.rs`
- `src-tauri/src/state.rs`
- `crates/app_core/src/bin/real_data_fidelity_audit.rs`

Own these symbols or decisions:

- robust-score and verdict contract
- PSR or DSR semantics
- bootstrap, CSCV/PBO, and superiority-test math
- surrogate slice-return assumptions
- reopt walk-forward summaries
- regime or bucket summaries
- cross-ticker aggregation and verdict rebuild math

Open problem:

- characterize a validation stack whose verdicts remain mathematically coherent
  under dependence, reruns, regime slicing, and cross-ticker aggregation

Why this is hard here:

- single-candidate statistics, walk-forward summaries, cross-ticker aggregation,
  and final verdict logic are coupled rather than separable post-processors
- dependence, overlapping windows, and surrogate slice-return constructions
  challenge naive exactness assumptions

Mathematical object:

- a layered validation functional that maps path data and rerun evidence into
  intermediate statistics, robust scores, and final verdict classes

Notation seed:

- let `E(c)` denote the full validation evidence collected for candidate `c`
- let `T(E(c))` be the vector of intermediate statistics
  `(psr, dsr, spa, cscv, bootstrap, reopt, regime, cross)`
- let `R(c) = \rho(T(E(c)))` be the robust-score map
- let `V(c) = \nu(T(E(c)), R(c))` be the final verdict class
- coherence requires that equivalent evidence implies equivalent verdicts:
  `T(E(c_1)) = T(E(c_2)) \Rightarrow V(c_1) = V(c_2)`

Primary questions:

- what sufficient statistics are needed for coherent verdicts across the full
  validation stack?
- when are current surrogate slice-return constructions exact, and when do they
  introduce identifiable bias?
- what dependence-aware aggregation rule yields stable verdicts across reruns,
  regimes, and tickers?

Required deliverables:

- formal verdict-stack model
- theorem or lemma candidates for verdict coherence
- one counterexample family for false confidence or incoherent verdicts
- explicit bias, regret, or instability bounds for any approximation

Acceptance bar:

- every non-exact validation shortcut must expose a measurable approximation
  contract

Verification hooks:

- verdict parity under rerun perturbations
- cross-ticker consistency checks
- adversarial dependence cases

Non-goals:

- do not redesign meta ranking or ML-confidence policy; that belongs to `PR-07`
- do not redesign ML-signal policy; that belongs to `PR-10`
- do not redesign report workbook structure; that is context only

Files to provide as context:

- all primary ownership files
- all context files

Prompt-specific addenda:

- explicitly assess the coupling between `robust_score`, `final_score`, and
  cross-ticker aggregation
- explicitly compare family differences in regime definitions
```

Required deliverables:

1. Formal problem statement
2. Notation for the search space, state space, path functional, ranking functional, and admissible approximations
3. Fixed invariants and interface constraints
4. Strongest plausible positive result
5. Strongest plausible impossibility result
6. Candidate theorem or lemma list
7. One explicit counterexample family
8. One lower-bound, identifiability, or impossibility angle
9. Unresolved questions that block implementation

## Second-Pass Prompt

Attack the first-pass answer for `PR-06` as an adversarial mathematician. Try to break the leading formalization, proposed sufficient statistics, pruning rules, ranking claims, or approximation contracts with edge cases, dependence structure, pathologies, and finite-grid effects. Do not redesign out-of-scope surfaces.

Use the following prompt scope block exactly:

```md
## `PR-06` Validation stack math

Owned problem:
the full validation ranking and verdict stack, including single-candidate
statistics, reopt/regime summaries, and cross-ticker aggregation

Source packets:
`MR-08`, `MR-09`, `MR-10`, `MR-13`

Primary ownership files:

- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/validation_shared.rs`
- `crates/app_core/src/sharpe_stats.rs`
- `crates/app_core/src/cscv_pbo.rs`
- `crates/app_core/src/validation_stats.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/composite_validate.rs`
- `src-tauri/src/validation_common.rs`
- `src-tauri/src/commands_validation.rs`

Context files:

- `src-tauri/src/commands_reports_validation_common.rs`
- `src-tauri/src/bin/ta_double_ma_validation_cost.rs`
- `src-tauri/src/bin/ta_headless_validate.rs`
- `src-tauri/src/state.rs`
- `crates/app_core/src/bin/real_data_fidelity_audit.rs`

Own these symbols or decisions:

- robust-score and verdict contract
- PSR or DSR semantics
- bootstrap, CSCV/PBO, and superiority-test math
- surrogate slice-return assumptions
- reopt walk-forward summaries
- regime or bucket summaries
- cross-ticker aggregation and verdict rebuild math

Open problem:

- characterize a validation stack whose verdicts remain mathematically coherent
  under dependence, reruns, regime slicing, and cross-ticker aggregation

Why this is hard here:

- single-candidate statistics, walk-forward summaries, cross-ticker aggregation,
  and final verdict logic are coupled rather than separable post-processors
- dependence, overlapping windows, and surrogate slice-return constructions
  challenge naive exactness assumptions

Mathematical object:

- a layered validation functional that maps path data and rerun evidence into
  intermediate statistics, robust scores, and final verdict classes

Notation seed:

- let `E(c)` denote the full validation evidence collected for candidate `c`
- let `T(E(c))` be the vector of intermediate statistics
  `(psr, dsr, spa, cscv, bootstrap, reopt, regime, cross)`
- let `R(c) = \rho(T(E(c)))` be the robust-score map
- let `V(c) = \nu(T(E(c)), R(c))` be the final verdict class
- coherence requires that equivalent evidence implies equivalent verdicts:
  `T(E(c_1)) = T(E(c_2)) \Rightarrow V(c_1) = V(c_2)`

Primary questions:

- what sufficient statistics are needed for coherent verdicts across the full
  validation stack?
- when are current surrogate slice-return constructions exact, and when do they
  introduce identifiable bias?
- what dependence-aware aggregation rule yields stable verdicts across reruns,
  regimes, and tickers?

Required deliverables:

- formal verdict-stack model
- theorem or lemma candidates for verdict coherence
- one counterexample family for false confidence or incoherent verdicts
- explicit bias, regret, or instability bounds for any approximation

Acceptance bar:

- every non-exact validation shortcut must expose a measurable approximation
  contract

Verification hooks:

- verdict parity under rerun perturbations
- cross-ticker consistency checks
- adversarial dependence cases

Non-goals:

- do not redesign meta ranking or ML-confidence policy; that belongs to `PR-07`
- do not redesign ML-signal policy; that belongs to `PR-10`
- do not redesign report workbook structure; that is context only

Files to provide as context:

- all primary ownership files
- all context files

Prompt-specific addenda:

- explicitly assess the coupling between `robust_score`, `final_score`, and
  cross-ticker aggregation
- explicitly compare family differences in regime definitions
```

Required deliverables:

1. Explicit failure modes
2. Adversarial constructions or separating examples
3. Corrected theorem or lemma candidates
4. Weakest assumptions under which the result survives
5. Distinction between exact, approximate, and heuristic claims
6. Revised time, memory, and retained-state costs
7. Dependency burden on other PR prompts

Narrow to the strongest surviving repo-fit result.

## Third-Pass Prompt

Produce the strongest implementation-ready mathematical result for `PR-06` that survives the second pass. Stay inside the owned problem. Treat listed dependencies as interfaces, not as permission to redesign adjacent prompts. Any non-exact claim must come with an explicit approximation, error, regret, rank-instability, or bias contract.

Use the following prompt scope block exactly:

```md
## `PR-06` Validation stack math

Owned problem:
the full validation ranking and verdict stack, including single-candidate
statistics, reopt/regime summaries, and cross-ticker aggregation

Source packets:
`MR-08`, `MR-09`, `MR-10`, `MR-13`

Primary ownership files:

- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/validation_shared.rs`
- `crates/app_core/src/sharpe_stats.rs`
- `crates/app_core/src/cscv_pbo.rs`
- `crates/app_core/src/validation_stats.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/composite_validate.rs`
- `src-tauri/src/validation_common.rs`
- `src-tauri/src/commands_validation.rs`

Context files:

- `src-tauri/src/commands_reports_validation_common.rs`
- `src-tauri/src/bin/ta_double_ma_validation_cost.rs`
- `src-tauri/src/bin/ta_headless_validate.rs`
- `src-tauri/src/state.rs`
- `crates/app_core/src/bin/real_data_fidelity_audit.rs`

Own these symbols or decisions:

- robust-score and verdict contract
- PSR or DSR semantics
- bootstrap, CSCV/PBO, and superiority-test math
- surrogate slice-return assumptions
- reopt walk-forward summaries
- regime or bucket summaries
- cross-ticker aggregation and verdict rebuild math

Open problem:

- characterize a validation stack whose verdicts remain mathematically coherent
  under dependence, reruns, regime slicing, and cross-ticker aggregation

Why this is hard here:

- single-candidate statistics, walk-forward summaries, cross-ticker aggregation,
  and final verdict logic are coupled rather than separable post-processors
- dependence, overlapping windows, and surrogate slice-return constructions
  challenge naive exactness assumptions

Mathematical object:

- a layered validation functional that maps path data and rerun evidence into
  intermediate statistics, robust scores, and final verdict classes

Notation seed:

- let `E(c)` denote the full validation evidence collected for candidate `c`
- let `T(E(c))` be the vector of intermediate statistics
  `(psr, dsr, spa, cscv, bootstrap, reopt, regime, cross)`
- let `R(c) = \rho(T(E(c)))` be the robust-score map
- let `V(c) = \nu(T(E(c)), R(c))` be the final verdict class
- coherence requires that equivalent evidence implies equivalent verdicts:
  `T(E(c_1)) = T(E(c_2)) \Rightarrow V(c_1) = V(c_2)`

Primary questions:

- what sufficient statistics are needed for coherent verdicts across the full
  validation stack?
- when are current surrogate slice-return constructions exact, and when do they
  introduce identifiable bias?
- what dependence-aware aggregation rule yields stable verdicts across reruns,
  regimes, and tickers?

Required deliverables:

- formal verdict-stack model
- theorem or lemma candidates for verdict coherence
- one counterexample family for false confidence or incoherent verdicts
- explicit bias, regret, or instability bounds for any approximation

Acceptance bar:

- every non-exact validation shortcut must expose a measurable approximation
  contract

Verification hooks:

- verdict parity under rerun perturbations
- cross-ticker consistency checks
- adversarial dependence cases

Non-goals:

- do not redesign meta ranking or ML-confidence policy; that belongs to `PR-07`
- do not redesign ML-signal policy; that belongs to `PR-10`
- do not redesign report workbook structure; that is context only

Files to provide as context:

- all primary ownership files
- all context files

Prompt-specific addenda:

- explicitly assess the coupling between `robust_score`, `final_score`, and
  cross-ticker aggregation
- explicitly compare family differences in regime definitions
```

Required deliverables:

1. Final formalization
2. Final theorem or lemma list
3. Proof sketch or proof plan
4. Sufficient statistics, or a proof they do not exist under the stated constraints
5. Exactness conditions and equivalence classes
6. Error, regret, or bias bounds for every non-exact claim
7. Time, memory, and retained-state complexity
8. File and symbol impact map
9. Verification obligations and falsifiers
10. Dependencies that must be reconciled during consolidation
11. Kill criteria

## Local Repo Copies

The copied files below are stored directly under this prompt folder, preserving repo-relative paths.

- `crates/app_core/src/bin/real_data_fidelity_audit.rs`
- `crates/app_core/src/composite_validate.rs`
- `crates/app_core/src/cscv_pbo.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/sharpe_stats.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/validation_shared.rs`
- `crates/app_core/src/validation_stats.rs`
- `src-tauri/src/bin/ta_double_ma_validation_cost.rs`
- `src-tauri/src/bin/ta_headless_validate.rs`
- `src-tauri/src/commands_reports_validation_common.rs`
- `src-tauri/src/commands_validation.rs`
- `src-tauri/src/state.rs`
- `src-tauri/src/validation_common.rs`
