# Portfolio Feasibility Workspace

Status: proposed
Risk: low to moderate
Primary value: turn an existing read-only backend capability into a first-class product workflow

## Summary

Build a Portfolio Research workspace around the existing `analyze_portfolio_feasibility` command. The first shipped slice should remain read-only: select saved artifacts, run feasibility analysis, inspect eligible and rejected candidates, and show what evidence is still missing before any portfolio-level allocation research is trusted.

This task is additive because the backend command, Tauri bridge, frontend service, and tests already exist. The missing part is a first-class workflow surface.

## Current Anchors

- Backend analysis: `crates/app_core/src/portfolio_feasibility.rs`
- Tauri command: `src-tauri/src/commands_portfolio.rs`
- Frontend service: `web/src/services/portfolioService.ts`
- Command risk policy: `src-tauri/command-risk-policy.json`
- Artifact library surfaces: `src-tauri/src/run_artifacts.rs`, `web/src/stores/workflowArtifactsStore.ts`, `web/src/services/historyService.ts`

## Additive Boundary

- Use only saved artifact JSON and existing read-only command output.
- Do not emit weights, target allocations, trades, broker orders, or durable portfolio artifacts in the first slice.
- Do not change validation, meta, ML Signal, or options artifact schemas except for optional UI metadata if required.
- Keep the command classified as research-only.
- Treat any future portfolio target, allocation, rebalance, margin, or execution output as a separate plan.

## Non-Goals

- No live/paper execution integration.
- No portfolio optimizer in the first slice.
- No automatic order generation.
- No changes to candidate scoring or ranking inside existing optimization/validation workflows.

## Implementation Checklist

### Phase 1: Product Surface And Navigation

- [ ] Decide the host surface: new workspace panel, History follow-up panel, or Advanced/Research panel subsection.
- [ ] Add a visible entry point that does not imply trading or allocation execution.
- [ ] Add empty state copy for no saved artifacts.
- [ ] Add artifact family filters for validation, meta, Stage 5, ML Signal, and options expression artifacts.
- [ ] Add source artifact selection with stable identifiers and provenance summary.
- [ ] Add a "Run feasibility" action wired to `analyzePortfolioFeasibility`.
- [ ] Add busy, success, error, and stale-selection states.

### Phase 2: Result Presentation

- [ ] Show overall status and next required step from the backend result.
- [ ] Show eligible candidates with family, source artifact, data id, timeframe, metrics, and provenance state.
- [ ] Show rejected candidates with explicit rejection reason.
- [ ] Show evidence requirements, including replay parity, covariance evidence, and source provenance.
- [ ] Add compact summary cards for candidates seen, eligible, rejected, source artifacts seen, and status.
- [ ] Make unsupported options families visible instead of hiding them.
- [ ] Keep all copy research-oriented.

### Phase 3: Artifact Integration

- [ ] Add a helper that extracts selected artifact payloads without mutating library state.
- [ ] Support selected artifacts from the saved library and current run where applicable.
- [ ] Add compatibility handling for missing durable hashes or legacy artifacts.
- [ ] Add a provenance warning when feasibility is limited by source identity gaps.
- [ ] Add hydration tests for reopening a workspace with a prior selected set if state is persisted.

### Phase 4: Reporting And Follow-Up

- [ ] Add local UI export only if it exports the analysis report, not allocations.
- [ ] Include the backend `next_required_step` in exported text/CSV if export is added.
- [ ] Add follow-up actions that route users back to validation/replay workflows without changing artifacts automatically.
- [ ] Keep all follow-up actions manual and reversible.

### Phase 5: Pre-Allocation Design Gate

- [ ] Before any future allocation output, define a separate portfolio construction contract.
- [ ] Separate feasibility evidence from target generation.
- [ ] Define risk model, rebalance cadence, margin assumptions, and execution isolation before weights are ever emitted.
- [ ] Keep this phase blocked until the read-only workspace has shipped and stabilized.

## Validation Checklist

- [ ] `npm --prefix web run test -- src/services/portfolioService.test.ts`
- [ ] Add component/store tests for empty, eligible, rejected, and provenance-limited states.
- [ ] Add a test proving the frontend invokes only `analyze_portfolio_feasibility`.
- [ ] Add a test proving no weights, trades, or broker-order fields are rendered as outputs.
- [ ] Run targeted Tauri tests for `commands_portfolio` if backend shape changes.
- [ ] Run `powershell -File scripts/run_repo_quality_gate.ps1 -IncludeDesktopTypecheck` before handoff for broad UI changes.

## Regression Watchpoints

- Artifact library load time must not regress noticeably.
- Existing History, Meta, Validation, ML Signal, and Options saved artifact flows must continue to hydrate.
- The workspace must not create a new decision path that bypasses validation gates.
- The command must remain read-only in risk policy.
- UI copy must not imply that eligible candidates are approved allocations.

## Definition Of Done

- Users can select saved research artifacts and understand whether portfolio-level research is feasible.
- Rejected candidates and missing evidence are explainable.
- No allocation, order, or execution output is produced.
- Tests cover service normalization, UI states, and command usage.
