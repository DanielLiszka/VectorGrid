# Second-Pass Research Feature Candidates

Date: 2026-04-30

## Purpose

This document captures additional add-on feature candidates from the second research pass. These are not commitments. They are higher-quality implementation ideas with source anchors, repo entry points, risks, and kill criteria.

## Candidate Matrix

| Candidate | Value | Initial risk | Recommended timing |
|---|---|---:|---|
| Dataset Manifest V2 | Stronger replay, support, and provider/data-quality evidence. | Medium | Batch 3 after artifact durability. |
| Market Calendar Service | Correct holidays, early closes, options sessions, and live-feed state. | Medium | Before live/options expansion. |
| Polygon Options Capability Audit | Makes quote/trade/Greeks/open-interest availability visible. | Medium | Before options GPU/public claims. |
| Alpaca Reconciliation Timeline | Distinguishes intent, REST acceptance, stream update, fill, cancel, reject. | Medium/high | Before any live execution expansion. |
| Portfolio Risk Dashboard | Adds research-only covariance, VaR/CVaR/ES, concentration, drawdown. | Medium | After portfolio replay evidence. |
| Overfit Evidence Panel | Makes trial count, DSR/PSR/PBO/SPA scope visible. | Medium | Before expanding strategy search spaces. |
| Local Model Registry | Adds champion/candidate aliases, model cards, schema hashes, rollback identity. | Medium | Before ML promotion. |
| Columnar Analytics Path | Arrow IPC/Parquet/Perspective for large artifacts after profiling. | Medium/high | Defer until UI/payload profiling proves need. |
| CUDA Residency Probe | First-class H2D/D2H/device lifetime evidence. | Medium | Before new GPU-route claims. |
| CUDA Graph Candidate Lane | Reduces CPU launch overhead for stable repeated workflows. | High | Research spike only. |

## Candidate A: Dataset Manifest V2

### Sources

- Polygon stock aggregates: <https://polygon.io/docs/rest/stocks/aggregates/custom-bars>
- Polygon options overview: <https://www.polygon.io/docs/options>
- SEC Regulation NMS market data context: <https://www.sec.gov/rules-regulations/2005/06/regulation-nms>
- NYSE/Nasdaq trading calendars: <https://www.nyse.com/markets/hours-calendars>, <https://www.nasdaq.com/market-activity/stock-market-holiday-schedule>

### Repo Entry Points

- `src-tauri/src/polygon.rs`
- `src-tauri/src/data_cache.rs`
- `src-tauri/src/commands_data.rs`
- `src-tauri/src/settings.rs`
- `src-tauri/src/local_path_policy.rs`
- `web/src/utils/core/datasetMeta.ts`
- `web/src/services/dataService.ts`
- `web/src/stores/dataStore.ts`
- `web/src/components/DataWorkspacePanel.vue`
- `web/src/services/settingsService.ts`
- `web/src/components/SettingsWorkspacePanel.vue`

### Fields

- provider canonical name and display label;
- endpoint identity;
- symbol/asset class;
- requested range and actual returned range;
- timeframe/multiplier;
- adjusted/unadjusted flag;
- timezone/session calendar version;
- provider plan/recency where known;
- corporate-action adjustment state;
- gap/duplicate/out-of-order repair summary;
- cache hash and source fingerprint.

### Implementation Notes

- Add fields additively.
- Keep old artifacts readable.
- Do not guess subscription entitlement from failed calls.
- Keep market-data root and open-path policy explicit.

### Kill Criteria

- Manifest forces full cache migration on startup.
- Manifest stores API keys, private absolute paths in public reports, or raw provider URLs containing secrets.

## Candidate B: Market Calendar Service

### Sources

- NYSE holidays and early closes: <https://www.nyse.com/markets/hours-calendars>
- Nasdaq trading hours: <https://www.nasdaq.com/market-activity/stock-market-holiday-schedule>
- Cboe U.S. options hours and holidays: <https://www.cboe.com/en/about/hours/us-options/>
- OCC options risk disclosure: <https://www.theocc.com/components/docs/riskstoc.pdf>

### Repo Entry Points

- new: `crates/app_core/src/market_calendar.rs`
- new: `web/src/utils/marketCalendar.ts`
- `web/src/utils/uiTime.ts`
- `web/src/utils/optionsValidationForm.ts`
- `src-tauri/src/commands_data.rs`
- `src-tauri/src/commands_options_data.rs`

### Feature

Add a small, versioned market-calendar service for:

- equity regular session;
- pre/after-hours stock live-feed labels;
- options regular hours;
- Cboe product-specific options exceptions;
- holidays and early closes;
- validation/holdout calendar labels.

### Risk Control

- Start with display/validation labels, not trading execution decisions.
- Make calendar source/version visible.
- Keep manual override possible for historical data quirks.

## Candidate C: Polygon Options Capability Audit

### Sources

- Polygon options overview: <https://www.polygon.io/docs/options>
- Polygon option contract snapshot: <https://polygon.io/docs/rest/options/snapshots/option-contract-snapshot>
- Polygon options trades: <https://polygon.io/docs/rest/options/trades-quotes/trades>

### Repo Entry Points

- `src-tauri/src/commands_options_data.rs`
- `src-tauri/src/commands_options_expression.rs`
- `crates/app_core/src/options_data_store.rs`
- `crates/app_core/src/options_types.rs`
- `web/src/services/optionsExpressionService.ts`
- `web/src/components/OptionsExpressionPanel.vue`

### Feature

Expose an options capability summary:

- trades available;
- quotes available;
- latest quote/trade fields present;
- Greeks/IV/open-interest from snapshot where retrieved;
- bid/ask spread distribution;
- quote staleness;
- crossed/locked quote count where represented;
- sequence gaps where represented;
- dataset timestamp basis.

### Risk Control

- Treat all provider-plan-gated fields as optional.
- Keep Greeks/snapshots out of scoring unless a separate CPU oracle and provenance plan exist.

## Candidate D: Alpaca Reconciliation Timeline

### Sources

- Alpaca order docs and bracket order semantics: <https://docs.alpaca.markets/docs/trading/orders/>
- Alpaca streaming market data: <https://docs.alpaca.markets/docs/streaming-market-data>
- Alpaca trade/account updates: <https://docs.alpaca.markets/v1.4.2/docs/websocket-streaming>
- FINRA intraday margin proposal: <https://www.finra.org/rules-guidance/rule-filings/sr-finra-2025-017>

### Repo Entry Points

- `src-tauri/src/commands_execution.rs`
- `src-tauri/src/broker_paper.rs`
- `src-tauri/src/execution_policy.rs`
- `src-tauri/src/execution_mocks.rs`
- `web/src/services/executionService.ts`
- `web/src/services/liveExecutionLifecycleMachine.ts`
- `web/src/stores/executionStore.ts`
- `web/src/components/ExecutionWorkspacePanel.vue`

### Feature

Persist credential-free execution evidence:

- local intent;
- risk-policy verdict;
- `client_order_id`;
- REST submit result;
- idempotent lookup result;
- stream events;
- partial fills;
- bracket child order activation/cancel;
- reject/cancel/replace reasons;
- runner disarm reason.

### Risk Control

- REST acceptance is not execution proof.
- Emergency halt/flatten/cancel bypass all pacing.
- Live UI should query broker/account constraints rather than hardcoding historical PDT logic.

## Candidate E: Portfolio Risk Dashboard

### Sources

- Markowitz portfolio selection reference: <https://traders.berkeley.edu/papers/Markowitz.pdf>
- Rockafellar/Uryasev CVaR reference: <https://sites.math.washington.edu/~rtr/papers/rtr179-CVaR1.pdf>
- BIS market risk framework as conceptual ES reference: <https://www.bis.org/bcbs/publ/d457.htm>

### Repo Entry Points

- `crates/app_core/src/portfolio_feasibility.rs`
- `src-tauri/src/commands_portfolio.rs`
- `web/src/services/portfolioService.ts`
- `web/src/utils/portfolioWorkspaceModel.ts`
- `web/src/components/PortfolioFeasibilityWorkspacePanel.vue`

### Feature

Research-only risk dashboard:

- correlation/covariance matrix summary;
- concentration metrics;
- historical VaR/CVaR/expected shortfall;
- drawdown and underwater duration;
- efficient-frontier-style comparison;
- source evidence completeness.

### Risk Control

- Label as research feasibility, not allocation advice.
- Do not generate broker orders.
- Do not claim regulatory capital compliance.

## Candidate F: Overfit Evidence Panel

### Sources

- Probability of Backtest Overfitting / CSCV: <https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2326253>
- Deflated Sharpe Ratio: <https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2460551>
- Hansen SPA: <https://papers.ssrn.com/sol3/papers.cfm?abstract_id=264569>
- Sullivan/Timmermann/White data snooping: <https://papers.ssrn.com/sol3/papers.cfm?abstract_id=65140>

### Repo Entry Points

- `crates/app_core/src/validation_stats.rs`
- `crates/app_core/src/cscv_pbo.rs`
- `crates/app_core/src/sharpe_stats.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `web/src/utils/workflowEvidenceModel.ts`
- `web/src/components/ValidationWorkspacePanel.vue`
- `web/src/components/MetaWorkspacePanel.vue`

### Feature

Add a concise overfit panel:

- trial count;
- candidate universe definition;
- selected objective;
- PSR/DSR availability;
- CSCV/PBO availability;
- SPA/Reality Check future flag;
- informal-trial caveat;
- minimum track record warning.

### Risk Control

- Never compute DSR/PBO over mixed incomparable return series.
- Never imply "proved profitable."

## Candidate G: Local Model Registry

### Sources

- NIST AI RMF: <https://www.nist.gov/itl/ai-risk-management-framework>
- MLflow model registry workflow: <https://mlflow.org/docs/latest/ml/model-registry/workflow/>
- Model Cards: <https://arxiv.org/abs/1810.03993>
- scikit-learn model persistence security: <https://scikit-learn.org/stable/model_persistence.html>

### Repo Entry Points

- `crates/app_core/src/ml_signal.rs`
- `crates/app_core/src/ml_confidence.rs`
- `src-tauri/src/commands_ml_signal.rs`
- `src-tauri/src/commands_meta.rs`
- `web/src/services/mlSignalService.ts`
- `web/src/services/metaService.ts`
- `web/src/components/MlSignalWorkspacePanel.vue`
- `web/src/components/MetaWorkspacePanel.vue`

### Feature

Add serverless/local model registry semantics:

- model id/version;
- champion/candidate alias;
- feature schema hash;
- dataset hash;
- training code/build hash;
- split policy;
- validation status;
- leakage checks;
- rollback identity;
- local model card Markdown/JSON export.

### Risk Control

- Lifecycle label is not approval.
- Avoid pickle-like unsafe model formats unless loading behavior is explicitly trusted and documented.

## Candidate H: Columnar Large-Result Path

### Sources

- Arrow columnar format: <https://arrow.apache.org/docs/format/Columnar.html>
- Arrow IPC: <https://arrow.apache.org/docs/format/IPC.html>
- Apache Parquet: <https://parquet.apache.org/docs/overview/>
- DuckDB Parquet query docs: <https://duckdb.org/docs/stable/guides/file_formats/query_parquet>
- Perspective data loading: <https://perspective.finos.org/guide/how_to/javascript/loading_data.html>

### Repo Entry Points

- `src-tauri/src/commands_reports_api.rs`
- `src-tauri/src/commands_reports_csv.rs`
- `web/src/services/historyService.ts`
- `web/src/services/artifactHistoryPerf.ts`
- `web/src/components/HistoryWorkspacePanel.vue`
- new: `web/src/services/artifactAnalyticsRows.ts`

### Feature

Use:

- JSON as compatibility path;
- Arrow IPC as optional transfer path;
- Parquet as optional durable export for large result tables;
- Perspective as flag-off-by-default advanced viewer.

### Risk Control

- Add schema version before binary output.
- Keep binary dependencies optional until profiling justifies them.

## Candidate I: CUDA Residency Probe And CUDA Graph Spike

### Sources

- CUDA Best Practices: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
- CUDA Graphs: <https://docs.nvidia.com/cuda/cuda-programming-guide/04-special-topics/cuda-graphs.html>
- Compute Sanitizer: <https://docs.nvidia.com/compute-sanitizer/ComputeSanitizer/index.html>
- Nsight Compute: <https://docs.nvidia.com/nsight-compute/NsightComputeCli/index.html>
- Nsight Systems: <https://docs.nvidia.com/nsight-systems/UserGuide/>

### Repo Entry Points

- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/vram_ma.rs`
- `crates/app_core/src/dynamic_indicator_adapter.rs`
- `crates/app_core/src/strategy_gpu_runtime.rs`
- `scripts/run_gpu_perf_gate.ps1`
- `scripts/run_gpu_validation_perf_gate.ps1`
- `scripts/profile_gpu_backtest_nsys.ps1`
- `scripts/profile_gpu_backtest_ncu.ps1`

### Feature

Add a residency probe:

- H2D/D2H copy counts;
- device buffer lifetime;
- pinned host buffer reuse;
- kernel launch counts;
- stream/graph candidate markers;
- route/device/build identity.

CUDA Graph spike:

- only for repeated stable launch topologies;
- include graph instantiate/update/replay timing;
- compare against current stream launches;
- fail closed if topology changes often.

### Risk Control

- Pinned memory can harm system health if overused.
- CUDA Graph APIs add complexity and only help specific launch-overhead cases.
- Probe output is diagnostic, not correctness authority.

