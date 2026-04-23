# Bundle 8 implementation plan: frontend orchestration and defaults

This document is a **bundle-8 implementation guide** for the following files:

- `web/src/services/optimizeService.ts`
- `web/src/services/optimizeService.test.ts`
- `web/src/services/recommendedOptimizationDefaults.ts`
- `src-tauri/src/commands_optimize.rs`

It is intentionally scoped to **frontend orchestration and benchmark-backed defaults**. It does **not** propose new product surfaces or broad backend redesigns.

---

## 1. Surface understanding

### What this bundle owns

Per the review map, this bundle owns:

- optimization command selection
- GPU status refresh and parity-check orchestration
- coupling from optimization into artifact persistence, analysis refresh, and optional validation
- benchmark-sensitive defaults such as `top_k` and `heatmap_bins`

### What the current implementation does well

The current surface already has several good properties:

1. **Frontend/backend command routing is aligned.**
   - `resolveOptimizationCommand()` in `optimizeService.ts` correctly routes:
     - `CompositeMo` -> `run_composite_mo_optimization`
     - `Asr` / `Vcc` / `Vemr` / `Rocb` / `Vcet` / `Kqsr` / `Carm` / `Kvdf` / `Mkcs` / `Cdmc` / `CompositeCpu` -> `run_composite_cpu_optimization`
     - the Double-MA family profiles (`DoubleMa`, `DynamicDoubleMa`, `Pit`, `Set`, `Mts`, `Orc`, `Marrt`, `Vfct`, `CompositeTrend`, `RegimeHybrid`, `BreakoutMomentum`) -> `run_double_ma_optimization`
   - `commands_optimize.rs` then performs product-level mode normalization on the Rust side.

2. **Product mode normalization is already correct.**
   - `normalize_product_double_ma_optimization_mode()` preserves non-grid modes unless exact-grid outputs are requested.
   - `normalize_product_composite_cpu_optimization_mode()` intentionally forces the composite CPU family to `Grid`.

3. **DynamicDoubleMa preview-before-run behavior is already correct.**
   - `buildDynamicDoubleMaRuntimeRequest()` always performs preview first and uses the normalized `ir` payload returned by the preview command.

4. **Route metadata is already surfaced into the optimize status line.**
   - `summarizeBackendExecution()` includes backend, route, indicator route plan, policy CPU reroute notes, and fallback text.
   - Existing tests already freeze that behavior.

### What is wrong today

The main problems are **not** command routing. The main problems are:

- the optimize success path performs too much extra work synchronously after the backend command returns
- timing/status labeling is misleading
- request-build errors are collapsed into a generic message
- save failures are hidden
- the route summary can be overwritten by auto-validation failure
- busy/cancel state is inconsistent between optimize and parity flows
- recommended defaults exist in a helper file but are not safely integrated into the optimize path

---

## 2. Findings

Below, each finding follows the required bundle format.

### Finding 1
- **Severity:** high
- **Type:** performance
- **Location:** `web/src/services/optimizeService.ts:2361-2366`, `2420-2469`, `2472-2499`, `2509-2605`
- **Timing boundary affected:** app-path
- **Why it matters:** after the backend command returns, the frontend still performs a full raw JSON stringify for the DOM, builds an artifact payload containing the full `res`, computes dataset metadata twice, updates workflow artifacts, refreshes analysis, optionally runs validation, and refreshes GPU status. This stretches the perceived “done” boundary well beyond the actual optimization command.
- **Recommended change:** reduce hot-path duplication, compute artifact context once, render a lightweight output preview instead of unconditional full-result stringify, and split command completion from post-run follow-up work.

### Finding 2
- **Severity:** high
- **Type:** correctness
- **Location:** `web/src/services/optimizeService.ts:2512-2525`
- **Timing boundary affected:** app-path
- **Why it matters:** request-build failures are swallowed and replaced with the generic `"Load data first, then configure strategy/optimization settings."` message. This hides real DynamicDoubleMa preview/build failures and makes troubleshooting slower.
- **Recommended change:** stop calling `tryBuildAutoOptimizationRequest()` inside a swallow-and-null block in `runOptimization()`. Use `buildOptimizationRequestFromVueStores()` instead and surface the real thrown message.

### Finding 3
- **Severity:** medium
- **Type:** benchmark-fidelity
- **Location:** `web/src/services/optimizeService.ts:2547-2560`, plus `1854-1883`
- **Timing boundary affected:** app-path
- **Why it matters:** the status line currently says `Done in Xms (frontend)`, but the measured interval only wraps `invokeCommand(command, { req })`. It excludes request build, DynamicDoubleMa preview, artifact save, analysis refresh, validation, and GPU status refresh. That label is wrong.
- **Recommended change:** rename the timing boundary to `command` or record full phase timings (`build`, `command`, `artifact`, `analysis`, `validation`, `gpu refresh`, `total`) and place them in `progressMeta`.

### Finding 4
- **Severity:** medium
- **Type:** correctness
- **Location:** `web/src/services/optimizeService.ts:2501-2507`, `2534-2603`, `1137-1342`
- **Timing boundary affected:** app-path
- **Why it matters:** `runOptimization()` keeps `canCancel=true` long after the backend command finished, while `runGpuParityCheck()` does not mark the UI busy at all. That makes the cancel affordance misleading on one path and absent on the other.
- **Recommended change:** split the current single boolean flow into at least two phases using the existing fields:
  - backend command running -> `runDisabled=true`, `validateDisabled=true`, `canCancel=true`
  - post-run follow-up / parity -> `runDisabled=true`, `validateDisabled=true`, `canCancel=false`
  - idle -> all false

### Finding 5
- **Severity:** medium
- **Type:** correctness
- **Location:** `web/src/services/optimizeService.ts:2450-2469`, `2472-2499`
- **Timing boundary affected:** app-path
- **Why it matters:** artifact save failures are swallowed. The workflow artifact store is then updated with an empty `artifact_path`, but the user is not told persistence failed.
- **Recommended change:** keep the run usable, but explicitly mark the run as unsaved in `progressMeta` and preserve the optimize completion status line.

### Finding 6
- **Severity:** medium
- **Type:** correctness
- **Location:** `web/src/services/optimizeService.ts:2557-2585`, with expected behavior anchored by `web/src/services/optimizeService.test.ts:587-722`
- **Timing boundary affected:** route-matrix / route-summary evidence
- **Why it matters:** the optimize success path generates a useful route summary, but any validation failure overwrites `optimize.statusLine`. That destroys the exact route/fallback evidence this bundle is supposed to preserve.
- **Recommended change:** keep the optimization completion status immutable once the command succeeds. Put validation outcomes into `progressMeta`, not `statusLine`.

### Finding 7
- **Severity:** medium
- **Type:** benchmark-fidelity
- **Location:** `web/src/services/recommendedOptimizationDefaults.ts:4-44`, `web/src/services/optimizeService.ts:152-220`
- **Timing boundary affected:** unknown
- **Why it matters:** default shaping currently exists in two different forms:
  - recommended fallback defaults: `topK=50`, `heatmapBins=64`
  - explicit preset profiles: `Quick=20/0`, `Balanced=20/128`, `Deep=50/256`
  These are not inherently wrong, but they are different concepts and are not clearly separated in code comments or orchestration.
- **Recommended change:** preserve the preset profiles, but treat `recommendedOptimizationDefaults.ts` as a **missing-value fallback only**, not a preset override. Wire it into the auto-build path only after fixing its semantics.

### Finding 8
- **Severity:** medium
- **Type:** correctness
- **Location:** `web/src/services/recommendedOptimizationDefaults.ts:17-44`
- **Timing boundary affected:** app-path
- **Why it matters:** the current helper treats `heatmapBins <= 0` as “missing.” That is unsafe if the user intentionally enters `0` to disable heatmap generation for Grid-mode runs.
- **Recommended change:** treat `topK <= 0` as invalid/missing, but treat `heatmapBins` as missing **only when blank/null**, not when explicitly `"0"`.

### Finding 9
- **Severity:** low
- **Type:** performance / profiling
- **Location:** `src-tauri/src/commands_optimize.rs:91-100`, `118-126`, `run_composite_mo_optimization`
- **Timing boundary affected:** profiling-only
- **Why it matters:** `run_double_ma_optimization()` has an NVTX scope around the blocking engine call, but the composite CPU and composite MO command closures do not. That makes Nsight-style comparison less consistent.
- **Recommended change:** add matching NVTX scopes to `run_composite_cpu_optimization()` and `run_composite_mo_optimization()`.

### Finding 10
- **Severity:** low
- **Type:** testing
- **Location:** `web/src/services/optimizeService.test.ts`
- **Timing boundary affected:** app-path / route-summary evidence
- **Why it matters:** existing tests are good on routing and request shaping, but they do not yet freeze the most important orchestration fixes:
  - build-error transparency
  - artifact-save warnings
  - status-line preservation when auto-validation fails
  - busy-state phase changes
  - safe wiring of recommended defaults
- **Recommended change:** extend the test file with focused orchestration tests before changing the implementation.

---

## 3. Safe optimization opportunities

These are the changes I recommend implementing first.

### Safe opportunity A: keep command routing unchanged
Do **not** change `resolveOptimizationCommand()` or Rust product-mode normalization.

Why:
- current frontend/backend command-selection parity looks correct
- existing tests already freeze the intended contract
- the biggest issues are orchestration and status semantics, not route mapping

### Safe opportunity B: make output preview cheap
Replace unconditional full-result stringify with a compact output preview that includes only:
- `best_params`
- `best_metrics`
- `backend_used`
- `backend_route`
- `backend_fallback_reason`
- `indicator_route_plan`
- `mode_used`

This removes the most obvious unnecessary DOM-path serialization.

### Safe opportunity C: reuse artifact context
Compute `csvPath`, `datasetMeta`, and `configFingerprint` once and reuse them for:
- artifact persistence
- workflow artifact sync

This is a small, low-risk reduction in post-run churn.

### Safe opportunity D: preserve real request-build errors
Change `runOptimization()` to call `buildOptimizationRequestFromVueStores()` directly when no request is injected. That immediately improves failure transparency without touching backend behavior.

### Safe opportunity E: split command completion from post-run phases
Use the current store fields more carefully:
- `canCancel=true` only while the backend optimization command is still running
- keep `runDisabled` and `validateDisabled` true during post-run follow-up
- never overwrite the route summary after the command succeeds

### Safe opportunity F: wire recommended defaults only after fixing helper semantics
`recommendedOptimizationDefaults.ts` is useful, but only if it is a safe fallback:
- fill `topK` if missing/invalid
- fill `heatmapBins` only when blank and `mode === "Grid"`
- never treat explicit `heatmapBins="0"` as missing

---

## 4. Higher-risk optimization opportunities

These should be treated as follow-up work, not phase-1 work.

### Higher-risk opportunity A: remove full `res` duplication from workflow artifact sync
Right now the success path keeps the full `res` object in:
- the local result
- the saved artifact payload
- the workflow artifact store payload

This may be reducible, but I would **not** make that change until you review `analysisService.ts`, `workflowArtifactsStore.ts`, and any replay/load paths that consume `currentRun.res`.

### Higher-risk opportunity B: decouple auto-validation from optimize critical path
You could make the optimize command finish “done” before auto-validation begins and allow the user to interact earlier. That would improve responsiveness, but it changes flow semantics and user expectations.

### Higher-risk opportunity C: add backend-side command timing fields
You could push timing metadata back from Rust or emit richer Tauri events. That would improve timing fidelity, but it requires editing adjacent backend result contracts not included in this bundle.

---

## 5. Benchmark and perf-gate review

### What can be concluded from the provided files

You can safely conclude the following:

1. The current optimize status line is mislabeled:
   - it measures **command roundtrip only**
   - it does **not** measure true end-to-end frontend completion

2. The current surface does not expose a phase breakdown for:
   - request build
   - DynamicDoubleMa preview
   - artifact persistence
   - analysis refresh
   - auto-validation
   - GPU status refresh

3. The current preset values and recommended defaults are **not enough, by themselves, to prove benchmark-backed correctness** for numeric defaults.

### What cannot be honestly concluded from the provided files

Do **not** claim that any of these numbers are definitely benchmark-optimal from this bundle alone:

- `RECOMMENDED_TOP_K = 50`
- `RECOMMENDED_HEATMAP_BINS = 64`
- `Balanced.topK = 20`
- `Balanced.heatmapBins = 128`
- `Deep.heatmapBins = 256`

Without the bundle-7 gate/harness files, the safe move is:

- make defaults semantics correct
- make their intended role explicit
- do not change preset numbers yet

### Practical benchmark rule for this bundle

For this bundle, use this contract:

- `statusLine` = stable command completion summary + route/fallback evidence
- `progressMeta` = full phase timing summary and non-fatal warnings

That aligns the optimize surface with the actual timing boundary without requiring backend contract changes.

---

## 6. Test review

### What is already well covered

`optimizeService.test.ts` already covers:

- preset application
- GPU status normalization
- parity orchestration
- DynamicDoubleMa preview-before-run
- mode preserve-vs-grid pin behavior
- backend route summary formatting
- route metadata for `CompositeTrend`, `RegimeHybrid`, and `BreakoutMomentum`
- injected request execution
- extensive request-building coverage
- cancellation dispatch

That is a strong base.

### What is missing

Add tests for the exact fixes in this plan:

1. request-build errors are surfaced instead of collapsed into the generic load-data message
2. optimize status line keeps backend route summary when auto-validation fails
3. artifact-save failure becomes a non-fatal unsaved warning
4. `canCancel` drops to `false` immediately after the optimization command completes
5. parity disables run controls while it is active
6. recommended defaults only fill missing values and do not override explicit zero heatmap bins
7. command-only timing wording replaces the misleading `(frontend)` label

---

## 7. Suggested improvement plan

Implement this in four phases.

### Phase 1: correctness and truthfulness
Scope:
- surface real build errors
- stop mislabeling timing
- preserve route summary
- fix cancel/busy phase semantics

Files:
- `optimizeService.ts`
- `optimizeService.test.ts`

### Phase 2: post-run overhead reduction
Scope:
- reuse artifact context
- compact output preview
- expose unsaved artifact warning without breaking analysis refresh

Files:
- `optimizeService.ts`
- `optimizeService.test.ts`

### Phase 3: defaults hardening
Scope:
- fix helper semantics
- wire defaults only for safe auto-build fallback
- add tests for missing-vs-explicit-zero behavior

Files:
- `recommendedOptimizationDefaults.ts`
- `optimizeService.ts`
- `optimizeService.test.ts`

### Phase 4: profiling and boundary hygiene
Scope:
- add NVTX scopes for composite CPU / composite MO command closures
- add one Rust normalization test for whitespace export paths

Files:
- `commands_optimize.rs`

---

## 8. File-by-file implementation details

---

# A. `web/src/services/recommendedOptimizationDefaults.ts`

## Goal
Make this file safe to wire into the optimize auto-build path.

## Problems to fix
- `heatmapBins <= 0` is currently treated as missing
- explicit zero heatmap bins should remain a valid “disable heatmap” choice
- the file is currently too eager to overwrite values if it were wired directly

## Recommended edit

### Replace the helper logic with this version

```ts
import { useAdvancedStore } from "../stores/advancedStore";
import { resolveRuntimePinia } from "../stores/runtimePinia";

export const RECOMMENDED_TOP_K = "50";
export const RECOMMENDED_HEATMAP_BINS = "64";

function parseOptionalNonNegativeInt(value: unknown): number | null {
  const text = String(value ?? "").trim();
  if (!text) return null;

  const next = Number(text);
  if (!Number.isFinite(next) || next < 0) return null;
  return Math.floor(next);
}

function modeSupportsRecommendedGridHeatmap(value: unknown): boolean {
  return String(value ?? "").trim() === "Grid";
}

export function applyRecommendedOptimizationDefaults(): boolean {
  const pinia = resolveRuntimePinia();
  if (!pinia) return false;

  const advancedStore = useAdvancedStore(pinia);
  let changed = false;
  const appliedDefaults: string[] = [];

  // Top K: blank, NaN, negative, or zero should fall back.
  const topK = parseOptionalNonNegativeInt(advancedStore.topK);
  if (topK == null || topK <= 0) {
    advancedStore.topK = RECOMMENDED_TOP_K;
    changed = true;
    appliedDefaults.push(`Top K ${advancedStore.topK}`);
  }

  // Heatmap bins: only fill when missing/blank, not when explicitly "0".
  const heatmapBins = parseOptionalNonNegativeInt(advancedStore.heatmapBins);
  if (
    heatmapBins == null &&
    modeSupportsRecommendedGridHeatmap(advancedStore.mode)
  ) {
    advancedStore.heatmapBins = RECOMMENDED_HEATMAP_BINS;
    changed = true;
    appliedDefaults.push(`Heatmap bins ${advancedStore.heatmapBins}`);
  }

  if (changed) {
    advancedStore.statusLine =
      `Applied recommended output defaults: ${appliedDefaults.join(", ")}.`;
    advancedStore.source = "vue";
    advancedStore.updatedAtUnixMs = Date.now();
  }

  return changed;
}
```

## Why this version is better

- `topK=0` remains invalid and safely gets repaired
- `heatmapBins=0` remains an explicit user choice and is preserved
- the helper becomes safe to call before auto-building a request

## Additional recommendation
Add a short comment near the exports:

```ts
// These are fallback defaults for missing/invalid user input.
// They are not the same thing as preset profiles such as Quick / Balanced / Deep.
```

---

# B. `web/src/services/optimizeService.ts`

## Goal
Keep routing intact, but make optimize orchestration more truthful, lighter, and easier to debug.

## 1. Import the recommended-default helper

Add:

```ts
import { applyRecommendedOptimizationDefaults } from "./recommendedOptimizationDefaults";
```

---

## 2. Add richer local helpers and state types

These helpers keep the implementation self-contained inside this file.

```ts
interface PersistRunArtifactResult {
  path: string;
  csvPath: string;
  datasetMeta: unknown;
  saved: boolean;
  warning: string;
}

interface RunArtifactContext {
  csvPath: string;
  datasetMeta: unknown;
  configFingerprint: string;
}

interface RunPhaseTimings {
  requestBuildMs: number;
  commandMs: number;
  artifactPersistMs: number;
  analysisRefreshMs: number;
  validationMs: number;
  gpuStatusRefreshMs: number;
  totalMs: number;
}

interface OptimizeBusyState {
  runDisabled: boolean;
  validateDisabled: boolean;
  canCancel: boolean;
}

const RAW_OUTPUT_PREVIEW_CHARS = 12_000;

function nowMs(): number {
  return typeof performance !== "undefined" ? performance.now() : Date.now();
}

function elapsedMs(startedAt: number): number {
  return Math.max(0, nowMs() - startedAt);
}

function appendProgressNote(base: string, note: string): string {
  const next = normalizeText(note);
  if (!next) return base;
  return base ? `${base} • ${next}` : next;
}

function buildCommandCompletionStatus(
  commandMs: number,
  backendSummary: string,
): string {
  const prefix = `Optimization command completed in ${commandMs.toFixed(0)}ms`;
  return backendSummary ? `${prefix} • ${backendSummary}` : prefix;
}

function buildRunPhaseSummary(
  strategyKind: string,
  timings: RunPhaseTimings,
): string {
  return [
    strategyKind,
    `build=${timings.requestBuildMs.toFixed(0)}ms`,
    `command=${timings.commandMs.toFixed(0)}ms`,
    `artifact=${timings.artifactPersistMs.toFixed(0)}ms`,
    `analysis=${timings.analysisRefreshMs.toFixed(0)}ms`,
    `validation=${timings.validationMs.toFixed(0)}ms`,
    `gpu=${timings.gpuStatusRefreshMs.toFixed(0)}ms`,
    `total=${timings.totalMs.toFixed(0)}ms`,
  ].join(" • ");
}

function setOptimizeBusyState(state: OptimizeBusyState): void {
  patchStoreState("optimize", state);
}

function markOptimizationCommandRunning(): void {
  setOptimizeBusyState({
    runDisabled: true,
    validateDisabled: true,
    canCancel: true,
  });
}

function markOptimizationPostRunBusy(): void {
  setOptimizeBusyState({
    runDisabled: true,
    validateDisabled: true,
    canCancel: false,
  });
}

function markNonCancelableBusy(): void {
  setOptimizeBusyState({
    runDisabled: true,
    validateDisabled: true,
    canCancel: false,
  });
}

function markOptimizeIdle(): void {
  setOptimizeBusyState({
    runDisabled: false,
    validateDisabled: false,
    canCancel: false,
  });
}
```

### Why this is worth doing
This fixes the current mismatch where:
- optimize remains cancelable after the backend command already completed
- parity exposes no busy-state gating even though it launches two backend runs

---

## 3. Replace full raw-result stringify with a compact preview

### Current problem
`safePrettyJson(res)` currently serializes the whole result for the DOM.

### Recommended edit

```ts
function safePrettyJson(
  value: unknown,
  maxChars = RAW_OUTPUT_PREVIEW_CHARS,
): string {
  try {
    const json = JSON.stringify(value ?? null, null, 2);
    if (json.length <= maxChars) return json;
    return `${json.slice(0, maxChars)}\n… truncated (${json.length} chars total).`;
  } catch {
    return String(value ?? "");
  }
}

function buildOptimizationOutputPreview(value: unknown): string {
  const row = asRecord(value);
  if (!row) return safePrettyJson(value);

  return safePrettyJson({
    best_params: row.best_params ?? null,
    best_metrics: row.best_metrics ?? null,
    backend_used: row.backend_used ?? null,
    backend_route: row.backend_route ?? null,
    backend_fallback_reason: row.backend_fallback_reason ?? null,
    indicator_route_plan: row.indicator_route_plan ?? null,
    mode_used: row.mode_used ?? null,
  });
}
```

Then change:

```ts
writeDomTextContent("output", safePrettyJson(res), OPTIMIZE_PANEL_SELECTOR);
```

to:

```ts
writeDomTextContent(
  "output",
  buildOptimizationOutputPreview(res),
  OPTIMIZE_PANEL_SELECTOR,
);
```

### Why this is better
- keeps the raw DOM output useful
- preserves route/fallback evidence
- avoids full-result stringify on the user-visible path

---

## 4. Reuse artifact context and surface persistence failure

### Current problem
- dataset meta is built twice
- save failure is swallowed
- the UI never communicates that persistence failed

### Recommended edit

```ts
function buildRunArtifactContext(
  req: Record<string, unknown>,
): RunArtifactContext {
  const dataStore = readPiniaStoreState("data");
  const csvPath = normalizeText(dataStore?.csvPath);
  const datasetMeta = buildDatasetMetaForArtifact(
    csvPath,
    buildRunDatasetContextState(),
  );

  return {
    csvPath,
    datasetMeta,
    configFingerprint: computeConfigFingerprint(req, null),
  };
}

function buildRunArtifactPayload(
  req: Record<string, unknown>,
  res: unknown,
  ts: string,
  context: RunArtifactContext,
): Record<string, unknown> {
  return {
    version: 1,
    kind: "vectorgrid_run_artifact",
    ts,
    config_fingerprint: context.configFingerprint,
    run: { req, res },
    validation: null,
    report_xlsx_path: "",
    data: { csv_path: context.csvPath },
    dataset_meta: context.datasetMeta,
    tags: [],
  };
}

async function persistRunArtifact(
  req: Record<string, unknown>,
  res: unknown,
  ts: string,
  context: RunArtifactContext,
): Promise<PersistRunArtifactResult> {
  const artifact = buildRunArtifactPayload(req, res, ts, context);

  try {
    const saved = (await invokeCommand("save_run_artifact", {
      artifact,
    })) as SaveRunArtifactResponse;

    return {
      path: normalizeText(saved?.path),
      csvPath: context.csvPath,
      datasetMeta: context.datasetMeta,
      saved: true,
      warning: "",
    };
  } catch (error) {
    return {
      path: "",
      csvPath: context.csvPath,
      datasetMeta: context.datasetMeta,
      saved: false,
      warning: `artifact_save_failed=${String(error || "")}`.trim(),
    };
  }
}

function applyVueRunArtifacts(
  ts: string,
  req: Record<string, unknown>,
  res: unknown,
  persisted: PersistRunArtifactResult,
): void {
  applyWorkflowArtifactsPayload(
    {
      currentRun: {
        ts,
        artifact_path: persisted.path,
        report_xlsx_path: "",
        csv_path: persisted.csvPath,
        dataset_meta: persisted.datasetMeta,
        tags: [],
        req,
        res,
      },
      currentValidation: null,
    },
    "vue",
  );
}
```

### Why this is better
- no duplicate dataset-meta computation
- save failure becomes explicit
- analysis refresh can still proceed using the in-memory run

---

## 5. Stop swallowing request-build failures

### Current problem
`runOptimization()` does this now:

```ts
let req = normalizeRequestPayload(options.request);
if (!req) {
  try {
    req = await tryBuildAutoOptimizationRequest();
  } catch {
    req = null;
  }
}
```

That destroys the actual failure message.

### Recommended replacement helper

```ts
async function resolveRunOptimizationRequest(
  options: OptimizeRunOptions,
): Promise<{ req: Record<string, unknown>; requestBuildMs: number }> {
  const injected = normalizeRequestPayload(options.request);
  if (injected) {
    return {
      req: injected,
      requestBuildMs: 0,
    };
  }

  applyRecommendedOptimizationDefaults();

  const buildStartedAt = nowMs();
  const req = await buildOptimizationRequestFromVueStores();

  return {
    req,
    requestBuildMs: elapsedMs(buildStartedAt),
  };
}
```

### Why this is better
- injected requests still work exactly as before
- auto-build requests now surface the actual error text
- DynamicDoubleMa preview/build errors remain visible

---

## 6. Rewrite `runOptimization()` around phase timing and stable status semantics

This is the highest-value change in the file.

### Recommended replacement

```ts
export async function runOptimization(
  options: OptimizeRunOptions = {},
): Promise<void> {
  const switchTab = options.switchTab !== false;
  const runTs = new Date().toISOString();
  const totalStartedAt = nowMs();
  const timings: RunPhaseTimings = {
    requestBuildMs: 0,
    commandMs: 0,
    artifactPersistMs: 0,
    analysisRefreshMs: 0,
    validationMs: 0,
    gpuStatusRefreshMs: 0,
    totalMs: 0,
  };
  const progressNotes: string[] = [];

  let req: Record<string, unknown>;
  try {
    const resolved = await resolveRunOptimizationRequest(options);
    req = resolved.req;
    timings.requestBuildMs = resolved.requestBuildMs;
  } catch (error) {
    const message =
      normalizeText(error instanceof Error ? error.message : error) ||
      "Load data first, then configure strategy/optimization settings.";

    patchStoreState("optimize", { statusLine: message });
    writeDomTextContent("status", message, OPTIMIZE_PANEL_SELECTOR);
    return;
  }

  const strategyKind = normalizeStrategyKindRaw(
    req.strategy_kind ?? req.strategyKind,
  );
  const command = resolveOptimizationCommand(
    req.strategy_kind ?? req.strategyKind,
  );
  const artifactContext = buildRunArtifactContext(req);

  markOptimizationCommandRunning();
  patchStoreState("optimize", {
    progressMeta: `${strategyKind} • command=running`,
    statusLine: "Running optimization…",
  });
  writeDomTextContent("progressMeta", `${strategyKind} • command=running`, OPTIMIZE_PANEL_SELECTOR);
  writeDomTextContent("status", "Running optimization…", OPTIMIZE_PANEL_SELECTOR);

  try {
    const commandStartedAt = nowMs();
    const res = await invokeCommand(command, { req });
    timings.commandMs = elapsedMs(commandStartedAt);

    // Backend command is done. Follow-up work is still busy, but no longer cancelable.
    markOptimizationPostRunBusy();

    const backendSummary = summarizeBackendExecution(res);
    const completionStatus = buildCommandCompletionStatus(
      timings.commandMs,
      backendSummary,
    );

    patchStoreState("optimize", {
      statusLine: completionStatus,
      progressMeta: `${strategyKind} • command=${timings.commandMs.toFixed(0)}ms • post-run=running`,
    });
    writeDomTextContent("status", completionStatus, OPTIMIZE_PANEL_SELECTOR);
    writeDomTextContent(
      "progressMeta",
      `${strategyKind} • command=${timings.commandMs.toFixed(0)}ms • post-run=running`,
      OPTIMIZE_PANEL_SELECTOR,
    );
    writeDomTextContent(
      "output",
      buildOptimizationOutputPreview(res),
      OPTIMIZE_PANEL_SELECTOR,
    );

    const persistStartedAt = nowMs();
    const persisted = await persistRunArtifact(req, res, runTs, artifactContext);
    timings.artifactPersistMs = elapsedMs(persistStartedAt);
    if (!persisted.saved) {
      progressNotes.push("artifact=unsaved");
      if (persisted.warning) progressNotes.push(persisted.warning);
    }

    const analysisStartedAt = nowMs();
    applyVueRunArtifacts(runTs, req, res, persisted);
    refreshAnalysisFromWorkflowRun();
    timings.analysisRefreshMs = elapsedMs(analysisStartedAt);

    if (switchTab) selectWorkflowResultsTab("top", false);

    if (readValidationAutoRunEnabled()) {
      const validationStartedAt = nowMs();
      try {
        await runValidationAction({ useFiltered: false, switchTab: false });
        timings.validationMs = elapsedMs(validationStartedAt);
        if (switchTab) selectWorkflowResultsTab("overview", true);
      } catch (error) {
        timings.validationMs = elapsedMs(validationStartedAt);
        progressNotes.push(
          isCancellationError(error)
            ? "validation=cancelled"
            : `validation_failed=${normalizeText(error)}`,
        );
      }
    }

    const gpuRefreshStartedAt = nowMs();
    try {
      const summary = await refreshGpuStatus();
      applyGpuSummaryToOptimizeStore(summary);
    } catch (error) {
      progressNotes.push(`gpu_status_refresh_failed=${normalizeText(error)}`);
    } finally {
      timings.gpuStatusRefreshMs = elapsedMs(gpuRefreshStartedAt);
    }

    timings.totalMs = elapsedMs(totalStartedAt);

    let progressMeta = buildRunPhaseSummary(strategyKind, timings);
    for (const note of progressNotes) {
      progressMeta = appendProgressNote(progressMeta, note);
    }

    patchStoreState("optimize", {
      statusLine: completionStatus,
      progressMeta,
    });
    writeDomTextContent("status", completionStatus, OPTIMIZE_PANEL_SELECTOR);
    writeDomTextContent("progressMeta", progressMeta, OPTIMIZE_PANEL_SELECTOR);
  } catch (error) {
    const statusLine = isCancellationError(error)
      ? "Cancelled."
      : String(error || "");
    patchStoreState("optimize", { statusLine });
    writeDomTextContent("status", statusLine, OPTIMIZE_PANEL_SELECTOR);
    throw error;
  } finally {
    markOptimizeIdle();
  }
}
```

## Why this version is better

It fixes all of the biggest orchestration bugs at once:

- build errors are preserved
- command timing is labeled honestly
- route summary survives validation failure
- `canCancel` only applies to the backend command phase
- artifact failures are surfaced without breaking analysis
- full phase timing is available in `progressMeta`

---

## 7. Make parity use the same busy-state model

### Current problem
`runGpuParityCheck()` launches backend work but does not set busy flags.

### Recommended edit

Wrap the body in a non-cancelable busy window:

```ts
export async function runGpuParityCheck(): Promise<void> {
  const optimizeStore = readPiniaStoreState("optimize");
  if (normalizeBoolean(optimizeStore?.canCancel)) return;

  markNonCancelableBusy();
  try {
    // existing parity logic stays mostly the same
  } finally {
    markOptimizeIdle();
  }
}
```

### Optional improvement
You can also set a clearer `progressMeta` during parity:

```ts
patchStoreState("optimize", {
  progressMeta: "Parity • running tiny CPU↔GPU check",
});
```

That is optional, but the busy-state fix is the important part.

---

## 8. Keep `buildDynamicDoubleMaRuntimeRequest()` behavior, but treat it as build time

Do not remove the preview call from `buildDynamicDoubleMaRuntimeRequest()`.

That preview is part of **request build**, not command execution. The fix is not to skip it. The fix is to:
- time it as part of `requestBuildMs`
- preserve its thrown error message

---

# C. `src-tauri/src/commands_optimize.rs`

## Goal
Do not change routing semantics. Only improve profiling consistency and close small contract gaps.

## 1. Add NVTX scopes for composite command paths

### Current state
Only `run_double_ma_optimization()` has an NVTX scope.

### Recommended edit

```rust
#[tauri::command]
pub async fn run_composite_cpu_optimization(
    mut req: CompositeCpuRequest,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<BackendOptimizationResult, String> {
    req.mode = normalize_product_composite_cpu_optimization_mode(req.mode);
    let _run_guard = state.try_begin_run()?;
    let cancel = state.cancel_token();
    let candles = state.get_candles(&req.data_id)?;

    let cancel2 = cancel.clone();
    let app2 = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _nvtx_scope = ta_app_core::nvtx::scope("engine", "tauri_composite_cpu_request");
        let sink = TauriProgressSink { app: app2 };
        ta_app_core::run_composite_cpu_optimization_blocking_with_candles(
            req,
            &candles,
            cancel2.as_ref(),
            Some(&sink),
        )
    })
    .await
    .map_err(|e| e.to_string())?
}
```

And similarly for `run_composite_mo_optimization()`:

```rust
#[tauri::command]
pub async fn run_composite_mo_optimization(
    req: CompositeMoRequest,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<CompositeMoResult, String> {
    let _run_guard = state.try_begin_run()?;
    let cancel = state.cancel_token();
    let candles = state.get_candles(&req.data_id)?;

    let cancel2 = cancel.clone();
    let app2 = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _nvtx_scope = ta_app_core::nvtx::scope("engine", "tauri_composite_mo_request");
        let sink = TauriProgressSink { app: app2 };
        ta_app_core::run_composite_mo_optimization_blocking_with_candles(
            req,
            &candles,
            cancel2.as_ref(),
            Some(&sink),
        )
    })
    .await
    .map_err(|e| e.to_string())?
}
```

## Why this is worth doing
This does not change behavior, but it makes profiling evidence more consistent across the three command entrypoints.

---

## 2. Add one extra normalization test for whitespace export paths

### Recommended test

```rust
#[test]
fn double_ma_product_mode_normalization_ignores_blank_export_path() {
    let mut req = base_double_ma_req();
    req.mode = OptimizationMode::GpuVdps;
    req.export_all_csv_path = Some("   ".to_string());

    assert_eq!(
        normalize_product_double_ma_optimization_mode(&req),
        OptimizationMode::GpuVdps
    );
}
```

## Why this is useful
You already trim `export_all_csv_path`, but this test freezes that behavior and protects against a future regression.

---

## 3. What not to change in this file

Do **not** change:

- `normalize_product_double_ma_optimization_mode()`
- `normalize_product_composite_cpu_optimization_mode()`
- the Tauri command names
- the current `try_begin_run()` single-active-run guard

This file is mostly in good shape for this bundle.

---

# D. `web/src/services/optimizeService.test.ts`

## Goal
Add tests that lock in the orchestration fixes before you refactor.

## 1. Add a test for real request-build error surfacing

```ts
it("surfaces request-build failures instead of collapsing them into a generic load-data message", async () => {
  (pinia.state.value as Record<string, unknown>).optimize = {
    strategyKind: "DynamicDoubleMa",
    backend: "GpuKernel",
    dynamicStrategyIrJson: '{"profile":"double_ma"}',
  };
  (pinia.state.value as Record<string, unknown>).advanced = {
    objective: "Sharpe",
    topK: "20",
    includeAll: "false",
  };
  (pinia.state.value as Record<string, unknown>).data = {
    dataId: "data-dyn-error",
    dataFields: ["close"],
    dataLen: 256,
  };

  invokeCommandMock.mockImplementation(async (command: string) => {
    if (command === "preview_dynamic_double_ma_strategy_ir_json") {
      return {
        schema_compatibility: "Compatible",
        ir: null,
        preview: {},
      };
    }
    return {};
  });

  await runOptimization({ switchTab: false });

  const optimize = (pinia.state.value as Record<string, unknown>)
    .optimize as Record<string, unknown>;
  expect(String(optimize.statusLine || "")).toContain(
    "Dynamic strategy preview did not return a normalized strategy_ir payload.",
  );
});
```

---

## 2. Add a test that validation failure does not erase route summary

```ts
it("preserves backend route summary when auto-validation fails", async () => {
  (pinia.state.value as Record<string, unknown>).validation = {
    autoRun: true,
  };

  runValidationActionMock.mockRejectedValueOnce(new Error("validation boom"));

  invokeCommandMock.mockImplementation(async (command: string) => {
    if (command === "run_double_ma_optimization") {
      return {
        best_metrics: { pnl: 1, sharpe: 0.5, max_dd: 0.2 },
        best_params: { fast_len: 5, slow_len: 20 },
        backend_used: "Cpu",
        backend_route: "GpuHybridKernel->CpuOnly",
        backend_fallback_reason: "Hybrid phase-1 fallback to CpuOnly.",
      };
    }
    if (command === "save_run_artifact") {
      return { path: "runs/vectorgrid_run_test.json", bytes: 128 };
    }
    if (command === "get_gpu_status") {
      return {
        built_with_cuda: true,
        built_with_cuda_backtest_kernel: true,
        cuda_available: true,
      };
    }
    return {};
  });

  await runOptimization({
    request: { strategy_kind: "DoubleMa", data_id: "data-1" },
    switchTab: false,
  });

  const optimize = (pinia.state.value as Record<string, unknown>)
    .optimize as Record<string, unknown>;

  expect(String(optimize.statusLine || "")).toContain("route=GpuHybridKernel->CpuOnly");
  expect(String(optimize.progressMeta || "")).toContain("validation_failed=validation boom");
});
```

---

## 3. Add a test for artifact-save warning behavior

```ts
it("marks the run unsaved when artifact persistence fails", async () => {
  invokeCommandMock.mockImplementation(async (command: string) => {
    if (command === "run_double_ma_optimization") {
      return {
        best_metrics: { pnl: 1, sharpe: 0.5, max_dd: 0.2 },
        best_params: { fast_len: 5, slow_len: 20 },
        backend_used: "Cpu",
        backend_route: "CpuOnly",
      };
    }
    if (command === "save_run_artifact") {
      throw new Error("disk full");
    }
    if (command === "get_gpu_status") {
      return {
        built_with_cuda: true,
        built_with_cuda_backtest_kernel: true,
        cuda_available: true,
      };
    }
    return {};
  });

  await runOptimization({
    request: { strategy_kind: "DoubleMa", data_id: "data-1" },
    switchTab: false,
  });

  const optimize = (pinia.state.value as Record<string, unknown>)
    .optimize as Record<string, unknown>;

  expect(String(optimize.statusLine || "")).toContain("Optimization command completed");
  expect(String(optimize.progressMeta || "")).toContain("artifact=unsaved");
});
```

---

## 4. Add a busy-state phase test for optimize

This test is a little more involved, but it is worth adding because it freezes the new `canCancel` semantics.

### Suggested structure

- use a deferred promise for `runValidationActionMock`
- let the backend command resolve immediately
- assert:
  - `runDisabled === true`
  - `validateDisabled === true`
  - `canCancel === false`
- then resolve validation and assert idle state

A compact helper works well:

```ts
function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error?: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}
```

Then add a test using that helper.

---

## 5. Add a parity busy-state test

Use the same deferred pattern to ensure `runGpuParityCheck()` sets:
- `runDisabled = true`
- `validateDisabled = true`
- `canCancel = false`

while parity work is in flight.

---

## 6. Add recommended-default behavior tests

You have two options:

### Option A: keep them in this file
Import the helper:

```ts
import { applyRecommendedOptimizationDefaults } from "./recommendedOptimizationDefaults";
```

Then add:

```ts
it("applies recommended defaults when topK is invalid and grid heatmap is blank", () => {
  (pinia.state.value as Record<string, unknown>).advanced = {
    mode: "Grid",
    topK: "0",
    heatmapBins: "",
    statusLine: "",
  };

  const changed = applyRecommendedOptimizationDefaults();

  const advanced = (pinia.state.value as Record<string, unknown>)
    .advanced as Record<string, unknown>;

  expect(changed).toBe(true);
  expect(advanced.topK).toBe("50");
  expect(advanced.heatmapBins).toBe("64");
});

it("does not overwrite explicit zero heatmap bins", () => {
  (pinia.state.value as Record<string, unknown>).advanced = {
    mode: "Grid",
    topK: "20",
    heatmapBins: "0",
    statusLine: "",
  };

  const changed = applyRecommendedOptimizationDefaults();

  const advanced = (pinia.state.value as Record<string, unknown>)
    .advanced as Record<string, unknown>;

  expect(changed).toBe(false);
  expect(advanced.heatmapBins).toBe("0");
});
```

### Option B: create `recommendedOptimizationDefaults.test.ts`
This is slightly cleaner, but not required.

---

## 7. Update the existing optimize completion wording expectations

Wherever tests currently expect:

- `"Done in Xms (frontend)"`

update them to expect either:

- `"Optimization command completed in Xms"`
- or your final chosen command-only wording

That locks the timing-boundary fix.

---

## 9. Rollout order

Use this exact order.

### Step 1
Add the new tests first:
- build-error transparency
- route summary survives validation failure
- artifact unsaved warning
- recommended-default semantics

### Step 2
Refactor `recommendedOptimizationDefaults.ts`.

### Step 3
Refactor `optimizeService.ts` in this order:
1. add helper interfaces/types
2. add busy-state helpers
3. add output preview helper
4. add artifact-context helper
5. add request-resolution helper
6. rewrite `runOptimization()`
7. wrap parity in busy-state handling

### Step 4
Add NVTX scopes and the whitespace normalization test in Rust.

### Step 5
Run the current frontend and Rust test suites.

---

## 10. Acceptance criteria

The implementation is complete when all of the following are true:

1. **Route contract unchanged**
   - current routing tests still pass
   - no strategy-kind command mapping regressed

2. **Request-build failures are explicit**
   - DynamicDoubleMa preview/build failures appear in `optimize.statusLine`

3. **Timing language is honest**
   - optimize completion message clearly refers to command time, not total frontend time
   - `progressMeta` includes a phase breakdown

4. **Route summary is preserved**
   - validation failure/cancel does not overwrite backend route/fallback info

5. **Busy/cancel semantics are correct**
   - cancel is available only during the backend optimization command
   - parity disables controls but does not expose a stale cancel button

6. **Artifact save failures are visible**
   - optimize still completes
   - the run can still hydrate analysis
   - the UI exposes that persistence failed

7. **Recommended defaults are safe**
   - `topK=0` gets repaired
   - blank `heatmapBins` in Grid mode gets repaired
   - explicit `heatmapBins=0` is preserved

8. **Composite profiling boundary is improved**
   - composite CPU and composite MO commands now have NVTX scopes matching the double-MA path

---

## 11. Risks and open questions

### Risk 1: dropping full `res` from workflow artifacts may break analysis/replay
Do not attempt that in this pass. You need adjacent-file review first.

### Risk 2: default numbers may not be benchmark-optimal
This plan fixes semantics and safe fallback behavior, but it does not prove that `50/64` or preset numbers are the best measured defaults. Keep numeric preset changes out of this pass unless you bring bundle-7 benchmark evidence into scope.

### Open question 1
`requestCancelCurrentRun()` and validation cancellation plumbing live outside this bundle. If you later want true cancel support during post-run validation, review:
- `runControlService.ts`
- `validationService.ts`

### Open question 2
If you want to slim artifact payloads or workflow artifact state, you should review:
- `analysisService.ts`
- `workflowArtifactsStore.ts`
- replay/load services

---

## 12. What I would not change in this pass

Do **not** change:

- `resolveOptimizationCommand()`
- Double-MA mode normalization in Rust
- composite CPU mode normalization in Rust
- DynamicDoubleMa preview-before-run behavior
- Quick / Balanced / Deep numeric preset values yet
- saved artifact shape beyond making save failure explicit
- backend result contracts beyond optional profiling scopes

Those are either already correct or not safely benchmarked from this bundle alone.

---

## 13. Comprehensive implementation checklist

### Phase 1: correctness and status semantics
- [ ] Add tests for request-build error transparency.
- [ ] Add tests for validation failure preserving the backend route summary.
- [ ] Add tests for artifact-save warning behavior.
- [ ] Replace generic request-build swallowing in `runOptimization()`.
- [ ] Change optimize completion wording from misleading frontend-total language to command-completion language.
- [ ] Keep route summary in `statusLine` after command success.
- [ ] Move validation failure/cancel notes into `progressMeta`.

### Phase 2: busy/cancel lifecycle
- [ ] Add `OptimizeBusyState` helpers in `optimizeService.ts`.
- [ ] Set `canCancel=true` only during the backend optimize command.
- [ ] Set `canCancel=false` during artifact/analysis/validation/GPU refresh follow-up.
- [ ] Wrap `runGpuParityCheck()` in a non-cancelable busy-state window.
- [ ] Add a test for optimize busy-state phase behavior.
- [ ] Add a test for parity busy-state behavior.

### Phase 3: output and artifact-path overhead
- [ ] Replace unconditional full-result DOM stringify with a compact preview helper.
- [ ] Add `RunArtifactContext`.
- [ ] Reuse dataset meta and config fingerprint across save + workflow sync.
- [ ] Surface unsaved artifact state through `progressMeta`.
- [ ] Preserve analysis refresh even when artifact save fails.

### Phase 4: recommended defaults hardening
- [ ] Refactor `recommendedOptimizationDefaults.ts` so explicit `heatmapBins="0"` is preserved.
- [ ] Keep `topK<=0` as an auto-repair case.
- [ ] Wire `applyRecommendedOptimizationDefaults()` into the auto-build optimize path only.
- [ ] Ensure injected request execution does not mutate advanced defaults.
- [ ] Add tests for:
  - [ ] missing Grid heatmap bins -> auto-fill to `64`
  - [ ] invalid/missing `topK` -> auto-fill to `50`
  - [ ] explicit `heatmapBins="0"` -> preserved
  - [ ] non-Grid mode -> no automatic heatmap fill

### Phase 5: profiling consistency
- [ ] Add NVTX scope to `run_composite_cpu_optimization()`.
- [ ] Add NVTX scope to `run_composite_mo_optimization()`.
- [ ] Add a Rust unit test for whitespace `export_all_csv_path`.
- [ ] Verify no Rust command behavior changed beyond profiling and tests.

### Phase 6: final verification
- [ ] Re-run the existing frontend test suite.
- [ ] Re-run the Rust unit tests covering `commands_optimize.rs`.
- [ ] Confirm all existing route-selection tests still pass unchanged.
- [ ] Manually verify that a successful optimize run:
  - [ ] shows command-completion status
  - [ ] preserves backend route info
  - [ ] shows phase timings in `progressMeta`
  - [ ] no longer shows a cancel affordance during post-run validation
- [ ] Manually verify that a failed artifact save:
  - [ ] still allows analysis refresh
  - [ ] surfaces `artifact=unsaved`
- [ ] Manually verify that DynamicDoubleMa preview/build failures surface the real error text.

---