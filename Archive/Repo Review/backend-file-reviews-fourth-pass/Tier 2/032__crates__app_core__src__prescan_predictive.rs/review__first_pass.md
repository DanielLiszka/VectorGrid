# Review Packet: `crates/app_core/src/prescan_predictive.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/prescan_predictive.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Internally useful and locally well-bounded, but not fully review-ready because several degenerate inputs collapse into silent empty output, the seed-selection contract is weakly typed for such an influential heuristic, and the predictive scoring semantics are not documented where callers can reason about them`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Internal-surface`, `Composite-strategy-local`, `Prescan-hot-path`, `CPU-only`, `Semantics-sensitive`, `Throughput-sensitive`

## What This File Owns

`prescan_predictive.rs` owns the predictive rescue heuristic used by composite prescan flows to seed MA `(fast, slow)` candidates from a coarse directional-edge estimate.

It owns:

- bounded period sampling helpers:
  - `sample_periods_by_stride`: `crates/app_core/src/prescan_predictive.rs:10-29`
  - `cap_periods_even`: `31-56`
- the prescan request contract:
  - `PredictivePrescanParams`: `58-74`
- the main predictive rescue entry point:
  - `predictive_prescan_seed_pairs`: `76-184`
- file-local tests:
  - `200-312`

It does not own:

- MA matrix computation or log-return primitives:
  - `crates/app_core/src/double_ma.rs`
- composite prescan orchestration:
  - `crates/app_core/src/composite_cpu.rs`
  - `crates/app_core/src/composite_mo.rs`
- CUDA or runtime-device behavior:
  - none in this file

## Classification

- Public surface: no
  - both `PredictivePrescanParams` and `predictive_prescan_seed_pairs` are `pub(crate)` at `58` and `76`
- Hot path: yes, bounded
  - it runs on optimization setup and directly changes which candidate pairs the larger search explores
- CUDA/runtime-critical: no
  - CPU-only helper
- Strategy-local: yes
  - used specifically by composite prescan rescue paths
- Low-risk support code: no
  - the file is small, but it changes search coverage and therefore optimization behavior

## Direct Interactions

### Backend interaction map

- primitive dependencies:
  - `compute_log_returns_f64` and `ma_matrix_cpu_f64` imported from `crates/app_core/src/double_ma.rs`: `1`
- Composite CPU rescue path:
  - import: `crates/app_core/src/composite_cpu.rs:125`
  - integration: `11221-11239`
  - representative regression coverage: `14540-14597`
- Composite MO rescue path:
  - import: `crates/app_core/src/composite_mo.rs:11`
  - integration: `1406-1423`
  - representative regression coverage: `4203-4286`

### Downstream verification impact if changed

- changing sampling behavior changes which MA pairs are even eligible for later optimization
- changing scoring math or positivity thresholds changes rescue coverage before the real search begins
- changing input handling affects both Composite CPU prescan rescue and Composite MO GPU-prescan rescue flows

## Standards And Research Log

This packet was written after targeted web research focused on input-validation posture, chronological time-series evaluation discipline, and the standard-library behavior behind the file’s current sort-and-dedup normalization strategy.

### Sources consulted

1. Rust API Guidelines dependability guidance, `C-VALIDATE`
   - Source: <https://rust-lang.github.io/api-guidelines/dependability.html#functions-validate-their-arguments-c-validate>
   - Why it was consulted:
     - the entry point currently treats multiple degenerate inputs as `Ok(Vec::new())`
   - Standard informed:
     - Rust code should validate inputs deliberately rather than accept malformed inputs opaquely

2. Rust API Guidelines documentation guidance, `C-FAILURE`
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html#function-docs-include-error-panic-and-safety-considerations-c-failure>
   - Why it was consulted:
     - this helper’s “empty versus error” behavior is subtle and important for callers
   - Standard informed:
     - behaviorally important fallible or quasi-fallible semantics should be documented explicitly

3. Rust `slice::sort_unstable`
   - Source: <https://doc.rust-lang.org/std/primitive.slice.html#method.sort_unstable>
   - Why it was consulted:
     - the helper normalizes output order by sorting after period selection
   - Standard informed:
     - sort-based normalization is cheap and appropriate for primitive period sets

4. Rust `Vec::dedup`
   - Source: <https://doc.rust-lang.org/std/vec/struct.Vec.html#method.dedup>
   - Why it was consulted:
     - the helper relies on sorted-then-deduped period lists
   - Standard informed:
     - dedup only removes consecutive duplicates, so ordering assumptions should be made explicit

5. scikit-learn `TimeSeriesSplit`
   - Source: <https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html>
   - Why it was consulted:
     - the heuristic uses next-step returns and therefore must stay chronology-respecting
   - Standard informed:
     - time-series evaluation logic should preserve temporal ordering and avoid hidden leakage assumptions

## Findings

## Correctness And Safety

### 1. Multiple degenerate inputs silently collapse to `Ok(Vec::new())`

- Evidence:
  - `top_k == 0` returns empty:
    - `crates/app_core/src/prescan_predictive.rs:79-81`
  - empty fast/slow period domains return empty:
    - `82-84`
  - empty sampled domains return empty:
    - `94-96`
  - mismatched source lengths or too-short data return empty:
    - `99-107`
  - only MA-matrix column mismatch becomes an actual error:
    - `129-130`
- Why this matters:
  - the helper uses `Result`, but most invalid or non-usable states are encoded as success with no seeds
  - that weakens diagnostics for callers and makes some malformed inputs operationally invisible
- Smallest justified fix:
  - keep intentional “no rescue candidates” outcomes as empty
  - distinguish malformed input from legitimate empty output with either early validation errors or explicit tracing/comments for each silent-empty branch

### 2. Period sampling is order-dependent before local normalization

- Evidence:
  - `sample_periods_by_stride` anchors on `periods[0]` and advances `next_keep` from encounter order:
    - `10-29`
  - sort/dedup happen only after sampling:
    - `24-28`
- Why this matters:
  - if an unsorted period slice ever reaches this helper, the sampled set depends on caller ordering, not just period values
  - that makes seed selection less deterministic than it appears
- Smallest justified fix:
  - locally sort/dedup a copied period slice before stride selection, or explicitly assert/document that callers must pass sorted domains

## API And Naming

### 3. The request contract is weakly typed for an influential selector

- Evidence:
  - `ma_source`, `fast_ma_type`, and `slow_ma_type` are raw `&str`:
    - `58-74`
  - the output is an anonymous `Vec<(u16, u16)>`:
    - `76-184`
- Why this matters:
  - the function is lightweight, but it still influences upstream search coverage materially
  - weak typing makes the contract less self-describing and pushes correctness onto caller discipline
- Smallest justified fix:
  - if full type migration is too broad, document accepted string domains and consider a small local `PredictivePrescanPair` alias or struct for readability

## Error Handling

### 4. The empty-versus-error split is under-specified for callers

- Evidence:
  - several “disabled or unusable” states return empty successfully:
    - `79-107`
  - MA-matrix construction failures propagate as `Err(String)`:
    - `111-128`
  - column mismatch also propagates as error:
    - `129-130`
- Why this matters:
  - callers cannot tell from the signature which failure classes should be treated as benign absence versus genuine prescan faults
  - this is especially relevant because both composite callers use the result to widen or rescue candidate sets
- Smallest justified fix:
  - add inline contract comments and, if warranted, a small enum or telemetry note describing why an empty set was returned

## Module Boundaries And Technical Debt

### 5. The ranking heuristic is semantics-sensitive and under-documented

- Evidence:
  - the score is based on mean `sign(fast - slow) * next_log_return`:
    - `141-175`
  - `long_only` zeroes short exposure:
    - `156-159`
  - only observations above `MIN_OBS` are kept:
    - `171`
  - only strictly positive scores survive:
    - `172-175`
- Why this matters:
  - this is not generic support code; it is selection policy
  - small changes here alter rescue behavior and upstream candidate composition
- Smallest justified fix:
  - document the heuristic contract and defer any scoring retune until backed by downstream quality checks

## Docs And Examples

### 6. The heuristic intent is not explained where maintainers need it

- Evidence:
  - the file has no module-level explanation and no comments describing why this scoring rule is used
  - only the call-site field names such as `predictive_top_k` and `predictive_stride` hint at intent:
    - `crates/app_core/src/composite_mo.rs:112-118`
    - `crates/app_core/src/composite_cpu.rs:11116-11117`
- Why this matters:
  - a future maintainer can read the loop and constants, but not the product rationale for this rescue path
  - that raises the risk of accidental semantic drift during cleanup
- Smallest justified fix:
  - add a short file-level note or function comment describing the heuristic and its non-goal of being an exact optimizer

## Tests And Missing Verification

### 7. Test coverage is useful but narrow relative to the contract surface

- Evidence:
  - current tests cover:
    - stride helper behavior: `229-234`
    - cap helper behavior: `236-240`
    - trending long-only happy path: `242-268`
    - disabled/empty inputs: `270-311`
- Missing verification:
  - unsorted period inputs
  - `long_only = false`
  - alternate `ma_source` behavior
  - short or noisy data where scores are non-positive
  - whether silent-empty branches are intentionally distinct from error branches
- Smallest justified fix:
  - add narrow regression tests for unsorted periods and a non-long-only path before changing semantics

## Performance, Allocation, And CUDA Concerns

### 8. The helper is bounded, but any expansion is still throughput-sensitive

- Evidence:
  - sampled periods are capped:
    - `MAX_SAMPLED_PERIODS = 48` at `5`
  - time steps are subsampled:
    - `MAX_TIME_STEPS = 20_000` at `6`
    - `time_stride` computed at `108-109`
  - full CPU MA matrices are still computed for sampled fast and slow domains:
    - `111-128`
- Why this matters:
  - the file is not huge, but it does real work on the optimization path
  - raising caps or adding richer scoring logic would directly change prescan cost
- Safe-now conclusion:
  - no measured performance issue demands immediate change
- Benchmark-sensitive conclusion:
  - do not widen period caps, time-step density, or scoring passes without timing evidence

## Code-Quality Improvement Register

- Safe now:
  - normalize period ordering before stride sampling
  - document the heuristic and its empty-versus-error semantics
  - introduce a named output alias or tiny struct if readability matters
- Defer:
  - any scoring-formula redesign
  - any expansion of prescan search breadth

## Technical-Debt Reduction Register

- Safe now:
  - make silent-empty branches explicit and intentional
  - add a couple of regression tests around ordering and `long_only = false`
- Defer:
  - broad typing changes if they require touching many composite request paths at once
- Do not casually remove:
  - the current caps and subsampling guards

## Performance Opportunity Register

- Safe structural work:
  - deterministic local sorting of period domains before sampling
- Benchmark-sensitive:
  - changing `MAX_SAMPLED_PERIODS`
  - changing `MAX_TIME_STEPS`
  - changing `MIN_OBS`
  - changing the positive-score admission rule

## Recommended Action

`Small surgical refactor`

This file does not need a redesign. It needs:

- clearer input-contract handling
- explicit documentation of heuristic semantics
- deterministic local normalization of period domains

The selection math should be treated as semantics-sensitive and verified downstream before any behavioral edits.

## Exact Local Verification Required If Changed

1. File-local heuristic tests:
   - `cargo test -p ta_app_core prescan_predictive::tests:: -- --nocapture`
2. Composite CPU rescue integration:
   - `cargo test -p ta_app_core composite_cpu_prescan_predictive_rescue_can_expand_candidates -- --nocapture`
3. Composite MO rescue integration:
   - `cargo test -p ta_app_core composite_mo_gpu_prescan_supports_multiple_ma_pairs -- --nocapture`
4. If caps, sampling density, or scoring thresholds change:
   - run the relevant CPU perf gate or targeted composite optimization timing workflow before accepting the change

### Verification run for this packet

- Passed:
  - `cargo test -p ta_app_core prescan_predictive::tests:: -- --nocapture`
  - `cargo test -p ta_app_core composite_cpu_prescan_predictive_rescue_can_expand_candidates -- --nocapture`
  - `cargo test -p ta_app_core composite_mo_gpu_prescan_supports_multiple_ma_pairs -- --nocapture`

## Regression Risk And Feature-Preservation Constraints

- Do not change the heuristic from “rescue seed generator” into a heavyweight optimizer
- Do not widen caps or tighten thresholds casually; both change prescan breadth and runtime cost
- Preserve chronology-respecting use of next-step returns
- Preserve the ability of Composite CPU and Composite MO flows to use this helper as an additive rescue path rather than a mandatory filter
