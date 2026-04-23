# Section 11 Addendum: additional fixes, missed items, and optional follow-on work

This addendum is meant to sit next to the original `section11_composite_cpu_implementation_plan.md`.
It focuses only on items that were either:

1. under-emphasized or missed in the first draft,
2. useful optional follow-ons that stay aligned with performance, optimization, and correctness,
3. small parity-safe changes that improve observability before deeper hot-path work.

It does **not** widen the bundle into unrelated feature work.

---

## 1. Important missed item: `optimizeService.ts` drops ROCB custom parameters

### Why this matters

The original plan correctly identified one service-layer forwarding bug for standalone shortlist controls, but it missed a second and more direct request-plumbing bug for `Rocb`.

In the current code snapshot:

- `optimizeService.ts` has store bindings for `rocbChannelPeriod`, `rocbAtrPeriod`, `rocbAtrStopMult`, and `rocbMaxTradeBars`.
- the `buildCompositeCpuOptimizationRequest(...)` call in `optimizeService.ts` does **not** forward those fields.
- `optimizeRequestBuilder.ts` does consume those fields and uses them to build the `rocb` request payload.

That means ROCB users can change those values in store-backed state, but the runtime request built through `optimizeService.ts` silently falls back to builder defaults.

This is a pure correctness / parity issue, and it also affects performance reproducibility because the request shape and resulting runtime can differ from the visible UI state.

### Files involved

- `web/src/services/optimizeService.ts`
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/services/optimizeService.test.ts` (adjacent test file to add)

### Current evidence in the attached files

`optimizeService.ts` already binds ROCB fields in `VUE_TEXT_FIELD_BINDINGS`:

```ts
  rocbChannelPeriod: { storeId: "strategy", key: "rocbChannelPeriod" },
  rocbAtrPeriod: { storeId: "strategy", key: "rocbAtrPeriod" },
  rocbAtrStopMult: { storeId: "strategy", key: "rocbAtrStopMult" },
  rocbMaxTradeBars: { storeId: "strategy", key: "rocbMaxTradeBars" },
```

But the `buildCompositeCpuOptimizationRequest(...)` call currently stops at VEMR fields and MA params:

```ts
      vemrAtrPeriod: readStoreTextInputValue("vemrAtrPeriod"),
      vemrAtrStopMult: readStoreTextInputValue("vemrAtrStopMult"),
      vemrConfirmBars: readStoreTextInputValue("vemrConfirmBars"),
      vemrMaxTradeBars: readStoreTextInputValue("vemrMaxTradeBars"),
      fastMaParams: fastParamState.params,
      slowMaParams: slowParamState.params,
```

Meanwhile, the builder does use ROCB inputs when they are provided:

```ts
  if (kind === "Rocb") {
    const mapFastToChannelPeriod =
      inputs.rocbMapFastToChannelPeriod == null
        ? true
        : normalizeBoolean(inputs.rocbMapFastToChannelPeriod);
    const mapSlowToAtrPeriod =
      inputs.rocbMapSlowToAtrPeriod == null
        ? true
        : normalizeBoolean(inputs.rocbMapSlowToAtrPeriod);
    const channelPeriod = Math.floor(num(inputs.rocbChannelPeriod, 20));
    const atrPeriod = Math.floor(num(inputs.rocbAtrPeriod, 14));
    const atrStopMult = num(inputs.rocbAtrStopMult, 2.0);
    const maxTradeBars = Math.floor(num(inputs.rocbMaxTradeBars, 80));
    ...
    rocb = {
      map_fast_to_channel_period: mapFastToChannelPeriod,
      map_slow_to_atr_period: mapSlowToAtrPeriod,
      channel_period: channelPeriod,
      atr_period: atrPeriod,
      atr_stop_mult: atrStopMult,
      max_trade_bars: maxTradeBars,
    };
  }
```

### Recommended code edit

#### `web/src/services/optimizeService.ts`

Add the missing ROCB field forwarding near the existing VEMR forwarding block.

```ts
      rocbChannelPeriod: readStoreTextInputValue("rocbChannelPeriod"),
      rocbAtrPeriod: readStoreTextInputValue("rocbAtrPeriod"),
      rocbAtrStopMult: readStoreTextInputValue("rocbAtrStopMult"),
      rocbMaxTradeBars: readStoreTextInputValue("rocbMaxTradeBars"),
```

Place them before `fastMaParams` / `slowMaParams` so the standalone strategy parameter block remains grouped.

### Recommended regression test

#### `web/src/services/optimizeService.test.ts`

```ts
it("forwards ROCB strategy parameters from Vue stores into the runtime request", async () => {
  // Arrange store state with strategyKind="Rocb" and custom ROCB values.
  // Reuse the same Pinia helpers already used by other optimizeService tests.

  const req = await buildOptimizationRequestFromVueStores()

  expect(req).toMatchObject({
    strategy_kind: "Rocb",
    rocb: {
      channel_period: 37,
      atr_period: 21,
      atr_stop_mult: 2.75,
      max_trade_bars: 95,
    },
  })
})
```

### Rollout priority

High. I would fold this into the same first patch set as the shortlist-field forwarding fix.

---

### Adjacent open question: VCET may have the same service/builder drift in a weaker form

`optimizeRequestBuilder.ts` exposes a real VCET parameter surface (`vcetVidyaShortPeriod`, `vcetVidyaLongPeriod`, `vcetVidyaAlpha`, `vcetCePeriod`, `vcetCeMult`, `vcetCeUseClose`, `vcetRequireVidyaSlope`, `vcetMaxTradeBars`), but the attached `optimizeService.ts` request-construction path does not forward any VCET-specific fields.

I am **not** marking this as a confirmed product bug in the same way as ROCB because the attached `optimizeService.ts` snapshot also does not show matching VCET Vue-store bindings. That makes it ambiguous whether VCET customization is intentionally not exposed yet, or whether the service layer is incomplete.

### Recommendation

Treat VCET as a short follow-up audit:

- if VCET customization is supposed to be exposed, add store bindings + service forwarding + tests
- if VCET customization is intentionally fixed/default-only, add an inline comment so future reviewers do not assume the builder surface is fully reachable from `optimizeService.ts`

---

## 2. Important missed item: auto-prescan backend/device inheritance is weaker than it should be

### Why this matters

The original plan covered the `allowed_pairs` route cliff, but it did not explicitly call out a second prescan issue: the auto-prescan block does not inherit the outer request backend/device as faithfully as it should.

In the current code:

- the outer request backend is normalized up front via `with_preferred_cuda_device()`.
- but when auto-prescan triggers, the internally constructed `DoubleMaRequest` always uses `Backend::Auto { device_id }`.
- and when no explicit `gpu_prescan` config is present, `device_id` defaults to `0`.

So an automatically triggered stage-0 prescan can ignore the outer request’s already-resolved GPU intent and device selection.

That is not just a route-label problem. It can create avoidable runtime instability on multi-GPU systems, and it makes route behavior harder to reason about in benchmarks.

### Files involved

- `crates/app_core/src/composite_cpu.rs`

### Current evidence in the attached files

The outer request is normalized once:

```rust
    #[cfg(feature = "cuda")]
    let normalized_backend = req.backend.clone().with_preferred_cuda_device()?;
```

But auto-prescan later builds a fresh request with `Backend::Auto { device_id }`, where `device_id` comes only from explicit prescan config or falls back to zero:

```rust
        let device_id = cfg.and_then(|c| c.device_id).unwrap_or(0);
        ...
        let dm_req = DoubleMaRequest {
            backend: Backend::Auto { device_id },
            ...
        };
```

### Recommended code edit

Add a small helper to derive the prescan backend/device from the already normalized outer request when `gpu_prescan` is implicit.

#### `crates/app_core/src/composite_cpu.rs`

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

Then update the prescan request construction:

```rust
        let dm_backend = if req.gpu_prescan.is_some() {
            Backend::Auto { device_id }
        } else {
            #[cfg(feature = "cuda")]
            {
                inherited_prescan_backend(&normalized_backend)
            }
            #[cfg(not(feature = "cuda"))]
            {
                Backend::CpuOnly
            }
        };

        let dm_req = DoubleMaRequest {
            backend: dm_backend,
            data_id: req.data_id.clone(),
            fast_range: req.fast_range,
            slow_range: req.slow_range,
            ...
        };
```

### Additional route diagnostics to add

Even if you keep the original behavior, add a diagnostic that makes prescan routing explicit.

```rust
route_diagnostics.push(format!(
    "stage-0 prescan backend={} (explicit_cfg={} device_id={})",
    composite_backend_label(&dm_req.backend),
    req.gpu_prescan.is_some(),
    match dm_req.backend {
        Backend::Auto { device_id }
        | Backend::GpuOnly { device_id }
        | Backend::GpuHybridKernel { device_id }
        | Backend::GpuKernel { device_id } => device_id,
        Backend::CpuOnly => 0,
    }
));
```

### Rollout priority

Medium-high. I would implement it together with the original prescan guard/fallback cleanup, not as a separate later refactor.

---

## 3. Missed observability improvement: `composite_cpu_opt.rs` should emit the same benchmark metadata style as `composite_validation_workflow.rs`

### Why this matters

The original plan correctly recommended reshaping `composite_cpu_opt.rs`, but I should have also recommended improving its benchmark metadata.

`composite_validation_workflow.rs` already prints explicit benchmark metadata and a route probe before timed runs. `composite_cpu_opt.rs` does not. That makes it harder to interpret benchmark artifacts later, especially when backend route behavior or request shape is under review.

This is a small change with outsized debugging value.

### Files involved

- `crates/app_core/benches/composite_cpu_opt.rs`
- `crates/app_core/benches/composite_validation_workflow.rs`

### Recommended code edit

Port the same metadata pattern used by `composite_validation_workflow.rs` into `composite_cpu_opt.rs`.

#### `crates/app_core/benches/composite_cpu_opt.rs`

```rust
fn print_bench_metadata(group_name: &str, details: &[(&str, String)]) {
    let mut fields = vec![
        "timing_boundary=Criterion".to_string(),
        "build_profile=bench".to_string(),
    ];
    fields.extend(details.iter().map(|(key, value)| format!("{key}={value}")));
    println!("benchmark metadata {group_name}: {}", fields.join(" "));
}
```

Then, before the measured bench body:

```rust
    print_bench_metadata(
        "composite_cpu/plain_product_proxy/50k",
        &[
            ("backend", "CpuOnly".to_string()),
            ("candles_len", candles.close.len().to_string()),
            ("heatmap_bins", "0".to_string()),
            ("include_all", "false".to_string()),
            ("gpu_prescan", "none".to_string()),
        ],
    );

    let route_probe = ta_app_core::run_composite_cpu_optimization_blocking_with_candles(
        req.clone(),
        &candles,
        &cancel,
        None,
    )
    .expect("composite cpu route probe");

    println!(
        "benchmark route composite_cpu/plain_product_proxy/50k: backend_used={:?} backend_route={} fallback={}",
        route_probe.backend_used,
        route_probe.backend_route.as_deref().unwrap_or("<none>"),
        route_probe
            .backend_fallback_reason
            .as_deref()
            .unwrap_or("<none>")
    );
```

### Rollout priority

Medium. I would land this together with the benchmark split/rename work.

---

## 4. Optional beyond-scope improvement: add a second workflow regression case for a standalone family request

### Why this is worth doing

`ta_workflow_regression.rs` is already a good section-11 proxy for the plain `CompositeCpu` product path, but it does not currently protect against service/builder drift in standalone family requests.

That leaves gaps such as:

- ROCB request-field loss at the service layer,
- requested GPU route stability for one standalone family member,
- strategy-specific parameter preservation under workflow-style orchestration.

### Good low-risk candidate

Add a second deterministic mini-workflow case for one strategy that has custom parameters and a meaningful standalone route.

My first choice would be **ROCB** if you want to catch the newly identified service bug.
My second choice would be **VEMR** if you want a stronger GPU-native route regression probe.

### Possible structure

- keep the current plain `CompositeCpu` CPU proxy unchanged
- add a second optional case behind a CLI flag or a hardcoded second summary block
- record stage times separately for the second surface

#### Sketch

```rust
let rocb_opt_req = CompositeCpuRequest {
    backend: Backend::CpuOnly,
    rocb: Some(ta_app_core::RocbStrategyParams {
        channel_period: 37,
        atr_period: 21,
        atr_stop_mult: 2.75,
        max_trade_bars: 95,
        ..Default::default()
    }),
    data_id: label.to_string(),
    fast_range: (5, 60, 5),
    slow_range: (20, 220, 10),
    fast_ma_types: vec!["sma".to_string()],
    slow_ma_types: vec!["sma".to_string()],
    ma_source: "close".to_string(),
    fast_ma_params: None,
    slow_ma_params: None,
    strategy: strategy.clone(),
    halftrend: halftrend.clone(),
    macz: macz.clone(),
    ott: ott.clone(),
    signal_mode: ta_app_core::CompositeSignalMode::HardGate,
    objective: ObjectiveKind::Sharpe,
    mode: OptimizationMode::Grid,
    gpu_prescan: None,
    extra_shortlist_objectives: None,
    shortlist_confidence_mode: None,
    shortlist_union_mode: None,
    top_k: Some(32),
    include_all: Some(false),
    heatmap_bins: Some(0),
    ..
};
```

### Rollout priority

Optional. Good follow-on after the service-layer fixes land.

---

## 5. Optional beyond-scope clarification: keep `trade_on_next_bar` intentionally locked unless product wants real support

### Why this deserves a note

`optimizeRequestBuilder.ts` currently hardcodes `trade_on_next_bar: true` in `buildStrategyConfigFromInputs(...)`, and the existing builder tests explicitly expect that behavior even when `tradeOnNext` input is `false`.

That means this is not an accidental regression in the current snapshot. It is an intentional product contract.

Still, because the store/service layer reads `tradeOnNext`, it is easy for future work to misread this as a live control.

### Recommendation

Do **not** change behavior in the current performance patch set.
Instead, add one of these:

1. a code comment near the store/UI control saying the optimize request path is fixed to next-bar execution for parity, or
2. one dedicated regression test that asserts the lock remains intentional.

This avoids future confusion while keeping the section-11 patch set focused.

---

## 6. Optional follow-on bundle after section 11: allocation/copy-elision audit

This is intentionally **not** something I would fold into the first section-11 implementation batch.

But if you want a next performance pass after the functional/request-shape fixes, the best adjacent follow-on is the map’s allocation/copy-elision bundle.

That bundle is explicitly called out as a cross-cutting performance surface for row-major buffers, host readbacks, and NaN-prefix series builders.

### Why this is the right follow-on

- it stays performance-focused,
- it avoids product-surface churn,
- it can be run after the request/route layer is stable,
- it is a better place for `MaybeUninit`-style changes than the current section-11 implementation patch.

### Recommendation

Treat it as a **separate** follow-up review / patch train, not a same-PR expansion.

---

## Suggested update to the original rollout order

If I were revising the original plan ordering, I would use this sequence instead:

1. service-layer forwarding fixes
   - shortlist controls
   - ROCB params
2. plain `CompositeCpu` result-shaping fix
   - preserve MA params
3. route/fallback visibility fixes
   - include_all serial-path diagnostic
   - prescan diagnostics
   - backend/device-prescan inheritance cleanup
4. benchmark cleanup
   - split/rename `composite_cpu_opt.rs`
   - add metadata / route probes
5. measurement-only hot-path instrumentation
   - fast tile
   - slow tile
   - pair eval
   - finalist exactify
6. only then consider optional hot-path refactors
   - bounded slow-tile cache
   - chunked parallel all-row collector
   - deeper plain `GpuKernel` route surgery

---

## Addendum checklist

### Must add to the original plan

- [ ] Add a new finding for `optimizeService.ts` dropping ROCB fields.
- [ ] In `optimizeService.ts`, forward `rocbChannelPeriod`.
- [ ] In `optimizeService.ts`, forward `rocbAtrPeriod`.
- [ ] In `optimizeService.ts`, forward `rocbAtrStopMult`.
- [ ] In `optimizeService.ts`, forward `rocbMaxTradeBars`.
- [ ] Add a service regression test proving ROCB custom params survive store -> request building.
- [ ] Add a new finding for prescan backend/device inheritance.
- [ ] In `composite_cpu.rs`, make implicit auto-prescan inherit the normalized outer backend/device rather than defaulting to `Auto { device_id: 0 }`.
- [ ] Emit a route diagnostic that reports the backend/device actually used for stage-0 prescan.
- [ ] Add a new benchmark-observability item for `composite_cpu_opt.rs` metadata and route probes.

### Good optional improvements

- [ ] Audit whether VCET customization is intentionally unreachable from `optimizeService.ts` or simply missing store/service wiring.
- [ ] Add a second workflow regression surface for one standalone family request (ROCB or VEMR).
- [ ] Add a comment or dedicated test clarifying that `trade_on_next_bar` is intentionally fixed on the optimize path.
- [ ] Schedule a separate allocation/copy-elision follow-on review after section-11 functional fixes land.

### Things I would still keep out of scope for this patch set

- [ ] do not expand the product surface with new knobs
- [ ] do not re-enable Double-MA-style heatmap/export UX for standalone `CompositeCpu`
- [ ] do not add kernel-surface `allowed_pairs` support until measurements prove it is needed
- [ ] do not introduce unsafe allocation changes into the same patch set as request/fallback correctness fixes

