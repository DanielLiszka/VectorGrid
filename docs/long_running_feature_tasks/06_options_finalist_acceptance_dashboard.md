# Options Finalist Acceptance Dashboard

Status: implemented for in-panel finalist dashboard
Risk: moderate
Primary value: make options-expression validation decisions easier to compare and defend

## Summary

The current options expression workflow has strong optimization, feasibility, validation, provenance, and artifact surfaces. The remaining opportunity is a clearer finalist-comparison and acceptance dashboard inside the existing options host, not a new separate options product.

Implemented slice:

- Added a display-only finalist dashboard view model derived from existing options validation artifact fields.
- Surfaced finalist comparison rows and compact gate badges inside `OptionsExpressionPanel`.
- Added normalization tests for missing/null metrics and partial legacy evidence.
- Preserved optimizer ranking, backend validation behavior, and artifact schema compatibility.

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

- [x] Define finalist comparison rows from existing result/artifact fields.
- [x] Include ranking metrics, holdout metrics, harsher-lane metrics, feasibility status, validation status, and provenance flags.
- [x] Add normalization for missing legacy fields.
- [x] Add unit tests for row construction.

### Phase 2: Acceptance Gates

- [x] Define display-only gate rows for data coverage, feasibility, validation replay, holdout robustness, quote realism, and provenance.
- [x] Keep gates advisory unless existing backend verdicts already provide hard blockers.
- [x] Show disagreement across realism lanes where data exists.
- [x] Avoid creating a hidden new pass/fail decision engine in the frontend.

### Phase 3: UI Integration

- [x] Add dashboard section to the current options panel or History options follow-up panel.
- [x] Add finalist compare table with stable sorting and column visibility.
- [x] Add compact gate badges and evidence details.
- [x] Add empty, partial, and legacy artifact states.
- [x] Ensure large result sets remain compact and readable.

### Phase 4: Exports And Artifacts

- [ ] Add optional report/export fields only after UI rows are stable.
- [x] Preserve existing options artifact schema compatibility.
- [x] Keep dashboard evidence UI-derived without changing backend ranking.
- [x] No XLSX/CSV output changes were made in this slice.

## Validation Checklist

- [x] `npm --prefix web run test -- optionsFinalistDashboardModel optionsExpressionService optionsExpressionStore OptionsExpressionPanel`
- [x] Add focused tests for dashboard row/gate view model.
- [x] Run options frontend component tests for changed panel layout.
- [x] No backend artifact fields were added.
- [x] No CUDA/options planner changes were made.

## Regression Watchpoints

- The panel is already large; avoid adding fragile inline logic directly into the component where a service/view-model helper is better.
- Saved artifact reopen and validation feasibility preview must keep working.
- Dashboard copy must not imply unsupported margin, quote-provider, or portfolio features.
- First slice must not change result ordering.

## Definition Of Done

- Users can compare retained finalists and understand acceptance evidence in one place.
- The dashboard is explainable from existing fields.
- Tests prove legacy artifacts and partial evidence render safely.
