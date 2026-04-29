# Options Finalist Acceptance Dashboard

Status: proposed
Risk: moderate
Primary value: make options-expression validation decisions easier to compare and defend

## Summary

The current options expression workflow has strong optimization, feasibility, validation, provenance, and artifact surfaces. The remaining opportunity is a clearer finalist-comparison and acceptance dashboard inside the existing options host, not a new separate options product.

## Current Anchors

- Options panel/store/service: `web/src/components/OptionsExpressionPanel.vue`, `web/src/stores/optionsExpressionStore.ts`, `web/src/services/optionsExpressionService.ts`
- Options backend: `crates/app_core/src/options_expression.rs`, `crates/app_core/src/options_types.rs`
- Tauri commands: `src-tauri/src/commands_options_expression.rs`
- Parent adapters: `src-tauri/src/options_parent_adapters.rs`
- Roadmap: `docs/options_research/options_expression_future_scope_roadmap_2026-04-07.md`

## Additive Boundary

- Keep the current unified options host.
- Do not change ranking/scoring behavior in the first dashboard slice.
- Do not broaden supported option structures unless explicitly scoped.
- Keep Polygon.io as the supported provider and legacy provider aliases only where already canonicalized.

## Implementation Checklist

### Phase 1: Dashboard View Model

- [ ] Define finalist comparison rows from existing result/artifact fields.
- [ ] Include ranking metrics, holdout metrics, harsher-lane metrics, feasibility status, validation status, and provenance flags.
- [ ] Add normalization for missing legacy fields.
- [ ] Add unit tests for row construction.

### Phase 2: Acceptance Gates

- [ ] Define display-only gate rows for data coverage, feasibility, validation replay, holdout robustness, quote realism, and provenance.
- [ ] Keep gates advisory unless existing backend verdicts already provide hard blockers.
- [ ] Show disagreement across realism lanes where data exists.
- [ ] Avoid creating a hidden new pass/fail decision engine in the frontend.

### Phase 3: UI Integration

- [ ] Add dashboard section to the current options panel or History options follow-up panel.
- [ ] Add finalist compare table with stable sorting and column visibility.
- [ ] Add compact gate badges and evidence details.
- [ ] Add empty, partial, and legacy artifact states.
- [ ] Ensure large result sets use existing table/virtualization helpers where appropriate.

### Phase 4: Exports And Artifacts

- [ ] Add optional report/export fields only after UI rows are stable.
- [ ] Preserve existing options artifact schema compatibility.
- [ ] Include dashboard evidence in exported summaries without changing backend ranking.
- [ ] Add report tests if XLSX/CSV output changes.

## Validation Checklist

- [ ] `npm --prefix web run test -- src/services/optionsExpressionService.test.ts src/stores/optionsExpressionStore.test.ts src/components/OptionsExpressionPanel.test.ts`
- [ ] Add focused tests for dashboard row/gate view model.
- [ ] Run options frontend QA if changed.
- [ ] If backend artifact fields are added, run targeted Rust tests for options expression commands/core.
- [ ] For CUDA/options planner changes, run the options GPU promotion gate.

## Regression Watchpoints

- The panel is already large; avoid adding fragile inline logic directly into the component where a service/view-model helper is better.
- Saved artifact reopen and validation feasibility preview must keep working.
- Dashboard copy must not imply unsupported margin, quote-provider, or portfolio features.
- First slice must not change result ordering.

## Definition Of Done

- Users can compare retained finalists and understand acceptance evidence in one place.
- The dashboard is explainable from existing fields.
- Tests prove legacy artifacts and partial evidence render safely.

