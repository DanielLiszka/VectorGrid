# Prompt for OpenAI Codex Agent: implement only the repo-state-valid outstanding Section 11 `CompositeCpu` work

You are working inside a repo that contains the standalone `CompositeCpu` optimization family and its frontend/Tauri/benchmark surfaces.

Your job is to **fully implement the concrete guidance** from the following source documents, while staying inside their scope guards and preserving product behavior:

## Source-of-truth documents to read first

1. `gpt54_feature_review_map.md`
   - Read **only** the section:
     - `## 11. CompositeCpu optimizer hot paths and request shaping`
2. `section11_composite_cpu_implementation_plan.md`
3. `section11_composite_cpu_plan_addendum.md`

### Priority and conflict rules

- Treat `gpt54_feature_review_map.md` section 11 as the **scope guard**.
- Treat `section11_composite_cpu_implementation_plan.md` as the **main implementation spec**.
- Treat `section11_composite_cpu_plan_addendum.md` as an **extension + correction layer**.
- If the addendum and the original plan differ, **the addendum wins**.
- Do **not** widen scope just because the engine surface is broader than the current product surface.

### Repo-state validation checklist

This prompt has already been checked against the current repo state. Keep these validated items as `[x]` unless new evidence from the code says otherwise.

- [x] `web/src/services/optimizeService.test.ts` already exists. Extend it instead of creating a new file.
- [x] `web/src/utils/strategyOptimizationCapabilities.test.ts` already exists. Extend it instead of creating a new file.
- [x] `web/src/services/optimizeRequestBuilder.test.ts` already contains the `trade_on_next_bar` regression. Do not add duplicate coverage for that behavior.
- [x] `web/src/services/optimizeRequestBuilder.test.ts` already covers standalone shortlist builder behavior plus ROCB/VCET builder payload construction. Do not duplicate those builder tests unless you are adding the specific missing plain-`CompositeCpu` shortlist regression called out below.
- [x] `crates/app_core/benches/composite_validation_workflow.rs` already has the benchmark metadata + route-probe pattern. Use it as the reference style for `composite_cpu_opt.rs`; do not add extra logic churn there.
- [x] `src-tauri/src/commands_optimize.rs` is already functionally correct for standalone `CompositeCpu` Grid-only normalization. Treat it as comment-only unless you uncover a real bug.
- [x] `src-tauri/src/bin/ta_workflow_regression.rs` is already the correct section-11 app-path proxy. Keep its existing request shape unless you intentionally add the clearly optional second case.
- [x] VCET customization is already exposed in the current frontend/store surface (`web/src/stores/strategyStore.ts` and `web/src/components/StrategyWorkspacePanel.vue`). Treat missing `optimizeService.ts` VCET bindings/forwarding as a **confirmed service-layer bug**, not as an optional “maybe unreachable” audit.
- [x] Plain `CompositeCpu` route-label semantics are currently only an evidence-driven sanity check. Do not spend standalone patch budget on route-metadata changes unless the new diagnostics / route probes show a concrete mismatch.

### Outstanding work summary

The still-missing work falls into four buckets:

- service-layer request forwarding for standalone shortlist controls, ROCB, and VCET,
- standalone capability-summary alignment plus request-builder readability constants/comments,
- plain `CompositeCpu` backend correctness/route transparency/timing instrumentation,
- benchmark cleanup and targeted regression coverage.

---

# Mission

Implement the required correctness, performance-observability, request-shaping, route/fallback, and benchmark-fidelity improvements for the standalone `CompositeCpu` request family.

The goal is to improve **performance, optimization behavior, benchmark fidelity, and correctness** without removing features, without breaking functionality, and without redesigning product surfaces.

Your work should favor:

- parity-safe fixes,
- route transparency,
- benchmark honesty,
- measurement before refactor,
- small, reviewable edits,
- compile-safe behavior on CPU-only systems,
- minimal product-surface churn.

---

# Hard scope guard

## In scope

The standalone `CompositeCpu` request family only:

- `CompositeCpu`
- `Asr`
- `Vcc`
- `Vemr`
- `Rocb`
- `Vcet`
- `Kqsr`
- `Carm`
- `Kvdf`
- `Mkcs`
- `Cdmc`

## Explicitly out of scope unless absolutely required to preserve existing behavior

- `CompositeTrend`
- `RegimeHybrid`
- `BreakoutMomentum`
- broad `CompositeMo` tuning changes
- new product knobs
- UI redesigns or new panels
- re-enabling Double-MA heatmap/export-all UX for standalone `CompositeCpu`
- speculative GPU expansion not grounded in the existing route surfaces
- kernel-surface `allowed_pairs` support as part of this patch set
- unsafe allocation/copy-elision refactors in the same patch train
- broad allocation-policy work (that is a follow-on bundle, not part of this patch set)

---

# Product-surface rules you must preserve

These are important and should shape the implementation:

1. Standalone `CompositeCpu`-family product requests are **Grid-only**.
2. For the current product surface, standalone `CompositeCpu`-family requests are about **Top-K** and **include_all**.
3. Double-MA-style heatmap/export-all assumptions are **stale** for this surface.
4. `gpu_prescan` tuning belongs to **`CompositeMo`**, not the standalone `CompositeCpu` frontend product path.
5. Plain `CompositeCpu` should not suddenly gain new frontend controls or product capabilities as part of this patch.
6. Do not remove internal engine capabilities that are still used for tests or internal callers unless the source docs explicitly require that.

---

# Files to inspect and potentially modify

## Primary files already identified in the plan

- `crates/app_core/src/composite_cpu.rs`
- `src-tauri/src/commands_optimize.rs`
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/services/optimizeService.ts`
- `web/src/utils/strategyOptimizationCapabilities.ts`
- `web/src/services/optimizeRequestBuilder.test.ts`
- `crates/app_core/benches/composite_cpu_opt.rs`
- `crates/app_core/benches/composite_validation_workflow.rs`
- `src-tauri/src/bin/ta_workflow_regression.rs`

## Adjacent files that already exist and should receive coverage or audit updates

- `web/src/services/optimizeService.test.ts`
- `web/src/utils/strategyOptimizationCapabilities.test.ts`
- `web/src/stores/strategyStore.ts`
- `web/src/components/StrategyWorkspacePanel.vue`

The two frontend test files already exist in the current repo. Extend them in the existing style instead of creating new files.

---

# Deliverable expectations

Make code changes directly in the repo.

At the end, provide:

1. a concise summary of what changed,
2. a list of files edited,
3. tests/bench compile checks you ran,
4. optional items you intentionally skipped and why,
5. any remaining ambiguity with a grounded note.

Do **not** just restate the plan. Implement it.

---

# Required implementation work

## 1. Frontend service-layer request plumbing fixes

### 1A. Fix missing standalone shortlist control forwarding in `web/src/services/optimizeService.ts`

The service currently routes standalone `CompositeCpu` family requests into `buildCompositeCpuOptimizationRequest(...)`, but it drops three fields that the builder already supports:

- `extraShortlistObjectives`
- `shortlistConfidenceMode`
- `shortlistUnionMode`

You must forward them through `buildCompositeCpuOptimizationRequest(...)`.

### Required behavior

- These fields must survive the service layer for standalone GPU-native strategy requests such as `Vemr`, `Asr`, etc.
- Keep the builder’s existing validation semantics intact.
- Do not widen support to plain `CompositeCpu`; keep builder validation unchanged.

### Expected patch shape

Near the existing `objective`, `topK`, and `includeAll` arguments, add the missing pass-through fields.

Use the existing store-reader helpers already used in that file.

Semantically equivalent to:

```ts
objective: readStoreTextInputValue("objective"),
extraShortlistObjectives: readStoreTextInputValue("extraShortlistObjectives"),
shortlistConfidenceMode: readStoreTextInputValue("shortlistConfidenceMode"),
shortlistUnionMode: readStoreTextInputValue("shortlistUnionMode"),
topK: readStoreTextInputValue("topK"),
includeAll: readStoreTextInputValue("includeAll"),
```

---

### 1B. Fix missing ROCB custom parameter forwarding in `web/src/services/optimizeService.ts`

The addendum identifies a second service-layer bug: ROCB fields are bound in store state, the builder supports them, but the service does not forward them.

You must forward:

- `rocbChannelPeriod`
- `rocbAtrPeriod`
- `rocbAtrStopMult`
- `rocbMaxTradeBars`

### Required behavior

- Custom ROCB values set in Vue/Pinia-backed state must appear in the built runtime request.
- Keep them grouped with the rest of the standalone strategy parameter forwarding block.

### Expected patch shape

Add these near the VEMR/strategy-specific forwarding block, before `fastMaParams` / `slowMaParams`.

Semantically equivalent to:

```ts
rocbChannelPeriod: readStoreTextInputValue("rocbChannelPeriod"),
rocbAtrPeriod: readStoreTextInputValue("rocbAtrPeriod"),
rocbAtrStopMult: readStoreTextInputValue("rocbAtrStopMult"),
rocbMaxTradeBars: readStoreTextInputValue("rocbMaxTradeBars"),
```

---

### 1C. Fix missing VCET service-layer bindings and forwarding in `web/src/services/optimizeService.ts`

The original addendum treated VCET as an open question. In the current repo, it is a confirmed service-layer bug:

- the store and UI already expose VCET customization,
- the builder already supports VCET inputs,
- `optimizeService.ts` does **not** currently bind and forward the VCET-specific fields.

### Required behavior

- Keep the current product surface exactly as-is.
- Add the missing `VUE_TEXT_FIELD_BINDINGS` entries for the exposed VCET numeric fields.
- Add the missing `VUE_CHECKBOX_FIELD_BINDINGS` entries for the exposed VCET toggle fields.
- Forward the exposed VCET fields through `buildCompositeCpuOptimizationRequest(...)`.
- Add service-level regressions proving the custom VCET values survive store -> request building.

### Minimum fields to bind and forward

Text fields:

- `vcetVidyaShortPeriod`
- `vcetVidyaLongPeriod`
- `vcetVidyaAlpha`
- `vcetCePeriod`
- `vcetCeMult`
- `vcetMaxTradeBars`

Checkbox fields:

- `vcetMapFastToVidyaShortPeriod`
- `vcetMapSlowToVidyaLongPeriod`
- `vcetMapSlowToCePeriod`
- `vcetCeUseClose`
- `vcetRequireVidyaSlope`

Do not invent a new UI. The current UI/store surface already exists.

---

## 2. Frontend capability-summary and builder-surface alignment

### 2A. Normalize legacy GPU aliases in `web/src/utils/strategyOptimizationCapabilities.ts`

The builder normalizes legacy GPU aliases (`GpuOnly`, `GpuHybridKernel`) into the unified kernel payload path, but the capability helper only treats the exact string `GpuKernel` as kernel-capable.

You must make capability exposure match the builder’s backend alias policy.

### Required change

Update `requestedGlobalBackendIsGpuKernel(value)` so it returns `true` for:

- `GpuKernel`
- `GpuOnly`
- `GpuHybridKernel`

Semantically equivalent to:

```ts
function requestedGlobalBackendIsGpuKernel(value: unknown): boolean {
  const backend = String(value ?? '').trim()
  return (
    backend === 'GpuKernel'
    || backend === 'GpuOnly'
    || backend === 'GpuHybridKernel'
  )
}
```

---

### 2B. Tighten standalone-family output/capability copy

Update the standalone-family copy so it explicitly communicates the current prerequisites/cost shape.

The summary should make it clear that:

- standalone family requests support `Top-K` and `include_all`,
- on requested standalone `GpuKernel` routes they can also use host-side extra shortlist heads, union reranking, and retained-shortlist confidence reranking,
- extra shortlist heads and union reranking currently require `include_all=true`,
- confidence reranking requires `top_k > 0`,
- the standalone family does **not** emit Double-MA heatmap or per-pair CSV export,
- `include_all` is meaningful and not free.

Keep the capability booleans aligned with the actual current behavior.

Do not re-enable heatmap/export support for this surface.

---

### 2C. Add maintainability constants/comments in `web/src/services/optimizeRequestBuilder.ts`

Do not change the current product behavior of the standalone `CompositeCpu` builder surface.

Instead, make the fixed behavior more explicit and less “magic value” shaped.

### Required change

Introduce clear constants/comments for the standalone `CompositeCpu` product surface, for example:

```ts
const COMPOSITE_CPU_PRODUCT_MODE = "Grid" as const
const COMPOSITE_CPU_PRODUCT_HEATMAP_BINS = 0
```

Then use them when returning the standalone `CompositeCpu` family request.

Add a short comment clarifying that this is intentional:

- Grid only
- no Double-MA heatmap/export-all product shaping on this surface

This is mainly a maintainability/readability change.

Do not change actual emitted behavior unless required by a confirmed bug.

---

## 3. Engine correctness and route/fallback transparency in `crates/app_core/src/composite_cpu.rs`

This is the most important backend file in the patch set.

### 3A. Preserve MA parameter maps in plain `CompositeCpu` result shaping

The current plain `CompositeCpu` result path accepts `fast_ma_params` / `slow_ma_params`, uses them during optimization, but drops them from the resolved result shape.

You must preserve them in the final `DoubleMaParamsResolved` for:

- `best_params`
- `top`
- `all`

### Required change

In the plain `CompositeCpu` result-shaping closure, do **not** hardcode MA param maps to `None`.

Use a closure semantically equivalent to:

```rust
let resolve_params = |p: &DoubleMaParams| -> DoubleMaParamsResolved {
    DoubleMaParamsResolved {
        fast_len: p.fast_len,
        slow_len: p.slow_len,
        fast_ma_type: fast_ma.clone(),
        slow_ma_type: slow_ma.clone(),
        fast_ma_params: req.fast_ma_params.clone(),
        slow_ma_params: req.slow_ma_params.clone(),
    }
};
```

---

### 3B. Add plain `CompositeCpu` route diagnostics and return them

The plan calls for route diagnostics on the plain `CompositeCpu` path.

You must add a local `route_diagnostics: Vec<String>` (or equivalent) and return it whenever non-empty.

### Diagnostics to add

At minimum, add diagnostics for:

1. `include_all=true` forcing the serial full-row path.
2. stage-0 prescan narrowing the search to an allowed-pair subset.
3. auto-prescan being skipped specifically to preserve a requested plain `CompositeCpu` `GpuKernel` route.
4. explicit `gpu_prescan` potentially forcing CPU fallback because the base plain `CompositeCpu` `GpuKernel` path does not support `allowed_pairs`.
5. the backend/device actually used for stage-0 prescan.

The exact text can vary slightly, but it should preserve those semantics.

Examples from the plan/addendum include messages like:

- `include_all=true forces serial full-row collection in optimize_composite_cpu_tiled`
- `stage-0 prescan narrowed CompositeCpu search to N allowed pairs`
- `auto-prescan skipped to preserve requested plain CompositeCpu GpuKernel route`
- `explicit gpu_prescan may force CPU fallback because the base CompositeCpu GpuKernel path does not yet support allowed_pairs`
- `stage-0 prescan backend=... (explicit_cfg=... device_id=...)`

When the result object is returned, stop hardcoding `route_diagnostics: None` for the plain `CompositeCpu` path if you have collected diagnostics.

---

### 3C. Fix the plain-`CompositeCpu` auto-prescan route cliff

The plan identifies a route/fallback problem:

- large plain `CompositeCpu` requests auto-trigger stage-0 prescan,
- prescan builds `allowed_pairs`,
- the base plain `CompositeCpu` `GpuKernel` path rejects `allowed_pairs`,
- so a request that asked for `GpuKernel` can silently lose eligibility before the main optimization step.

### Required behavior

- Preserve explicit `req.gpu_prescan` support for internal callers and tests.
- But do **not** auto-enable stage-0 prescan for plain `CompositeCpu` requests that explicitly asked for `Backend::GpuKernel`.
- Add diagnostics so this behavior is visible.

### Implementation guidance

Use the structure from the plan as the intended behavior:

- detect `explicit_prescan_requested = req.gpu_prescan.is_some()`
- detect `requested_plain_gpu_kernel = matches!(req.backend, Backend::GpuKernel { .. })`
- only auto-prescan large requests if `!requested_plain_gpu_kernel`
- if explicit prescan is requested, keep existing explicit behavior
- if the request is a plain `GpuKernel` request and it crosses the auto threshold, skip implicit prescan and emit the explicit diagnostic

Do **not** add kernel `allowed_pairs` support in this patch set.

---

### 3D. Fix stage-0 prescan backend/device inheritance

The addendum identifies another prescan problem:

- the outer request backend is normalized up front,
- but implicit/auto prescan rebuilds a `DoubleMaRequest` with `Backend::Auto { device_id }`,
- and when there is no explicit prescan config, the code defaults to `device_id = 0`,
- so the prescan phase can ignore the already-normalized backend/device intent.

### Required behavior

When prescan is **implicit** (auto-triggered, not explicitly requested via `req.gpu_prescan`), inherit the backend/device from the already normalized outer request.

### Expected helper

Add a helper equivalent to:

```rust
#[cfg(feature = "cuda")]
fn inherited_prescan_backend(normalized_backend: &Backend) -> Backend {
    match normalized_backend {
        Backend::Auto { device_id }
        | Backend::GpuOnly { device_id }
        | Backend::GpuHybridKernel { device_id }
        | Backend::GpuKernel { device_id } => Backend::Auto {
            device_id: *device_id,
        },
        Backend::CpuOnly => Backend::CpuOnly,
    }
}
```

Then use it when constructing the stage-0 `DoubleMaRequest` for implicit prescan.

### Rules

- If `req.gpu_prescan.is_some()`, explicit config can continue to drive the prescan backend/device.
- If prescan is implicit, inherit from the normalized outer backend instead of defaulting to `Auto { device_id: 0 }`.
- Keep CPU-only behavior compile-safe when CUDA features are not enabled.

---

### 3E. Add opt-in timing instrumentation for the tiled plain `CompositeCpu` hot path

The plan explicitly says to instrument before doing medium-risk hot-loop refactors.

You must add opt-in phase timing to `optimize_composite_cpu_tiled` (or the equivalent plain tiled hot path) so it can report:

- `fast_tile_ms`
- `slow_tile_ms`
- `pair_eval_ms`
- `finalist_exactify_ms`

### Required behavior

- Guard the timing log behind an environment variable such as `VECTORBT_COMPOSITE_CPU_TIMING`.
- Do not change hot-loop math/semantics just to add timing.
- Emit one concise log line with the counters and processed count.
- The instrumentation should be cheap when disabled.

### Expected shape

The plan’s intent is roughly:

- time fast tile matrix creation once per fast chunk,
- time slow tile matrix creation once per slow chunk,
- time the pair-eval region separately,
- time the finalist exactification block separately,
- print the values only when the env var is set.

Do **not** implement the bounded slow-tile cache yet unless you have very strong evidence and the rest of the patch is already done.

---

### 3F. Only touch plain `CompositeCpu` route-label semantics if the new evidence shows a real mismatch

The main plan flags one open question: the plain `CompositeCpu` path may set `backend_route` too early (for example when CUDA indicator preparation succeeds) even if the actual evaluator remains CPU-side.

Current repo-state note:

- existing backend tests already encode route strings such as `GpuKernel->CompositeCpu` on some CPU-fallback shapes,
- so treat this as an evidence-driven sanity check, not as a mandatory route-metadata rewrite.

While you are already adding route diagnostics and the benchmark route probe, sanity-check:

- the plain `CompositeCpu` route-label behavior,
- when `backend_route` is assigned versus when `backend_used` switches to GPU,
- whether the new route diagnostics / route probe show a real mismatch.

### Required handling

- If you find that `backend_route` clearly overstates real GPU evaluator usage, fix it in the smallest safe way.
- Prefer separating concepts such as **indicator staging route** vs **optimizer evaluator route** rather than changing unrelated behavior.
- If the evidence is ambiguous, do not force a redesign; leave behavior as-is and note it in your final summary.

Do not turn this into a broad route-metadata rewrite.

---

## 4. Benchmarks and benchmark-fidelity cleanup

## 4A. Reshape `crates/app_core/benches/composite_cpu_opt.rs`

The current plan says this benchmark file does not honestly reflect the current product path.

You must split it into clearly named benchmark groups.

### Required benchmark groups

1. **plain product proxy**
   - `heatmap_bins = 0`
   - `include_all = false`
   - `gpu_prescan = None`
2. **plain include_all proxy**
   - `heatmap_bins = 0`
   - `include_all = true`
3. **engine-only heatmap**
   - `heatmap_bins = 64`
4. **engine-only stage-0 prescan**
   - keep only if useful, but rename so it is obviously engine-only and not the default product proxy

### Naming guidance

Use names equivalent to the plan’s examples:

- `composite_cpu/plain_product_proxy/50k`
- `composite_cpu/plain_include_all/50k`
- `composite_cpu/engine_heatmap/50k`
- `composite_cpu/engine_stage0_prescan_double_ma/50k`

Do not keep a benchmark name that looks like “default grid” while actually measuring heatmap or prescan-heavy shapes.

### Optional but encouraged benchmark add-on

If it is easy and keeps the file readable, add one small **engine-only non-return-stat objective** benchmark variant (for example `Pnl` or `Calmar`) so the finalist exactification path remains visible in measurements.

This is optional. Do not let it complicate the required benchmark split.

---

## 4B. Add metadata printing + route probe to `composite_cpu_opt.rs`

The addendum explicitly says to port the metadata/route-probe style used by `composite_validation_workflow.rs` into `composite_cpu_opt.rs`.

### Required behavior

- Add a small `print_bench_metadata(group_name, details)` helper similar to the one in `composite_validation_workflow.rs`.
- Before the measured benchmark body, print metadata for the benchmark group.
- Also run one route probe before timing and print:
  - `backend_used`
  - `backend_route`
  - `backend_fallback_reason`

This is an observability improvement and should not change the timed benchmark body.

---

## 4B.5. `src-tauri/src/commands_optimize.rs` is already functionally aligned

The current repo already matches the intended behavior here: standalone `CompositeCpu` product requests are Grid-only.

### Repo-state handling

- No functional change is currently required.
- Leave this file untouched unless you need a tiny clarifying comment while already touching nearby code.

---

## 4C. `ta_workflow_regression.rs` is already the main section-11 app-path proxy

Do **not** casually change the existing plain `CompositeCpu` workflow proxy shape.

The current repo already uses the intended section-11 product proxy shape.

### Repo-state handling

- No functional change is currently required.
- Keep the current plain `CompositeCpu` request shape unchanged.
- A tiny clarifying comment is optional, but do not add a second workflow case in this patch unless you discover you need it for a narrowly scoped regression and it stays trivial.

---

## 4D. `crates/app_core/benches/composite_validation_workflow.rs` is already aligned

This file is validation-cost evidence, not the main section-11 optimizer request-shape proxy. It already contains the benchmark metadata + route-probe pattern you should mirror in `composite_cpu_opt.rs`.

### Repo-state handling

- No functional change is required.
- A short clarifying comment is optional.

### Do not do

- Do not repurpose it as the primary section-11 optimizer benchmark.
- Do not add unnecessary logic churn here.

---

## 5. Tests you must add or update

## 5A. Frontend tests

### `web/src/services/optimizeService.test.ts`

This file already exists. Extend it.

Add or update tests that prove the service layer now forwards:

1. standalone shortlist controls,
2. ROCB custom parameters,
3. VCET custom parameters and toggles.

### Minimum expected cases

#### Standalone shortlist passthrough

Use `buildOptimizationRequestFromVueStores()` (or the closest existing service test entry point) and set up store state for something like:

- `strategyKind = "Vemr"`
- `backend = "GpuKernel"`
- `objective = "Sharpe"`
- `includeAll = "true"`
- `extraShortlistObjectives = "Pnl,Omega"`
- `shortlistUnionMode = "weighted-rank"`
- `shortlistConfidenceMode = "DSR"`

Expected request semantics:

- `strategy_kind: "Vemr"`
- `backend: { backend: "GpuKernel", device_id: 0 }`
- `include_all: true`
- `extra_shortlist_objectives: ["Pnl", "Omega"]`
- `shortlist_union_mode: "weighted_rank"`
- `shortlist_confidence_mode: "dsr"`

#### ROCB custom parameter passthrough

Set store state for `strategyKind = "Rocb"` and custom values such as:

- `channel_period = 37`
- `atr_period = 21`
- `atr_stop_mult = 2.75`
- `max_trade_bars = 95`

Expected request semantics:

```ts
{
  strategy_kind: "Rocb",
  rocb: {
    channel_period: 37,
    atr_period: 21,
    atr_stop_mult: 2.75,
    max_trade_bars: 95,
  },
}
```

---

#### VCET custom parameter passthrough

Set store state for `strategyKind = "Vcet"` and custom values such as:

- `vcetMapFastToVidyaShortPeriod = false`
- `vcetMapSlowToVidyaLongPeriod = false`
- `vcetMapSlowToCePeriod = false`
- `vcetVidyaShortPeriod = 7`
- `vcetVidyaLongPeriod = 31`
- `vcetVidyaAlpha = 0.24`
- `vcetCePeriod = 18`
- `vcetCeMult = 2.6`
- `vcetCeUseClose = false`
- `vcetRequireVidyaSlope = false`
- `vcetMaxTradeBars = 77`

Expected request semantics:

```ts
{
  strategy_kind: "Vcet",
  vcet: {
    map_fast_to_vidya_short_period: false,
    map_slow_to_vidya_long_period: false,
    map_slow_to_ce_period: false,
    vidya_short_period: 7,
    vidya_long_period: 31,
    vidya_alpha: 0.24,
    ce_period: 18,
    ce_mult: 2.6,
    ce_use_close: false,
    require_vidya_slope: false,
    max_trade_bars: 77,
  },
}
```

---

### `web/src/utils/strategyOptimizationCapabilities.test.ts`

This file already exists. Extend it.

Add or update capability tests that prove legacy aliases are treated as kernel-capable for the standalone family.

Minimum expected checks:

- `describeStrategyOptimizationCapabilities("Vemr", { requestedBackend: "GpuOnly" })`
  - `supportsExtraShortlistObjectives === true`
- `describeStrategyOptimizationCapabilities("Vemr", { requestedBackend: "GpuHybridKernel" })`
  - `supportsShortlistConfidenceMode === true`
- the standalone-family `outputSummary` explicitly mentions:
  - extra shortlist heads and union reranking require `include_all=true`
  - confidence reranking requires `top_k > 0`
  - no Double-MA heatmap / per-pair CSV export is emitted

---

### `web/src/services/optimizeRequestBuilder.test.ts`

Keep existing tests, and add one explicit regression proving that plain `CompositeCpu` still rejects shortlist extras even if the requested backend is GPU.

Use a case semantically equivalent to:

- `strategy_kind = "CompositeCpu"`
- `backend = "GpuKernel"`
- `includeAll = "true"`
- `extraShortlistObjectives = "Pnl"`

Expected behavior:

- builder throws with the existing standalone unified-GPU route restriction.

Do not weaken that validation.

---

## 5B. Rust tests in `crates/app_core/src/composite_cpu.rs`

Add or update backend tests for at least the following:

### MA param round-trip preservation

Add a plain `CompositeCpu` test using non-default ALMA parameter maps and assert:

- `res.best_params.fast_ma_params == req.fast_ma_params`
- `res.best_params.slow_ma_params == req.slow_ma_params`

The plan refers to helper-style tests using existing test helpers such as `make_trending_candles(...)` and `make_base_composite_req()`.

Follow the existing local test style in that file.

### Auto-prescan route stability

Add a test that specifically covers the **implicit/auto** prescan threshold interaction, not just explicit prescan.

The key acceptance condition is that a large enough plain `CompositeCpu` request that explicitly asks for `Backend::GpuKernel`:

- does not silently auto-prescan into an `allowed_pairs` route cliff, and/or
- emits the explicit skip diagnostic showing auto-prescan was skipped to preserve the requested plain `GpuKernel` route.

Do not make this test depend on actual CUDA availability if you can avoid it.

---

# Things you must not do

1. Do not widen the product surface.
2. Do not re-enable heatmap/export-all for standalone `CompositeCpu`.
3. Do not add kernel `allowed_pairs` support in this patch train.
4. Do not perform unsafe allocation/copy-elision refactors in the same change set.
5. Do not change `commands_optimize.rs` functional behavior for standalone `CompositeCpu` unless a true bug is proven; a clarifying comment is fine.
6. Do not convert this task into a broad GPU redesign.
7. Do not remove existing internal prescan support used by explicit callers/tests.

---

# Suggested implementation order

Use this order unless the repo layout forces a minor adjustment:

1. `web/src/services/optimizeService.ts`
   - shortlist forwarding
   - ROCB forwarding
   - VCET bindings + forwarding
2. frontend tests
   - `optimizeService.test.ts`
   - `strategyOptimizationCapabilities.test.ts`
   - `optimizeRequestBuilder.test.ts`
3. `web/src/utils/strategyOptimizationCapabilities.ts`
   - legacy alias normalization
   - output summary clarification
4. `web/src/services/optimizeRequestBuilder.ts`
   - constants/comments only
5. `crates/app_core/src/composite_cpu.rs`
   - MA param preservation
   - route diagnostics
   - auto-prescan gate fix
   - prescan backend/device inheritance
   - timing instrumentation
   - tests
6. `crates/app_core/benches/composite_cpu_opt.rs`
   - benchmark split
   - benchmark metadata
   - route probe
7. leave `src-tauri/src/commands_optimize.rs`, `crates/app_core/benches/composite_validation_workflow.rs`, and `src-tauri/src/bin/ta_workflow_regression.rs` functionally unchanged unless a tiny comment is genuinely helpful

---

# Repo-state-validated implementation checklist

Implement the outstanding items below. Do not spend time “re-adding” behavior that the repo already has.

## Frontend service + capability work

- [x] In `web/src/services/optimizeService.ts`, forward `extraShortlistObjectives`.
- [x] In `web/src/services/optimizeService.ts`, forward `shortlistConfidenceMode`.
- [x] In `web/src/services/optimizeService.ts`, forward `shortlistUnionMode`.
- [x] In `web/src/services/optimizeService.ts`, forward the missing ROCB fields required for the exposed service path.
- [x] In `web/src/services/optimizeService.ts`, add the missing VCET text/checkbox store bindings.
- [x] In `web/src/services/optimizeService.ts`, forward the exposed VCET fields into `buildCompositeCpuOptimizationRequest(...)`.
- [x] In `web/src/utils/strategyOptimizationCapabilities.ts`, treat `GpuOnly` as kernel-capable for standalone-family capability preview.
- [x] In `web/src/utils/strategyOptimizationCapabilities.ts`, treat `GpuHybridKernel` as kernel-capable for standalone-family capability preview.
- [x] In `web/src/utils/strategyOptimizationCapabilities.ts`, tighten standalone-family output copy so it states the real `include_all` / `top_k` prerequisites and explicitly says no Double-MA heatmap / per-pair CSV export is produced.
- [x] In `web/src/services/optimizeRequestBuilder.ts`, replace the hard-coded standalone `CompositeCpu` `mode: "Grid"` / `heatmap_bins: 0` literals with named constants or an equally clear comment-backed shape.

## Frontend tests

- [x] Extend `web/src/services/optimizeService.test.ts` with a standalone shortlist passthrough regression.
- [x] Extend `web/src/services/optimizeService.test.ts` with a ROCB custom-parameter passthrough regression.
- [x] Extend `web/src/services/optimizeService.test.ts` with a VCET custom-parameter/toggle passthrough regression.
- [x] Extend `web/src/utils/strategyOptimizationCapabilities.test.ts` with standalone alias coverage for `GpuOnly`.
- [x] Extend `web/src/utils/strategyOptimizationCapabilities.test.ts` with standalone alias coverage for `GpuHybridKernel`.
- [x] Extend `web/src/utils/strategyOptimizationCapabilities.test.ts` with output-summary assertions for the new standalone prerequisite copy.
- [x] Extend `web/src/services/optimizeRequestBuilder.test.ts` with an explicit regression proving plain `CompositeCpu` still rejects standalone shortlist extras on requested GPU routes.

## Backend `CompositeCpu` work

- [x] In `crates/app_core/src/composite_cpu.rs`, preserve `fast_ma_params` / `slow_ma_params` in plain `CompositeCpu` result shaping.
- [x] In `crates/app_core/src/composite_cpu.rs`, add plain `CompositeCpu` route diagnostics and return them when non-empty.
- [x] In `crates/app_core/src/composite_cpu.rs`, skip implicit auto-prescan for plain `CompositeCpu` requests that explicitly asked for `Backend::GpuKernel`.
- [x] In `crates/app_core/src/composite_cpu.rs`, make implicit prescan inherit the normalized outer backend/device instead of defaulting to `Auto { device_id: 0 }`.
- [x] In `crates/app_core/src/composite_cpu.rs`, add opt-in timing instrumentation for the tiled plain `CompositeCpu` hot path.
- [x] While adding route diagnostics / route probes, sanity-check plain `CompositeCpu` route-label semantics and only change them if the evidence shows a real mismatch.
- [x] Extend backend tests with MA-param round-trip coverage.
- [x] Extend backend tests with implicit auto-prescan route-stability coverage.

## Benchmark work

- [x] Split `crates/app_core/benches/composite_cpu_opt.rs` into clearly named product-proxy and engine-only benchmark groups.
- [x] Add benchmark metadata printing to `crates/app_core/benches/composite_cpu_opt.rs`.
- [x] Add a pre-timing route probe to `crates/app_core/benches/composite_cpu_opt.rs`.
- [x] Keep `src-tauri/src/commands_optimize.rs`, `crates/app_core/benches/composite_validation_workflow.rs`, and `src-tauri/src/bin/ta_workflow_regression.rs` functionally unchanged unless a tiny comment is truly helpful.

## Verification + handoff

- [x] Run the targeted frontend tests for `optimizeRequestBuilder.test.ts`, `optimizeService.test.ts`, and `strategyOptimizationCapabilities.test.ts` with the repo’s actual package-manager/test-runner command.
- [x] Run the targeted Rust tests needed to validate the `CompositeCpu` backend changes.
- [x] Run `cargo bench --bench composite_cpu_opt --no-run`.
- [ ] Run `cargo bench --bench composite_validation_workflow --no-run`.
- [x] Run a lightweight `ta_workflow_regression` smoke if feasible, or explicitly record why it was skipped.
- [x] In the final write-up, list files changed, checks run, optional items intentionally skipped, and any grounded remaining ambiguity.

Verification note:
`cargo bench -p ta_app_core --bench composite_validation_workflow --no-run` currently fails outside this section-11 patch because that existing bench references multiple candidate-builder helpers that are cfg-gated behind `cuda` / `cuda-backtest-kernel` while its CPU bench body still calls them in the default build.

---

# Validation and test execution requirements

Run the most relevant checks you can inside the environment.

## Minimum expected checks

### Frontend

Use the repo’s configured Vitest/test runner. If `pnpm`/Vitest is available, run targeted tests equivalent to:

```bash
pnpm vitest run \
  web/src/services/optimizeRequestBuilder.test.ts \
  web/src/services/optimizeService.test.ts \
  web/src/utils/strategyOptimizationCapabilities.test.ts
```

If the repo uses a different package manager wrapper, use the local equivalent.

### Rust

Run targeted Rust tests for the changed backend logic and ensure the benches still compile.

At minimum, do a targeted test/build pass equivalent to:

```bash
cargo test
cargo bench --bench composite_cpu_opt --no-run
cargo bench --bench composite_validation_workflow --no-run
```

If full `cargo test` is too heavy for the environment, run the narrowest meaningful subset and say exactly what you ran.

### Workflow smoke check

If feasible, run a lightweight smoke of `ta_workflow_regression.rs` with small inputs to ensure nothing broke the section-11 app-path proxy. For example, something equivalent to:

```bash
cargo run --bin ta_workflow_regression -- \
  --out /tmp/ta_workflow_regression_smoke.json \
  --candles-len 1000 \
  --train-assets 2 \
  --holdout-assets 1
```

If that is not feasible, explain why.

### CUDA/GPU rule

Do not make the patch depend on GPU availability in the local environment. Keep GPU-gated logic behind existing cfgs and route checks.

---

# Acceptance criteria

Your implementation is only complete if all of the following are true:

1. `optimizeService.ts` no longer drops standalone shortlist controls.
2. `optimizeService.ts` no longer drops ROCB custom parameters.
3. `optimizeService.ts` no longer drops the already-exposed VCET parameter/toggle surface.
4. Plain `CompositeCpu` result shaping preserves MA parameter maps.
5. Plain `CompositeCpu` route diagnostics are returned when relevant.
6. Auto-prescan no longer silently sabotages explicitly requested plain `CompositeCpu` `GpuKernel` routes.
7. Implicit stage-0 prescan inherits the normalized outer backend/device instead of defaulting to `Auto { device_id: 0 }`.
8. `composite_cpu_opt.rs` clearly distinguishes product-proxy and engine-only benchmark shapes.
9. `composite_cpu_opt.rs` prints benchmark metadata and a route probe before timed runs.
10. Capability summaries no longer drift when legacy GPU aliases are present and their standalone copy matches the current prerequisites.
11. Builder behavior remains Grid-only / no-heatmap for standalone `CompositeCpu` product requests.
12. Plain `CompositeCpu` still rejects standalone shortlist extras.
13. No product-surface regression is introduced.

---

# Final response format

When you are done, respond with:

## Summary
- one short paragraph

## Files changed
- bullet list of edited files with one-line purpose each

## Tests and checks run
- bullet list with pass/fail or skipped

## Optional items handled
- bullet list of optional items implemented or intentionally skipped

## Notes
- only grounded caveats, if any

Do not output a vague plan. Output implemented results.
