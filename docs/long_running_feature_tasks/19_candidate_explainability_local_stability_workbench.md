# Candidate Explainability And Local Stability Workbench

Date: 2026-05-06
Status: researched implementation plan
Risk: medium
Primary value: make optimized finalists inspectable, comparable, and easier to reject before deeper validation or campaign expansion

## Summary

Candidate Explainability and Local Stability Workbench is the most viable near-term product feature. The repo already has Double-MA and composite drilldown commands, local-stability requests, trade marker models, equity/drawdown bins, heatmap markers, analysis service tests, and implemented advanced-strategy drilldown parity. The missing layer is a coherent finalist review experience that answers: what did this candidate do, where did it make or lose money, how sensitive is it to nearby parameters, and why did it rank where it ranked?

This should remain explanatory. Warnings about fragile neighborhoods, drawdown concentration, regime dependence, or friction sensitivity must not become hidden rank penalties. Users should be able to inspect and compare candidates, then choose whether to validate, export, or include them in campaigns.

## External Research Basis

- Lightweight Charts marker API: https://tradingview.github.io/lightweight-charts/docs/api/functions/createSeriesMarkers
- Lightweight Charts marker tutorial: https://tradingview.github.io/lightweight-charts/tutorials/how_to/series-markers
  - The charting library already used by the repo supports series markers for data-point annotations, which fits entry/exit overlays without custom drawing.
- Apache ECharts visual mapping: https://echarts.apache.org/handbook/en/concepts/visual-map/
  - ECharts' visual mapping model is a good fit for parameter-neighborhood heatmaps and sensitivity surfaces.
- SALib project: https://github.com/SALib/SALib
  - SALib implements Sobol, Morris, FAST, and related sensitivity methods. VectorGrid should not import SALib, but its method taxonomy supports separating cheap local-neighborhood checks from broader global sensitivity analysis.
- NIST/SEMATECH e-Handbook of Statistical Methods: https://www.itl.nist.gov/div898/software/dataplot/handbook.htm
  - The NIST handbook is a useful reference for keeping uncertainty, sensitivity, and exploratory data analysis clearly labeled.
- Pyfolio slippage analysis example: https://quantopian.github.io/pyfolio/notebooks/slippage_example/
  - Pyfolio's slippage sweep pattern supports the proposed friction sensitivity lane: show how metrics change under execution-cost assumptions without changing the original backtest.
- Probability of Backtest Overfitting: https://www.davidhbailey.com/dhbpapers/backtest-prob.pdf
  - The paper argues that multiple trials increase false-positive risk and that in-sample winners can underperform out of sample. Candidate review should expose trial context and fragility rather than presenting the finalist as inherently robust.
- Deflated Sharpe Ratio: https://www.davidhbailey.com/dhbpapers/deflated-sharpe.pdf
  - DSR/PSR evidence should appear as supporting context where available, not as a stand-alone approval badge.

## Current Repo Anchors

- Tauri commands: `src-tauri/src/commands_analysis.rs`, `src-tauri/src/commands.rs`
- App-core drilldown exports: `crates/app_core/src/lib.rs`
- Double-MA core and follow-up benches: `crates/app_core/src/double_ma.rs`, `crates/app_core/benches/double_ma_followup_workflow.rs`
- Composite detail/stability: `crates/app_core/src/composite_cpu.rs`
- Frontend analysis service: `web/src/services/analysisService.ts`
- Drill chart runtime: `web/src/composables/useDrillPriceChartRuntime.ts`
- Advanced workspace: `web/src/components/AdvancedWorkspacePanel.vue`
- Heatmap/top result surfaces: `web/src/components/TopWorkspacePanel.vue`, `web/src/components/HeatmapWorkspacePanel.vue`
- Existing implemented plan: `docs/long_running_feature_tasks/02_advanced_strategy_drilldown_stability.md`
- Route/evidence context: `docs/long_running_feature_tasks/17_validation_evidence_pack.md`

## Existing Behavior To Preserve

- `compute_double_ma_drilldown` and `compute_composite_cpu_drilldown` remain scientific-compute commands.
- Local stability requests remain strategy-local and deterministic.
- Existing unsupported states remain explicit for any future family not covered.
- Optimization ranking remains unchanged.
- Existing chart/table models and tests continue to work.

## Product Shape

The workbench should center on selected finalists from optimization, validation, campaign, or artifact history.

Recommended sections:

1. Candidate Header
   - candidate id/rank
   - strategy family
   - objective and score
   - backend route
   - dataset identity
   - evidence pack status where available

2. Price And Trade Path
   - candlestick or line chart
   - fast/slow/indicator overlays where available
   - entry/exit markers
   - selected trade focus
   - trade table synchronized to marker selection

3. Equity And Drawdown
   - equity curve
   - drawdown bands
   - worst drawdown segment
   - drawdown contribution by trade or period

4. Local Stability
   - parameter-neighborhood heatmap
   - objective sensitivity
   - neighbor disagreement score
   - stable/fragile/unknown labels

5. Candidate Comparison
   - candidate A vs B metrics
   - trade count, exposure, return, drawdown, Sharpe/Sortino where available
   - stability delta
   - route/provenance delta

6. Warnings
   - fragile neighborhood
   - concentrated PnL
   - drawdown concentration
   - low trade count
   - evidence unavailable
   - route/fallback/cpu-tail caveats

## Evidence Families

Keep these evidence families separate in the UI and exports.

| Family | Source | Meaning | Must not imply |
|---|---|---|---|
| Trade path | drilldown trade records and markers | what happened on the selected candidate | robustness |
| Curve behavior | equity/drawdown bins or precise curves | when gains/drawdowns occurred | exact trade attribution if downsampled |
| Local stability | bounded parameter-neighborhood reruns | sensitivity near the selected candidate | global robustness |
| Friction sensitivity | fee/spread/slippage perturbation | implementation-cost sensitivity | future live execution quality |
| Validation evidence | validation evidence pack | out-of-sample/evidence status | approval to trade |
| Route provenance | backend route metadata | where computation ran and fell back | performance claim without benchmark |

This separation matters because a candidate can have an attractive trade path but unstable neighbors, or stable neighbors but weak validation evidence.

## Proposed View Models

### CandidateExplainabilityContextV1

```ts
export interface CandidateExplainabilityContextV1 {
  schemaVersion: 1;
  source:
    | "optimization"
    | "validation"
    | "campaign"
    | "artifact"
    | "options_parent";
  candidateId: string;
  strategyKind: string;
  objective: string;
  rank?: number;
  score?: number;
  dataId?: string;
  datasetLabel?: string;
  artifactId?: string;
  backendRoute?: string;
  evidencePackStatus?: "complete" | "incomplete" | "absent" | "unknown";
}
```

### CandidateStabilitySummaryV1

```ts
export interface CandidateStabilitySummaryV1 {
  schemaVersion: 1;
  status: "stable" | "fragile" | "mixed" | "not_computed" | "not_applicable";
  axes: string[];
  centerScore?: number;
  neighborCount: number;
  validNeighborCount: number;
  bestNeighborScore?: number;
  worstNeighborScore?: number;
  medianNeighborScore?: number;
  disagreementRate?: number;
  warnings: string[];
}
```

### CandidateAttributionSummaryV1

```ts
export interface CandidateAttributionSummaryV1 {
  schemaVersion: 1;
  tradeCount?: number;
  winningTradeCount?: number;
  losingTradeCount?: number;
  grossProfit?: number;
  grossLoss?: number;
  worstTradePnl?: number;
  worstDrawdown?: number;
  maxUnderwaterBars?: number;
  pnlConcentrationTopTradePct?: number;
  warnings: string[];
}
```

These can be derived in the frontend first from existing drilldown/local-stability payloads. Add backend fields only where deriving from binned curves would be misleading.

### CandidateFrictionSensitivityV1

```ts
export interface CandidateFrictionSensitivityV1 {
  schemaVersion: 1;
  mode: "fee_bps" | "spread_bps" | "slippage_bps" | "combined";
  scenarios: Array<{
    label: string;
    feeBps?: number;
    spreadBps?: number;
    slippageBps?: number;
    pnl?: number;
    sharpe?: number;
    maxDrawdown?: number;
    tradeCount?: number;
  }>;
  worstScenarioLabel?: string;
  warnings: string[];
}
```

### CandidateFragilityScoreV1

```ts
export interface CandidateFragilityScoreV1 {
  schemaVersion: 1;
  status: "low" | "medium" | "high" | "not_computed";
  basis: string[];
  neighborDisagreementRate?: number;
  scoreDropPctFromCenterToMedian?: number;
  invalidNeighborPct?: number;
  frictionFailureScenario?: string;
  warnings: string[];
}
```

The fragility score is an explanation badge only. It is not an optimizer objective.

## Backend Scope

First slice should mostly reuse current commands:

- `compute_double_ma_drilldown`
- `compute_composite_cpu_drilldown`
- `compute_double_ma_local_stability`
- `compute_composite_cpu_local_stability`

Possible additive backend work:

- Add optional route/provenance passthrough to detail results if missing.
- Add explicit warning fields when a stability grid has invalid neighbors.
- Add optional `CandidateAttributionSummaryV1` to drilldown results if precise trade-level attribution is easier and safer in Rust.
- Add optional friction-sensitivity detail only if existing validation friction stress cannot be reused at candidate scope.

Avoid:

- A generic strategy abstraction layer.
- Recomputing optimized candidates with altered defaults.
- Changing the candidate score or rank.
- Inferring exact trade attribution from downsampled bins when trade records are available.

## Candidate Review Entry Contract

Every entry point should provide enough identity to avoid accidental reconstruction.

Required:

- source surface: optimization, validation, campaign, artifact, or options parent;
- strategy kind;
- data id or artifact dataset identity;
- candidate row id or parameter identity;
- objective and score mode;
- backend route/provenance where known.

Blocked:

- opening detail from a row whose params are display-only and cannot reconstruct the exact candidate;
- opening cross-candidate comparison across incompatible objectives without a warning;
- mixing candidates from different datasets without explicit compare mode.

## Frontend Scope

Recommended implementation:

- New `web/src/utils/candidateExplainabilityModel.ts`
- New `web/src/utils/candidateStabilityModel.ts`
- Extend or refactor `web/src/services/analysisService.ts`
- Add `CandidateExplainabilityPanel.vue` or add a dedicated mode inside `AdvancedWorkspacePanel.vue`
- Reuse `useDrillPriceChartRuntime.ts`
- Reuse existing heatmap canvas/table components where possible

UI requirements:

- Entry points from top results, validation rows, campaign rows, and artifact history.
- Selected candidate state is stable across workspace navigation.
- Marker selection focuses trade row and vice versa.
- Heatmap and chart text summaries are present for accessibility.
- Every warning includes source and availability, not just a colored badge.

## Implementation Phases

### Phase 1: Model Consolidation

- [ ] Inventory current drilldown/local-stability response shapes.
- [ ] Add `candidateExplainabilityModel.ts`.
- [ ] Add normalizers for Double-MA and Composite detail results.
- [ ] Add tests for empty, partial, and unsupported states.
- [ ] Add stability summary derivation from existing local-stability payload.

### Phase 2: Candidate Header And Entry Points

- [ ] Add "Review candidate" actions in top results and validation rows.
- [ ] Carry candidate source context into the analysis workspace.
- [ ] Render candidate header with objective, rank, route, data id, and evidence status.
- [ ] Add tests that route candidates from Double-MA and Composite rows to the correct command family.

### Phase 3: Trade Path And Curves

- [ ] Reuse Lightweight Charts series markers for entries/exits.
- [ ] Add selected-trade synchronization between chart markers and trade table.
- [ ] Render equity/drawdown summaries from precise trade/curve data where available.
- [ ] Add blank/partial states for missing curve data.

### Phase 4: Stability And Neighbor Warnings

- [ ] Render local stability heatmap around selected params.
- [ ] Add neighbor disagreement summary.
- [ ] Add stable/fragile/mixed labels.
- [ ] Add warnings for insufficient neighbors, invalid params, and unstable objective surfaces.
- [ ] Add `CandidateFragilityScoreV1` from local stability plus optional friction sensitivity.
- [ ] Add explicit "local only" labeling for neighborhood results.

### Phase 5: Compare Candidates

- [ ] Allow pinning candidate A and candidate B.
- [ ] Compare metrics, trade behavior, drawdown, route, stability, and warnings.
- [ ] Add export/copy compact comparison summary.
- [ ] Link comparison back to campaign/artifact context.

### Phase 6: Friction And Contribution Diagnostics

- [ ] Reuse existing validation friction stress where possible.
- [ ] Add candidate-local friction sensitivity only if exact candidate reruns are available.
- [ ] Add PnL concentration and drawdown contribution summaries.
- [ ] Add warnings for low trade count or single-trade dominance.
- [ ] Add tests that these diagnostics do not alter rank/order.

## Verification Plan

Rust if backend fields change:

- `cargo test -p ta_app_core --lib drilldown_smoke_runs`
- `cargo test -p ta_app_core --lib local_stability_smoke_runs`

Frontend:

- `npm --prefix web run test -- analysisService`
- `npm --prefix web run test -- candidateExplainabilityModel`
- `npm --prefix web run test -- AdvancedWorkspacePanel`
- `npm --prefix web run typecheck`

Browser/UI:

- Run frontend E2E gate if the workspace flow changes:
  - `powershell -File scripts/run_frontend_e2e_gate.ps1`

Performance:

- If local stability or drilldown execution changes materially:
  - `powershell -File scripts/run_cpu_perf_gate.ps1 -Mode candidate`

Accessibility:

- Chart marker counts must have a text/table equivalent.
- Heatmap stability status must be summarized outside canvas.
- Keyboard selection of candidate rows and marker-linked trade rows must be tested if UI changes.

## Regression Watchpoints

- Candidate detail must match the exact candidate, not a reconstructed similar parameter set.
- Downsampled curves must not be used for precise attribution where trade records exist.
- Local stability must not mutate original request state.
- Fragility warnings must not silently affect ranking.
- Route labels must preserve CPU/GPU/fallback truth.
- Marker-heavy charts must remain usable on large trade counts.
- Accessibility summaries must describe chart evidence outside canvas-only rendering.
- Local sensitivity must be labeled as local; do not imply Sobol/Morris-style global sensitivity unless a separate global design is implemented.
- Friction sensitivity must record the cost assumptions used.

## Definition Of Done

- Users can open any supported finalist into a dedicated review surface.
- Trade markers, equity, drawdown, and local-stability evidence render together.
- Candidate A/B comparison works for same-family candidates.
- Unsupported or partial states render explicitly.
- Warnings are explanatory and do not change scoring.
- Tests prove Double-MA and composite paths route through the correct command families.
