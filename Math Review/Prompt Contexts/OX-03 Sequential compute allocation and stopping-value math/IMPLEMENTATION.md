# OX-03 Implementation Guide

Source response: `ox03_sequential_compute_review_passes.md`

Decision: `Valid`, but only for guarded fast paths, honest accounting, and descriptive telemetry.

## Valid Scope
- [ ] Add a guarded sequential fast path only in `validation_pipeline.rs`, after separating candidate-local monotone fold logic from set-level CSCV/PBO semantics.
- [ ] Keep the full-set path whenever SPA, PBO, or CSCV-wide gates matter.
- [ ] Add honest end-to-end compute accounting for Pareto enrichment in `composite_mo.rs`.
- [ ] Add trim-ratio and provenance telemetry in `commands_campaign.rs` without changing default selection semantics.

## Non-Goals
- [ ] Do not add one global stopping rule across optimizer, validation, meta, and campaign stages.
- [ ] Do not add SPA-based pruning without admissible witness bounds.
- [ ] Do not prune mid-CSCV while PBO or CSCV-wide gates still depend on the full retained set.
- [ ] Do not present heuristic admission-policy changes as exact.

## Additive Guardrails
- [ ] Default behavior must remain the current full path unless guarded eligibility is satisfied.
- [ ] Any fast path must be bypassed automatically when global set-level obligations are enabled.
- [ ] Telemetry and provenance additions must not change selection semantics by default.

## Primary Touchpoints
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/composite_mo.rs`
- `src-tauri/src/commands_campaign.rs`
- `crates/app_core/src/meta_select.rs`
- `crates/app_core/src/double_ma.rs`

## Implementation Checklist
- [ ] In `validation_pipeline.rs`, separate candidate-local monotone fold logic from CSCV/PBO-wide obligations.
- [ ] Add a guarded fast path only for runs where that separation preserves current semantics.
- [ ] Keep a full fallback path for any run that requires full-set CSCV, PBO, or SPA-sensitive reporting.
- [ ] Add explicit compute-accounting fields for Pareto enrichment stages in `composite_mo.rs`.
- [ ] Add campaign-level trim ratio, cap/provenance, and selection-bias telemetry without changing current selection behavior.
- [ ] Keep all heuristic downsampling or frontier subsampling clearly labeled.

## Verification Checklist
- [ ] Add equivalence tests between guarded fast path and full path on eligible runs.
- [ ] Add fallback tests when global CSCV/PBO/SPA obligations are enabled.
- [ ] Add telemetry tests for Pareto enrichment accounting.
- [ ] Add campaign provenance tests for candidate-cap trimming.
- [ ] Add no-change regression coverage for default campaign selection behavior.

## Rollout Checklist
- [ ] Enable the fast path only when eligibility conditions are explicit and test-backed.
- [ ] Roll out telemetry before any default-path changes.
- [ ] Keep any frontier trimming described as heuristic unless separately proven exact.

## Kill Criteria
- [ ] Reject if the fast path can run while full-set CSCV/PBO/SPA semantics still matter.
- [ ] Reject if telemetry changes are allowed to silently alter default selection behavior.

