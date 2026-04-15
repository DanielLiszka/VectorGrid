# Second-Pass Review: `crates/app_core/src/kvdf.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/kvdf.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/kvdf.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this strategy file as mostly sound but permissive in output selection and fail-open input handling.

## Stale Check

The live source still:

- emits route diagnostics
- normalizes VWAP anchor input permissively
- resolves output IDs and malformed desired-position inputs permissively

The first-pass packet remains current.

## Second-Pass Research Log

1. VectorTA Keltner docs
   - Source: <https://vectoralpha.dev/projects/ta/indicators/keltner/>
   - Why consulted in second pass:
     - verify multi-output semantics and signal expectations
   - Additional constraint informed:
     - output-selection permissiveness remains a real contract concern

2. VectorTA VWAP docs
   - Source: <https://vectoralpha.dev/projects/ta/indicators/vwap/>
   - Why consulted in second pass:
     - confirm anchor semantics behind current normalization
   - Additional constraint informed:
     - anchor normalization still accepts a broader shape than the effective contract suggests

3. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why consulted in second pass:
     - ground public helper and fail-open API review
   - Additional constraint informed:
     - desired-position degradation to flat output remains the sharpest silent-failure path

## Delta vs First Pass

### 1. Missing route diagnostics turned out to be less severe than the first pass implied because CUDA fallback reporting already exists

- Why this matters:
  - the sharper remaining issues are elsewhere

### 2. `resolve_output_id()` still fails open and can mask indicator-catalog drift

- Practical implication:
  - keep this on the same small-refactor list as anchor and desired-position permissiveness

## Verification If Changed

- `cargo test -p ta_app_core kvdf::tests:: -- --nocapture`
- `cargo test -p ta_app_core kvdf_optimization_smoke_runs -- --nocapture`

## Regression And Feature-Preservation Constraints

- Preserve current route-diagnostic emission
- Preserve current KVDF signal semantics
- Preserve current CPU/CUDA route availability

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly narrows the focus: route diagnostics are less concerning than the still-permissive output selection, anchor normalization, and fail-open desired-position behavior.
