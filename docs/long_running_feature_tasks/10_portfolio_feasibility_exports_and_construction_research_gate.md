# Portfolio Feasibility Exports And Construction Research Gate

Status: phase 1/2 compact research export implemented; backend reports and construction research remain planned
Risk: moderate if it implies allocation; low for compact research export and readiness diagnostics
Primary value: let users preserve feasibility evidence while keeping portfolio construction blocked behind explicit gates

## Summary

The Portfolio Feasibility workspace is already read-only and intentionally does not emit allocations. The next additive feature is a compact export/report of the feasibility result and a construction-research readiness section that explains why portfolio construction is blocked or what evidence is still missing.

QuantConnect's Algorithm Framework separates portfolio construction, risk management, and execution. Portfolio construction produces target quantities or weights, risk management adjusts targets, and execution places orders. That separation supports this repo's current risk policy: feasibility can explain readiness, but it must not create weights, targets, trades, or broker orders.

Sources:

- QuantConnect portfolio construction concepts: https://www.quantconnect.com/docs/v2/writing-algorithms/algorithm-framework/portfolio-construction/key-concepts
- QuantConnect risk management concepts: https://www.quantconnect.com/docs/v2/writing-algorithms/algorithm-framework/risk-management/key-concepts

## Current Anchors

- Backend feasibility engine: `crates/app_core/src/portfolio_feasibility.rs`
- Tauri command: `src-tauri/src/commands_portfolio.rs`
- Frontend service/model/UI: `web/src/services/portfolioService.ts`, `web/src/utils/portfolioWorkspaceModel.ts`, `web/src/components/PortfolioFeasibilityWorkspacePanel.vue`
- Command risk policy: `src-tauri/command-risk-policy.json`
- Existing parent plan: `01_portfolio_feasibility_workspace.md`

## Existing Behavior

- `analyze_portfolio_feasibility` accepts selected artifact sources and evidence gates.
- The backend admits/rejects candidates and returns `next_required_step`.
- The UI shows status, warnings, eligible rows, rejected rows, and missing evidence.
- No export/report action exists.
- No allocation, covariance, target weight, or execution plan is produced.

## Additive Boundary

- Export compact evidence summaries only.
- Exclude raw source artifacts, raw replay matrices, run requests/responses, secrets, and live broker data.
- Include explicit `research_only: true`.
- No target weights, target quantities, allocation frontier, rebalance plan, orders, notional, or margin outputs.
- No backend command is needed for first-slice browser download/report text.

## Non-Goals

- No portfolio optimizer.
- No portfolio target/rebalance output.
- No broker integration.
- No durable portfolio artifact.
- No hidden artifact mutation during export.

## Implementation Checklist

### Phase 1: Compact Export Model

- [x] Add `buildPortfolioFeasibilityExportPayload` in `portfolioWorkspaceModel.ts`.
- [x] Include schema version, generated timestamp, selected source summaries, request gates, result summary, rows, missing evidence, warnings, and `next_required_step`.
- [x] Add `buildPortfolioFeasibilityCsv` with a `row_type` column for eligible, rejected, missing evidence, and metadata rows.
- [x] Add `buildPortfolioConstructionReadiness` with diagnostic-only gates.
- [x] Add tests proving raw artifacts and allocation-shaped keys are excluded.

### Phase 2: Workspace UI

- [x] Add `Export JSON` and `Export CSV` actions after an analysis exists.
- [x] Use browser Blob download first; do not add a Tauri save command in the first slice.
- [x] Add a construction-readiness section with statuses: blocked, needs replay matrix, ready for covariance diagnostics.
- [x] Keep UI copy research-oriented and avoid allocation language.
- [x] Add QA manifest entries for export actions.

### Phase 3: Optional Backend Report Gate

- [ ] Only if desktop-saved reports become necessary, add a narrow portfolio report module.
- [ ] Register new commands explicitly in command registry, parity, capabilities, and risk policy.
- [ ] Classify any backend report command as read-only/report-only.
- [ ] Keep source artifacts immutable.

### Phase 4: Future Construction Research Gate

- [ ] Define `PortfolioReplayMatrixV1` with bounded dimensions before covariance work.
- [ ] Define risk model, rebalance cadence, margin assumptions, and execution isolation before weights.
- [ ] Require replay parity and source provenance before any covariance diagnostics.
- [ ] Keep any future optimizer CPU-only and bounded at first.

## Validation Checklist

- [ ] `npm --prefix web run test -- portfolioWorkspaceModel portfolioService PortfolioFeasibilityWorkspacePanel`
- [ ] `npm --prefix web run typecheck`
- [ ] `npm --prefix web run check:qa-actions`
- [ ] `powershell -File scripts/run_repo_quality_gate.ps1 -IncludeDesktopTypecheck`
- [ ] If backend commands are added, run `npm --prefix web run parity` and `npm --prefix web run check:commands`.
- [ ] If app-core feasibility changes are made, run `cargo test -p ta_app_core portfolio_feasibility`.

## Regression Watchpoints

- Artifact selection and analysis must remain read-only.
- Export must not include full raw artifacts or matrices.
- Export must not create durable portfolio artifacts.
- The feature must not imply eligible candidates are approved allocations.
- Candidate extraction must remain bounded before any heavier construction diagnostics.

## Definition Of Done

- Users can export compact feasibility evidence as JSON/CSV.
- Users can see construction readiness gates without receiving weights or orders.
- Tests prove exported payloads are research-only and compact.
