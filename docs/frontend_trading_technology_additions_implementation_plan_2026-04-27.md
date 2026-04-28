# Frontend Trading Technology Additions Implementation Plan

Date: 2026-04-27

Status: implementation in progress; the low-risk visual analytics slice is implemented and validated

## Goal

Add only frontend/npm technology that provides a concrete product benefit for VectorGrid's existing workflows:

- live trading and broker-control safety
- live feed and event-status backpressure
- backtest candidate inspection
- validation and options-validation review
- large artifact exploration
- market-date and session-time correctness

This plan is intentionally additive. It does not replace the current Vue 3, Pinia, Tauri command, Rust optimization, Rust validation, or Polygon.io-only data-provider architecture.

## Current Stack Baseline

The application already uses a strong frontend stack:

- Vue 3, Pinia, Vite, strict TypeScript
- `@pinia/colada` for idempotent read/cache surfaces
- `@tanstack/vue-form` for the options-validation form/request model
- `@tanstack/vue-table` and `@tanstack/vue-virtual` for dense table surfaces
- XState for the live-execution lifecycle model
- `lightweight-charts` for trading charts
- `zod` for selected runtime-boundary validation
- `ts-pattern` for typed branching in selected model logic
- `@floating-ui/vue`, `reka-ui`, `@vueuse/core`, `fuse.js`, and `lucide-vue-next`
- Playwright browser QA, WDIO desktop smoke tests, a11y checks, bundle budgets, UI performance budgets, generated Tauri command checks, and Specta binding checks

The high-ROI additions therefore need to fill real gaps rather than duplicate state, routing, tables, or query caching.

## Research Basis

Research reviewed on 2026-04-27:

- TanStack Pacer is a timing-control library for debouncing, throttling, rate limiting, queuing, batching, async retry, abort support, and type-safe execution control. Its docs currently mark it as beta and list React, Preact, Angular, and Solid adapters, but no Vue adapter. Use the framework-agnostic core package in isolated services rather than component hooks.
  - Source: <https://tanstack.com/pacer/latest/docs>
  - Source: <https://tanstack.com/pacer/latest/docs/installation>
  - Source: <https://tanstack.com/pacer/latest/docs/guides/rate-limiting>
  - Source: <https://tanstack.com/pacer/latest/docs/guides/async-rate-limiting>
- Lightweight Charts already supports the charting foundation in this app. Its plugin model supports custom series and drawing primitives, with drawing primitives being the better fit for most annotations. This favors expanding existing chart integrations rather than adding a second charting library.
  - Source: <https://tradingview.github.io/lightweight-charts/docs/4.1/plugins/intro>
  - Source: <https://tradingview.github.io/lightweight-charts/docs/plugins/series-primitives>
- `@date-fns/tz` provides small IANA-time-zone-aware `Date` helpers and explicit time-zone calculations. `@js-temporal/polyfill` is more expressive, but its package is larger and still a polyfill, so it should be deferred unless market-date logic becomes complex enough to justify it.
  - Source: <https://www.npmjs.com/package/%40date-fns/tz>
  - Source: <https://www.npmjs.com/package/%40js-temporal/polyfill>
- Perspective is a WebAssembly-backed interactive analytics and visualization component for large or streaming tabular data. Its browser install uses `@finos/perspective` plus `@finos/perspective-viewer`, with plugins such as `@finos/perspective-viewer-datagrid`. It can load JSON, CSV, and Apache Arrow data.
  - Source: <https://perspective.finos.org/guide/perspective.html>
  - Source: <https://perspective.finos.org/guide/how_to/javascript/installation.html>
  - Source: <https://perspective.finos.org/guide/explanation/javascript_module_structure.html>
  - Source: <https://perspective.finos.org/guide/how_to/javascript/loading_data.html>
- Apache Arrow JavaScript can read IPC data with `tableFromIPC`, but Arrow is only useful here after backend profiling shows JSON transfer or large tabular payload shaping is a bottleneck.
  - Source: <https://arrow.apache.org/js/>
  - Source: <https://arrow.apache.org/js/current/functions/Arrow.dom.tableFromIPC.html>
- Sentry's current Tauri guidance says there is no dedicated Tauri v2 SDK; the recommended approach is Browser SDK plus Rust SDK separately. This is release/telemetry scope, not a core trading feature.
  - Source: <https://sentry.zendesk.com/hc/en-us/articles/28526715924251-Does-Sentry-have-a-Tauri-v2-SDK>

## Non-Goals

- Do not add CI/CD.
- Do not add another data provider. Polygon.io remains the sole market/options data provider.
- Do not replace Pinia with Redux, Zustand, TanStack Store, or another app-state system.
- Do not replace `@pinia/colada` with TanStack Query.
- Do not replace existing tables with AG Grid, Handsontable, or a licensed enterprise table stack.
- Do not replace `lightweight-charts` with ECharts or another general charting library for current trading charts.
- Do not add Comlink unless a fresh profiling run identifies meaningful browser-main-thread compute that cannot be moved to Rust/Tauri.
- Do not expose broad frontend SQL access. Rust-owned catalog/query commands remain the app boundary.
- Do not pace, delay, drop, or queue safety-critical live-trading actions such as emergency halt, flatten, or cancel-open-orders.

## Package Decision Matrix

| Candidate | Decision | Benefit | Risk Control |
| --- | --- | --- | --- |
| `@tanstack/pacer` core | Implement first | Backpressure for live feed refreshes, broker review refreshes, repeated UI commands, and bursty status/log updates. | Use core package only, pin version, isolate behind service wrappers, avoid safety-critical actions. |
| Lightweight Charts primitives and built-in annotation APIs | Implement first, no new package | Better live order markers, position/risk lines, validation window overlays, and candidate trade review. | Extend existing chart runtime and tests; no global chart replacement. |
| `@date-fns/tz` | Implement narrowly | Explicit market time-zone handling for Polygon ranges, options-validation holdout dates, and live timestamp labels. | Use only in `marketTime` helpers; no broad date rewrite. |
| `@js-temporal/polyfill` | Defer | Stronger date/time domain model if market calendar logic grows. | Larger package and future native overlap; use only if `@date-fns/tz` is insufficient. |
| `@finos/perspective`, `@finos/perspective-viewer`, `@finos/perspective-viewer-datagrid` | Optional gated pilot | Interactive exploration of large optimization, validation, options-validation, and artifact rows. | Lazy import, feature flag, datagrid-only pilot, no replacement of production tables. |
| `apache-arrow` JavaScript | Defer until backend payload profiling | Faster large tabular transfer if Rust emits Arrow IPC. | Requires Rust/frontend contract work; not worth adding for current JSON payloads without evidence. |
| Sentry Browser SDK plus Rust SDK | Defer to release/telemetry decision | Production diagnostics after external distribution. | No dedicated Tauri v2 SDK; privacy and config needed before adoption. |
| RxJS | Do not add now | Could model streams, but overlaps with Pinia, XState, task throttling, and Pacer. | Revisit only if live-stream composition becomes much more complex. |
| ECharts | Implemented narrowly for non-trading analytics | Adds a compact Pareto metric-distribution histogram that Lightweight Charts is not responsible for. | Lazy import only, direct side-effect module imports instead of barrels, no trading-chart replacement, bundle-budget gate must remain green. |

## Implemented Visual Analytics Addendum

The later visual-technology pass changed the ECharts decision from "defer" to a narrow, additive implementation:

- `echarts@6.0.0` is installed for a single non-trading analytics surface.
- The Pareto workspace now shows a lazy-loaded metric-distribution histogram below the existing Pareto scatter.
- Top results and validation rows keep the existing TanStack table/virtualization stack, but now decorate selected numeric/status cells with compact bars and badges.
- Existing payload shape, sorting text, column persistence, row virtualization, and table controls are unchanged.
- ECharts is not used for price/trading charts; `lightweight-charts` remains the trading-chart technology.
- ECharts barrel imports were avoided after the first build exceeded the JS budget; the final implementation imports only core, canvas renderer, bar chart, and grid modules.

Validation completed for this visual slice:

- `npm --prefix web run typecheck`
- `npm --prefix web run test`
- `npm --prefix web run build`
- `npm --prefix web run check:bundle-budget` (`total js: 2794.7 kB`, under the 2800 kB cap)
- `npm --prefix web run lint`
- `npm --prefix web run test:e2e:qa:built`
- `npm --prefix web run a11y:smoke:built`
- `npm --prefix web run check:qa-static`
- `npm --prefix web run perf:ui:budget`

## Implementation Principles

1. Keep Rust/Tauri as the execution owner.
   - Optimization, validation, live execution, filesystem writes, and provider IO remain backend-owned.
   - Frontend packages should improve control, rendering, exploration, or validation of frontend state.

2. Use adapters before component usage.
   - New packages should first appear in small services, composables, or visualization runtimes.
   - Components should consume app-specific helpers, not package-specific abstractions.

3. Protect live-trading safety actions.
   - Emergency halt, flatten position, and cancel open orders must bypass rate limits, throttles, and queues.
   - Pacing is appropriate for refresh/status/review/event surfaces, not for urgent trading controls.

4. Prefer lazy and feature-flagged loading for heavy additions.
   - Perspective and any future Arrow code should be lazy chunks.
   - Gated pilots should have flag-off regression coverage.

5. Keep package adoption measurable.
   - Every package addition must have a bundle-budget check, focused tests, and a concrete before/after product behavior.

6. Do not migrate working surfaces wholesale.
   - Current tables, charts, stores, and E2E selectors should continue to work.
   - Add new capabilities around stable IDs and existing services.

## Phase 0: Baseline And Guardrails

### Scope

Prepare the package additions with explicit baselines so follow-on implementation can be judged objectively.

### Tasks

1. Record dependency baseline:
   - `npm --prefix web ls @tanstack/pacer @date-fns/tz @finos/perspective @finos/perspective-viewer @finos/perspective-viewer-datagrid apache-arrow --depth=0`
   - `npm --prefix web run check:bundle-budget`
   - `npm --prefix web run perf:ui:budget`

2. Confirm current event and live-control integration points:
   - [web/src/services/executionService.ts](../web/src/services/executionService.ts)
   - [web/src/stores/executionStore.ts](../web/src/stores/executionStore.ts)
   - [web/src/stores/taskStore.ts](../web/src/stores/taskStore.ts)
   - [web/src/tauri/instrumentation.ts](../web/src/tauri/instrumentation.ts)
   - [web/src/components/ExecutionWorkspacePanel.vue](../web/src/components/ExecutionWorkspacePanel.vue)

3. Confirm current chart integration points:
   - [web/src/services/executionChartService.ts](../web/src/services/executionChartService.ts)
   - [web/src/visualizations/lightweightChartRuntime.ts](../web/src/visualizations/lightweightChartRuntime.ts)
   - [web/src/composables/useDrillPriceChartRuntime.ts](../web/src/composables/useDrillPriceChartRuntime.ts)
   - [web/src/visualizations/drillChartModel.ts](../web/src/visualizations/drillChartModel.ts)

4. Confirm date/time risk surfaces:
   - [web/src/components/OptionsExpressionPanel.vue](../web/src/components/OptionsExpressionPanel.vue)
   - [web/src/stores/optionsExpressionStore.ts](../web/src/stores/optionsExpressionStore.ts)
   - [web/src/utils/optionsValidationForm.ts](../web/src/utils/optionsValidationForm.ts)
   - [web/src/services/optionsExpressionService.ts](../web/src/services/optionsExpressionService.ts)
   - [web/src/services/dataService.ts](../web/src/services/dataService.ts)

5. Record current validation gates that must still pass after each phase:
   - `npm --prefix web run lint`
   - `npm --prefix web run typecheck`
   - `npm --prefix web run test`
   - `npm --prefix web run build`
   - `npm --prefix web run check:bundle-budget`
   - `npm --prefix web run perf:ui:budget`
   - `npm --prefix web run test:e2e:qa:built`
   - `npm --prefix web run test:e2e:workflow:full:built`
   - `npm --prefix web run a11y:smoke:built`
   - `npm --prefix web run typecheck:desktop`

### Acceptance Criteria

- No package is added in Phase 0.
- Baseline commands are recorded in the implementation summary.
- Existing dirty worktree changes are preserved.

## Phase 1: Pacer Service For Live And Control-Plane Backpressure

### Scope

Add `@tanstack/pacer` core only. Do not use a framework adapter because current official docs do not list a Vue adapter.

### Files

- [web/package.json](../web/package.json)
- [web/package-lock.json](../web/package-lock.json)
- `web/src/services/frontendPacing.ts`
- `web/src/services/frontendPacing.test.ts`
- [web/src/services/executionService.ts](../web/src/services/executionService.ts)
- [web/src/stores/executionStore.ts](../web/src/stores/executionStore.ts)
- [web/src/stores/taskStore.ts](../web/src/stores/taskStore.ts)
- [web/src/components/ExecutionWorkspacePanel.vue](../web/src/components/ExecutionWorkspacePanel.vue)

### Design

Create an app-specific pacing service with these primitives:

- `createIdempotentRefreshLimiter`
  - For status and broker-review refreshes.
  - Uses async rate limiting or async throttling.
  - Returns either the original result or a typed pacing rejection result that callers can display.

- `createBurstCoalescer`
  - For high-frequency live/event status text updates.
  - Coalesces repeated status churn without hiding terminal errors.
  - Keeps the newest event payload, not the oldest.

- `createSequentialRefreshQueue`
  - For operations where dropping is wrong but overlap is wasteful.
  - Concurrency must be `1`.
  - Use only for idempotent reads or explicit refresh commands.

- `isSafetyCriticalExecutionAction`
  - Hard-coded guard that prevents emergency halt, flatten, cancel-open-orders, and stop-runner safety paths from using pacing wrappers.

### Candidate Call Sites

Good candidates:

- `live_execution_runner_status`
- `polygon_live_candle_stream_status`
- `get_live_execution_snapshot`
- broker review refresh actions
- live feed status refresh actions
- repeated non-critical "refresh session artifacts" style actions
- task/event status ingestion where current throttling is manually duplicated

Do not pace:

- `emergency_halt_live_execution`
- `flatten_live_execution_position`
- `cancel_live_execution_open_orders`
- `stop_live_execution_runner`
- any future real order placement command
- any command where dropping the call changes trading state or hides a user's explicit safety intent

### UI Behavior

- Repeated refresh clicks should not produce overlapping Tauri calls.
- The user should see a clear status when a refresh is already in flight or temporarily limited.
- Emergency controls should remain immediately available.
- Existing disabled states and XState lifecycle behavior remain authoritative.

### Tests

Add focused unit coverage:

- pacing wrapper returns the underlying async result
- overlapping idempotent refreshes are coalesced or rejected according to the selected primitive
- safety-critical action names are never paceable
- failures are not swallowed
- fake timers prove rate/throttle windows

Add component/store coverage:

- repeated refresh button clicks do not call the command more than expected
- emergency halt bypasses pacing
- live workspace status text reports pacing state without blocking urgent actions

### Regression Risks

- Pacer is beta, so direct imports should stay limited to `frontendPacing.ts`.
- Rate limiting is lossy; only apply it where dropping or rejecting a repeated call is acceptable.
- Queuing can make stale results arrive late; use "latest wins" when displaying status.
- Do not couple Pacer state to Pinia global state except for user-facing status summaries.

### Acceptance Criteria

- `@tanstack/pacer` is installed and pinned.
- No component imports Pacer directly.
- Safety-critical live actions bypass all pacing wrappers.
- Existing E2E selectors and command names remain unchanged.
- Full frontend verification passes.

## Phase 2: Trading Chart Annotation Upgrade

### Scope

Use the existing `lightweight-charts` dependency more deeply. Do not add a new charting library.

### Files

- [web/src/visualizations/lightweightChartRuntime.ts](../web/src/visualizations/lightweightChartRuntime.ts)
- [web/src/services/executionChartService.ts](../web/src/services/executionChartService.ts)
- [web/src/composables/useDrillPriceChartRuntime.ts](../web/src/composables/useDrillPriceChartRuntime.ts)
- [web/src/visualizations/drillChartModel.ts](../web/src/visualizations/drillChartModel.ts)
- [web/src/components/ExecutionWorkspacePanel.vue](../web/src/components/ExecutionWorkspacePanel.vue)
- [web/src/components/CandidateDetailWorkspacePanel.vue](../web/src/components/CandidateDetailWorkspacePanel.vue)
- [web/src/components/visualizations/PriceChartPanel.vue](../web/src/components/visualizations/PriceChartPanel.vue)

### Live Trading Enhancements

Add chart affordances that directly improve live review:

- broker-confirmed order markers
  - submitted
  - accepted
  - partial fill
  - filled
  - canceled
  - rejected

- position/risk price lines
  - average entry
  - current stop
  - take-profit target
  - trailing stop if present in current snapshot

- feed freshness indicators
  - chart metadata showing delayed or stale feed state
  - non-intrusive overlay only when the latest bar is stale relative to the expected cadence

- chart marker legend
  - compact legend close to the chart metadata
  - accessible text equivalent in the existing status region

### Backtest And Validation Enhancements

Add candidate-review improvements:

- selected trade marker emphasis
- entry/exit marker consistency across candidate detail and live chart surfaces
- validation split/window overlay where the result payload exposes train/holdout boundaries
- rejected-trade or filtered-candidate annotations only if the underlying result payload already exposes enough provenance

Do not invent chart data that is not in the backend payload.

### Technical Approach

- Prefer built-in series markers and price lines first.
- Use drawing primitives only where built-in APIs cannot express the annotation.
- Keep the chart runtime lazy-loaded through the existing dynamic import pattern.
- Keep marker model construction pure and unit-tested.
- Keep all chart object calls guarded for test mocks and browser/desktop differences.

### Tests

Add or extend:

- `web/src/visualizations/lightweightChartRuntime.test.ts`
- `web/src/services/executionChartService.test.ts`
- `web/src/visualizations/drillChartModel.test.ts`
- `web/src/components/ExecutionWorkspacePanel.test.ts`
- `web/src/components/CandidateDetailWorkspacePanel.test.ts`
- Playwright workflow assertion for live chart marker legend when mock data includes broker markers

### Regression Risks

- Chart rendering is imperative and runtime-object based; tests should mock chart APIs closely.
- Too many markers can reduce readability. Cap labels or collapse low-value markers where needed.
- Price lines must not imply active broker protection unless the snapshot explicitly confirms them.

### Acceptance Criteria

- No new charting package is added.
- Existing chart host sizes and lazy loading still work.
- Candidate detail and live charts expose clearer trade/order context.
- Bundle size does not materially increase.

## Phase 3: Market Time-Zone Helper

### Scope

Add `@date-fns/tz` narrowly for explicit market-time handling. Keep all backend contracts unchanged.

### Files

- [web/package.json](../web/package.json)
- [web/package-lock.json](../web/package-lock.json)
- `web/src/utils/marketTime.ts`
- `web/src/utils/marketTime.test.ts`
- [web/src/utils/optionsValidationForm.ts](../web/src/utils/optionsValidationForm.ts)
- [web/src/stores/optionsExpressionStore.ts](../web/src/stores/optionsExpressionStore.ts)
- [web/src/services/optionsExpressionService.ts](../web/src/services/optionsExpressionService.ts)
- [web/src/services/dataService.ts](../web/src/services/dataService.ts)
- [web/src/services/executionService.ts](../web/src/services/executionService.ts)

### Design

Create one helper module that owns frontend market-date normalization:

- `MARKET_TIME_ZONE = "America/New_York"`
- `parsePolygonDateInput`
  - Accepts `YYYY-MM-DD`.
  - Interprets it as a market date, not as the user's local-machine date.
  - Produces stable ISO strings for Tauri command payloads.

- `normalizeMarketDateRange`
  - Enforces start <= end.
  - Defines inclusive end-date behavior explicitly for date-range holdout requests.
  - Preserves current request shape where possible.

- `formatMarketTimestamp`
  - Formats live feed and artifact timestamps for UI display with an explicit market-time label.
  - Keeps raw ISO values available in diagnostics.

- `describePolygonRange`
  - Produces user-facing labels for Polygon minute-aggregate downloads and options validation ranges.

### Initial Use Cases

- options-expression provider download start/end inputs
- options subsequent-validation date-range holdout
- options artifact validation provenance display
- live feed timestamp labels
- data import/download summaries where Polygon ranges are shown

### Tests

Add DST and boundary tests:

- spring DST transition does not shift a market date
- fall DST transition does not duplicate or skip a date label
- date-only input is interpreted consistently regardless of local OS time zone
- invalid dates are rejected with existing user-facing validation style
- inclusive end-date semantics are stable

### Regression Risks

- Existing code uses `Date` and `toISOString` in several places. Do not replace every date call.
- Tauri commands may already expect UTC ISO strings. Preserve command payload compatibility.
- Options validation has provenance guards; date normalization must not weaken them.

### Acceptance Criteria

- Market-date behavior is centralized.
- Polygon.io remains the only provider label and command path.
- Existing options validation tests still pass.
- New tests prove market-date stability around DST.

## Phase 4: Perspective Artifact Analytics Lab

### Scope

Add an optional, feature-flagged analytics lab for large result/artifact exploration. This should not replace production tables.

### Decision

Use the current FINOS package names documented by Perspective:

- `@finos/perspective`
- `@finos/perspective-viewer`
- `@finos/perspective-viewer-datagrid`

Do not add `@finos/perspective-viewer-d3fc` initially. Start with datagrid-only to limit bundle and UI scope.

### Files

- [web/package.json](../web/package.json)
- [web/package-lock.json](../web/package-lock.json)
- `web/src/services/artifactAnalyticsRows.ts`
- `web/src/services/artifactAnalyticsRows.test.ts`
- `web/src/components/ArtifactAnalyticsLab.vue`
- `web/src/components/ArtifactAnalyticsLab.test.ts`
- [web/src/components/HistoryWorkspacePanel.vue](../web/src/components/HistoryWorkspacePanel.vue)
- [web/src/components/SettingsWorkspacePanel.vue](../web/src/components/SettingsWorkspacePanel.vue)
- [web/e2e/browser/qa/actionCoverageManifest.ts](../web/e2e/browser/qa/actionCoverageManifest.ts)

### Feature Flag

Use a flag such as:

- `VITE_VECTORGRID_ENABLE_PERSPECTIVE_LAB=true`

Flag-off behavior:

- no new UI entry point
- no Perspective imports
- no chunk loaded
- all existing History/Top/Validation/Options tables remain unchanged

Flag-on behavior:

- Settings exposes an "Artifact analytics lab" diagnostic toggle or link.
- History exposes an "Open analytics lab" action when saved artifacts exist.
- The lab loads a flattened row model from the selected artifact or current result set.
- The lab uses Perspective datagrid for ad hoc grouping, filtering, sorting, and export-oriented inspection.

### Initial Data Model

Start from JSON rows already present in the frontend:

- optimization Top rows
- validation result rows
- options-expression tuple rows
- options validation frontier rows
- saved artifact metadata rows

Do not add Arrow IPC in this phase. Perspective can load JSON/column data first, which keeps the backend untouched.

### UX Constraints

- Treat the lab as an advanced analysis surface, not the default workflow.
- Keep normal production tables as the stable guided path.
- Avoid changing existing saved-run selection or validation flows.
- Keep the lab visually integrated with the workbench shell.
- Provide clear empty states when the selected artifact cannot be flattened.

### Tests

Flag-off:

- no lab action is rendered
- no Perspective package is imported in normal workflow
- existing History/Top/Validation behavior is unchanged

Flag-on:

- row flattener creates stable rows from representative optimization, validation, and options artifacts
- component lazy-loads Perspective modules
- failed Perspective import shows a contained error state
- E2E smoke opens the lab from a fixture artifact and verifies the datagrid host renders

### Regression Risks

- Perspective is WASM and can affect bundle/build behavior.
- Custom elements and CSS themes can leak styles.
- Large local datasets can increase memory usage.
- Test environments may need custom-element and dynamic-import guards.

### Acceptance Criteria

- The feature is fully off by default.
- Bundle budget remains acceptable with the default flag.
- The lazy chunk is measurable and documented.
- Production workflow tests pass both before and after the lab addition.

## Phase 5: Arrow IPC Bridge Decision Point

### Scope

Do not implement Arrow in the first pass. Add it only if profiling shows frontend JSON payload costs are material.

### Trigger Conditions

Consider `apache-arrow` JavaScript only if at least one of these becomes true:

- large optimization or validation artifacts spend meaningful time parsing JSON in the browser
- Perspective lab usage is valuable but row loading is slow with JSON
- Rust can emit Arrow IPC for a stable row schema without disrupting existing JSON commands
- UI performance budget or profiling identifies payload deserialization as a bottleneck

### Potential Files

- backend row-to-Arrow encoder in `src-tauri` or `crates/app_core`
- `web/src/services/artifactArrowReader.ts`
- `web/src/services/artifactArrowReader.test.ts`
- Perspective lab row-loading adapter

### Acceptance Criteria For Future Adoption

- JSON remains the compatibility path.
- Arrow is an optional faster path, not the only path.
- The row schema is versioned.
- Parser failures fall back to JSON where possible.

## Phase 6: Release Telemetry Decision Point

### Scope

Do not implement Sentry as part of trading feature work. Keep it as a future release-readiness decision.

### Decision Criteria

Revisit Browser SDK plus Rust SDK only when:

- the app is distributed beyond local/internal users
- privacy and market-data redaction requirements are written down
- DSN, environment, release naming, and opt-in behavior are defined
- Rust and browser error reports can be correlated without sending sensitive strategy or account data

### Non-Goals

- Do not add telemetry silently.
- Do not add community Tauri telemetry plugins without review.
- Do not report broker/order payloads, API keys, strategy parameters, or raw market data.

## Verification Matrix

Run the smallest useful lane after each slice, then the broader lane before handoff.

### Phase 1 Pacer

- `npm --prefix web run test -- src/services/frontendPacing.test.ts src/services/executionService.test.ts src/stores/executionStore.test.ts src/stores/taskStoreEventThrottle.test.ts src/components/ExecutionWorkspacePanel.test.ts`
- `npm --prefix web run typecheck`
- `npm --prefix web run lint`
- `npm --prefix web run test:e2e:qa:built`

### Phase 2 Charts

- `npm --prefix web run test -- src/visualizations/lightweightChartRuntime.test.ts src/services/executionChartService.test.ts src/visualizations/drillChartModel.test.ts src/components/ExecutionWorkspacePanel.test.ts src/components/CandidateDetailWorkspacePanel.test.ts`
- `npm --prefix web run test:e2e:workflow:full:built`
- `npm --prefix web run a11y:smoke:built`

### Phase 3 Market Time

- `npm --prefix web run test -- src/utils/marketTime.test.ts src/utils/optionsValidationForm.test.ts src/stores/optionsExpressionStore.test.ts src/services/optionsExpressionService.test.ts src/components/OptionsExpressionPanel.test.ts`
- `npm --prefix web run typecheck`
- `npm --prefix web run test:e2e:workflow:full:built`

### Phase 4 Perspective Lab

- `npm --prefix web run test -- src/services/artifactAnalyticsRows.test.ts src/components/ArtifactAnalyticsLab.test.ts src/components/HistoryWorkspacePanel.test.ts src/components/SettingsWorkspacePanel.test.ts`
- `npm --prefix web run build`
- `npm --prefix web run check:bundle-budget`
- `npm --prefix web run perf:ui:budget`
- flag-off `npm --prefix web run test:e2e:qa:built`
- flag-on focused Playwright smoke for the lab

### Full Handoff Gate

- `npm --prefix web run lint`
- `npm --prefix web run typecheck`
- `npm --prefix web run typecheck:desktop`
- `npm --prefix web run test`
- `npm --prefix web run build`
- `npm --prefix web run check:bundle-budget`
- `npm --prefix web run perf:ui:budget`
- `npm --prefix web run check:qa-static`
- `npm --prefix web run test:e2e:workflow:full:built`
- `npm --prefix web run test:e2e:qa:built`
- `npm --prefix web run a11y:smoke:built`

## Recommended Implementation Order

1. Phase 1 Pacer service.
   - Highest immediate value for live/control-plane correctness.
   - Smallest package and UI blast radius.

2. Phase 2 Lightweight Charts annotation upgrade.
   - No new package.
   - Directly improves live trading review and candidate/backtest inspection.

3. Phase 3 market time-zone helper.
   - Small package.
   - Reduces subtle options-validation and Polygon date-range risk.

4. Phase 4 Perspective lab.
   - Useful for advanced artifact exploration, but heavier and best kept behind a feature flag.

5. Phase 5 Arrow IPC bridge only after profiling.

6. Phase 6 Sentry only after release telemetry policy exists.

## Success Criteria

The implementation should be considered successful only if:

- live refresh/control behavior is less bursty without weakening emergency controls
- chart surfaces make broker/order/trade state easier to inspect
- market-date handling is explicit and tested around DST
- optional heavy analytics are off by default and lazy when enabled
- bundle and UI performance budgets stay green
- current Playwright and WDIO coverage does not need broad selector churn
- no backend data-provider scope expands beyond Polygon.io
