# Monte Carlo Reporting And Export Hardening

Status: implemented for normalized validation exports
Risk: moderate
Primary value: make existing Monte Carlo/trade-bootstrap evidence more explicit, portable, and reportable

## Summary

Monte Carlo/trade-bootstrap support is already present in active validation code and UI. This task should therefore harden reporting, exports, compatibility, and evidence presentation rather than treating Monte Carlo as a from-zero backend feature.

Implemented slice:

- Added normalized Monte Carlo fields to validation export rows while preserving legacy bootstrap compatibility.
- Extended CSV and XLSX candidate outputs with canonical Monte Carlo mode/probability/percentile/sample fields.
- Added report tests for legacy-bootstrap fallback and canonical Monte Carlo-only artifacts.
- Preserved validation pass/fail behavior.

## Current Anchors

- Validation backend: `crates/app_core/src/double_ma_validate.rs`, `crates/app_core/src/composite_validate.rs`
- Validation request builder/service/UI: `web/src/services/validationRequestBuilder.ts`, `web/src/services/validationService.ts`, `web/src/components/ValidationWorkspacePanel.vue`
- Report exports: `src-tauri/src/commands_reports_validation_common.rs`, `src-tauri/src/commands_reports_validation_xlsx.rs`, `src-tauri/src/commands_reports_csv.rs`
- Existing plan: `docs/monte_carlo_validation_implementation_plan_2026-04-25.md`

## Additive Boundary

- Preserve `trade_bootstrap` request/result compatibility.
- Preserve advisory default behavior unless a hard gate is explicitly requested.
- Do not add randomized scoring inside optimization.
- Do not make CUDA mandatory.
- Keep Monte Carlo auxiliary CPU-tail reporting explicit when main validation route is GPU.

## Implementation Checklist

### Phase 1: Contract Audit

- [x] Inventory current `trade_bootstrap` and `monte_carlo` request/result fields.
- [x] Document legacy aliases and current canonical fields through export normalization.
- [x] Add tests for old result shapes if missing.
- [x] Ensure request builder preserves both legacy and canonical backend expectations.

### Phase 2: UI Evidence Improvements

- [x] Preserve existing compact validation-service Monte Carlo/bootstrap summaries.
- [x] Show probability of positive PnL, p05/p50 PnL, max drawdown percentiles, sample count, mode, and warnings in normalized exports.
- [x] Preserve advisory vs hard-gate policy behavior.
- [x] Preserve empty state behavior for candidates with too few trades.
- [x] Preserve route note when Monte Carlo runs as auxiliary CPU work after GPU validation.

### Phase 3: CSV/XLSX Export Hardening

- [x] Add normalized Monte Carlo columns while preserving legacy bootstrap columns.
- [x] Add workbook candidate fields for simulations/sample count and mode where data exists.
- [ ] Add distribution chart sheet only when binned data is available.
- [x] Add report tests for old and new field shapes.

### Phase 4: Options Follow-Up Scoping

- [x] Keep options Monte Carlo as a separate options-validation stress lane.
- [x] Do not reuse closed-trade conventional bootstrap blindly for options.
- [ ] Define options-specific stress variables before implementation.
- [x] No options Monte Carlo coding was done in this validation-reporting slice.

## Validation Checklist

- [x] `cargo test -p ta_desktop_demo_app --lib validation_shortlist_csv`
- [x] `cargo test -p ta_desktop_demo_app --lib rows_derive_legacy_bootstrap_metrics_from_monte_carlo_when_absent`
- [x] `cargo test -p ta_desktop_demo_app --lib validation_report_xlsx_uses_monte_carlo_when_legacy_bootstrap_is_absent`
- [x] `cargo test -p ta_desktop_demo_app --lib commands_reports_csv`
- [x] `npm --prefix web run test -- validationRequestBuilder validationService`
- [x] Run output parity / workflow gate before final handoff if report sample artifacts are regenerated.

## Regression Watchpoints

- Existing `trade_bootstrap` artifacts must remain readable.
- Candidate pass/fail logic must not change unless a hard-gate option is explicitly enabled.
- Monte Carlo seed/defaults must be deterministic in tests.
- Export columns must not break downstream consumers by removing legacy fields.

## Definition Of Done

- Monte Carlo evidence is visible, exported, and compatible with legacy bootstrap artifacts.
- Tests cover old and new shapes.
- GPU route summaries remain honest about auxiliary CPU work.
