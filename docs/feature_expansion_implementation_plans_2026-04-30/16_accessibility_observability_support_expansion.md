# Accessibility, Observability, And Support Expansion Plan

Date: 2026-04-30

## Goal

Expand supportability and accessibility without leaking sensitive data or adding network telemetry prematurely.

## Current Owner Files

Accessibility/frontend:

- `web/src/components/ui/*`
- `web/src/components/*WorkspacePanel.vue`
- `web/src/components/workbench/*`
- `web/src/components/ActionStatusBar.vue`
- `web/src/components/TaskCenterOverlay.vue`
- `web/src/utils/statusPresentation.ts`
- `web/e2e/accessibility-review-matrix.md`
- `web/scripts/check-a11y-smoke.mjs`

Diagnostics/logging:

- `src-tauri/src/redaction.rs`
- `src-tauri/src/commands_system.rs`
- `src-tauri/src/settings.rs`
- `web/src/utils/redaction.ts`
- `web/src/services/desktopProductization.ts`
- `web/src/services/settingsService.ts`
- `web/src/stores/settingsStore.ts`
- `scripts/check_release_secret_hygiene.mjs`

## Research Basis

- WCAG 2.2: <https://www.w3.org/TR/WCAG22/>
- WAI-ARIA APG patterns: <https://www.w3.org/WAI/ARIA/apg/patterns/>
- Vue accessibility: <https://vuejs.org/guide/best-practices/accessibility.html>
- ECharts ARIA: <https://echarts.apache.org/handbook/en/best-practices/aria/>
- Tauri logging plugin: <https://v2.tauri.app/plugin/logging/>
- OpenTelemetry Rust: <https://opentelemetry.io/docs/languages/rust/>
- Sentry Tauri v2 note: <https://sentry.zendesk.com/hc/en-us/articles/28526715924251-Does-Sentry-have-a-Tauri-v2-SDK>
- NIST Privacy Framework: <https://www.nist.gov/privacy-framework>

## Expansion Track A: Dense Table Accessibility Policy

### Problem

Virtualized result tables are necessary for performance, but virtualization can hide rows from assistive technologies.

### Implementation

For each dense table surface, record:

- native table vs ARIA grid decision;
- keyboard navigation behavior;
- row count announcement;
- selected row announcement;
- export/copy fallback;
- screen-reader summary.

Initial surfaces:

- Top results;
- Validation results;
- Meta results;
- History artifacts;
- Options tuples;
- Portfolio candidates.

### Rules

- Use semantic table behavior for read-only results where possible.
- Use ARIA grid only if arrow-key, focus, selection, and active-descendant behavior are implemented and tested.
- Provide export/copy summary for screen-reader users who need non-virtualized review.

## Expansion Track B: Chart Accessibility Layer

### Problem

Canvas/SVG chart surfaces can be visually useful but inaccessible without text equivalents.

### Implementation

For each chart:

- concise title;
- source dataset;
- selected metric;
- top insight summary;
- marker/line legend;
- table/export equivalent;
- color-independent status labels.

ECharts:

- enable ARIA component only with curated descriptions;
- avoid generic auto descriptions for financial conclusions.

Lightweight Charts:

- add text summary and marker legend next to chart;
- ensure trade/order markers are represented outside canvas.

## Expansion Track C: Live Region Policy

### Problem

Tasks, feeds, and execution status change frequently. Screen-reader announcements can become noisy.

### Implementation

Add policy:

- announce terminal task states;
- announce feed disconnected/stale/recovered;
- announce execution emergency/failure states;
- do not announce every progress tick;
- keep error messages in an accessible status region.

Current owners:

- `web/src/components/ActionStatusBar.vue`
- `web/src/components/TaskCenterOverlay.vue`
- `web/src/stores/taskStore.ts`
- `web/src/stores/executionStore.ts`
- `web/src/utils/statusPresentation.ts`

## Expansion Track D: Local Diagnostics Summary

### Problem

Users and maintainers need support evidence without uploading private data.

### Implementation

Add local-first diagnostics:

- app version;
- git/build/release provenance where available;
- OS/WebView2/Tauri version;
- GPU status;
- enabled feature flags;
- data directory paths redacted or user-approved;
- recent task failures;
- command boundary status;
- release variant/resource status.

Potential new command:

- `get_local_diagnostics_summary`

Potential frontend:

- Settings diagnostics panel;
- copy redacted summary;
- save redacted summary to user-selected path.

## Expansion Track E: Redacted Support Bundle Dry Run

### Problem

Support bundles can help debugging but can leak secrets, account/order IDs, local paths, strategy parameters, and market data.

### Implementation

Potential new files:

- `src-tauri/src/support_bundle.rs`
- `web/src/services/supportBundleService.ts`

Behavior:

- dry-run default;
- show included file classes and redaction summary;
- require explicit user destination;
- exclude raw market data by default;
- exclude API keys, signing keys, account IDs, bearer tokens, and unredacted local paths;
- include only bounded log tail by default.

Verification:

- redaction unit tests;
- release secret hygiene check;
- Settings component tests.

## Expansion Track F: Observability Decision Point

### Local First

Preferred initial path:

- structured local logs;
- redacted diagnostics summary;
- local workflow timing summary;
- optional local trace export.

### OpenTelemetry

Consider only if:

- local trace spans materially improve workflow debugging;
- overhead is measured;
- no network exporter is enabled by default;
- schema and redaction policy are documented.

### Sentry

Defer until:

- public release privacy policy is explicit;
- telemetry is opt-in;
- Browser SDK and Rust SDK correlation is designed;
- sensitive payload scrubbing is tested.

Sentry currently states there is no dedicated Tauri v2 SDK; recommended setup is Browser SDK and Rust SDK separately.

## Non-Goals

- No silent telemetry.
- No uploading support bundles from the app in the first implementation.
- No raw market-data or broker-order payloads in support bundles.
- No ARIA grid without full keyboard implementation.

## Verification

Targeted:

- `npm --prefix web run a11y:smoke:built`
- `npm --prefix web run a11y:smoke:wcag:built`
- redaction tests;
- Settings diagnostics tests.

Broad:

- `npm --prefix web run check:qa-static`
- `npm --prefix web run test:e2e:qa:built`
- `powershell -File scripts/run_repo_quality_gate.ps1`

