# Artifact Analytics Lab

Date: 2026-05-06
Status: researched implementation plan
Risk: medium
Primary value: let advanced users explore large saved results without destabilizing production tables

## Summary

Artifact Analytics Lab should be an Advanced, feature-flagged workspace for exploring saved optimization, validation, campaign, options, ML/meta, and portfolio artifacts as analytical row sets. The normal workbench cannot reasonably answer every exploratory question, and saved artifacts already contain far more information than the primary UI should show by default.

The correct first slice is a flattened JSON row model and virtualized table, not Apache Arrow or Perspective. Arrow IPC, Perspective, DuckDB, and other analytical engines are valuable later paths, but they should follow profiling. This keeps bundle risk low and preserves current artifact compatibility while creating a clear upgrade path when JSON parsing or table interactivity becomes the bottleneck.

## External Research Basis

- Apache Arrow columnar format: https://arrow.apache.org/docs/format/Columnar.html
  - Arrow is language-agnostic, columnar, vectorization-friendly, and built for analytical locality. It is a good future transfer/interchange format for large row sets.
- Apache Arrow IPC format: https://arrow.apache.org/docs/format/IPC.html
  - IPC is the relevant Arrow serialization family if the app later needs a binary transfer path.
- Apache Parquet documentation: https://parquet.apache.org/docs/
  - Parquet is a column-oriented file format for efficient storage and retrieval. It is a durable export/storage candidate, distinct from Arrow IPC's transfer/interchange role.
- Perspective overview: https://perspective.finos.org/guide/perspective.html
  - Perspective is designed for interactive analytics and large/streaming datasets. It is a good optional lab viewer, not a production-table replacement.
- Perspective loading data: https://perspective-dev.github.io/guide/explanation/table/loading_data.html
  - Perspective accepts row-oriented JSON, column-oriented JSON, NDJSON, CSV, and Arrow IPC. That supports a phased JSON-first path.
- Perspective save/restore: https://perspective.finos.org/guide/how_to/javascript/save_restore.html
  - Saved viewer state maps to "local perspectives" once the feature is stable.
- TanStack Virtual Vue docs: https://tanstack.com/virtual/v3/docs/framework/vue
  - The repo already uses Vue and virtualized tables; keeping markup ownership in app components is appropriate for first slice.
- DuckDB JSON docs: https://duckdb.org/docs/current/data/json/overview.html
  - DuckDB can read JSON and run SQL over it, but its docs also note JSON is common as interchange rather than efficient tabular storage. Treat DuckDB as an optional future local-analysis lane, not a dependency for first slice.
- MDN Web Workers guide: https://developer.mozilla.org/docs/Web/API/Web_Workers_API/Using_web_workers
  - Large artifact parsing/flattening should move off the main thread once payload size makes UI stalls visible.
- MDN IndexedDB API: https://developer.mozilla.org/en-US/docs/Web/API/IndexedDB_API
  - IndexedDB supports significant client-side structured data and is available in workers. It is a candidate for local perspective caches, not an artifact authority.
- Tauri command scopes: https://v2.tauri.app/security/scope/
  - Any backend-backed artifact reads must respect scoped filesystem access and should not become an arbitrary file explorer.

## Current Repo Anchors

- Artifact commands: `src-tauri/src/commands_artifacts.rs`
- Artifact store and hydration: `src-tauri/src/run_artifacts.rs`
- Report commands: `src-tauri/src/commands_reports.rs`, `src-tauri/src/commands_reports_api.rs`
- History frontend: `web/src/services/historyService.ts`, `web/src/components/HistoryWorkspacePanel.vue`
- Virtual table helpers: `web/src/composables/useVirtualTableRows.ts`
- Existing tables: `web/src/components/TopWorkspacePanel.vue`, `web/src/components/ValidationWorkspacePanel.vue`
- Perspective settings concept: `web/src/stores/settingsStore.ts`, `web/src/utils/perspectiveSearch.ts`
- Existing plans:
  - `docs/feature_expansion_implementation_plans_2026-04-30/07_frontend_workbench_visualization_expansion.md`
  - `docs/feature_expansion_implementation_plans_2026-04-30/08_artifacts_reports_history_expansion.md`
  - `docs/feature_expansion_implementation_plans_2026-04-30/13_manifest_toolchain_dependency_contract.md`

## Product Shape

The lab should live behind an Advanced feature flag or settings switch.

Core views:

1. Artifact Row Explorer
   - choose saved artifact
   - choose row family
   - inspect flattened rows
   - filter/sort/group
   - export current slice

2. Run Comparison
   - compare multiple optimization artifacts
   - compare validation distributions
   - compare campaign item outcomes
   - compare options tuples or ML policy sweep rows

3. Saved Perspectives
   - local saved column/filter/sort/group state
   - built-in perspectives for common tasks
   - no data stored inside perspective state

4. Diagnostics
   - row count
   - parse time
   - flatten time
   - render window size
   - dropped/unknown fields
   - legacy artifact fallback notes

## Architecture Options

| Option | When to use | Tradeoff |
|---|---|---|
| Main-thread JSON flattening | small/medium artifacts and first implementation | simplest, but can freeze UI on large artifacts |
| Web Worker JSON flattening | large artifacts where parse/flatten blocks UI | more plumbing and transfer costs |
| Backend row flattener | artifact truth/redaction must stay in Rust/Tauri | more command surface and schema versioning |
| Arrow IPC | JSON transfer/parsing is measured bottleneck | binary contract and dependency complexity |
| Perspective viewer | users need pivot/aggregate exploration beyond app table controls | WASM/bundle complexity; keep feature-flagged |
| Parquet export | durable large analytical export is needed | storage/export format, not ideal first UI transport |
| DuckDB local query | advanced local SQL exploration is explicitly desired | heavier dependency and sandbox/security questions |

The first slice should implement only the first row. Add workers before adding heavy analytical libraries.

## Row Families

Use typed row families. Do not flatten every JSON object indiscriminately.

| Row family | Source | First-slice fields |
|---|---|---|
| `optimization_candidate` | optimization artifacts | artifact id, rank, strategy, objective, params, score, metrics, backend route |
| `validation_candidate` | validation artifacts | candidate id, verdict, evidence status, PSR/DSR/PBO, route, warnings |
| `campaign_item` | campaign summaries | dataset label, status, skipped/failed reason, artifact paths, validation status |
| `options_tuple` | options artifacts | tuple id, validation verdict, EV/premium, stress metrics, quote quality, price semantics |
| `ml_policy_row` | ML/meta artifacts | model id, policy id, threshold, score cache status, metrics, provenance |
| `portfolio_candidate` | portfolio feasibility | asset class, concentration, capacity/capital fields, rejection reason |

## Proposed Contracts

### ArtifactAnalyticsRowSetV1

```ts
export interface ArtifactAnalyticsRowSetV1 {
  schemaVersion: 1;
  sourceArtifactId: string;
  sourceArtifactKind: string;
  rowFamily: ArtifactAnalyticsRowFamily;
  generatedAtIso: string;
  rows: ArtifactAnalyticsRowV1[];
  diagnostics: ArtifactAnalyticsDiagnosticsV1;
}
```

### ArtifactAnalyticsRowV1

```ts
export interface ArtifactAnalyticsRowV1 {
  rowId: string;
  artifactId: string;
  family: ArtifactAnalyticsRowFamily;
  labels: Record<string, string>;
  dimensions: Record<string, string | number | boolean | null>;
  metrics: Record<string, number | null>;
  warnings: string[];
  sourcePath?: string;
}
```

### ArtifactAnalyticsPerspectiveV1

```ts
export interface ArtifactAnalyticsPerspectiveV1 {
  schemaVersion: 1;
  id: string;
  name: string;
  rowFamily: ArtifactAnalyticsRowFamily;
  columns: string[];
  filters: Array<{ key: string; op: string; value: unknown }>;
  sort: Array<{ key: string; dir: "asc" | "desc" }>;
  groupBy: string[];
  createdAtIso: string;
  updatedAtIso: string;
}
```

### ArtifactAnalyticsLoadDiagnosticsV1

```ts
export interface ArtifactAnalyticsLoadDiagnosticsV1 {
  schemaVersion: 1;
  artifactBytes?: number;
  rawJsonParseMs?: number;
  flattenMs?: number;
  rowCount: number;
  columnCount: number;
  workerUsed: boolean;
  droppedFieldCount: number;
  unknownFieldCount: number;
  truncated: boolean;
  warnings: string[];
}
```

### ArtifactAnalyticsExportRequestV1

```ts
export interface ArtifactAnalyticsExportRequestV1 {
  schemaVersion: 1;
  rowFamily: ArtifactAnalyticsRowFamily;
  sourceArtifactIds: string[];
  filters: Array<{ key: string; op: string; value: unknown }>;
  columns: string[];
  format: "csv" | "json";
  maxRows?: number;
}
```

## Additive Boundary

- Feature flag off by default.
- Flattened JSON row model first.
- No production table replacement.
- No required new backend dependency in first slice.
- No artifact schema migration.
- No loading arbitrary files outside the artifact library scope.
- No dynamic code execution, plugin loading, or SQL exposure in first slice.
- No unbounded row expansion; every flattener must have row and field limits.
- No implicit persistence of filtered slices unless user explicitly exports.

## Implementation Phases

### Phase 1: Row Flattener Library

- [ ] Add `web/src/utils/artifactAnalyticsRows.ts`.
- [ ] Implement row family extractors for optimization, validation, campaign, options, ML/meta, and portfolio artifacts.
- [ ] Keep all fields optional/tolerant.
- [ ] Add stable row ids.
- [ ] Add diagnostics for missing, unknown, and dropped fields.
- [ ] Add tests with legacy/partial artifacts.

### Phase 2: Lab Shell And Feature Flag

- [ ] Add feature flag or Advanced setting for Artifact Analytics Lab.
- [ ] Add lab route/panel under History or Advanced workspace.
- [ ] Add artifact picker constrained to known artifact library entries.
- [ ] Add row-family selector.
- [ ] Render diagnostics before table.

### Phase 3: Virtualized Table

- [ ] Use existing virtual table helpers.
- [ ] Add column chooser.
- [ ] Add filter/sort/group controls.
- [ ] Keep keyboard navigation and focus behavior explicit.
- [ ] Provide copy/export current filtered slice.
- [ ] Add row and column count diagnostics.
- [ ] Add truncation warning if row limits are reached.

### Phase 4: Saved Local Perspectives

- [ ] Add `ArtifactAnalyticsPerspectiveV1` settings storage.
- [ ] Provide built-in perspectives:
  - validation evidence review
  - campaign failed/skipped rows
  - options stress review
  - GPU route review
  - ML score-cache review
- [ ] Add create, duplicate, rename, delete, reset.

### Phase 4A: Worker Thresholds

- [ ] Add instrumentation for parse and flatten time.
- [ ] Define thresholds such as `artifactBytes > 5MB`, `rowCount > 25_000`, or `flattenMs > 100ms`.
- [ ] Move flattening into a worker only after thresholds are observed in local evidence.
- [ ] Use transferable/binary paths only after worker message cost is measured.

### Phase 5: Optional Perspective/Arrow Spike

Only after profiling:

- [ ] Measure JSON flattening and render cost on large saved artifacts.
- [ ] Define row-set size thresholds.
- [ ] Add optional dependency flag for Perspective.
- [ ] Add Vite/WASM handling if Perspective is adopted.
- [ ] Compare JSON rows versus Arrow IPC for load time, memory, and bundle impact.
- [ ] Keep production tables unchanged.

### Phase 6: Backend Export/Redaction Pass

- [ ] Add backend export only if frontend-generated CSV/JSON is too slow or cannot enforce redaction.
- [ ] Keep export path scoped to app artifact directories.
- [ ] Add output parity snapshots for every new export format.
- [ ] Add a redaction test if artifacts can contain credentials, paths, or provider keys.

## Backend Scope

First slice can be frontend-only if artifact hydration already returns the needed JSON. Backend changes are justified only if:

- large artifacts cannot be loaded safely;
- flattened rows need backend-owned truth;
- artifact privacy/redaction rules must be enforced before frontend read;
- export needs a backend writer.

Potential backend helper later:

- `load_artifact_analytics_rows(kind, artifact_id, row_family)` returning a bounded row set.
- `export_artifact_analytics_slice(rows, format)` for CSV/JSON.

## Verification Plan

Frontend:

- `npm --prefix web run test -- artifactAnalyticsRows`
- `npm --prefix web run test -- HistoryWorkspacePanel`
- `npm --prefix web run typecheck`

Browser:

- Add a browser workflow only once the lab is visible:
  - open artifact library
  - open lab
  - choose row family
  - filter rows
  - export slice

Performance:

- Add local timing diagnostics before adopting Arrow or Perspective.
- Test large synthetic row sets with virtualized rendering.
- Verify no visible freeze on common artifact sizes before adding heavier libraries.
- For bundle impact:
  - `npm --prefix web run build`
  - existing bundle/perf budget checks if available.

Backend if changed:

- targeted artifact tests in `ta_desktop_demo_app`
- output parity checks if exports change:
  - `node scripts/run_output_parity_check.mjs`

## Regression Watchpoints

- Do not load huge artifacts synchronously into the main UI without progress/diagnostics.
- Do not let flattened rows invent precision or comparability.
- Do not persist raw artifact data inside saved perspectives.
- Do not expose arbitrary local file reads.
- Do not add Perspective as a default dependency without bundle and WASM verification.
- Do not replace existing optimized production tables with lab components.
- Do not make JSON row flattening a new artifact schema authority.
- Do not cache large artifact-derived rows indefinitely without retention controls.
- Do not expose local filesystem paths beyond the artifact/library paths already visible in the app.
- Do not add SQL-style filtering until injection, resource limits, and sandboxing are explicitly designed.

## Definition Of Done

- Advanced users can open supported artifacts as flattened row sets.
- The lab supports filtering, sorting, grouping, and exporting current slices.
- Row-set diagnostics show shape, timing, and missing-field status.
- Saved perspectives persist view state without embedding data.
- Feature is off or clearly Advanced-only by default.
- Optional Arrow/Perspective adoption is backed by profiling, not assumption.
