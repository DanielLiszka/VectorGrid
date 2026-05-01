# Options Expression Expansion Plan

Date: 2026-04-30

## Goal

Expand the current options-expression workflow as a research-grade optimization and embedded-validation lane. The priority is stronger dataset provenance, quote-quality visibility, validation windows, CPU/GPU parity, and artifact replay safety.

## Current Owner Files

Start here:

- `docs/options_research/options_expression_optimizer_viable_plan_2026-04-02.md`
- `docs/options_research/options_expression_future_scope_roadmap_2026-04-07.md`
- `docs/options_research/options_expression_polygon_validation_hardening_implementation_plan_2026-04-25.md`
- `web/src/components/OptionsExpressionPanel.vue`
- `web/src/stores/optionsExpressionStore.ts`
- `web/src/services/optionsExpressionService.ts`
- `web/src/utils/optionsValidationForm.ts`
- `src-tauri/src/commands_options_expression.rs`
- `src-tauri/src/commands_options_data.rs`
- `src-tauri/src/options_parent_adapters.rs`
- `crates/app_core/src/options_types.rs`
- `crates/app_core/src/options_signal_manifest.rs`
- `crates/app_core/src/options_data_store.rs`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_expression_gpu.rs`
- `crates/app_core/src/kernels/options_expression.cu`
- `crates/app_core/src/kernels/options_expression.prebuilt.ptx`

Representative tests:

- `web/src/components/OptionsExpressionPanel.test.ts`
- `web/src/stores/optionsExpressionStore.test.ts`
- `web/src/services/optionsExpressionService.test.ts`
- options Rust tests and phase/promotion scripts.

## Research Basis

- Polygon/Massive options docs describe OPRA-sourced options data, regular options market hours, UTC timestamps, and quote/trade data: <https://www.polygon.io/docs/options>
- Polygon options trades expose tick-level trade data with participant/SIP timestamps and pagination: <https://polygon.io/docs/rest/options/trades-quotes/trades>
- Polygon option-chain snapshots can expose last quote, greeks, implied volatility, and open interest, with documented cases where greeks are absent: <https://polygon.io/docs/rest/options/snapshots/option-chain-snapshot>
- OCC's options disclosure document is the baseline public risk-disclosure reference for options behavior and should inform product copy boundaries: <https://www.theocc.com/company-information/documents-and-archives/options-disclosure-document>
- Backtest overfitting research supports visible out-of-sample and walk-forward evidence before presenting optimized tuples as accepted: <https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2326253>
- CUDA best practices apply if the options scorer is promoted as a GPU-performance feature: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>

## Scope Boundary

This plan expands the existing options optimization plus embedded validation workflow. It does not turn options into a public Simple Mode path and does not add a second provider.

Polygon.io remains the product provider label. `massive` identifiers remain compatibility aliases only where the code already canonicalizes them.

## Expansion Track A: Per-Window Walk-Forward Results

### Problem

The workflow supports walk-forward union-holdout style plans, but per-window tables remain future scope.

### Implementation

Backend:

- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_types.rs`
- `src-tauri/src/commands_options_expression.rs`

Frontend:

- `web/src/components/OptionsExpressionPanel.vue`
- `web/src/services/optionsExpressionService.ts`
- `web/src/stores/optionsExpressionStore.ts`

Add:

- per-window metrics;
- tuple availability per window;
- accepted/degraded/rejected verdict per window;
- drift reason summary;
- window coverage/low-sample flags;
- artifact schema version field.

### Tests

- old artifacts deserialize without per-window details;
- validation with anchored and rolling plans produces deterministic window order;
- frontend can render empty, partial, and full window results;
- report/export impact is explicitly tested if output shape changes.

## Expansion Track B: Quote Quality And Price Semantics

### Problem

Options outcomes can depend materially on whether prices come from trades, quotes, midpoint, or fallback logic.

### Implementation

Current owners:

- `crates/app_core/src/options_types.rs`
- `crates/app_core/src/options_data_store.rs`
- `crates/app_core/src/options_expression.rs`
- `web/src/services/optionsExpressionService.ts`
- `web/src/components/OptionsExpressionPanel.vue`

Add a visible quality summary:

- price semantics: trade, quote, midpoint, fallback;
- quote coverage;
- midpoint coverage;
- stale quote count, if represented;
- crossed/locked quote count, if represented;
- fill model family;
- timestamp basis: UTC raw plus market-time display;
- provider dataset provenance.

### Acceptance Criteria

- Objective scores do not change in the first quality-summary phase.
- Validation acceptance explains whether a tuple depended on weak price coverage.
- Saved artifacts carry enough data to explain the verdict later.

## Expansion Track C: CPU Oracle And GPU Promotion Gate

### Problem

Options GPU scoring exists, but promotion needs a strict CPU oracle and ranking parity.

### Implementation

Current owners:

- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_expression_gpu.rs`
- `crates/app_core/src/kernels/options_expression.cu`
- `crates/app_core/src/kernels/options_expression.prebuilt.ptx`
- `scripts/run_options_expression_gpu_promotion_gate.ps1`
- `scripts/profile_options_expression_compute_sanitizer.ps1`
- `scripts/profile_options_expression_ncu.ps1`
- `scripts/profile_options_expression_nsys.ps1`

Add:

- deterministic CPU oracle fixtures;
- CPU/GPU score tolerances by metric;
- top-k ranking parity threshold;
- explicit fallback when PTX is missing;
- manifest/resource check if options PTX becomes release-required.

### Tests And Gates

- options CPU scorer tests;
- options GPU scorer tests under CUDA features;
- `powershell -File scripts/run_options_expression_gpu_promotion_gate.ps1 -Mode candidate`;
- compute-sanitizer profile script when kernel memory behavior changes.

### Kill Criteria

- ranking parity cannot be made stable;
- GPU route requires hidden CPU tails under a GPU claim;
- options PTX packaging cannot be made explicit in release manifest.

## Expansion Track D: Event Path And Tuple Drilldown

### Problem

Users need to understand why a tuple passed, failed, or drifted after validation.

### Implementation

Current owners:

- `web/src/utils/optionsFinalistDashboardModel.ts`
- `web/src/components/OptionsExpressionPanel.vue`
- `web/src/services/optionsExpressionService.ts`
- `crates/app_core/src/options_expression.rs`

Add:

- event path inspection summary;
- entry/exit reason distribution;
- invalid candidate reason counts;
- selected tuple comparison;
- retained-tuple original-split replay indicator;
- session-chronological replay indicator;
- replacement data compatibility status.

### Tests

- tuple drilldown model handles missing optional event path;
- invalid reason counts sort deterministically;
- provenance mismatch blocks replay with clear reason.

## Expansion Track E: Options Report And Export Hardening

### Problem

Options results are complex enough that report/export shape must become an explicit contract.

### Implementation

Current owners:

- `src-tauri/src/commands_reports_api.rs`
- `src-tauri/src/commands_reports.rs`
- `web/src/services/historyService.ts`
- options artifact fields in app-core.

Add only after schema is stable:

- compact options validation Markdown summary;
- CSV export for tuple results;
- XLSX workbook sheets for dataset quality, tuple frontier, validation windows, and event-path diagnostics;
- output parity pair for representative options artifact.

### Verification

- `node scripts/run_output_parity_check.mjs`
- options component/service/store tests.

## Second-Pass Expansion: Finalist Dashboard And Stress Lanes

### Problem

Options tuples need a clear finalist workflow. A tuple can look attractive on aggregate score while depending on thin quotes, unstable windows, or stress-sensitive legs.

### Current Anchors

- `web/src/utils/optionsFinalistDashboardModel.ts`
- `web/src/components/OptionsExpressionPanel.vue`
- `web/src/stores/optionsExpressionStore.ts`
- `web/src/services/optionsExpressionService.ts`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_types.rs`

### Implementation

Add a finalist dashboard model with:

- tuple identity and expression hash;
- optimization score and validation verdict;
- per-window status count;
- quote/midpoint quality badge;
- invalid-event reason distribution;
- replacement-data replay class;
- retained-vs-full split comparison;
- event-path drilldown availability.

Add stress lanes as additive diagnostics:

- underlying move shocks;
- implied-volatility move shocks where data supports it;
- spread widening/slippage stress;
- missing-greeks fallback state;
- max loss/profit labels only when the expression/fill model supports a valid calculation.

Do not change tuple ranking in the first phase. Stress lanes should explain sensitivity, not become a hidden objective.

### Tests

- finalist rows sort deterministically across tied scores;
- stress lanes render absent-greeks and absent-quotes cases;
- stress diagnostics do not change the accepted tuple set;
- legacy artifacts show "not available" rather than a failed stress status.

## Second-Pass Expansion: Options-Local Monte Carlo

### Problem

Options validation should not borrow generic equity trade-bootstrap semantics without naming the options-specific assumptions.

### Implementation

Add optional options-local Monte Carlo detail:

- source: trade event path, per-window returns, or simulated leg path;
- fill model version;
- quote/midpoint coverage used;
- spread stress policy;
- sample count and seed policy;
- retained tuple subset;
- low-sample and stale-quote warnings.

Backend:

- keep raw simulated paths out of default artifacts unless an export explicitly requests them;
- include summaries in options result payloads only after CPU oracle parity is stable;
- keep GPU promotion separate from Monte Carlo expansion.

Frontend:

- expose the summary under Advanced diagnostics;
- use neutral labels such as `sensitivity evidence`, not `risk guarantee`;
- link back to dataset quote quality.

### Acceptance Criteria

- options-local Monte Carlo is optional and backward-compatible;
- tuple validation remains explainable without Monte Carlo;
- report/export parity is added before any public release claim.

## Non-Goals

- No live options trading.
- No Simple Mode public options path.
- No provider other than Polygon.io.
- No hidden trade/quote interpolation that changes scoring without explicit price semantics.
- No cross-stage VRAM pipeline from parent optimization into options validation.

## Verification

Targeted:

- `npm --prefix web run test -- src/components/OptionsExpressionPanel.test.ts src/stores/optionsExpressionStore.test.ts src/services/optionsExpressionService.test.ts src/utils/optionsValidationForm.test.ts`
- options Rust tests as added.

Promotion:

- `powershell -File scripts/run_options_expression_phase05.ps1`
- `powershell -File scripts/run_options_expression_gpu_promotion_gate.ps1 -Mode candidate`

Broad:

- `powershell -File scripts/run_repo_quality_gate.ps1`
