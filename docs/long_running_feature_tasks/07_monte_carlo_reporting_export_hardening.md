# Monte Carlo Reporting And Export Hardening

Status: proposed
Risk: moderate
Primary value: make existing Monte Carlo/trade-bootstrap evidence more explicit, portable, and reportable

## Summary

Monte Carlo/trade-bootstrap support is already present in active validation code and UI. This task should therefore harden reporting, exports, compatibility, and evidence presentation rather than treating Monte Carlo as a from-zero backend feature.

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

- [ ] Inventory current `trade_bootstrap` and `monte_carlo` request/result fields.
- [ ] Document legacy aliases and current canonical fields.
- [ ] Add tests for old result shapes if missing.
- [ ] Ensure request builder preserves both legacy and canonical backend expectations.

### Phase 2: UI Evidence Improvements

- [ ] Add compact distribution summary for top candidates where histogram/bin data exists.
- [ ] Show probability of positive PnL, p05/p50/p95 PnL, max drawdown percentiles, sample count, mode, and warnings.
- [ ] Show advisory vs hard-gate policy clearly.
- [ ] Add empty state for candidates with too few trades.
- [ ] Add route note when Monte Carlo runs as auxiliary CPU work after GPU validation.

### Phase 3: CSV/XLSX Export Hardening

- [ ] Add normalized Monte Carlo columns while preserving legacy bootstrap columns.
- [ ] Add workbook summary rows for simulations, mode, seed, and policy.
- [ ] Add distribution chart sheet only when binned data is available.
- [ ] Add report tests for old and new field shapes.

### Phase 4: Options Follow-Up Scoping

- [ ] Keep options Monte Carlo as a separate options-validation stress lane.
- [ ] Do not reuse closed-trade conventional bootstrap blindly for options.
- [ ] Define options-specific stress variables before implementation.
- [ ] Add a separate plan update before coding options Monte Carlo.

## Validation Checklist

- [ ] `cargo test -p ta_app_core double_ma_validate`
- [ ] `cargo test -p ta_app_core composite_validate`
- [ ] `cargo test -p ta_desktop_demo_app validation_xlsx_report_tests`
- [ ] `npm --prefix web run test -- src/services/validationRequestBuilder.test.ts src/services/validationService.test.ts src/components/ValidationWorkspacePanel.test.ts`
- [ ] Run workflow gate if report fields affect artifact/report outputs.

## Regression Watchpoints

- Existing `trade_bootstrap` artifacts must remain readable.
- Candidate pass/fail logic must not change unless a hard-gate option is explicitly enabled.
- Monte Carlo seed/defaults must be deterministic in tests.
- Export columns must not break downstream consumers by removing legacy fields.

## Definition Of Done

- Monte Carlo evidence is visible, exported, and compatible with legacy bootstrap artifacts.
- Tests cover old and new shapes.
- GPU route summaries remain honest about auxiliary CPU work.

