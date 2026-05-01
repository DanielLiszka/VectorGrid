# Data, Polygon, And Market Data Expansion Plan

Date: 2026-04-30

## Goal

Expand market-data reliability and usability without changing the provider model or weakening artifact provenance. The target is better cache identity, market-time correctness, live-feed state clarity, and options data quality gates.

## Current Owner Files

Start here:

- `web/src/stores/dataStore.ts`
- `web/src/services/dataService.ts`
- `web/src/components/DataWorkspacePanel.vue`
- `src-tauri/src/commands_data.rs`
- `src-tauri/src/polygon.rs`
- `src-tauri/src/state.rs`
- `src-tauri/src/data_cache.rs`
- `src-tauri/src/settings.rs`
- `src-tauri/src/commands_options_data.rs`
- `crates/app_core/src/options_data_store.rs`
- `crates/app_core/src/options_types.rs`

Related tests:

- `web/src/services/dataService.test.ts`
- `web/src/stores/dataStore.actions.test.ts`
- options service/store/component tests
- existing Tauri data/settings tests

## Research Basis

- Polygon/Massive stock aggregates are range-based OHLC endpoints, Eastern Time oriented, split-adjusted by default, and may omit bars for empty intervals: <https://polygon.io/docs/rest/stocks/aggregates/custom-bars>
- Polygon/Massive options data is OPRA-sourced; options timestamps are UTC and require explicit ET conversion for market-session display: <https://www.polygon.io/docs/options>
- Polygon option-chain snapshots can include last quote, greeks, implied volatility, and open interest, but greeks can be absent in documented edge cases: <https://polygon.io/docs/rest/options/snapshots/option-chain-snapshot>
- NYSE, Nasdaq, and Cboe publish separate holiday and trading-hours references; do not infer market sessions from one venue when a feature spans equities and options: <https://www.nyse.com/markets/hours-calendars>, <https://www.nasdaq.com/market-activity/stock-market-holiday-schedule>, and <https://www.cboe.com/about/hours/>
- SQLite documents atomic commit behavior; cache manifests and settings should use atomic writes or transactional storage where practical: <https://www.sqlite.org/atomiccommit.html>
- Serde supports default/skip attributes that are useful for additive schema evolution: <https://serde.rs/attributes.html>

## Product Position

Keep Polygon.io as the supported provider label in product copy and repo APIs. The external docs currently redirect to Massive-branded pages; that should not cause accidental rename churn across the app. Existing `massive` aliases should remain compatibility handling only where already canonicalized.

## Expansion Track A: Market-Time Canonicalization

### Problem

Date-only inputs, live timestamps, options UTC timestamps, and report labels can drift when frontend code uses local OS time implicitly.

### Implementation

Add a narrow frontend helper:

- new: `web/src/utils/marketTime.ts`
- new: `web/src/utils/marketTime.test.ts`

Responsibilities:

- parse `YYYY-MM-DD` as a market date, not a local-machine timestamp;
- format live and artifact timestamps with explicit market-time labels;
- normalize inclusive/exclusive end-date labels for data download and validation;
- keep raw UTC ISO values available in diagnostics;
- expose `MARKET_TIME_ZONE = "America/New_York"` for product labels only.

Initial call sites:

- `web/src/services/dataService.ts`
- `web/src/services/optionsExpressionService.ts`
- `web/src/stores/optionsExpressionStore.ts`
- `web/src/utils/optionsValidationForm.ts`
- `web/src/services/executionService.ts`
- `web/src/utils/uiTime.ts` only if there is overlap after local review.

### Tests

- DST spring and fall date-only cases.
- Local OS timezone independence.
- Options validation date-range holdout labels.
- Polygon download summary labels.

### Acceptance Criteria

- Command payload compatibility is preserved.
- Existing options/data tests pass.
- Dates displayed to users identify market time versus UTC where ambiguity matters.

## Expansion Track B: Cache Manifest And Dataset Identity

### Problem

The app has binary cache and derived dataset invalidation, but expansion will need more inspectable cache identity and repair evidence.

### Implementation

Add or extend backend cache manifest logic around:

- `src-tauri/src/data_cache.rs`
- `src-tauri/src/polygon.rs`
- `src-tauri/src/commands_data.rs`
- `src-tauri/src/settings.rs`

Candidate persisted fields:

- provider canonical name: `polygon`;
- source URL identity without API key;
- symbol;
- asset class;
- requested range;
- actual returned min/max timestamp;
- timeframe/multiplier;
- adjusted flag;
- result count;
- gap repair summary;
- duplicate/out-of-order repair summary;
- content hash or chunk hashes;
- app schema version.

Use additive serde defaults for old cache manifests. Do not force migration of existing `.vta` cache files during normal startup.

### Frontend

Add cache inventory display as Advanced-only diagnostics:

- `web/src/components/DataWorkspacePanel.vue`
- `web/src/stores/dataStore.ts`
- `web/src/services/dataService.ts`

Display:

- loaded dataset identity;
- cache freshness;
- repair status;
- derived timeframe count;
- invalidation reason after base data changes.

### Tests

- cache manifest roundtrip with missing new fields;
- cache manifest redacts API keys and private paths;
- derived dataset invalidation remains correct;
- corrupted manifest produces a contained warning, not startup failure.

## Expansion Track C: Polygon Historical Download Hardening

### Problem

Historical download currently caches minute data then resamples upward. Expansion should make provider constraints visible and testable.

### Implementation

In `src-tauri/src/polygon.rs`:

- preserve split-adjusted default semantics unless the request explicitly chooses otherwise;
- record provider recency/plan limitations only as observed response metadata or user configuration, not guessed entitlement labels;
- treat empty intervals as data quality facts, not automatic errors;
- add explicit `provider_alias` handling only at the command boundary if needed.

In `web/src/components/DataWorkspacePanel.vue`:

- show a compact quality summary after download/import;
- offer a repair/rescan action only for loaded datasets;
- avoid provider rename churn.

### Tests

- no eligible-trade interval is represented as a gap;
- adjusted flag is persisted in provenance;
- cache identity changes when adjusted changes;
- sorted ascending output is enforced before app-core use.

## Expansion Track D: Live Feed State And Backpressure

### Problem

Only one live stream slot exists. Live feed status, runner dependency, and interruption behavior need clear state as live features expand.

### Implementation

Current files:

- `src-tauri/src/polygon.rs`
- `src-tauri/src/commands_data.rs`
- `web/src/stores/dataStore.ts`
- `web/src/services/dataService.ts`
- `web/src/services/executionService.ts`
- `web/src/stores/executionStore.ts`

Add:

- explicit feed state enum in frontend and backend responses;
- stale-feed threshold summary;
- active stream symbol/data_id match check display;
- event counters: bars received, duplicates ignored, out-of-order ignored, last error, last reconnect.

Do not add a second active stream until the single-slot lifecycle is fully visible and covered.

### Tests

- feed interruption disarms live runner;
- mismatched symbol/data_id blocks runner start;
- live bar mutation invalidates derived resamples;
- repeated status events do not churn UI excessively.

## Expansion Track E: Options Dataset Quality Gate

### Problem

The options workflow has stronger provenance than standard stock CSV imports. Expansion should preserve that separation and make quote/mid quality explicit.

### Implementation

Current files:

- `src-tauri/src/commands_options_data.rs`
- `src-tauri/src/commands_options_expression.rs`
- `crates/app_core/src/options_data_store.rs`
- `crates/app_core/src/options_types.rs`
- `web/src/components/OptionsExpressionPanel.vue`
- `web/src/services/optionsExpressionService.ts`
- `web/src/stores/optionsExpressionStore.ts`

Add dataset quality fields:

- quote coverage percentage;
- midpoint availability;
- crossed/locked quote count when represented;
- trade-only fallback count;
- UTC timestamp range;
- market-session coverage;
- parent underlying alignment summary.

Do not promote quote-lane expansion into the public stock data flow. Keep options-specific provenance in the options workflow.

### Tests

- quote-quality summary included when imported dataset has quote/mid fields;
- older datasets deserialize without quote summary;
- validation rejects incompatible replacement datasets with clear reason;
- frontend displays quality warnings without changing objective scores.

## Second-Pass Expansion: Dataset Health Contract

### Problem

The first cache-manifest track identifies dataset fields, but implementation needs a stronger health contract that can drive Simple Mode, Advanced data diagnostics, artifact replay, and support bundles without reopening raw market-data files on every refresh.

### Current Anchors

- `web/src/utils/core/datasetMeta.ts`
- `web/src/services/settingsService.ts`
- `web/src/stores/dataStore.ts`
- `web/src/components/DataWorkspacePanel.vue`
- `src-tauri/src/local_path_policy.rs`
- `src-tauri/src/settings.rs`
- `src-tauri/src/data_cache.rs`
- `src-tauri/resources/sample-data/vectorgrid_synthetic_ohlcv.csv`
- `src-tauri/tauri.windows.conf.json`
- `tauri.windows.conf.json`
- `scripts/check_tauri_config_sync.mjs`
- `scripts/generate_release_manifest.ps1`

### Health Fields

Add a derived, additive health summary:

- `dataset_id` and `dataset_hash`;
- `provider_canonical` and `provider_alias_seen`;
- `asset_class` and `venue_scope`;
- `market_calendar_id`;
- `time_zone_policy`;
- `requested_range` and `actual_range`;
- `bar_count`, `gap_count`, `duplicate_count`, and `out_of_order_count`;
- `repair_status`: `none`, `derived`, `manual_review`, `blocked`;
- `cache_manifest_version`;
- `source_path_class`: `bundled_sample`, `user_file`, `provider_download`, `unknown`;
- `privacy_summary`: private path redacted, API key absent, raw contents not included.

### Implementation Phases

Phase 1: backend summary.

- add the fields as optional serde/defaulted data on cache inventory/detail responses;
- never include API keys or full private user paths in returned diagnostics;
- use `local_path_policy.rs` before any path display/open/reveal action;
- keep the current `.vta` cache load path compatible.

Phase 2: frontend display.

- normalize the new fields in `datasetMeta.ts`;
- show compact status in Simple Mode only for the bundled sample and loaded dataset;
- show the full health table in Advanced data diagnostics;
- include an explicit "not scanned" state when a refresh deliberately avoids expensive file reads.

Phase 3: packaged sample and release contract.

- treat `src-tauri/resources/sample-data/vectorgrid_synthetic_ohlcv.csv` as the release sample authority;
- verify Windows Tauri config sync so the sample appears in the staged bundle;
- include sample hash and resource path in the release manifest;
- fail release doctor if the public first-run path depends on a missing sample.

### Acceptance Criteria

- Simple Mode can identify the bundled sample without internet access.
- Advanced Mode can explain dataset health without leaking secrets or private paths.
- Cache inventory refresh is bounded and does not hash every raw data file by default.
- Release manifest proves the bundled sample exists for the intended variant.

## Second-Pass Expansion: Provider Error Taxonomy

### Problem

Provider failures are not all equal. Rate limits, missing entitlement, market closure, empty result windows, malformed symbols, and network failures need different recovery copy and retry policy.

### Implementation

Add an internal taxonomy around `src-tauri/src/polygon.rs` and command response mapping:

- `auth_missing_or_invalid`;
- `entitlement_or_plan_limit`;
- `rate_limited`;
- `market_closed_or_no_session`;
- `empty_valid_window`;
- `symbol_not_found`;
- `provider_schema_changed`;
- `network_transient`;
- `local_cache_corrupt`;
- `unknown_provider_error`.

Frontend behavior:

- retry only transient classes;
- keep entitlement/rate-limit copy factual and provider-neutral;
- preserve raw provider status code in redacted diagnostics;
- do not label a no-trade interval as a data corruption event.

### Tests

- mocked Polygon status/error mapping;
- frontend retry policy table tests;
- report/support-bundle redaction tests for provider diagnostics.

## Non-Goals

- No new provider.
- No live options feed.
- No broker execution triggered by data import.
- No automatic forward-fill that changes strategy semantics unless explicitly represented in dataset provenance.
- No destructive cache migration.

## Verification

Targeted:

- `npm --prefix web run test -- src/services/dataService.test.ts src/stores/dataStore.actions.test.ts src/services/optionsExpressionService.test.ts src/stores/optionsExpressionStore.test.ts src/components/OptionsExpressionPanel.test.ts`
- targeted Rust tests for `polygon`, `data_cache`, `commands_data`, and options data modules as added.

Broader:

- `npm --prefix web run typecheck`
- `npm --prefix web run lint`
- `powershell -File scripts/run_repo_quality_gate.ps1`
