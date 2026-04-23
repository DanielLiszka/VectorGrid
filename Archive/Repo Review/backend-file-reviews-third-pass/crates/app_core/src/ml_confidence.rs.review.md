# Third-Pass Review: `crates/app_core/src/ml_confidence.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/ml_confidence.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/ml_confidence.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/ml_confidence.rs.review.md`
- Second-pass action bucket: `Small surgical refactor`
- Third-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Full Third Pass

The second-pass packet already called this as `Bucket unchanged`. The earlier targeted third pass stopped short of this file, so the expanded full-corpus pass reopened the live source to verify whether the ML confidence health and validation owner still looked like the right framing under fresh research. The third pass revisited whether the second-pass packet had been specific enough about validation quality and concluded that two behaviors needed sharper wording: group assignment is hash-based rather than explicitly time-aware, and health can still fall back to train-side metrics when CV produced no predictions.

## Third-Pass Research Log

1. scikit-learn GroupKFold docs
   - Source: <https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.GroupKFold.html>
   - Why consulted in third pass:
     - recheck what grouped validation is actually promising relative to the current group assignment
   - Third-pass constraint informed:
     - grouped CV should not be described as stronger than the grouping policy that feeds it

2. scikit-learn TimeSeriesSplit docs
   - Source: <https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html>
   - Why consulted in third pass:
     - compare the current grouped CV posture to explicitly time-aware validation guidance
   - Third-pass constraint informed:
     - time-sensitive model health should not silently fall back to weaker train-side evidence when out-of-sample predictions are absent

3. Rust API Guidelines dependability
   - Source: <https://rust-lang.github.io/api-guidelines/dependability.html>
   - Why consulted in third pass:
     - confirm that the stronger issue is still contract clarity, not a sweeping redesign
   - Third-pass constraint informed:
     - health outputs should stay honest about what evidence actually exists

## What Third Pass Sharpened

### 1. Grouped validation quality is weaker than the current health surface can imply
- Evidence:
  - Grouped CV assignment is driven by hashing group_id rather than an explicitly time-aware split.
- Why this matters:
  - That makes the grouping policy more heuristic than the label grouped validation can imply in a time-sensitive workflow.
- Practical implication:
  - Keep the existing bucket, but describe grouped-CV health as heuristic and add tests that pin fallback behavior.

### 2. Health can still look acceptable without real out-of-sample predictions
- Evidence:
  - The health decision can fall back to train-side metrics even when CV produced no predictions.
- Why this matters:
  - That is stronger than a generic validation fragility caveat because it can make missing out-of-sample evidence look like usable health evidence.
- Practical implication:
  - If hardened, make the first step an explicit evidence-state distinction rather than a wholesale modeling rewrite.

## Implementation Priority

- First hardening step: make health outputs differentiate real out-of-sample evidence from train-side fallback.
- Then evaluate whether group assignment needs a more explicit time-aware contract.

## Verification If Changed

- cargo test -p ta_app_core ml_confidence -- --nocapture
- representative ML-confidence tests where grouped CV produces no predictions or only weak group coverage

## Regression And Feature-Preservation Constraints

- Preserve current model-training entry points unless frontend and artifact consumers move together.
- Do not overstate grouped-CV strength relative to the actual grouping policy.
- Keep health reporting honest about fallback versus real out-of-sample evidence.

## Third-Pass Conclusion

`Bucket unchanged, but evidence quality sharpened`

ml_confidence.rs stays in Small surgical refactor. The third pass mainly sharpens two contract issues: grouped validation is weaker than it can sound because grouping is hash-based rather than explicitly time-aware, and the health surface can still rely on train-side evidence when cross-validation produced no predictions.
