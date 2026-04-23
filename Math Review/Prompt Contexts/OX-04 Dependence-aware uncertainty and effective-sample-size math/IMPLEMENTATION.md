# OX-04 Implementation Guide

Source response: `ox04_all_passes.md`

Decision: `Valid`, but only for ESS-aware seams, overlap-aware warnings, and descriptive count adjustments.

## Valid Scope
- [ ] Add ESS-aware plumbing as wrappers/seams in `validation_stats.rs` and `sharpe_stats.rs`, not new headline Sharpe formulas.
- [ ] In `validation_pipeline.rs`, preserve raw `track_record_bars` for reporting while computing PSR/DSR from effective counts.
- [ ] Store a raw-bar min-track-record threshold inflated from effective observations.
- [ ] In `ml_signal.rs`, derive conservative `eff_obs` from overlap metadata and add overlap-adjusted warnings where current scopes are not effectively iid.
- [ ] Add overlap-adjusted descriptive counts for walk-forward stats instead of treating raw windows as independent evidence.

## Non-Goals
- [ ] Do not keep treating raw bars/windows/peer counts as iid evidence once ESS seams exist.
- [ ] Do not infer global correlation-clustered `eff_trials` centrally without path/parameter similarity inputs.
- [ ] Do not redesign the verdict interface just to carry ESS.
- [ ] Do not replace current meta ranking wholesale with dataset-collapsed scoring in one step.

## Additive Guardrails
- [ ] Raw reporting fields must remain intact.
- [ ] Existing outputs must reproduce current behavior when `eff_obs == raw_obs`.
- [ ] Any new warnings should be additive rather than silently changing gating unless explicitly wired.

## Primary Touchpoints
- `crates/app_core/src/validation_stats.rs`
- `crates/app_core/src/sharpe_stats.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/ml_signal.rs`
- `crates/app_core/src/meta_select.rs`

## Implementation Checklist
- [ ] Add effective-count overloads/wrappers to the probability/statistics helpers while keeping current raw-count wrappers for compatibility.
- [ ] In `validation_pipeline.rs`, feed PSR/DSR with `eff_obs`/`eff_trials` while preserving raw observation counts in reporting fields.
- [ ] Convert the iid-required minimum track record back into raw-bar units before storing/reporting it.
- [ ] In `ml_signal.rs`, derive conservative overlap-aware effective counts from existing metadata such as `horizon_bars` and split-window spans.
- [ ] Add overlap-adjusted warnings for non-iid walk-forward or ML-signal scopes.
- [ ] Keep meta-selection changes descriptive-first unless a later patch deliberately promotes ESS into ranking.

## Verification Checklist
- [ ] Add compatibility tests where `eff_obs == raw_obs` reproduces current outputs.
- [ ] Add regressions proving raw reporting fields remain unchanged while min-track-record thresholds tighten under overlap.
- [ ] Add ML overlap tests using `horizon_bars` and split-window metadata.
- [ ] Add warning-flag tests for overlap-adjusted walk-forward scopes.
- [ ] Add descriptive effective-window-count tests that do not alter current gating unless explicitly enabled.

## Rollout Checklist
- [ ] Start with internal seams and additive warnings.
- [ ] Keep existing verdict interfaces stable while the new ESS data paths are proven.
- [ ] Promote ESS-aware gating only after compatibility and overlap cases are well covered.

## Kill Criteria
- [ ] Reject if raw reporting fields are silently replaced by effective counts.
- [ ] Reject if ESS-aware seams are presented as full dependence correction without the required inputs.
