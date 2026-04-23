# Second-Pass Review: `crates/app_core/src/ml_confidence.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/ml_confidence.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/ml_confidence.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as mostly sound but contract-sensitive because training-status semantics, grouped validation, and trust indicators can diverge from what the public status surface appears to promise.

## Stale Check

The live source still:

- groups CV behavior partly by artifact timestamps
- exposes trust and health-oriented status fields
- counts artifact usage and dataset-row participation separately

The first-pass packet remains current.

## Second-Pass Research Log

1. scikit-learn calibration guide
   - Source: <https://scikit-learn.org/stable/modules/calibration.html>
   - Why consulted in second pass:
     - ground the review of calibration-health expectations
   - Additional constraint informed:
     - persisted status semantics still need stronger documentation rather than implication

2. `roc_auc_score`
   - Source: <https://scikit-learn.org/stable/modules/generated/sklearn.metrics.roc_auc_score.html>
   - Why consulted in second pass:
     - verify the metric meaning behind surfaced trust/quality fields
   - Additional constraint informed:
     - AUC semantics remain specific enough that fallback interpretation should stay explicit

3. `brier_score_loss`
   - Source: <https://scikit-learn.org/stable/modules/generated/sklearn.metrics.brier_score_loss.html>
   - Why consulted in second pass:
     - verify calibration/error semantics behind status reporting
   - Additional constraint informed:
     - health indicators should not silently inherit stronger claims than the metrics warrant

## Delta vs First Pass

### 1. Missing artifact timestamps can still collapse CV grouping keys and distort grouped validation

- Why this matters:
  - grouped CV semantics become weaker in exactly the cases where provenance is already thin
- Practical implication:
  - still a good bounded hardening target if this file is touched

### 2. `meta_artifacts_used` still appears to count artifacts earlier than true row contribution

- Why this matters:
  - usage counts can overstate actual training participation
- Practical implication:
  - status documentation should stay careful even if the current behavior is preserved

### 3. `healthy` still falls back to train metrics when CV metrics are absent

- Practical implication:
  - preserve behavior for compatibility, but keep its semantics explicit rather than implied

## Verification If Changed

- `cargo test -p ta_app_core ml_confidence::tests:: -- --nocapture`
- `cargo test -p ta_desktop_demo_app build_ml_confidence_status_prefers_dataset_rows_and_keeps_scan_error -- --nocapture`

## Regression And Feature-Preservation Constraints

- Preserve current training, prediction, and calibration behavior
- Preserve persisted-model compatibility unless the trust/status contract is deliberately versioned
- Do not silently strengthen the meaning of `healthy` or artifact-usage counters

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly clarifies that grouped validation and status trust indicators are still more fragile than the public surface suggests.
