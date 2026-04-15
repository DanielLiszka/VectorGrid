# Second-Pass Review: `crates/app_core/src/prescan_predictive.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/prescan_predictive.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/prescan_predictive.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this helper as a bounded but semantics-sensitive heuristic because it influences candidate coverage before the real composite search begins.

## Stale Check

The live source still owns:

- bounded period sampling
- rescue-seed pair selection
- a chronology-respecting but lightweight predictive score

The first-pass packet remains current.

## Second-Pass Research Log

1. scikit-learn `TimeSeriesSplit`
   - Source: <https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html>
   - Why consulted in second pass:
     - ground chronology and gap discussion for time-series heuristics
   - Additional constraint informed:
     - this helper still belongs on the heuristic side of the boundary, not the validation regime side

## Delta vs First Pass

### 1. The current predictive score should stay framed as rescue-seed heuristic only, not as quality evidence

- Why this matters:
  - it scores on the immediately next log return with no embargo or richer validation structure
- Practical implication:
  - preserve its bounded rescue role

### 2. Unsorted caller input changes more than cosmetic ordering

- Evidence:
  - `sample_periods_by_stride` anchors on first encountered value and preserves the last encountered value before sorting
- Why this matters:
  - unsorted input changes stride stepping and endpoint retention
- Practical implication:
  - caller ordering assumptions deserve stronger documentation or validation

## Verification If Changed

- `cargo test -p ta_app_core prescan_predictive::tests:: -- --nocapture`
- `cargo test -p ta_app_core composite_cpu_prescan_predictive_rescue_can_expand_candidates -- --nocapture`
- `cargo test -p ta_app_core composite_mo_gpu_prescan_supports_multiple_ma_pairs -- --nocapture`

## Regression And Feature-Preservation Constraints

- Preserve chronology-respecting use of next-step returns
- Preserve the bounded rescue role of this helper
- Do not widen caps or retune thresholds casually

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly clarifies that this helper should stay framed as a lightweight rescue heuristic and that unsorted caller input changes more than just output ordering.
