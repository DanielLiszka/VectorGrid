# Review Packet: `crates/app_core/src/cscv_pbo.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/cscv_pbo.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Useful shared validation helper, but not public-review ready as-is because the current relative-rank normalization appears to diverge from the primary CSCV/PBO method, the public tuple contract is under-specified on skipped paths, and several important assumptions are only encoded implicitly in code`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Public-surface`, `Validation-contract-sensitive`, `Methodology-sensitive`, `Cross-layer-reporting-sensitive`, `Low-throughput-risk`

## What This File Owns

`cscv_pbo.rs` is the shared CSCV/PBO math helper for validation flows. It owns the public config and result DTOs, slice-level sufficient-stat inputs, combination sampling helpers, rank/logit computation, and the exported function that turns per-candidate slice statistics into CSCV/PBO summary output plus per-candidate annotations.

This file owns:

- public CSCV/PBO config:
  - `CscvPboConfig`: `crates/app_core/src/cscv_pbo.rs:18-27`
- slice sufficient-stat input shape:
  - `SliceReturnStats`: `31-35`
- serialized per-candidate and summary outputs:
  - `CscvCandidatePboStats`: `38-45`
  - `CscvPboResult`: `47-63`
- slice sanitization and combination helpers:
  - `sanitize_slices`: `65-72`
  - `n_choose_k`: `74-87`
  - `all_k_masks`: `89-106`
  - `sample_k_masks`: `108-126`
- in-file metric/rank/stat helpers:
  - `sharpe_from_sum_sumsq`: `128-140`
  - `ranks_average_ascending`: `142-165`
  - `quantile`: `167-177`
  - `regression_slope_intercept_r2`: `179-207`
- public computation entrypoint:
  - `compute_cscv_pbo_from_slices`: `236-476`
- file-local smoke test:
  - `482-525`

This file does not own:

- building slice statistics from validated candidates
  - that belongs to validation callers such as `crates/app_core/src/validation_pipeline.rs:1392-1416`
- hard-gate policy that consumes CSCV outputs
  - `crates/app_core/src/validation_pipeline.rs:364-410`
- report/export/front-end presentation of CSCV fields
  - downstream Tauri/web/report layers consume the serialized output, but this file does not own their thresholds or sort policy

## Classification

- Public surface: yes
  - the module is exported at `crates/app_core/src/lib.rs:17`, and the DTOs are re-exported at `crates/app_core/src/lib.rs:164`
- Hot path: moderate
  - it runs only when CSCV/PBO is enabled, but then it sits on the validation finishing path
- CUDA/runtime-critical: no
  - CPU-only statistics helper
- Strategy-local: no
  - shared across validation families
- Low-risk support code: mixed
  - implementation is compact and bounded, but it carries methodology-sensitive semantics that feed hard gates, ranking, and reports

## Direct Interactions

### Backend interaction map

- public re-export surface:
  - `crates/app_core/src/lib.rs:17`
  - `crates/app_core/src/lib.rs:164`
- generic validation orchestration:
  - `crates/app_core/src/validation_core.rs:7-19`
  - `crates/app_core/src/validation_core.rs:44-58`
- caller that prepares per-candidate slice stats and attaches per-candidate CSCV annotations:
  - `crates/app_core/src/validation_pipeline.rs:1371-1418`
- caller that applies hard gates using `oos_rank_frac_p50`:
  - `crates/app_core/src/validation_pipeline.rs:364-410`
- double-MA validation DTO surface:
  - `crates/app_core/src/double_ma_validate.rs:1514`
  - `1575`
  - `1677`
- composite validation DTO surface:
  - `crates/app_core/src/composite_validate.rs:9606`
  - `9659`
  - `9763`

### Downstream verification impact if changed

- any change to rank normalization affects:
  - `CscvPboResult.lambda_p25/p50/p75`
  - `CscvCandidatePboStats.oos_rank_frac_p50`
  - hard-gate outcomes in `validation_pipeline.rs`
  - report ordering and summaries in Tauri/web layers
- any change to skipped-path tuple shape affects:
  - generic validation callers that may assume positional `cand_stats.len() == candidates`
- any change to the metric definition affects:
  - double-MA and composite validation parity expectations
  - report wording, if the exported fields continue to look metric-agnostic

## Standards And Research Log

This packet was written after targeted web research focused on the primary CSCV/PBO method, public Rust API contract clarity, and test expectations for small numerical helpers whose main risk is silent semantic drift rather than memory safety.

### Sources consulted

1. Bailey et al., *The Probability of Backtest Overfitting*
   - Source: <https://carmamaths.org/jon/backtest2.pdf>
   - Why it was consulted:
     - this file implements CSCV/PBO directly, so the primary method matters more than repo-local convention
   - Standard informed:
     - Algorithm 2.3 defines relative OOS rank as `r / (N + 1)` and PBO as the mass where the logit is at or below the OOS median boundary

2. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this module exports config/result types and a public helper that other modules consume positionally
   - Standard informed:
     - public contracts should be explicit, predictable, and unsurprising on edge paths

3. Rust API Guidelines, Documentation
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why it was consulted:
     - the public types and function currently rely on caller knowledge rather than rustdoc
   - Standard informed:
     - public APIs should document assumptions, units, and important caveats

4. Rust Book, How to Write Tests
   - Source: <https://doc.rust-lang.org/book/ch11-01-writing-tests.html>
   - Why it was consulted:
     - the most important risks here are methodological edge cases that should be pinned by small focused tests
   - Standard informed:
     - narrow behavior contracts should be locked with explicit tests rather than left implicit

## Findings

## Correctness And Safety

### 1. The relative-rank normalization appears to diverge from the primary CSCV/PBO method

- Evidence:
  - current implementation computes:
    - `omega = os_rank / candidates`: `crates/app_core/src/cscv_pbo.rs:389-392`
    - stored candidate rank fraction uses the same normalization: `407`
  - Bailey et al. define relative OOS rank as:
    - `omega = r / (N + 1)`: `turn5view0` lines `387-401`
  - PBO is then derived from the logit distribution relative to the OOS median boundary:
    - `turn5view0` lines `434-448`
- Why it matters:
  - this is not a cosmetic convention change
  - it shifts `lambda_p25`, `lambda_p50`, `lambda_p75`, and every exported `oos_rank_frac_p50`
  - for odd candidate counts it changes the median boundary case itself
  - concrete example:
    - with `N = 3` and selected OOS rank `2`, the paper’s method yields `omega = 2 / 4 = 0.5`, so `lambda = 0`
    - the current code yields `omega = 2 / 3`, so `lambda > 0`, and that split is no longer counted as at-or-below-median overfit
- Cross-file impact:
  - hard gates consume `oos_rank_frac_p50` directly:
    - `crates/app_core/src/validation_pipeline.rs:389-400`
  - report and UI layers consume and sort on the same exported value
- Smallest justified fix:
  - change the normalization in both places to divide by `candidates + 1`
  - keep the current `lambda <= 0.0` PBO cutoff
  - add an odd-`N` unit test that pins the exact median-rank case
- Review judgment:
  - highest-priority finding in this file
  - semantics-sensitive because downstream thresholds and report fixtures may need to move in lockstep

## API And Naming

### 1. The public helper returns positional per-candidate stats on the normal path but returns `Vec::new()` on skip paths

- Evidence:
  - public helper signature returns `(CscvPboResult, Vec<CscvCandidatePboStats>)`:
    - `crates/app_core/src/cscv_pbo.rs:236-243`
  - normal path builds one stats entry per candidate:
    - `432-454`
  - skip paths for `candidates < 2`, `slice_bars < 2`, and `combos == 0` return `Vec::new()`:
    - `247-267`
    - `269-289`
    - `311-331`
- Why it matters:
  - callers can reasonably assume the second tuple element remains positionally aligned with the candidate input, because that is the normal-path contract
  - the current in-repo generic caller avoids the issue by short-circuiting before the call when the candidate count or slice bars are too small:
    - `crates/app_core/src/validation_pipeline.rs:1388-1422`
  - but the exported helper itself does not say that skipped calls collapse the per-candidate payload to length zero
- Smallest justified fix:
  - either document that skipped results intentionally return no per-candidate stats
  - or better, return `candidates` placeholder entries with `is_selected_frac = 0.0` and `None` medians
- Review judgment:
  - real public-surface footgun
  - safe-now fix if handled as documentation or placeholder-fill

### 2. The result surface reads metric-agnostic, but the implementation is hardwired to Sharpe

- Evidence:
  - the file-level score function is explicitly `sharpe_from_sum_sumsq`:
    - `crates/app_core/src/cscv_pbo.rs:128-140`
  - IS and OOS scores both use that helper:
    - `374-375`
  - exported field names are generic:
    - `CscvCandidatePboStats.oos_metric_p50`: `41-44`
    - `CscvPboResult.threshold`, `prob_oos_below_threshold`: `58-62`
- Why it matters:
  - the current naming is flexible, but it can be misread as metric-pluggable when this helper presently assumes Sharpe-style sufficient statistics
  - this matters for future reuse and for documentation of caller expectations
- Smallest justified fix:
  - document explicitly that this helper currently computes CSCV/PBO over Sharpe estimated from per-slice `sum` and `sumsq`
  - if true metric-pluggability is ever desired, that should be a later bounded design change, not inferred from generic field names
- Review judgment:
  - documentation-first fix is sufficient now

### 3. `max_combinations = 0` is silently coerced into one sampled combination

- Evidence:
  - configuration default exists:
    - `crates/app_core/src/cscv_pbo.rs:10-16`
  - runtime logic forces `cfg.max_combinations.max(1)`:
    - `303`
- Why it matters:
  - a caller can reasonably expect `0` to mean disabled or zero combinations, but the implementation always samples at least one split
  - this is a contract ambiguity, not a crash bug
- Smallest justified fix:
  - document the coercion explicitly
  - if the team wants `0` to mean disabled, that is a semantics change that should be reviewed across callers
- Review judgment:
  - low-severity API surprise

## Error Handling

### 1. The file’s operational error handling is otherwise coherent

- Evidence:
  - malformed slice counts and cancellation return ordinary `Err(String)` values:
    - `291-297`
    - `353-355`
  - skip conditions return structured `CscvPboResult` values with `skipped_reason`
    - `247-331`
- Why it matters:
  - the main risk in this module is semantic drift, not missing recoverable error transport
- Review judgment:
  - no immediate error-handling rewrite is needed

## Module Boundaries And Technical Debt

### 1. A shared helper hardcodes the composite-specific CSCV progress phase label

- Evidence:
  - `compute_cscv_pbo_from_slices` emits progress with `"composite_validate_cscv"`:
    - `crates/app_core/src/cscv_pbo.rs:351`
    - `412-417`
  - the generic caller already carries a caller-supplied phase label:
    - `crates/app_core/src/validation_pipeline.rs:1377`
    - `1392`
    - `1405-1410`
- Why it matters:
  - this module is shared by multiple validation owners, but one stage name is baked into the deeper helper
  - that makes progress taxonomy less truthful and makes later refactors harder than necessary
- Smallest justified fix:
  - thread the phase label into this helper as an argument, or centralize all CSCV progress emission in the caller
- Review judgment:
  - small technical-debt reduction
  - safe now

## Docs And Examples

### 1. The public DTOs and entrypoint do not document their assumptions

- Evidence:
  - public types and function have no rustdoc beyond a few private helpers:
    - `CscvPboConfig`: `18`
    - `SliceReturnStats`: `31`
    - `CscvCandidatePboStats`: `38`
    - `CscvPboResult`: `47`
    - `compute_cscv_pbo_from_slices`: `236`
- Why it matters:
  - important assumptions are currently implicit:
    - slice count is clamped to an even range `4..=32`: `65-72`
    - all candidates must have the same sanitized slice count: `291-297`
    - the metric is Sharpe estimated from `sum` and `sumsq`: `128-140`, `374-375`
    - skipped results can still be “successful” with `skipped_reason`: `247-331`
- Smallest justified fix:
  - add rustdoc documenting these assumptions and the meaning of each exported field
- Review judgment:
  - safe-now documentation gap

## Tests And Missing Verification

### 1. The existing smoke test is useful, but it does not pin the highest-risk methodology edge case

- Evidence:
  - current test exercises an overfit pattern and a simple candidate-stat expectation:
    - `crates/app_core/src/cscv_pbo.rs:482-525`
  - there is no explicit odd-`N` median-rank test
- Why it matters:
  - the normalization boundary is exactly where silent semantic drift is most likely
- Smallest justified fix:
  - add a test with `N = 3` or another odd count where the selected strategy lands exactly at the OOS median rank and assert:
    - exported `oos_rank_frac_p50`
    - `lambda`
    - whether the split counts toward `pbo`

### 2. The skip-path tuple contract is not explicitly tested

- Evidence:
  - there is no test that asserts the shape of the returned candidate stats vector when:
    - `candidates < 2`
    - `slice_bars < 2`
    - sampled combination count is zero
- Why it matters:
  - without an explicit test, callers and maintainers can infer different contracts
- Smallest justified fix:
  - add targeted tests for skipped paths, either locking `Vec::new()` as intentional or locking placeholder-fill semantics if the contract is improved

### 3. The current verification surface should include downstream consumers if normalization changes

- Evidence:
  - hard-gate logic consumes `oos_rank_frac_p50`:
    - `crates/app_core/src/validation_pipeline.rs:389-400`
  - validation DTOs and reports expose the same field downstream
- Why it matters:
  - a method correction in this file can change ordering/gating/report values without changing compile-time types
- Review judgment:
  - any methodology change here needs cross-file regression verification, not just a local unit test

## Performance, Allocation, And CUDA Concerns

### 1. No immediate throughput issue was identified in this file

- Evidence:
  - slice count is clamped to a small even range:
    - `crates/app_core/src/cscv_pbo.rs:65-72`
  - combination count is capped by `max_combinations`:
    - `300-309`
  - major scratch buffers are allocated once and reused across combinations:
    - `337-349`
- Why it matters:
  - this is not the right module for speculative micro-optimization without benchmark evidence
- Review judgment:
  - accept current structure as performance-adequate for now

### 2. Sampling and normalization changes are semantics-sensitive, not optimization opportunities

- Evidence:
  - `max_combinations` and rank normalization influence the statistical output itself, not just runtime cost
- Why it matters:
  - these are not “perf cleanup” knobs
  - touching them changes methodology and downstream validation behavior
- Review judgment:
  - treat such changes as semantics-sensitive and verify accordingly

## Code-Quality Improvement Register

- Safe now:
  - document public DTO assumptions and skipped-path semantics
  - remove the hardcoded composite-specific progress phase coupling
  - add focused methodology and contract tests
- Coordination required:
  - correct rank normalization to `r / (N + 1)` and update any downstream thresholds, fixtures, or report expectations that were tuned against the old behavior
- Defer until later pass:
  - any attempt to make the helper metric-pluggable

## Technical-Debt Reduction Register

- Safe to reduce now:
  - undocumented public contract edges
  - hardcoded progress phase naming inside a shared helper
- Defer:
  - generic metric abstraction
  - broader CSCV configuration redesign
- Useful debt that should not be “cleaned up” casually:
  - bounded combinatorial sampling via `sanitize_slices` and `max_combinations`
  - skip-with-structured-result behavior, provided it is documented clearly

## Performance Opportunity Register

- Safe structural improvements:
  - none required right now
- Benchmark-sensitive or semantics-sensitive candidates:
  - changing sampling policy
  - changing score definition
  - changing relative-rank normalization
- Not the right place to chase performance:
  - this helper is bounded and numerically sensitive; correctness and contract clarity matter more than shaving small allocations

## Observability And Diagnostics Gaps

- missing explicit exposure of which rank normalization convention is in use
- progress labeling leaks one validation owner into a shared helper
- skipped-path contract is not obvious from the public signature

## Implementation Staging Guidance

- Safe now:
  - rustdoc for types and helper
  - progress phase decoupling
  - explicit tests for odd-`N` median-rank behavior and skip contracts
- Coordination required:
  - rank-normalization correction
  - any threshold or report updates that depend on `oos_rank_frac_p50`
- Defer until later pass:
  - true metric-pluggability or broader validation-stat API redesign

## Change Blast Radius

- Documentation/tests only:
  - local to `cscv_pbo.rs`
- Methodology correction:
  - same subsystem plus downstream validation/report consumers
- Threshold retuning:
  - cross-backend and frontend-reporting contract surface

## Recommended Action

`Small surgical refactor`

The best next pass is narrow:

- document the public contract
- add the missing boundary tests
- correct the relative-rank normalization if the team confirms adherence to Bailey et al. is the intended standard
- keep broader redesign ideas deferred

## Exact Local Verification Required If Changed

If only documentation or local tests change:

- `cargo test -p ta_app_core cscv_pbo -- --nocapture`

If rank normalization or tuple-shape semantics change:

- `cargo test -p ta_app_core cscv_pbo -- --nocapture`
- `cargo test -p ta_app_core double_ma_validation_cscv_smoke -- --nocapture`
- `cargo test -p ta_app_core composite_validation_cscv_pbo_runs_and_populates_fields -- --nocapture`
- targeted report/export tests that assert CSCV rank fields if any serialized expectations are updated

If hard-gate behavior is expected to move:

- add or run targeted tests around `validation_pipeline` CSCV rank-gate behavior
- verify any frontend or report fixtures that sort or filter on `oos_rank_frac_p50`

## Regression Risk And Feature-Preservation Constraints

The intended outcome here is better correctness and clearer contract ownership, not removal of CSCV/PBO functionality.

- do not remove skipped-result support
- do not narrow current caller flexibility unless a contract violation is proven
- if the normalization bug is fixed, treat it as a correctness change with downstream threshold/report verification, not as an incidental cleanup
- preserve current feature coverage for both double-MA and composite validation flows
