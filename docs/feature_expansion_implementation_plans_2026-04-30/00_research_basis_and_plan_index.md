# Research Basis And Plan Index

Date: 2026-04-30

## Method

This planning package was built from two inputs:

1. Local repo routing and current source inventory:
   - `docs/REPO_MAP.md`
   - `README.md`
   - `docs/ARCHITECTURE_NOTES.md`
   - `web/README.md`
   - `docs/repo_wide_deep_dive_2026-04-30/`
   - `docs/long_running_feature_tasks/`
   - current `Cargo.toml`, `web/package.json`, `src-tauri/Cargo.toml`, `crates/app_core/Cargo.toml`
   - current source and script file inventory

2. Web research from primary or authoritative sources:
   - official framework documentation;
   - official provider documentation;
   - official NVIDIA documentation;
   - official NIST/OWASP security guidance;
   - peer-reviewed or paper-hosted backtesting research where strategy validation is involved.

## Source Ledger

### Tauri Desktop Runtime, Security, Release

- Tauri Runtime Authority: <https://v2.tauri.app/security/runtime-authority/>
- Tauri Security overview: <https://v2.tauri.app/security/>
- Tauri capabilities: <https://v2.tauri.app/security/capabilities/>
- Tauri CSP: <https://v2.tauri.app/security/csp/>
- Tauri WebView versions: <https://v2.tauri.app/reference/webview-versions/>
- Tauri single instance plugin: <https://v2.tauri.app/plugin/single-instance/>
- Tauri deep-link plugin: <https://v2.tauri.app/plugin/deep-linking/>
- Tauri Windows code signing: <https://v2.tauri.app/distribute/sign/windows/>
- Tauri updater plugin and update signing: <https://v2.tauri.app/plugin/updater/>
- Tauri WebDriver testing: <https://v2.tauri.app/develop/tests/webdriver/>
- Microsoft WebView2 security: <https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/security>
- Microsoft SmartScreen reputation: <https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/smartscreen-reputation>

Source-informed rules:

- Tauri command expansion must treat the WebView-to-Rust boundary as a trust boundary.
- Capability files and command scopes are not documentation-only; they are runtime enforcement inputs.
- External open-artifact paths, deep links, and file associations are new trust boundaries and need strict path/scheme validation.
- Updater work requires signed update artifacts and private-key handling outside the repo.
- Windows public release work must distinguish packaging, Authenticode signing, updater signing, and clean-machine smoke evidence.
- Code signing improves trust but does not guarantee zero SmartScreen warning for every new file hash.

### Vue, Vite, Frontend Testing, Visualization

- Vue performance guide: <https://vuejs.org/guide/best-practices/performance>
- Vue accessibility guide: <https://vuejs.org/guide/best-practices/accessibility.html>
- Vue security guide: <https://vuejs.org/guide/best-practices/security.html>
- Vue Test Utils async behavior: <https://test-utils.vuejs.org/guide/advanced/async-suspense.html>
- Pinia testing cookbook: <https://pinia.vuejs.org/cookbook/testing.html>
- Vite performance guide: <https://vite.dev/guide/performance/>
- Playwright best practices: <https://playwright.dev/docs/best-practices>
- Playwright locators: <https://playwright.dev/docs/locators>
- Playwright visual comparisons: <https://playwright.dev/docs/test-snapshots>
- Playwright accessibility testing with axe: <https://playwright.dev/docs/accessibility-testing>
- WAI-ARIA Authoring Practices patterns: <https://www.w3.org/WAI/ARIA/apg/patterns/>
- WCAG 2.2: <https://www.w3.org/TR/WCAG22/>
- Lightweight Charts markers: <https://tradingview.github.io/lightweight-charts/tutorials/how_to/series-markers>
- Lightweight Charts marker API: <https://tradingview.github.io/lightweight-charts/docs/api/functions/createSeriesMarkers>
- Apache ECharts tree-shakable imports: <https://echarts.apache.org/handbook/en/basics/import/>
- Apache ECharts ARIA guidance: <https://echarts.apache.org/handbook/en/best-practices/aria/>
- TanStack Virtual: <https://tanstack.com/virtual/latest/docs>
- TanStack Form Vue validation: <https://tanstack.com/form/v1/docs/framework/vue/guides/validation>
- XState v5 docs: <https://stately.ai/docs>

Source-informed rules:

- Large result surfaces should remain virtualized.
- Frontend QA should use user-visible behavior, role/text/test-id locators, and web-first assertions.
- Heavy visualization and analytics dependencies must be lazy, feature-flagged where appropriate, and checked against bundle budgets.
- Form validation and async workflows need explicit tests for async state settling.
- Virtualized result surfaces need an accessibility decision: native table semantics for read-only tabular data, ARIA grid only where grid keyboard behavior is fully implemented.
- Canvas/chart surfaces need curated text summaries because automatically generated descriptions can be misleading for financial charts.
- Local CSV and artifact metadata must not flow into `v-html`, opener URLs, or deep-link handlers without validation.

### CUDA, GPU Profiling, Benchmarking

- CUDA C++ Programming Guide: <https://docs.nvidia.com/cuda/cuda-c-programming-guide/contents.html>
- CUDA C++ Best Practices Guide: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
- CUDA Graphs programming guide: <https://docs.nvidia.com/cuda/cuda-programming-guide/04-special-topics/cuda-graphs.html>
- Compute Sanitizer: <https://docs.nvidia.com/compute-sanitizer/ComputeSanitizer/index.html>
- NVIDIA CCCL/CUB: <https://nvidia.github.io/cccl/unstable/cub/index.html>
- Nsight Compute CLI: <https://docs.nvidia.com/nsight-compute/NsightComputeCli/index.html>
- Nsight Systems User Guide: <https://docs.nvidia.com/nsight-systems/UserGuide/>
- Cargo feature resolver: <https://doc.rust-lang.org/cargo/reference/resolver.html>
- Cargo features: <https://doc.rust-lang.org/stable/cargo/reference/features.html>
- Criterion docs: <https://docs.rs/criterion/latest/criterion/>

Source-informed rules:

- GPU expansion should prefer device-resident intermediate data and batched host/device transfer.
- Memory access, coalescing, occupancy, and launch/resource limits are profiling questions, not style preferences.
- Nsight Systems and Nsight Compute answer different questions; use both when a claim spans timeline/transfer and kernel-level metrics.
- Workspace feature wiring must preserve CPU-only default behavior.
- CUDA Graphs are candidates only for repeated, stable launch topologies; they are not a generic optimization switch.
- Compute Sanitizer is correctness evidence for memory/race/sync issues, not a performance benchmark.
- CUB reductions/top-k can improve throughput but must be checked against deterministic ordering and numeric reproducibility policy.

### Market Data And Broker APIs

- Polygon/Massive stock aggregates: <https://polygon.io/docs/rest/stocks/aggregates/custom-bars>
- Polygon/Massive options overview: <https://www.polygon.io/docs/options>
- Polygon options trades: <https://polygon.io/docs/rest/options/trades-quotes/trades>
- Polygon stocks WebSocket overview: <https://polygon.io/docs/stocks/ws_getting-started>
- Polygon stock aggregate WebSocket bars: <https://polygon.io/docs/websocket/stocks/aggregates-per-second>
- Polygon options contract snapshot: <https://polygon.io/docs/rest/options/snapshots/option-contract-snapshot>
- Alpaca orders: <https://docs.alpaca.markets/docs/trading/orders/>
- Alpaca market-data WebSocket stream: <https://docs.alpaca.markets/docs/streaming-market-data>
- Alpaca trade/account update stream: <https://docs.alpaca.markets/v1.4.2/docs/websocket-streaming>
- NYSE holidays and trading hours: <https://www.nyse.com/markets/hours-calendars>
- Nasdaq holiday and trading hours: <https://www.nasdaq.com/market-activity/stock-market-holiday-schedule>
- Cboe U.S. options hours and expiration calendar: <https://www.cboe.com/en/about/hours/us-options/>
- OCC options disclosure document: <https://www.theocc.com/components/docs/riskstoc.pdf>
- SEC Regulation NMS: <https://www.sec.gov/rules-regulations/2005/06/regulation-nms>
- FINRA SR-FINRA-2025-017 intraday margin proposal: <https://www.finra.org/rules-guidance/rule-filings/sr-finra-2025-017>

Source-informed rules:

- Stocks aggregate ranges are Eastern Time oriented and may have empty intervals when no eligible trades occur.
- Options timestamps are UTC and need explicit conversion for market-session interpretation.
- Options data licensing, OPRA sourcing, and quote/trade granularity matter for artifact provenance.
- Broker order lifecycle, bracket order relationships, partial fills, cancellations, and streaming updates must be modeled as evidence rather than inferred from a single REST response.
- Market calendar and early-close behavior should be versioned evidence, not hardcoded folk knowledge.
- Options hours differ by product/session; equity/ETF options and index options cannot share one generic trading-hours rule.
- Alpaca bracket/OCO/OTO behavior must preserve child-leg relationships, partial-fill quantity adjustments, and rejection rules in local-paper previews.
- FINRA day-trading margin rules are changing in 2026; any live-execution risk UI should display broker/account-derived constraints instead of hardcoding historical PDT assumptions.

### Artifacts, Reports, Schemas, Analytics

- SQLite atomic commit: <https://www.sqlite.org/atomiccommit.html>
- SQLite WAL: <https://www.sqlite.org/wal.html>
- Serde attributes: <https://serde.rs/attributes.html>
- Serde skip-serializing behavior: <https://serde.rs/attr-skip-serializing.html>
- rust_xlsxwriter docs: <https://docs.rs/rust_xlsxwriter>
- Apache Arrow columnar and IPC format: <https://arrow.apache.org/docs/format/Columnar.html>
- Apache Arrow JS `tableFromIPC`: <https://arrow.apache.org/js/current/functions/Arrow.dom.tableFromIPC.html>
- Perspective data loading: <https://perspective.finos.org/guide/how_to/javascript/loading_data.html>
- Apache Parquet overview: <https://parquet.apache.org/docs/overview/>
- DuckDB Parquet query docs: <https://duckdb.org/docs/stable/guides/file_formats/query_parquet>

Source-informed rules:

- Report and artifact durability should prefer atomic write/replace patterns and schema-compatible serde defaults.
- XLSX output parity should use sheet/cell invariants, not ZIP byte equality.
- Arrow IPC should be deferred until JSON payload parsing is shown to be a bottleneck.
- Perspective-style artifact exploration should be optional and should not replace the existing production tables.
- Parquet is a durable export/storage candidate for large result sets, while Arrow IPC is a transfer/interchange candidate. Treat them as different contracts.
- DuckDB-style local analytics should be considered only as an optional advanced lab, not a new runtime requirement for standard workflows.

### Security And Software Supply Chain

- NIST SP 800-218 SSDF: <https://csrc.nist.gov/publications/detail/sp/800-218/final>
- NIST Privacy Framework: <https://www.nist.gov/privacy-framework>
- OWASP Secure Coding Practices Quick Reference: <https://owasp.org/www-project-secure-coding-practices-quick-reference-guide/>
- OWASP Software Component Verification Standard: <https://owasp.org/www-project-software-component-verification-standard/>
- SLSA specification: <https://slsa.dev/spec/>
- GitHub Actions artifact retention: <https://docs.github.com/organizations/managing-organization-settings/configuring-the-retention-period-for-github-actions-artifacts-and-logs-in-your-organization>
- GitHub actions/cache: <https://github.com/actions/cache>
- OpenTelemetry Rust: <https://opentelemetry.io/docs/languages/rust/>
- OpenTelemetry semantic conventions: <https://opentelemetry.io/docs/concepts/semantic-conventions/>
- Sentry Tauri v2 note: <https://sentry.zendesk.com/hc/en-us/articles/28526715924251-Does-Sentry-have-a-Tauri-v2-SDK>

Source-informed rules:

- Release expansion should include documented security practices, secret hygiene, provenance, dependency update policy, and artifact retention.
- CI expansion should make evidence artifacts intentional and retention-aware.
- Public release logs must avoid secrets, credentials, account IDs, private local paths, and raw market-data dumps unless explicitly redacted.
- Local diagnostics and support bundles should be dry-run and redaction-first before any network telemetry is considered.
- Sentry currently recommends separate Browser and Rust SDKs for Tauri v2; adoption should wait for explicit privacy policy and opt-in behavior.
- OpenTelemetry Rust signals are useful for local workflow tracing only if the dependency/overhead is justified by profiling needs.

### Strategy Validation Research

- Bailey, Borwein, Lopez de Prado, Zhu, "The Probability of Backtest Overfitting": <https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2326253>
- Bailey and Lopez de Prado, backtest overfitting discussion: <https://academic.oup.com/jrssig/article/18/6/22/7038278>
- Bailey and Lopez de Prado, "The Deflated Sharpe Ratio": <https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2460551>
- Hansen, "A Test for Superior Predictive Ability": <https://papers.ssrn.com/sol3/papers.cfm?abstract_id=264569>
- Sullivan, Timmermann, and White, data-snooping bootstrap for technical trading rules: <https://papers.ssrn.com/sol3/papers.cfm?abstract_id=65140>
- NIST AI Risk Management Framework: <https://www.nist.gov/itl/ai-risk-management-framework>
- Model Cards for Model Reporting: <https://arxiv.org/abs/1810.03993>
- scikit-learn model persistence security: <https://scikit-learn.org/stable/model_persistence.html>

Source-informed rules:

- Expanding optimization surfaces should increase evidence against selection bias, not only add more candidate search.
- Validation plans should make train/holdout, walk-forward, CSCV/PBO, DSR/PSR, and replay class visible where the repo already computes or can safely derive those values.
- Product language must avoid implying profitable trading or production trading readiness from a backtest.
- SPA/Reality Check style tests are candidates for advanced research evidence, but they must be tied to a clearly defined candidate universe.
- ML/model features need local model-card style evidence: dataset, features, split plan, validation status, limitations, and rollback identity.

## Plan Index By Implementation Facet

| Facet | Start file |
|---|---|
| Data, Polygon, market-time, cache | `01_data_polygon_market_data_expansion.md` |
| Optimization, strategies, route policy | `02_optimization_strategy_route_expansion.md` |
| Validation, evidence, GPU validation | `03_validation_evidence_gpu_expansion.md` |
| Options expression | `04_options_expression_expansion.md` |
| Meta, Stage 5, ML Signal | `05_ml_meta_signal_expansion.md` |
| Portfolio and live execution | `06_portfolio_live_execution_expansion.md` |
| Frontend, UX, charts, QA | `07_frontend_workbench_visualization_expansion.md` |
| Artifacts, reports, exports | `08_artifacts_reports_history_expansion.md` |
| Desktop runtime, security, release | `09_desktop_runtime_security_release_expansion.md` |
| CUDA, GPU runtime, performance | `10_gpu_cuda_performance_expansion.md` |
| Verification, CI, quality gates | `11_verification_ci_quality_expansion.md` |
| Sequencing and first batches | `12_prioritized_execution_program.md` |
| Toolchain, manifests, dependencies | `13_manifest_toolchain_dependency_contract.md` |
| Second-pass add-on feature candidates | `14_second_pass_research_feature_candidates.md` |
| Frontend QA script wiring | `15_frontend_qa_gate_wiring_matrix.md` |
| Accessibility, observability, support | `16_accessibility_observability_support_expansion.md` |
