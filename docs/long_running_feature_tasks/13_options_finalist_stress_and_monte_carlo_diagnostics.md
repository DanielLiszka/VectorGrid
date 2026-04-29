# Options Finalist Stress And Monte Carlo Diagnostics

Status: display-only stress diagnostics implemented; MC contract/engine remain separate high-risk work
Risk: high for modeling changes; moderate for display-only stress diagnostics
Primary value: make options finalist robustness clearer without silently changing ranking or validation semantics

## Summary

The options workflow already has deterministic stress lanes and a finalist dashboard. A useful expansion is to make stress diagnostics more explicit in the finalist surface, then design options-local Monte Carlo as a separate diagnostic feature. Generic strategy Monte Carlo samples ordinary closed-trade P&L; options outcomes have premium-normalized, asymmetric, support-limited behavior, so reuse without an options-specific contract would be misleading.

OCC's options disclosure emphasizes that options carry substantial risk. OCC's STANS margin methodology uses Monte Carlo and stress components at clearing scale; this supports the idea that stochastic diagnostics and deterministic stress are different evidence families. In this repo, MC should remain an advisory diagnostic until it is explicitly sourced from sealed/holdout options outcomes.

Sources:

- OCC Characteristics and Risks of Standardized Options: https://www.theocc.com/company-information/documents-and-archives/options-disclosure-document
- OCC margin methodology: https://www.theocc.com/risk-management/margin-methodology

## Current Anchors

- Options roadmap: `docs/options_research/options_expression_future_scope_roadmap_2026-04-07.md`
- Core schema/engine: `crates/app_core/src/options_types.rs`, `crates/app_core/src/options_expression.rs`
- Tauri command/artifact boundary: `src-tauri/src/commands_options_expression.rs`
- Frontend service/store/UI: `web/src/services/optionsExpressionService.ts`, `web/src/stores/optionsExpressionStore.ts`, `web/src/components/OptionsExpressionPanel.vue`
- Finalist dashboard model: `web/src/utils/optionsFinalistDashboardModel.ts`
- Existing parent plan: `06_options_finalist_acceptance_dashboard.md`

## Existing Behavior

- Deterministic stress lanes include baseline, conservative/stress, harsher, and extreme.
- Sealed finalist ranking already considers multiple stress fields.
- Subsequent validation recomputes selected tuples across validation plans and stress lanes.
- The finalist dashboard shows stress EV and harsher stress EV but no options-local MC stats.
- Validation requests do not include options MC config.

## Additive Boundary

- First slice can improve stress dashboard/reporting only.
- MC config/result fields must be optional and default-off.
- MC diagnostics must not change ranking, promotion, acceptance, or gates until a separate decision rule is approved.
- MC must run only on selected tuples/finalists, not the full candidate grid.
- CUDA/GPU broad-grid benchmarking is deferred.

## Non-Goals

- No generic strategy MC reuse without options outcome mapping.
- No new options stress lane unless it provides distinct decision value.
- No silent rank changes.
- No live trading or broker integration.
- No GPU benchmark requirement while CUDA work is deferred.

## Implementation Checklist

### Phase 1: Stress Dashboard Expansion

- [ ] Add explicit stress summary rows for baseline, stress, harsher, and extreme where present.
- [x] Add a compact "strongest available stress" field to finalist rows.
- [x] Add warnings when only aggregate price semantics are available.
- [ ] Extend CSV export with display-only stress columns if absent.
- [x] Add tests for legacy artifacts with missing stress fields.

### Phase 2: MC Contract Design

- [ ] Add optional `options_monte_carlo` config type in `options_types.rs`.
- [ ] Include schema version, enabled flag, simulations, mode, block length, source lane, tail alpha, thresholds, bins, and seed.
- [ ] Add optional MC result structs with probability positive, p05/p50/p95, tail mean, worst run/drawdown, sample count, warnings, and fail reasons.
- [ ] Keep serde defaults for old artifacts.

### Phase 3: Options-Local MC Engine

- [ ] Implement in a dedicated options module or tightly scoped section near options expression validation.
- [ ] Decide sampling unit: option event, session block, or trade-like lifecycle.
- [ ] Support IID and stationary/session-block sampling.
- [ ] Source MC from holdout/sealed/stress lanes explicitly.
- [ ] Bound simulations and finalist count.

### Phase 4: Frontend/Artifact Wiring

- [ ] Add validation form fields only after backend contract exists.
- [ ] Normalize MC stats in `optionsExpressionService.ts`.
- [ ] Render MC columns in finalist dashboard as advisory diagnostics.
- [ ] Persist MC config/stats through options artifact builders.
- [ ] Extend CSV export with optional MC columns.

## Validation Checklist

- [ ] `cargo test -p ta_app_core options_expression`
- [ ] `cargo test -p ta_desktop_demo_app commands_options_expression`
- [ ] `npm --prefix web run test -- optionsExpressionService optionsExpressionStore OptionsExpressionPanel optionsFinalistDashboardModel`
- [ ] `npm --prefix web run typecheck`
- [ ] `powershell -File scripts/run_repo_quality_gate.ps1 -IncludeWorkspaceTests`
- [ ] Defer `scripts/run_gpu_perf_gate.ps1 -Mode candidate` and `scripts/run_gpu_validation_perf_gate.ps1 -Mode candidate`.

## Regression Watchpoints

- Stress diagnostics must not become hidden ranking inputs.
- MC over selected finalists is post-selection diagnostics, not proof.
- IID sampling can understate clustered risk; label the mode.
- Quote semantics and fill realism must be carried into the display.
- Thresholds must be recorded with any MC result.

## Definition Of Done

- Finalist stress diagnostics are easier to inspect.
- MC contract is optional, versioned, and explicit.
- Any future MC engine is bounded to selected tuples and validated with deterministic seeds.
