# Second-Pass Review: `crates/app_core/src/composite_gpu_metric_decode.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/composite_gpu_metric_decode.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/composite_gpu_metric_decode.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this decoder as small but contract-sensitive because it owns the fixed GPU metric wire layout that downstream consumers silently trust.

## Stale Check

The live source still decodes:

- a fixed prefix by direct indexing
- a tolerant tail via `get(...).unwrap_or(0.0)`

The first-pass packet remains current.

## Second-Pass Research Log

1. Rust reference on cast expressions
   - Source: <https://doc.rust-lang.org/beta/reference/expressions/operator-expr.html>
   - Why consulted in second pass:
     - verify exact float-to-int cast behavior for decoded integer fields
   - Additional constraint informed:
     - casts truncate toward zero and saturate on overflow/`NaN`; they do not fail loudly

2. Rust slice docs
   - Source: <https://doc.rust-lang.org/std/primitive.slice.html>
   - Why consulted in second pass:
     - compare direct indexing vs tolerant `get` behavior
   - Additional constraint informed:
     - the current compatibility contract is internally split between panic-on-short-prefix and defaulted tail decoding

## Delta vs First Pass

### 1. The short-row compatibility story is still inconsistent across the row layout

- Why this matters:
  - the fixed prefix panics while the tail quietly defaults
- Practical implication:
  - if decode hardening is added, apply it consistently across the layout

## Verification If Changed

- add decode tests for short, 12-slot, and full 19-slot rows
- include `NaN` and overflow cases for integer-cast fields

## Regression And Feature-Preservation Constraints

- Preserve the current 19-slot GPU metric wire layout
- Preserve zero-default decoding for legacy tail omission unless the producer and consumer are versioned together

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly sharpens that the current decoder mixes panic-on-short-prefix and tolerant tail-default behavior within the same wire contract.
