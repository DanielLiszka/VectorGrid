# Second-Pass Review: `crates/app_core/src/request_normalization.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/request_normalization.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/request_normalization.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Document only`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass treated this file as tiny but slightly opaque normalization glue and suggested a small cleanup around sentinel semantics.

## Stale Check

The live source still does exactly what the first pass described:

- collapses empty/whitespace paths to `None`
- normalizes `top_k` / `include_all` to simple sentinels
- uses `0` bins as a disabled sentinel

The second pass found less structural risk here than the first pass assumed.

## Second-Pass Research Log

1. Rust API Guidelines `C-VALIDATE`
   - Source: <https://rust-lang.github.io/api-guidelines/dependability.html#functions-validate-their-arguments-c-validate>
   - Why consulted in second pass:
     - reassess whether these helpers should become richer typed wrappers
   - Additional constraint informed:
     - narrow normalization helpers like these can stay small if their sentinels are well documented

2. Rust API Guidelines documentation guidance
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html#function-docs-include-error-panic-and-safety-considerations-c-failure>
   - Why consulted in second pass:
     - ground the sentinel-semantics review
   - Additional constraint informed:
     - the main gap is documentation clarity, not code shape

3. Serde default fields
   - Source: <https://serde.rs/attr-default.html>
   - Why consulted in second pass:
     - compare these sentinels to the wider request-default posture used elsewhere
   - Additional constraint informed:
     - the current normalization choices still fit the repo’s omission/default conventions

## Delta vs First Pass

### 1. The real gap here is documentation, not implementation complexity

- Why this matters:
  - surrounding callers already normalize and interpret these sentinels consistently enough
- Practical implication:
  - lower the bucket to `Document only`

### 2. `normalize_optional_path` remains usefully lossy

- Why this matters:
  - empty and whitespace-only export paths are not meaningful states for current callers

## Verification If Changed

- `cargo test -p ta_app_core request_normalization::tests:: -- --nocapture`
- representative `double_ma` and `composite_cpu` request-path tests

## Regression And Feature-Preservation Constraints

- Keep `None`/empty-path collapse
- Keep `(top_k.unwrap_or(0), include_all.unwrap_or(false))`
- Keep `0` heatmap bins as the disabled sentinel

## Second-Pass Conclusion

`First-pass bucket changed`

The second pass moves this file from `Small surgical refactor` to `Document only`. The helpers are narrow and cohesive enough that richer wrappers would be churn; the real need is to state the sentinel semantics explicitly.
