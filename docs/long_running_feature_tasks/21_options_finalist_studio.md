# Options Finalist Studio

Date: 2026-05-06
Status: researched implementation plan
Risk: medium-high
Primary value: make options-expression finalists reviewable through quote quality, price semantics, per-window validation, stress, and event-path diagnostics

## Summary

Options Finalist Studio should be built before options GPU promotion or broader options strategy expansion. The existing options-expression workflow is already substantial: dataset import/download, provenance, optimization, feasibility, validation, event-path inspection, finalist dashboard, stress metrics, artifact save/load, and replay checks. The remaining product gap is a dedicated finalist review flow that makes options-specific fragility visible.

The first slice is diagnostic and display-only. It should not change scoring, ranking, acceptance, or GPU promotion. It should organize existing and additive evidence into a clearer finalist experience: tuple identity, validation verdict, per-window results, quote/midpoint coverage, price semantics, stress lanes, event-path invalid reasons, greeks/IV/open-interest availability, and timestamp/session labels.

## External Research Basis

- Massive/Polygon options overview: https://polygon.io/docs/rest/options/overview?auth=signup
  - The provider states that options timestamps are Unix UTC and must be explicitly converted for market-hours analysis. It also describes options data as consolidated from U.S. options exchanges through OPRA.
- Massive/Polygon options trades endpoint: https://polygon.io/docs/rest/options/trades-quotes/trades
  - Trade rows distinguish exchange/SIP timestamp semantics. The studio should preserve timestamp basis rather than flattening everything into one "time" label.
- Massive/Polygon options flat-file trades: https://www.polygon.io/docs/flat-files/options/trades
  - Tick-level options trades can include nanosecond timestamps across U.S. options markets. This supports preserving precision and source in provenance fields.
- Massive/Polygon option chain snapshot: https://polygon.io/docs/rest/options/snapshots/option-chain-snapshot
  - Chain snapshots can include greeks, implied volatility, quote, trade, and open interest fields, while fields are plan-dependent or optional.
- Massive/Polygon option contract snapshot: https://polygon.io/docs/rest/options/snapshots/option-contract-snapshot
  - Contract snapshots expose break-even, IV, open interest, greeks, latest quote, latest trade, and underlying context.
- OCC options disclosure page: https://www.theocc.com/company-information/documents-and-archives/options-disclosure-document
  - OCC emphasizes the characteristics and risks of standardized options and the need to treat options as risk-bearing instruments, supporting explicit diagnostics rather than simplified approval badges.
- Cboe U.S. options hours: https://www.cboe.com/en/about/hours/us-options/
  - Options sessions differ by product and can include RTH, curb, global trading hours, holidays, and early closes. Do not treat one equity calendar rule as universal for options.
- NYSE holidays and trading hours: https://www.nyse.com/markets/hours-calendars
  - NYSE publishes holidays and early close details, including cases where eligible options close at a different time from equities. This reinforces explicit venue/session labels.
- Nasdaq trading calendar: https://nasdaqtrader.com/Trader.aspx?id=Calendar
  - Nasdaq calendar references and alerts should be treated as source material for any future versioned calendar rule set.

## Current Repo Anchors

- Options current scope:
  - `docs/options_research/options_expression_optimizer_viable_plan_2026-04-02.md`
  - `docs/options_research/options_expression_future_scope_roadmap_2026-04-07.md`
  - `docs/options_research/options_expression_polygon_validation_hardening_implementation_plan_2026-04-25.md`
- Core schema/engine:
  - `crates/app_core/src/options_types.rs`
  - `crates/app_core/src/options_data_store.rs`
  - `crates/app_core/src/options_expression.rs`
  - `crates/app_core/src/options_expression_gpu.rs`
  - `crates/app_core/src/options_signal_manifest.rs`
- Tauri:
  - `src-tauri/src/commands_options_expression.rs`
  - `src-tauri/src/commands_options_data.rs`
  - `src-tauri/src/options_parent_adapters.rs`
- Frontend:
  - `web/src/components/OptionsExpressionPanel.vue`
  - `web/src/stores/optionsExpressionStore.ts`
  - `web/src/services/optionsExpressionService.ts`
  - `web/src/utils/optionsValidationForm.ts`
- Existing implemented slices:
  - `docs/long_running_feature_tasks/06_options_finalist_acceptance_dashboard.md`
  - `docs/long_running_feature_tasks/13_options_finalist_stress_and_monte_carlo_diagnostics.md`

## Existing Behavior To Preserve

- Polygon/Massive remains the supported provider.
- Legacy provider aliases remain compatibility-only where already canonicalized.
- Direct provider download remains within current shipped data scope.
- Saved artifact replay remains provenance guarded.
- Existing stress metrics remain display/advisory unless backend verdicts already use them.
- Existing finalist dashboard remains valid; the studio can become a richer mode over it.

## Product Shape

Options Finalist Studio should be a focused review mode inside the existing options host or History follow-up panel.

Recommended sections:

1. Finalist Header
   - tuple index/rank
   - tuple identity
   - parent artifact identity
   - signal manifest identity
   - dataset id/hash
   - price semantics
   - validation verdict
   - stress-fragility status

2. Quality And Semantics
   - quote coverage
   - midpoint coverage
   - trade-only fallback
   - crossed/locked quote count
   - stale quote count
   - provider capability status
   - timestamp basis: UTC and market-session label

3. Per-Window Validation
   - retained split
   - sealed/holdout split
   - date-range holdout
   - anchored walk-forward
   - rolling walk-forward
   - per-window accepted/degraded/rejected/invalid status

4. Stress Lanes
   - baseline
   - stress
   - harsher stress
   - extreme stress
   - sealed stress where available
   - stress drop/regret summary

5. Event Path
   - entry/exit event count
   - invalid-event reason counts
   - missing quote/trade reasons
   - spread/midpoint fallback usage
   - selected tuple event path inspection

6. Optional Market Context
   - greeks availability
   - implied volatility availability
   - open interest availability
   - underlying move context
   - explicit "not available" state when provider plan/data does not include fields

## Availability States

Options diagnostics should use explicit availability states rather than booleans.

| State | Meaning |
|---|---|
| `available` | field was present and used/displayed |
| `absent_from_dataset` | dataset does not contain the field |
| `provider_plan_unavailable` | provider plan or endpoint does not expose the field |
| `not_requested` | request did not ask for this diagnostic |
| `not_applicable` | diagnostic does not apply to this tuple or price semantics |
| `not_comparable` | diagnostic exists but cannot be compared across selected rows |
| `computed_from_fallback` | diagnostic was derived from fallback semantics and must be labeled |

Use these states for greeks, IV, open interest, quotes, midpoint, event path, stress lanes, and per-window rows.

## Proposed Contracts

### OptionsFinalistStudioSummaryV1

```rust
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct OptionsFinalistStudioSummaryV1 {
    pub schema_version: u16,
    pub tuple_index: usize,
    pub tuple_identity: String,
    pub validation_verdict: Option<String>,
    pub dataset_quality: Option<OptionsDatasetQualitySummaryV1>,
    pub price_semantics: Option<OptionsPriceSemanticsSummaryV1>,
    pub window_summary: Option<OptionsValidationWindowSummaryV1>,
    pub stress_summary: Option<OptionsStressLaneSummaryV1>,
    pub event_path_summary: Option<OptionsEventPathSummaryV1>,
    #[serde(default)]
    pub warnings: Vec<String>,
}
```

### OptionsValidationWindowResultV1

```rust
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct OptionsValidationWindowResultV1 {
    pub window_id: String,
    pub mode: String,
    pub start_utc: Option<String>,
    pub end_utc: Option<String>,
    pub market_label: Option<String>,
    pub status: String,
    pub expectancy_per_premium: Option<f64>,
    pub sample_count: Option<usize>,
    pub invalid_event_count: Option<usize>,
    pub quote_coverage_pct: Option<f64>,
    pub midpoint_coverage_pct: Option<f64>,
    #[serde(default)]
    pub warnings: Vec<String>,
}
```

### OptionsEventPathSummaryV1

```rust
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct OptionsEventPathSummaryV1 {
    pub inspected: bool,
    pub event_count: usize,
    pub invalid_event_count: usize,
    pub invalid_reason_counts: Vec<(String, usize)>,
    pub used_midpoint_count: Option<usize>,
    pub used_trade_count: Option<usize>,
    pub used_fallback_count: Option<usize>,
}
```

Keep these fields optional if added to artifacts. The studio can derive some of them from existing response fields first.

### OptionsDataAvailabilitySummaryV1

```rust
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct OptionsDataAvailabilitySummaryV1 {
    pub quote_state: String,
    pub midpoint_state: String,
    pub greeks_state: String,
    pub implied_volatility_state: String,
    pub open_interest_state: String,
    pub timestamp_basis: Option<String>,
    pub session_rule_version: Option<String>,
    #[serde(default)]
    pub warnings: Vec<String>,
}
```

### OptionsInvalidEventReasonV1

```rust
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct OptionsInvalidEventReasonV1 {
    pub reason_code: String,
    pub count: usize,
    pub severity: String,
    pub example_timestamp_utc: Option<String>,
    pub example_contract: Option<String>,
}
```

## Additive Boundary

- No live options trading.
- No Simple Mode options.
- No hidden interpolation or price repair that changes scoring.
- No scoring/ranking changes in first slice.
- No GPU promotion.
- No assumption that greeks, IV, quotes, trades, or open interest always exist.
- No one-size-fits-all options session calendar.
- No automatic conversion from missing quotes to interpolated midpoints.
- No comparing tuples across different price semantics without a visible warning.

## Implementation Phases

### Phase 1: Studio View Model From Existing Fields

- [ ] Add `web/src/utils/optionsFinalistStudioModel.ts`.
- [ ] Normalize tuple identity, finalist rank, validation verdict, stress fields, price semantics, and dataset quality where already present.
- [ ] Add missing/legacy-safe states.
- [ ] Add tests for full, partial, legacy, and no-quote artifacts.

### Phase 2: Quality And Price Semantics Panel

- [ ] Surface quote coverage, midpoint coverage, crossed/locked count, stale count, and trade-only fallback status.
- [ ] Show provider capability status explicitly.
- [ ] Show UTC timestamp basis and market-session label where available.
- [ ] Link quality warnings back to dataset manifest/provenance.
- [ ] Add `OptionsDataAvailabilitySummaryV1` normalization.
- [ ] Add separate timestamp fields for provider/SIP/exchange basis where available.

### Phase 3: Per-Window Validation Results

- [ ] Extend backend result/artifact types with optional per-window rows if not already available.
- [ ] Preserve old artifacts through serde defaults.
- [ ] Render per-window table with deterministic ordering.
- [ ] Add status categories: accepted, degraded, rejected, invalid, not available.
- [ ] Add tests for anchored and rolling walk-forward row ordering.
- [ ] Add aggregate counts for accepted/degraded/rejected/invalid windows.
- [ ] Add "not comparable" when window definitions differ across artifacts.

### Phase 4: Event Path Drilldown

- [ ] Reuse existing tuple/event-path inspection commands.
- [ ] Add invalid reason summary.
- [ ] Add selected-event row details.
- [ ] Preserve explicit missing-data reasons.
- [ ] Avoid rendering event path as proof of execution realism.
- [ ] Add invalid reason taxonomy and examples.

Suggested invalid reason codes:

- `missing_quote`
- `missing_trade`
- `no_midpoint`
- `crossed_or_locked_quote`
- `stale_quote`
- `outside_supported_session`
- `expiration_or_contract_mismatch`
- `underlying_bar_missing`
- `price_semantics_mismatch`
- `unsupported_contract_type`

### Phase 5: Stress And Optional Market Context

- [ ] Expand stress lane display for baseline, stress, harsher, extreme, sealed stress.
- [ ] Add greeks/IV/open-interest availability context where data exists.
- [ ] Add "not available from dataset/provider plan" states.
- [ ] Add export-only diagnostic columns after UI stabilizes.
- [ ] Label deterministic stress and Monte Carlo, if later implemented, as separate evidence families.
- [ ] Keep stress lane source and thresholds visible in every summary.

### Phase 6: Report And Artifact Integration

- [ ] Add optional `options_finalist_studio_summary` block to artifacts if needed.
- [ ] Add CSV export columns for display-only diagnostics.
- [ ] Add XLSX sheet only after row model is stable.
- [ ] Add output parity tests for export shape changes.

## Verification Plan

Rust:

- `cargo test -p ta_app_core options_expression`
- `cargo test -p ta_desktop_demo_app --lib options_expression`
- targeted serialization tests for legacy options artifacts

Frontend:

- `npm --prefix web run test -- optionsExpressionService`
- `npm --prefix web run test -- optionsExpressionStore`
- `npm --prefix web run test -- OptionsExpressionPanel`
- `npm --prefix web run test -- optionsFinalistStudioModel`
- `npm --prefix web run typecheck`

Reports/exports:

- output parity checks if CSV/XLSX changes:
  - `node scripts/run_output_parity_check.mjs`

Data/time:

- Add DST and early-close fixtures once session labeling is introduced.
- Verify UTC input remains available in diagnostics after market-label formatting.

Broad:

- `powershell -File scripts/run_repo_quality_gate.ps1 -IncludeDesktopTypecheck`

## Regression Watchpoints

- Options timestamps must not be shown as local time without explicit UTC/ET/session labeling.
- Quote/midpoint diagnostics must not change objective scoring unless a later accepted proposal does so.
- Missing greeks/IV/open-interest is a data availability state, not a computation failure.
- Per-window validation must preserve chronological and mode-specific ordering.
- Stress diagnostics must distinguish baseline, retained, sealed, harsher, and extreme lanes.
- Event-path inspection should not mutate saved artifacts.
- GPU promotion must wait for CPU/GPU parity and route evidence.
- Provider field absence must not be converted into zero values.
- A missing quote-quality field should render `not available`, not `good`.
- Event-path diagnostics must preserve original price semantics.

## Definition Of Done

- Users can open a tuple/finalist into a dedicated studio view.
- Quote quality, price semantics, validation windows, stress lanes, and event-path diagnostics are visible together.
- Missing provider fields render as explicit availability states.
- Legacy artifacts remain readable.
- No scoring, ranking, acceptance, or GPU route behavior changes in the diagnostic slice.
