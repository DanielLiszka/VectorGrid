# Section 11 Implementation Plan: `CompositeCpu` optimizer hot paths and request shaping

## Scope and prompt alignment

This document follows the section-11 review shape from `gpt54_feature_review_map.md` and keeps the review scoped to the standalone `CompositeCpu` request family:

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

It explicitly does **not** treat `CompositeTrend`, `RegimeHybrid`, or `BreakoutMomentum` as section-11 product paths. It also respects the current product surface for this bundle: `Top-K` and `include_all` matter, while Double-MA heatmap/export-all assumptions are stale, and `gpu_prescan` tuning belongs to `CompositeMo`, not the standalone `CompositeCpu` frontend path (`gpt54_feature_review_map.md:537-580`).

## Surface under review

This surface is the shared standalone `CompositeCpu` optimizer family across Rust engine code, Tauri command normalization, frontend request building, capability summaries, and supporting benchmarks. The most important questions here are whether the current request shape matches the real product surface, whether route/fallback behavior is transparent and stable, and whether shared hot paths do repeated work that is worth fixing without changing product behavior.

## Files reviewed

Primary files:

- `crates/app_core/src/composite_cpu.rs`
- `src-tauri/src/commands_optimize.rs`
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/services/optimizeService.ts`
- `web/src/utils/strategyOptimizationCapabilities.ts`
- `web/src/services/optimizeRequestBuilder.test.ts`

Supporting benchmark files:

- `crates/app_core/benches/composite_cpu_opt.rs`
- `crates/app_core/benches/composite_validation_workflow.rs`
- `src-tauri/src/bin/ta_workflow_regression.rs`

Adjacent-but-not-attached files that should receive new regression coverage:

- `web/src/services/optimizeService.test.ts`
- `web/src/utils/strategyOptimizationCapabilities.test.ts`

## Extra context

The current frontend product contract is already narrower than the engine surface:

- `optimizeRequestBuilder.ts` emits standalone `CompositeCpu` requests with `mode: "Grid"` and `heatmap_bins: 0` (`optimizeRequestBuilder.ts:3364-3371`).
- `commands_optimize.rs` forces the Tauri command surface to `Grid` for `CompositeCpu` (`commands_optimize.rs:74-76`, `commands_optimize.rs:106-112`).
- `strategyOptimizationCapabilities.ts` marks the standalone family and plain `CompositeCpu` as **not** supporting heatmap or export-all on the current product surface (`strategyOptimizationCapabilities.ts:313-365`).
- `ta_workflow_regression.rs` already uses a shape that matches the intended section-11 product proxy much more closely than the Criterion bench: `CpuOnly`, `Grid`, `top_k=64`, `include_all=false`, `heatmap_bins=0`, `gpu_prescan=None` (`ta_workflow_regression.rs:442-478`).

The attached engine code still supports broader internal shapes such as heatmap, explicit `gpu_prescan`, and a base plain-`CompositeCpu` `GpuKernel` path. That is fine at the engine level, but it means benchmarks and frontend summaries can drift if they are not kept tightly aligned.

---

# 1. Surface understanding

The current surface is split into three different layers, and they are **not** perfectly aligned.

## 1.1 Frontend product surface

The request builder and capability summary are already trying to enforce the current section-11 product contract:

- standalone family requests are always emitted with `mode: "Grid"` and `heatmap_bins: 0` (`optimizeRequestBuilder.ts:3364-3371`)
- standalone family capability summaries say `supportsHeatmap: false` and `supportsExportAll: false` (`strategyOptimizationCapabilities.ts:313-365`)
- plain `CompositeCpu` does not use the global backend selector in the frontend capability layer (`strategyOptimizationCapabilities.ts:347-365`)

That is consistent with the map’s scope guard.

## 1.2 Tauri command surface

`commands_optimize.rs` is intentionally simple here and does the right thing: it force-normalizes `CompositeCpu` requests to `Grid` mode regardless of input (`commands_optimize.rs:74-76`, `commands_optimize.rs:106-112`, `commands_optimize.rs:241-259`).

This means the main frontend/backend mismatch risk is **not** the Tauri command layer.

## 1.3 Engine surface

`crates/app_core/src/composite_cpu.rs` still exposes a broader engine than the current product path:

- stage-0 prescan can activate either when `req.gpu_prescan.is_some()` **or** when `total_pairs > 250_000` (`composite_cpu.rs:10158-10305`)
- a base plain-`CompositeCpu` `GpuKernel` path exists, but it rejects `allowed_pairs` / prescan-subset execution (`composite_cpu.rs:1199-1209`)
- the shared tiled CPU path still supports internal heatmap work and full `include_all` collection (`composite_cpu.rs:11406-11420`, `11468-11690`)
- plain `CompositeCpu` final result shaping currently drops `fast_ma_params` and `slow_ma_params` from `DoubleMaParamsResolved` even though the request accepted them (`composite_cpu.rs:10493-10502`)

So the engine is more flexible than the product surface, but some of that flexibility currently introduces drift and avoidable confusion.

---

# 2. Findings

## Finding 1 — standalone shortlist controls are implemented in the builder but dropped by the service layer

- **Status:** confirmed issue
- **Severity:** high
- **Type:** correctness / parity
- **Location:** `web/src/services/optimizeService.ts` `tryBuildAutoOptimizationRequest`, compared against `web/src/services/optimizeRequestBuilder.ts` `buildCompositeCpuOptimizationRequest`
- **Timing boundary affected:** app-path
- **Why it matters:**
  - `CompositeCpuOptimizationBuildInputs` explicitly includes `extraShortlistObjectives`, `shortlistConfidenceMode`, and `shortlistUnionMode` (`optimizeRequestBuilder.ts:217-221`).
  - The builder validates and forwards those fields for standalone GPU-native routes (`optimizeRequestBuilder.ts:2843-2928`, `3365-3367`).
  - Builder tests already prove that the request builder accepts and emits those fields on requested standalone `GpuKernel` routes when `includeAll=true` (`optimizeRequestBuilder.test.ts:3189-3210`).
  - But `optimizeService.ts` does **not** pass those three values into `buildCompositeCpuOptimizationRequest` at all (`optimizeService.ts:2204-2350`).
  - The result is a service-layer request that silently drops controls the UI/capability layer says are available.
- **Recommended change:**
  - Forward all three standalone shortlist fields from `optimizeService.ts` into `buildCompositeCpuOptimizationRequest`.
  - Add a service-level regression test in `optimizeService.test.ts` that uses `buildOptimizationRequestFromVueStores()` and proves the fields survive end-to-end for a standalone `GpuKernel` request.

### Proposed code edit

#### `web/src/services/optimizeService.ts`

Add the missing pass-through fields near the existing `objective`, `topK`, and `includeAll` arguments.

```ts
      objective: readStoreTextInputValue("objective"),
      extraShortlistObjectives: readStoreTextInputValue(
        "extraShortlistObjectives",
      ),
      shortlistConfidenceMode: readStoreTextInputValue(
        "shortlistConfidenceMode",
      ),
      shortlistUnionMode: readStoreTextInputValue("shortlistUnionMode"),
      topK: readStoreTextInputValue("topK"),
      includeAll: readStoreTextInputValue("includeAll"),
```

### Recommended regression test (adjacent file)

`web/src/services/optimizeService.test.ts`

```ts
it("forwards standalone shortlist controls from Vue stores into CompositeCpu-family requests", async () => {
  // Arrange with the existing Pinia helpers used by the service tests:
  // - optimize.strategyKind = "Vemr"
  // - optimize.backend = "GpuKernel"
  // - advanced.objective = "Sharpe"
  // - advanced.includeAll = "true"
  // - advanced.extraShortlistObjectives = "Pnl,Omega"
  // - advanced.shortlistUnionMode = "weighted-rank"
  // - advanced.shortlistConfidenceMode = "DSR"

  const req = await buildOptimizationRequestFromVueStores()

  expect(req).toMatchObject({
    strategy_kind: "Vemr",
    backend: { backend: "GpuKernel", device_id: 0 },
    include_all: true,
    extra_shortlist_objectives: ["Pnl", "Omega"],
    shortlist_union_mode: "weighted_rank",
    shortlist_confidence_mode: "dsr",
  })
})
```

---

## Finding 2 — plain `CompositeCpu` optimization results drop MA parameter maps

- **Status:** confirmed issue
- **Severity:** high
- **Type:** correctness / parity
- **Location:** `crates/app_core/src/composite_cpu.rs` result-shaping closure inside `run_composite_cpu_optimization_blocking_with_candles`
- **Timing boundary affected:** app-path
- **Why it matters:**
  - The plain `CompositeCpu` request accepts `fast_ma_params` and `slow_ma_params` (`composite_cpu.rs:370-420`).
  - The optimizer uses those parameter maps during execution (`composite_cpu.rs:10132-10135`, `10377-10411`).
  - But the final `DoubleMaParamsResolved` emitted for `best_params`, `top`, and `all` hardcodes both param maps to `None` (`composite_cpu.rs:10493-10502`).
  - Standalone strategy-specific paths do preserve the MA params in their `resolve_params` closures, so plain `CompositeCpu` is inconsistent with the rest of the section-11 family.
  - This can break exact follow-up drilldown or saved-result replay whenever non-default MA params were part of the winning request.
- **Recommended change:**
  - Preserve `req.fast_ma_params.clone()` and `req.slow_ma_params.clone()` in the plain `CompositeCpu` result path.
  - Add a unit test that exercises non-default ALMA parameter maps and asserts the returned `best_params` preserve them.

### Proposed code edit

#### `crates/app_core/src/composite_cpu.rs`

Replace the plain `CompositeCpu` `resolve_params` closure with the following:

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

### Recommended regression test

#### `crates/app_core/src/composite_cpu.rs`

```rust
#[test]
fn composite_cpu_result_preserves_ma_params() {
    use std::collections::HashMap;

    let candles = make_trending_candles(512);
    let cancel = AtomicBool::new(false);
    let mut req = make_base_composite_req();

    req.fast_ma_types = vec!["alma".to_string()];
    req.slow_ma_types = vec!["alma".to_string()];
    req.fast_ma_params = Some(HashMap::from([
        ("offset".to_string(), 0.85),
        ("sigma".to_string(), 6.0),
    ]));
    req.slow_ma_params = Some(HashMap::from([
        ("offset".to_string(), 0.90),
        ("sigma".to_string(), 7.0),
    ]));

    let res = run_composite_cpu_optimization_blocking_with_candles(
        req.clone(),
        &candles,
        &cancel,
        None,
    )
    .expect("plain CompositeCpu run should succeed");

    assert_eq!(res.best_params.fast_ma_params, req.fast_ma_params);
    assert_eq!(res.best_params.slow_ma_params, req.slow_ma_params);
}
```

---

## Finding 3 — `composite_cpu_opt.rs` no longer matches the current product path and its benchmark names are misleading

- **Status:** confirmed issue
- **Severity:** high
- **Type:** benchmark-fidelity
- **Location:** `crates/app_core/benches/composite_cpu_opt.rs`
- **Timing boundary affected:** Criterion
- **Why it matters:**
  - The current product path for this bundle is `Grid + Top-K + include_all + no heatmap/export-all + no frontend gpu_prescan` (`gpt54_feature_review_map.md:556-580`, `optimizeRequestBuilder.ts:3364-3371`, `commands_optimize.rs:74-76`).
  - The main Criterion bench still calls the plain request `composite_cpu/grid/50k` while setting `heatmap_bins: Some(64)` (`composite_cpu_opt.rs:40-42`, `79-81`).
  - The second bench explicitly measures a prescan shape with `gpu_prescan: Some(...)` (`composite_cpu_opt.rs:108-160`), even though section 11 says `gpu_prescan` tuning belongs to `CompositeMo`, not the standalone `CompositeCpu` frontend path.
  - The current names therefore encourage reviewers to read engine-only and prescan-heavy shapes as if they were the default product shape.
- **Recommended change:**
  - Split `composite_cpu_opt.rs` into explicit benchmark groups:
    1. **plain product proxy** (`heatmap_bins=0`, `include_all=false`, `gpu_prescan=None`)
    2. **plain product proxy with `include_all=true`** to expose the serial full-row path
    3. **engine-only heatmap** (`heatmap_bins=64`) if you still want that measurement
    4. **engine-only stage-0 prescan** if you still want that measurement
  - Rename benchmark groups so no engine-only bench looks like the default product path.

### Proposed code edit

#### `crates/app_core/benches/composite_cpu_opt.rs`

Use a shared request constructor, then split the benchmarks by intent.

```rust
fn make_plain_req() -> CompositeCpuRequest {
    CompositeCpuRequest {
        backend: Backend::CpuOnly,
        asr: None,
        vcc: None,
        vemr: None,
        rocb: None,
        vcet: None,
        mkcs: None,
        kqsr: None,
        carm: None,
        kvdf: None,
        cdmc: None,
        data_id: "bench".to_string(),
        fast_range: (5, 80, 1),
        slow_range: (90, 250, 1),
        fast_ma_types: vec!["sma".to_string()],
        slow_ma_types: vec!["sma".to_string()],
        ma_source: "close".to_string(),
        fast_ma_params: None,
        slow_ma_params: None,
        strategy: StrategyConfig::default(),
        halftrend: ta_app_core::double_ma::HalfTrendRegimeFilter {
            amplitude: 2,
            channel_deviation: 2.0,
            atr_period: 100,
        },
        macz: ta_app_core::composite_cpu::MaczGateParams::default(),
        ott: ta_app_core::composite_cpu::OttGateParams::default(),
        signal_mode: ta_app_core::CompositeSignalMode::HardGate,
        objective: ObjectiveKind::Sharpe,
        mode: OptimizationMode::Grid,
        gpu_prescan: None,
        extra_shortlist_objectives: None,
        shortlist_confidence_mode: None,
        shortlist_union_mode: None,
        top_k: Some(50),
        include_all: Some(false),
        heatmap_bins: Some(0),
    }
}

fn bench_composite_cpu_plain_product_proxy(c: &mut Criterion) {
    let candles = make_synth_candles(50_000);
    let cancel = AtomicBool::new(false);
    let req = make_plain_req();

    let mut group = c.benchmark_group("composite_cpu/plain_product_proxy/50k");
    group.measurement_time(Duration::from_secs(8));
    group.sample_size(10);

    group.bench_function(
        "topk50_include_all_false_no_heatmap/fast5-80_slow90-250",
        |b| {
            b.iter(|| {
                let _ = ta_app_core::run_composite_cpu_optimization_blocking_with_candles(
                    req.clone(),
                    &candles,
                    &cancel,
                    None,
                )
                .expect("plain CompositeCpu product proxy run");
            })
        },
    );

    group.finish();
}

fn bench_composite_cpu_plain_include_all(c: &mut Criterion) {
    let candles = make_synth_candles(50_000);
    let cancel = AtomicBool::new(false);
    let mut req = make_plain_req();
    req.include_all = Some(true);

    let mut group = c.benchmark_group("composite_cpu/plain_include_all/50k");
    group.measurement_time(Duration::from_secs(8));
    group.sample_size(10);

    group.bench_function(
        "topk50_include_all_true_no_heatmap/fast5-80_slow90-250",
        |b| {
            b.iter(|| {
                let _ = ta_app_core::run_composite_cpu_optimization_blocking_with_candles(
                    req.clone(),
                    &candles,
                    &cancel,
                    None,
                )
                .expect("plain CompositeCpu include_all proxy run");
            })
        },
    );

    group.finish();
}

fn bench_composite_cpu_engine_heatmap(c: &mut Criterion) {
    let candles = make_synth_candles(50_000);
    let cancel = AtomicBool::new(false);
    let mut req = make_plain_req();
    req.heatmap_bins = Some(64);

    let mut group = c.benchmark_group("composite_cpu/engine_heatmap/50k");
    group.measurement_time(Duration::from_secs(8));
    group.sample_size(10);

    group.bench_function("topk50_heatmap64/fast5-80_slow90-250", |b| {
        b.iter(|| {
            let _ = ta_app_core::run_composite_cpu_optimization_blocking_with_candles(
                req.clone(),
                &candles,
                &cancel,
                None,
            )
            .expect("plain CompositeCpu engine heatmap run");
        })
    });

    group.finish();
}
```

For the current prescan bench, keep the body if you still need it, but rename the group to make it unambiguously engine-only, such as:

```rust
let mut group = c.benchmark_group(
    "composite_cpu/engine_stage0_prescan_double_ma/50k"
);
```

Also update the `criterion_group!` declaration accordingly.

---

## Finding 4 — auto-prescan can silently invalidate the requested plain `CompositeCpu` `GpuKernel` route

- **Status:** confirmed issue
- **Severity:** medium
- **Type:** performance / parity
- **Location:** `crates/app_core/src/composite_cpu.rs` auto-prescan block and `try_optimize_base_composite_gpu_kernel`
- **Timing boundary affected:** app-path / route-matrix
- **Why it matters:**
  - The plain `CompositeCpu` engine auto-enables stage-0 prescan whenever `total_pairs > 250_000`, even if the request did not explicitly ask for it (`composite_cpu.rs:10158-10305`).
  - That prescan narrows the search to `allowed_pairs` (`composite_cpu.rs:10295-10305`).
  - The base plain `CompositeCpu` `GpuKernel` path rejects any request with `allowed_pairs.is_some()` (`composite_cpu.rs:1205-1209`).
  - So a large plain `CompositeCpu` request can cross the auto-prescan threshold and lose eligibility for the specialized base `GpuKernel` route before optimization even starts.
  - Section 11 explicitly says `gpu_prescan` tuning now belongs to `CompositeMo`, not the standalone `CompositeCpu` frontend path, so this auto-trigger is especially easy to misread.
- **Recommended change:**
  - Preserve explicit `req.gpu_prescan` support for internal callers and tests.
  - But do **not** auto-enable stage-0 prescan for plain `CompositeCpu` requests that explicitly asked for `Backend::GpuKernel`.
  - Add route diagnostics so this behavior is visible whenever prescan is applied or intentionally skipped.

### Proposed code edit

#### `crates/app_core/src/composite_cpu.rs`

Add route diagnostics and gate auto-prescan more carefully.

```rust
    let mut route_diagnostics: Vec<String> = Vec::new();
```

```rust
    if include_all {
        route_diagnostics.push(
            "include_all=true forces serial full-row collection in optimize_composite_cpu_tiled"
                .to_string(),
        );
    }
```

Then replace the start of the prescan gate with:

```rust
    let mut allowed_pairs: Option<HashSet<(u16, u16)>> = None;
    let explicit_prescan_requested = req.gpu_prescan.is_some();
    let requested_plain_gpu_kernel = matches!(req.backend, Backend::GpuKernel { .. });

    if explicit_prescan_requested
        || (!requested_plain_gpu_kernel && total_pairs > AUTO_PRESCAN_MIN_PAIRS)
    {
        // keep the existing prescan body
        // ...
        total_pairs = pair_set.len();
        route_diagnostics.push(format!(
            "stage-0 prescan narrowed CompositeCpu search to {} allowed pairs",
            total_pairs
        ));
        allowed_pairs = Some(pair_set);
    } else if total_pairs > AUTO_PRESCAN_MIN_PAIRS && requested_plain_gpu_kernel {
        route_diagnostics.push(
            "auto-prescan skipped to preserve requested plain CompositeCpu GpuKernel route"
                .to_string(),
        );
    }

    if explicit_prescan_requested && requested_plain_gpu_kernel {
        route_diagnostics.push(
            "explicit gpu_prescan may force CPU fallback because the base CompositeCpu \
GpuKernel path does not yet support allowed_pairs"
                .to_string(),
        );
    }
```

Finally, return the diagnostics instead of hardcoding `None`:

```rust
        route_diagnostics: (!route_diagnostics.is_empty()).then_some(route_diagnostics),
```

### Recommended regression test

Add a plain-`CompositeCpu` test that constructs a large enough `GpuKernel` request to cross `AUTO_PRESCAN_MIN_PAIRS` and verifies either:

- `route_diagnostics` contains the explicit skip message, or
- the request remains eligible for the base `GpuKernel` path instead of falling back due to `allowed_pairs`

The current test corpus already exercises explicit prescan behavior (`composite_cpu.rs:13040-13164`), so this new test should focus on the **auto** path rather than the explicit path.

---

## Finding 5 — `optimize_composite_cpu_tiled` recomputes every slow tile once per fast tile and lacks phase visibility

- **Status:** likely issue
- **Severity:** medium
- **Type:** performance
- **Location:** `crates/app_core/src/composite_cpu.rs` `optimize_composite_cpu_tiled`
- **Timing boundary affected:** app-path / Criterion
- **Why it matters:**
  - `optimize_composite_cpu_tiled` computes `fast_mat` once per `f_chunk`, but computes `slow_mat` inside the nested `for s_chunk in slow_periods.chunks(ps_tile)` loop that itself is inside the `f_chunk` loop (`composite_cpu.rs:11428-11463`).
  - That means every slow tile is rebuilt once per fast tile.
  - The code does not currently expose phase timing, so it is difficult to prove whether slow-tile rebuild cost is material relative to per-pair evaluation.
  - Section 11 explicitly asks for repeated indicator/cache work review inside the shared `CompositeCpu` path.
- **Recommended change:**
  - First, instrument the tiled path so you can measure `fast_tile_ms`, `slow_tile_ms`, `pair_eval_ms`, and `finalist_exactify_ms` under a debug env flag.
  - Only after measuring, consider a bounded slow-tile cache (for example, precompute/cache all slow tiles only when the estimated memory footprint stays below a fixed ceiling).

### Proposed instrumentation patch

#### `crates/app_core/src/composite_cpu.rs`

```rust
    let mut fast_tile_ms = 0u64;
    let mut slow_tile_ms = 0u64;
    let mut pair_eval_ms = 0u64;
    let mut finalist_exactify_ms = 0u64;
```

```rust
    for f_chunk in fast_periods.chunks(pf_tile) {
        let fast_t0 = std::time::Instant::now();
        let (fast_mat, cols) = ma_matrix_with_backend_f64(
            candles,
            fast_ma_type,
            ma_source,
            f_chunk,
            fast_ma_params,
            fast_alma_offset,
            fast_alma_sigma,
            Some(indicator_backend),
            #[cfg(feature = "cuda")]
            ma_source_f32,
            #[cfg(not(feature = "cuda"))]
            None,
        )?;
        fast_tile_ms += fast_t0.elapsed().as_millis() as u64;

        for s_chunk in slow_periods.chunks(ps_tile) {
            let slow_t0 = std::time::Instant::now();
            let (slow_mat, cols2) = ma_matrix_with_backend_f64(
                candles,
                slow_ma_type,
                ma_source,
                s_chunk,
                slow_ma_params,
                slow_alma_offset,
                slow_alma_sigma,
                Some(indicator_backend),
                #[cfg(feature = "cuda")]
                ma_source_f32,
                #[cfg(not(feature = "cuda"))]
                None,
            )?;
            slow_tile_ms += slow_t0.elapsed().as_millis() as u64;

            let pair_t0 = std::time::Instant::now();
            if include_all {
                // existing include_all branch
            } else {
                // existing parallel tile branch
            }
            pair_eval_ms += pair_t0.elapsed().as_millis() as u64;
        }
    }
```

```rust
    let finalist_t0 = std::time::Instant::now();
    if !objective_needs_return_stats(objective) && !include_all {
        // existing exactification block
    }
    finalist_exactify_ms += finalist_t0.elapsed().as_millis() as u64;

    if std::env::var_os("VECTORBT_COMPOSITE_CPU_TIMING").is_some() {
        eprintln!(
            "[composite-cpu-timing] fast_tile_ms={} slow_tile_ms={} pair_eval_ms={} finalist_exactify_ms={} processed={}",
            fast_tile_ms,
            slow_tile_ms,
            pair_eval_ms,
            finalist_exactify_ms,
            processed,
        );
    }
```

### Optional medium-risk follow-up after measurement

If the timing log shows slow-tile rebuilds are materially expensive, add a bounded cache only when the total slow-tile footprint is below a fixed memory ceiling. Do **not** make that change first.

---

## Finding 6 — `include_all=true` is not just a result-shaping toggle; it flips execution shape to a serial full-row path

- **Status:** confirmed issue
- **Severity:** medium
- **Type:** performance / benchmark-fidelity
- **Location:** `crates/app_core/src/composite_cpu.rs` `optimize_composite_cpu_tiled`
- **Timing boundary affected:** app-path / Criterion
- **Why it matters:**
  - The `include_all` branch uses a serial nested loop (`composite_cpu.rs:11468-11546`), while the non-`include_all` branch uses Rayon with chunk-local `StreamAggregator`s and `try_reduce` (`composite_cpu.rs:11547-11687`).
  - That means `include_all=true` is not only about returning more rows; it is also an execution-path switch with different scaling behavior.
  - The frontend capability summaries currently describe `Top-K` and `include_all`, but the product/benchmark surface does not make this cost change obvious.
- **Recommended change:**
  - Treat `include_all=true` as its own benchmark shape in `composite_cpu_opt.rs`.
  - Surface the execution-shape change in `route_diagnostics` and capability copy.
  - Do **not** parallelize the full-row path until you have measurements showing the serial collector is a real user-facing bottleneck.

### Proposed copy update

#### `web/src/utils/strategyOptimizationCapabilities.ts`

Tighten the standalone-family summary so it matches builder constraints and explains that these controls are not free.

```ts
      outputSummary: standaloneGpuKernelPreview
        ? `${kind} supports Top-K and include-all native candidate returns. On the requested standalone GpuKernel route it also supports host-side extra shortlist heads, union reranking, and retained-shortlist confidence reranking. Extra shortlist heads and union reranking currently require include_all=true, and confidence reranking requires top_k>0. It does not emit the Double MA heatmap or per-pair CSV export.`
        : `${kind} supports Top-K and include-all native candidate returns, but does not emit the Double MA heatmap or per-pair CSV export.`,
```

---

## Finding 7 — capability summaries under-report support when the frontend still uses legacy GPU backend aliases

- **Status:** confirmed issue
- **Severity:** medium
- **Type:** parity / maintainability
- **Location:** `web/src/utils/strategyOptimizationCapabilities.ts` `requestedGlobalBackendIsGpuKernel`
- **Timing boundary affected:** app-path
- **Why it matters:**
  - `optimizeRequestBuilder.ts` still normalizes legacy `GpuOnly` and `GpuHybridKernel` aliases into unified `GpuKernel` payloads (`optimizeRequestBuilder.ts:792-801`).
  - But `strategyOptimizationCapabilities.ts` treats only the exact string `"GpuKernel"` as kernel-capable (`strategyOptimizationCapabilities.ts:107-109`).
  - That means capability exposure can under-report standalone shortlist support or exact-kernel support when a legacy backend alias is still present in store state, saved settings, or replayed workflow state.
- **Recommended change:**
  - Make `requestedGlobalBackendIsGpuKernel()` match the same alias normalization policy that the request builder already uses.
  - Add a capability regression test for `GpuOnly` and `GpuHybridKernel` aliases.

### Proposed code edit

#### `web/src/utils/strategyOptimizationCapabilities.ts`

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

### Recommended regression tests (adjacent file)

`web/src/utils/strategyOptimizationCapabilities.test.ts`

```ts
it("treats legacy GPU aliases as kernel-capable for standalone CompositeCpu-family summaries", () => {
  expect(
    describeStrategyOptimizationCapabilities("Vemr", {
      requestedBackend: "GpuOnly",
    }).supportsExtraShortlistObjectives,
  ).toBe(true)

  expect(
    describeStrategyOptimizationCapabilities("Vemr", {
      requestedBackend: "GpuHybridKernel",
    }).supportsShortlistConfidenceMode,
  ).toBe(true)
})
```

---

## Finding 8 — route-label semantics for plain `CompositeCpu` need a focused follow-up audit

- **Status:** open question
- **Severity:** low
- **Type:** parity / route-fidelity
- **Location:** `crates/app_core/src/composite_cpu.rs` around `prepared_cuda_inputs`, `indicator_backend`, and `backend_route`
- **Timing boundary affected:** route-matrix / app-path
- **Why it matters:**
  - `backend_route` is assigned as soon as CUDA input preparation succeeds (`composite_cpu.rs:10047-10087`), before the code decides whether the plain request will actually take the base `GpuKernel` evaluator or stay on the CPU tiled path.
  - `backend_used` is only switched to `Gpu` inside `try_optimize_base_composite_gpu_kernel` success (`composite_cpu.rs:10414-10490`).
  - Without the adjacent `composite_profile_route.rs` file, it is not fully safe to claim a bug here, but this is the exact kind of route-summary drift section 11 warns about.
- **Recommended change:**
  - Run a narrow route-matrix audit for plain `CompositeCpu` on `CpuOnly`, `Auto`, `GpuOnly`, and `GpuKernel` inputs.
  - If the route label overstates GPU usage on CPU-evaluated runs, separate “indicator staging route” from “optimizer evaluator route” in the returned diagnostics.

---

# 3. Safe optimization opportunities

These are the low-risk changes I would implement first.

## 3.1 Fix contract drift before touching hot loops

The first changes should be the ones that restore contract parity without changing engine math:

1. `optimizeService.ts` passthrough fix for standalone shortlist controls
2. plain `CompositeCpu` result MA param preservation
3. capability helper alias normalization and clearer copy for shortlist prerequisites

These are all small and high leverage.

## 3.2 Make the benchmark surface honest

Split `composite_cpu_opt.rs` into:

- plain product proxy
- plain `include_all=true` proxy
- engine-only heatmap
- engine-only stage-0 prescan

That gives you cleaner measurement before any engine refactor.

## 3.3 Add route diagnostics and phase timing

Before any cache redesign, make the engine tell you when:

- `include_all=true` forced the serial path
- stage-0 prescan narrowed the pair set
- auto-prescan was skipped to preserve a requested plain `GpuKernel` route
- time is being spent in fast-tile build vs slow-tile build vs pair evaluation vs finalist exactification

These are safe, measurable, and directly actionable.

## 3.4 Keep `commands_optimize.rs` simple

`commands_optimize.rs` already does the right thing by forcing standalone `CompositeCpu` requests to `Grid` (`commands_optimize.rs:74-76`, `241-259`). I would not add behavior here. At most, add a short comment that this is intentional and aligned with the section-11 product surface.

Suggested optional comment:

```rust
// Standalone CompositeCpu-family product requests are Grid-only.
// The frontend already suppresses Double-MA heatmap/export-all shaping on this surface.
fn normalize_product_composite_cpu_optimization_mode(_mode: OptimizationMode) -> OptimizationMode {
    OptimizationMode::Grid
}
```

---

# 4. Higher-risk optimization opportunities

These changes may be worthwhile, but only after the safe work above is in place and measured.

## 4.1 Bounded slow-tile cache across fast chunks

If `VECTORBT_COMPOSITE_CPU_TIMING` shows slow-tile rebuilds are materially expensive, add a bounded slow-tile cache.

Recommended design:

- estimate total slow-tile cache bytes
- only cache when total footprint is below a fixed threshold such as 256 MiB
- otherwise keep the current streaming behavior

I would **not** precompute every slow tile unconditionally.

## 4.2 Parallel full-row collection for `include_all=true`

This is possible, but it is riskier than it sounds because:

- you are no longer only maintaining top-k state
- you now need predictable, memory-heavy merge behavior for all rows
- the real bottleneck may remain per-pair evaluation rather than row collection

Do this only if benchmarks show `include_all=true` is both common and slow enough to matter.

## 4.3 Device-native MA precompute for the base plain `CompositeCpu` `GpuKernel` route

`try_optimize_base_composite_gpu_kernel` currently computes MA matrices on the CPU, converts them to `f32`, and then feeds them into the GPU kernel (`composite_cpu.rs:1231-1269`).

That may be acceptable for an internal or niche path, but if plain `CompositeCpu` `GpuKernel` becomes a serious workload again, this is where the route should eventually be revisited.

I would not do this until:

- route fidelity is fixed
- benchmark drift is cleaned up
- the app-path evidence says this route matters enough to justify more invasive work

## 4.4 Full `allowed_pairs` support in the base plain `GpuKernel` path

Adding prescan-subset support directly to the base `GpuKernel` evaluator would remove the current route cliff entirely, but it is a materially larger kernel-surface change than simply skipping auto-prescan on requested plain `GpuKernel` runs.

That is why it belongs in the second wave, not the first.

---

# 5. Benchmark and perf-gate review

## 5.1 Best section-11 product proxy: `ta_workflow_regression.rs`

Among the attached files, `ta_workflow_regression.rs` is the best current app-path proxy for the section-11 product surface because it already uses:

- plain `CompositeCpu`
- `Backend::CpuOnly`
- `OptimizationMode::Grid`
- `top_k: Some(64)`
- `include_all: Some(false)`
- `heatmap_bins: Some(0)`
- `gpu_prescan: None`

See `ta_workflow_regression.rs:442-478`.

I would keep this file functionally unchanged and treat it as the canonical section-11 workflow gate.

### Optional comment only

```rust
        // Section-11 app-path proxy:
        // plain CompositeCpu + Grid + top_k=64 + include_all=false + heatmap_bins=0 + gpu_prescan=None.
```

## 5.2 `composite_cpu_opt.rs` should stop pretending engine-only shapes are product-default shapes

The current Criterion bench is useful, but only if renamed and split. Right now:

- the “grid” bench is actually `heatmap_bins=64`
- the prescan bench is a stage-0 Double-MA narrowing benchmark, not a default product path benchmark

That is a benchmark-fidelity problem, not a reason to delete the benches.

## 5.3 `composite_validation_workflow.rs` is useful but secondary for section 11

`composite_validation_workflow.rs` is valuable for validation throughput and post-optimization candidate-cost review, but it is **not** the main request-shape benchmark for section 11.

I would leave the logic alone. If you want to make the intent explicit, add a lightweight comment near the benchmark metadata:

```rust
// Validation-cost harness only. Do not use this file as the section-11 optimizer
// product-shape proxy; use ta_workflow_regression.rs and composite_cpu_opt.rs for that.
```

## 5.4 Current perf-gap coverage still misses one path: finalist exactification on non-return-stat objectives

`objective_needs_return_stats()` only returns `true` for `Sharpe`, `Sortino`, `Omega`, and `Volatility` (`composite_cpu.rs:552-560`).

The current section-11 workflow proxy and current Criterion bench both use `Sharpe`, which means the explicit finalist exactification block (`composite_cpu.rs:11698-11799`) is **not** covered by the default benchmark path.

That is fine if `Sharpe` is the only product objective that matters for this surface. If not, add one smaller engine-only Criterion variant for a non-return-stat objective such as `Pnl` or `Calmar` so the exactification path remains visible.

---

# 6. Test review

## 6.1 What is already good

The attached test surfaces already cover several important pieces:

- `commands_optimize.rs` verifies that standalone `CompositeCpu` requests are Grid-only (`commands_optimize.rs:241-259`)
- `optimizeRequestBuilder.test.ts` already verifies that plain `CompositeCpu` is emitted as `mode: "Grid"`, `heatmap_bins: 0`, and `backend: { backend: "CpuOnly" }` (`optimizeRequestBuilder.test.ts:1921-2005`)
- `optimizeRequestBuilder.test.ts` also already verifies that standalone GPU-native shortlist controls are accepted by the builder on requested `GpuKernel` routes when `includeAll=true` (`optimizeRequestBuilder.test.ts:3189-3210`)
- `composite_cpu.rs` already has explicit prescan tests (`composite_cpu.rs:13040-13164`)

So the issue is not lack of builder coverage.

## 6.2 What is missing

The most important missing tests are:

1. **service-layer passthrough test**
   - because the bug lives in `optimizeService.ts`, not the builder
2. **plain `CompositeCpu` MA param round-trip test**
   - because the engine currently drops param maps in its result shape
3. **auto-prescan route-stability test**
   - because current tests only cover explicit prescan, not the auto threshold interaction
4. **capability alias regression tests**
   - because capability exposure should match the builder’s legacy-backend normalization

## 6.3 What I would not add right now

I would not add elaborate new benchmark assertions or force every benchmark file to parse and validate its own metadata. The higher-value test work is still contract and route stability, not benchmark-framework overengineering.

---

# 7. Suggested improvement plan

## Phase 1 — contract parity and user-facing drift

Implement these first:

1. fix `optimizeService.ts` passthrough for standalone shortlist controls
2. normalize legacy GPU alias handling in `strategyOptimizationCapabilities.ts`
3. tighten capability copy so standalone shortlist prerequisites are explicit
4. add the missing service/capability regressions

This is the smallest set of edits that makes the frontend tell the truth.

## Phase 2 — engine correctness and route transparency

Next:

1. preserve `fast_ma_params` / `slow_ma_params` in plain `CompositeCpu` results
2. add `route_diagnostics` for `include_all`, explicit prescan, and auto-prescan skip
3. stop auto-prescan from silently sabotaging requested plain `GpuKernel` routes
4. add the matching engine regression tests

This gives you much better replay/follow-up correctness and route debuggability.

## Phase 3 — benchmark cleanup

Then:

1. split `composite_cpu_opt.rs` into product-proxy vs engine-only groups
2. add an `include_all=true` benchmark shape
3. optionally add one non-return-stat objective benchmark for finalist exactification
4. keep `ta_workflow_regression.rs` unchanged as the main app-path section-11 proxy
5. keep `composite_validation_workflow.rs` unchanged except for optional clarifying comments

This fixes the evidence surface before deeper optimization.

## Phase 4 — measurement-driven engine work

After the above ships, add timing instrumentation to `optimize_composite_cpu_tiled` and run:

- Criterion on the split benchmark set
- workflow regression before/after
- optionally a manual large-range route-matrix probe for plain `GpuKernel`

Only then decide whether slow-tile caching is worth adding.

## Phase 5 — optional medium-risk optimization

If the timing output proves it worthwhile:

1. add a bounded slow-tile cache
2. evaluate whether `include_all=true` merits a chunked parallel collector
3. revisit the base plain `CompositeCpu` `GpuKernel` route if it becomes a real product or research workload again

---

# Appendix A — file-by-file edit plan

## `crates/app_core/src/composite_cpu.rs`

**Must change**

- preserve MA param maps in plain `CompositeCpu` result shaping
- add `route_diagnostics` collection and return it
- guard auto-prescan so it does not silently invalidate requested plain `GpuKernel` routes
- add an opt-in timing log for fast-tile, slow-tile, pair-eval, and finalist-exactify phases
- add unit tests for MA param round-trip and auto-prescan route stability

**Why this file matters most**

This is the section-11 hot path owner and the place where route/fallback, request shape, and repeated tile work all meet.

## `src-tauri/src/commands_optimize.rs`

**No functional change required**

- current Grid-only normalization is already correct for section 11
- optional doc comment only

## `web/src/services/optimizeRequestBuilder.ts`

**Small maintainability change recommended**

- keep the current behavior
- add explicit constants/comments so `mode: "Grid"` and `heatmap_bins: 0` are clearly intentional and not accidental magic values

Suggested cleanup:

```ts
const COMPOSITE_CPU_PRODUCT_MODE = "Grid" as const
const COMPOSITE_CPU_PRODUCT_HEATMAP_BINS = 0
```

Then:

```ts
    // Standalone CompositeCpu-family product surface:
    // Grid only, no Double-MA heatmap/export-all outputs.
    mode: COMPOSITE_CPU_PRODUCT_MODE,
    top_k: topK,
    include_all: includeAll,
    heatmap_bins: COMPOSITE_CPU_PRODUCT_HEATMAP_BINS,
```

## `web/src/services/optimizeService.ts`

**Must change**

- forward standalone shortlist controls

This is the main frontend contract bug in the current attached bundle.

## `web/src/utils/strategyOptimizationCapabilities.ts`

**Should change**

- normalize legacy backend aliases the same way the builder already does
- make standalone shortlist prerequisites explicit in copy
- optionally clarify that plain `CompositeCpu` is a fixed frontend CPU path while the standalone strategies can use CPU or unified GPU execution

## `web/src/services/optimizeRequestBuilder.test.ts`

**Should change**

- keep existing builder tests
- add one more explicit regression that plain `CompositeCpu` rejects shortlist extras even if a GPU backend is requested

Suggested additional test:

```ts
it("rejects shortlist extras on plain CompositeCpu even when a GPU backend is requested", () => {
  expect(() =>
    buildCompositeCpuOptimizationRequest({
      context: {
        strategy_kind: "CompositeCpu",
        data_id: "data-plain",
        current_data_meta: {
          has_open: true,
          has_high: true,
          has_low: true,
          has_close: true,
          has_volume: true,
        },
      },
      backend: "GpuKernel",
      fastStart: "5",
      fastEnd: "15",
      fastStep: "5",
      slowStart: "20",
      slowEnd: "40",
      slowStep: "10",
      fastMaType: "sma",
      slowMaType: "ema",
      maSource: "close",
      objective: "Sharpe",
      includeAll: "true",
      extraShortlistObjectives: "Pnl",
    }),
  ).toThrow(/standalone unified-GPU GpuKernel routes/i)
})
```

## `crates/app_core/benches/composite_cpu_opt.rs`

**Must change**

- split product-proxy vs engine-only benchmark shapes
- add `include_all=true` benchmark shape
- rename the current prescan bench so it does not look like the default product proxy

## `crates/app_core/benches/composite_validation_workflow.rs`

**No functional change required**

- keep as validation-cost evidence
- optionally add a short comment making that role explicit

## `src-tauri/src/bin/ta_workflow_regression.rs`

**No functional change required**

- keep as the canonical section-11 app-path proxy
- optionally add one comment clarifying the current shape so future edits do not accidentally drift

---

# Appendix B — implementation order I would personally use

1. `optimizeService.ts` passthrough fix
2. `strategyOptimizationCapabilities.ts` alias and copy fix
3. `composite_cpu.rs` MA param preservation
4. `composite_cpu.rs` route diagnostics + auto-prescan gate
5. `optimizeRequestBuilder.test.ts` extra plain-`CompositeCpu` regression
6. service/capability regression tests in adjacent test files
7. `composite_cpu_opt.rs` benchmark split
8. `composite_cpu.rs` timing instrumentation
9. optional bounded slow-tile cache only after measurement

---

# Comprehensive implementation checklist

## Phase 1 — frontend contract and capability parity

- [ ] In `web/src/services/optimizeService.ts`, pass `extraShortlistObjectives` into `buildCompositeCpuOptimizationRequest`.
- [ ] In `web/src/services/optimizeService.ts`, pass `shortlistConfidenceMode` into `buildCompositeCpuOptimizationRequest`.
- [ ] In `web/src/services/optimizeService.ts`, pass `shortlistUnionMode` into `buildCompositeCpuOptimizationRequest`.
- [ ] In `web/src/utils/strategyOptimizationCapabilities.ts`, treat `GpuOnly` as kernel-capable for section-11 capability exposure.
- [ ] In `web/src/utils/strategyOptimizationCapabilities.ts`, treat `GpuHybridKernel` as kernel-capable for section-11 capability exposure where the builder already normalizes it.
- [ ] In `web/src/utils/strategyOptimizationCapabilities.ts`, update the standalone-family `outputSummary` text to say that extra shortlist heads and union reranking currently require `include_all=true`.
- [ ] In `web/src/utils/strategyOptimizationCapabilities.ts`, update the standalone-family `outputSummary` text to say that confidence reranking requires `top_k > 0`.
- [ ] In `web/src/services/optimizeRequestBuilder.ts`, add explicit constants/comments for the fixed section-11 product surface (`Grid`, `heatmap_bins=0`).
- [ ] In `web/src/services/optimizeRequestBuilder.test.ts`, add a regression that plain `CompositeCpu` still rejects shortlist extras even when a GPU backend is requested.
- [ ] Add a service regression in `web/src/services/optimizeService.test.ts` covering standalone shortlist passthrough.
- [ ] Add a capability regression in `web/src/utils/strategyOptimizationCapabilities.test.ts` covering `GpuOnly` / `GpuHybridKernel` aliases.

## Phase 2 — engine correctness and route transparency

- [ ] In `crates/app_core/src/composite_cpu.rs`, preserve `req.fast_ma_params.clone()` in plain `CompositeCpu` result shaping.
- [ ] In `crates/app_core/src/composite_cpu.rs`, preserve `req.slow_ma_params.clone()` in plain `CompositeCpu` result shaping.
- [ ] Add a plain `CompositeCpu` engine test proving MA params survive into `best_params`.
- [ ] Add a local `route_diagnostics: Vec<String>` collection in `run_composite_cpu_optimization_blocking_with_candles`.
- [ ] Emit a route diagnostic when `include_all=true` forces the serial full-row path.
- [ ] Emit a route diagnostic when explicit `gpu_prescan` narrows the allowed pair set.
- [ ] Emit a route diagnostic when auto-prescan is skipped to preserve a requested plain `GpuKernel` route.
- [ ] Return `route_diagnostics` instead of hardcoding `None` for plain `CompositeCpu` results.
- [ ] Change the auto-prescan gate so that it does not silently trigger on plain `CompositeCpu` requests that explicitly asked for `Backend::GpuKernel`.
- [ ] Keep explicit `req.gpu_prescan` behavior intact for internal tests and engine-only callers.
- [ ] Add a regression test that covers the auto-prescan threshold interaction separately from explicit prescan.

## Phase 3 — benchmark and perf-gate alignment

- [ ] In `crates/app_core/benches/composite_cpu_opt.rs`, introduce a plain product-proxy bench with `heatmap_bins=0`, `include_all=false`, and `gpu_prescan=None`.
- [ ] In `crates/app_core/benches/composite_cpu_opt.rs`, introduce a plain `include_all=true` bench with `heatmap_bins=0`.
- [ ] In `crates/app_core/benches/composite_cpu_opt.rs`, keep a separate engine-only heatmap bench if that measurement is still useful.
- [ ] In `crates/app_core/benches/composite_cpu_opt.rs`, keep a separate engine-only prescan bench if that measurement is still useful.
- [ ] Rename benchmark groups so engine-only benches no longer look like default product-path benches.
- [ ] Decide whether to add one small non-return-stat benchmark variant (`Pnl` or `Calmar`) to expose finalist exactification cost.
- [ ] In `src-tauri/src/bin/ta_workflow_regression.rs`, keep the current request shape unchanged as the canonical section-11 app-path proxy.
- [ ] Optionally add a short comment in `ta_workflow_regression.rs` documenting the exact section-11 proxy shape.
- [ ] In `crates/app_core/benches/composite_validation_workflow.rs`, keep the logic unchanged and treat it as validation-cost support, not optimizer request-shape evidence.
- [ ] Optionally add a clarifying comment in `composite_validation_workflow.rs` to make that boundary explicit.

## Phase 4 — measurement-driven hot-path work

- [ ] In `crates/app_core/src/composite_cpu.rs`, add opt-in timing counters for `fast_tile_ms`.
- [ ] In `crates/app_core/src/composite_cpu.rs`, add opt-in timing counters for `slow_tile_ms`.
- [ ] In `crates/app_core/src/composite_cpu.rs`, add opt-in timing counters for `pair_eval_ms`.
- [ ] In `crates/app_core/src/composite_cpu.rs`, add opt-in timing counters for `finalist_exactify_ms`.
- [ ] Guard timing logs behind an environment variable such as `VECTORBT_COMPOSITE_CPU_TIMING`.
- [ ] Capture before/after Criterion numbers on the split `composite_cpu_opt.rs` suite.
- [ ] Capture before/after workflow numbers on `ta_workflow_regression.rs`.
- [ ] Decide whether slow-tile rebuild time is large enough to justify a bounded cache.

## Phase 5 — optional medium-risk optimizations

- [ ] If instrumentation shows slow-tile rebuild time is material, add a bounded slow-tile cache with a fixed memory ceiling.
- [ ] Re-run the split Criterion suite after the cache change.
- [ ] Only if `include_all=true` proves to be a real workload, evaluate a chunked parallel all-row collector.
- [ ] Only if plain `CompositeCpu` `GpuKernel` becomes important again, revisit CPU-side MA precompute in `try_optimize_base_composite_gpu_kernel`.
- [ ] Only if explicit prescan + plain `GpuKernel` must coexist, consider adding real `allowed_pairs` support to the base kernel path.

## Verification checklist

- [ ] Run the relevant Rust unit tests after the engine changes.
- [ ] Run the relevant frontend tests after the service/capability changes.
- [ ] Run `cargo bench --bench composite_cpu_opt` after the benchmark split.
- [ ] Run `cargo bench --bench composite_validation_workflow` to confirm the validation harness still behaves as expected.
- [ ] Run `cargo run --bin ta_workflow_regression -- --out <path>` to confirm the workflow gate still represents the intended section-11 product shape.
- [ ] Manually inspect one standalone `GpuKernel` request (for example `Vemr`) and verify the emitted request now contains the shortlist fields.
- [ ] Manually inspect one plain `CompositeCpu` request with non-default MA params and verify the returned `best_params` preserve those params.
- [ ] Manually inspect one large plain `CompositeCpu` `GpuKernel` request and verify the route diagnostics make auto-prescan behavior explicit.

## Done criteria

- [ ] The service no longer drops standalone shortlist controls.
- [ ] Plain `CompositeCpu` results preserve MA parameter maps.
- [ ] Benchmarks clearly distinguish product proxies from engine-only shapes.
- [ ] Route diagnostics make `include_all` and prescan behavior visible.
- [ ] Capability summaries no longer drift on legacy GPU backend aliases.
- [ ] Any deeper hot-path optimization is driven by measured phase timings rather than guesswork.
