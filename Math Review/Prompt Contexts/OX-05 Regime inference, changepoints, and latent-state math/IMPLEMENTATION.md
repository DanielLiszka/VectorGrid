# OX-05 Implementation Guide

Source response: `ox05_regime_changepoint_answers.md`

Decision: `Valid`, but only for lightweight finalist-side and train-stage regime awareness.

## Valid Scope
- [ ] Add a finalist-only contiguous-segment or changepoint-aligned robustness summary in `double_ma_validate.rs`.
- [ ] Use the existing finalist-only aux-analytics pattern and cached `CandidateStepReturnBundle`.
- [ ] Add a clustered-group disagreement evidence flag in `meta_select.rs` using existing `train_rows`, `representatives`, and `evidence_flags`.

## Non-Goals
- [ ] Do not add heavyweight HMM or latent-state systems.
- [ ] Do not push regime logic into search-time evaluation or GPU kernels.
- [ ] Do not make `composite_validate.rs` own new latent-state machinery.
- [ ] Do not make changepoint-aware `MlSignalSplitPlan` snapping or per-state threshold policies the first patch.

## Additive Guardrails
- [ ] Finalist-only analytics must remain finalist-only.
- [ ] Existing bucketed regime summaries must stay available for compatibility.
- [ ] Clustered grouping must not change default selection semantics unless disagreement is only surfaced as evidence first.

## Primary Touchpoints
- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/meta_select.rs`
- `crates/app_core/src/ml_signal.rs`
- `crates/app_core/src/composite_validate.rs`

## Implementation Checklist
- [ ] In `double_ma_validate.rs`, add a contiguous-segment robustness summary beside existing bucketed regime/checkpoint summaries.
- [ ] Build that summary from cached finalist step-return bundles and current finalist-only aux-analytics plumbing.
- [ ] Surface metrics the current fixed halves/quartiles can miss, such as worst aligned segment and segment-spread diagnostics.
- [ ] In `meta_select.rs`, add a lightweight disagreement/conflict evidence flag for quantized clusters whose train-row behavior is strongly opposed across datasets/slices.
- [ ] Keep the first patch descriptive; do not let the conflict flag silently rewrite grouping/ranking by default.
- [ ] If `composite_validate.rs` is touched at all, keep it to lightweight CPU-side finalist summaries only.

## Verification Checklist
- [ ] Add synthetic piecewise-return tests where fixed halves/quartiles miss a short severe segment.
- [ ] Add finalists-only coverage tests proving population limits stay intact.
- [ ] Add clustered-parameter fixtures that surface a conflict flag without regressing representative selection.
- [ ] Add serialization/backward-compat coverage for any new finalist summaries or evidence flags.

## Rollout Checklist
- [ ] Ship aligned segment summaries as additive diagnostics first.
- [ ] Keep cluster conflict evidence informational before any behavior change.
- [ ] Defer ML split snapping or state-conditioned thresholding until a narrower follow-on patch.

## Kill Criteria
- [ ] Reject if the patch widens into search-time latent-state inference.
- [ ] Reject if clustered grouping behavior changes by default instead of surfacing evidence first.

