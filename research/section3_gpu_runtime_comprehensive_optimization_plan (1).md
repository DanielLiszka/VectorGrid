
# Section 3 comprehensive performance / optimization plan  
## GPU runtime, residency, and dynamic-strategy overhead

This document is a file-by-file implementation plan for the section-3 surface from the uploaded repo map. It is intentionally optimization-first, benchmark-first, and parity-preserving. It focuses on reducing unnecessary memory copies, keeping warm-path data resident longer, avoiding silent slow fallbacks, and using uninitialized memory only where it is safe and fully written before read.

## Scope

Covered files:

- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/dynamic_indicator_adapter.rs`
- `crates/app_core/src/strategy_dynamic.rs`
- `crates/app_core/src/strategy_plan.rs`
- `crates/app_core/src/strategy_gpu_program.rs`
- `crates/app_core/src/vram_ma.rs`
- `src-tauri/src/state.rs`
- `src-tauri/src/data_cache.rs`
- `src-tauri/src/bin/ta_app_path_faceoff.rs`

Not uploaded, therefore treated as open-context only:

- `strategy_gpu_runtime.rs`
- descriptor/catalog registry files beyond what is referenced from the uploaded files
- `ta_app_core` export surfaces that would be needed to expose internal kernel timings to the app-path harness

---

## Executive summary

### Highest-value changes

1. **Collapse resident market uploads into one batched upload phase with one synchronization point**
   - `gpu_backtest_kernel.rs`
   - Today, cold resident rebuilds and first-use field uploads can synchronize the stream repeatedly, once per uploaded series.
   - This is the clearest immediate warm/cold-path optimization target.

2. **Stop rebuilding prepared CUDA indicator inputs eagerly for all OHLCV fields**
   - `dynamic_indicator_adapter.rs`
   - Today, rebuild converts and uploads all five OHLCV arrays even when only `close` or a smaller field set is needed.
   - This leaves a lot of avoidable host conversion and H2D work on the table.

3. **Remove large cloned outputs from dynamic indicator prep caching**
   - `strategy_dynamic.rs`
   - The local slot cache currently clones full `DynamicIndicatorComputeOutput` payloads on insert and on cache hit.
   - This is a direct and avoidable memory-copy hotspot.

4. **Stop precomputing the full CPU context when the resolved route is GPU and CPU is only a fallback/shadow possibility**
   - `strategy_dynamic.rs`
   - This is likely a large warm-path waste for GPU-routed dynamic plans.

5. **Preserve dataset identity across repeated loads**
   - `state.rs` + `data_cache.rs`
   - Right now the GPU/runtime caches are pointer-signature based, while `load_price_data` always inserts a fresh base dataset.
   - That breaks residency even if the exact same CSV is loaded again.

6. **Measure the right boundary**
   - `ta_app_path_faceoff.rs`
   - Keep the existing app-path timing, but also record engine-only timing and expose kernel sub-timings when possible.
   - Without that, warm-path improvements can be real but hard to localize.

---

## Guiding rules for all edits

### 1) Prefer reuse over raw speed tricks
The biggest wins here are not micro-optimizations. They are:

- keep data resident longer
- reuse already-converted host buffers
- reuse already-uploaded device buffers
- avoid full-output clones
- avoid “prepare both worlds just in case”

### 2) Use uninitialized memory only in narrow, fully-written paths
Good places:

- `f64 -> f32` conversion buffers
- `f32 -> f32` staging buffers when every element is written
- return vectors like `simple_returns_from_log_returns_f32`
- binary cache decoding, which already does this correctly in `data_cache.rs`

Bad places:

- partially initialized structures that can panic before completion
- code paths that read before fill
- multi-branch code where one branch leaves elements untouched

### 3) Do not hide slow fallback behind silent success
Prepared-device paths should not quietly degrade to host-backed CUDA or CPU-like behavior without visible attribution.

### 4) Align retention policy with residency policy
If the app keeps only one base and one derived dataset active, the GPU-resident caches should either match that intentionally or the state layer should be widened enough to make warm reuse realistic.

### 5) Keep benchmark and app-path claims separate
- `Criterion` answers algorithmic micro-cost questions.
- `ta_app_path_faceoff` answers real app-path questions.
- internal H2D / indicator-prep counters answer why the app-path moved.

---

## Cross-file diagnosis

### Confirmed issue: repeated upload barriers on resident-market rebuild
`gpu_backtest_kernel.rs` currently uploads resident series via helpers that create a fresh pinned-upload guard and finalize immediately. That means a rebuild that needs `close`, then `high`, then `low`, then `volume`, then `source_override` can pay multiple stream synchronizations instead of one.

### Confirmed issue: eager prepared-input rebuild does too much work
`dynamic_indicator_adapter.rs` rebuilds host/device OHLCV buffers for all fields up front when the dataset signature changes. That is simple, but it is not minimal.

### Confirmed issue: dynamic prep cache clones large outputs
`strategy_dynamic.rs` clones `DynamicIndicatorComputeOutput` values into and out of a local `HashMap`. If those outputs carry long vectors, the dedupe cache still pays substantial memory copy cost.

### Likely issue: warm-path residency is fragile across repeated loads
`state.rs` creates a new base dataset entry on each `load_price_data` call. The runtime caches in `gpu_backtest_kernel.rs` and `dynamic_indicator_adapter.rs` are pointer-signature keyed. If the same source is loaded twice, those pointers change and warm caches are invalidated.

### Likely issue: warm-path reuse depends on thread affinity
Both the main CUDA runtime cache and the prepared CUDA input cache are thread-local. If real app requests hop worker threads, warm reuse can disappear even when data is logically unchanged.

### Open question: repeated GPU program / ABI upload frequency
`strategy_gpu_program.rs` clearly recompiles ABI/program structures from plan data, but without `strategy_gpu_runtime.rs` we cannot confirm how much of that is already cached before device upload.

### Open question: route policy descriptor quality
`strategy_plan.rs` bases GPU defaulting on descriptor flags such as `prefers_gpu_by_speedup`, `supports_gpu_kernel`, and `supports_cuda_single/sweep`. The descriptor registry itself was not uploaded, so route-quality changes should be treated as targeted policy review, not a blind rewrite.

---

## Priority order

### Phase 1: immediate, parity-safe, high return
- batch resident uploads and reduce stream synchronizations
- add reusable uninitialized conversion helpers
- lazy prepared CUDA fields instead of eager all-field upload
- remove dynamic slot output clones with `Arc`
- avoid unconditional CPU context prep on GPU-resolved plans
- add base dataset in-memory reuse

### Phase 2: still safe, slightly broader
- widen resident caches from single-entry to small LRU
- add cached `first_valid` values for prepared CUDA inputs
- improve app-path harness with sub-timings and engine-only timing
- use precomputed `SourceIdentity` across state/data_cache boundary

### Phase 3: higher-risk or lower-ROI
- change pointer-based residency signatures to explicit version/data keys
- revisit thread-local cache scope
- add reusable ABI/program scratch/builders
- async stage period vectors if profiling proves meaningful

---

# File-by-file plan

---

## 1. `crates/app_core/src/gpu_backtest_kernel.rs`

## What this file already does well
- It already uses `alloc_uninit_f32`, `resize_uninit_f32`, and `alloc_uninit_copy` in several hot paths.
- It already tracks `gpu_indicator_prep_ms` and `h2d_upload_ms`.
- It already has resident market-series reuse and persistent scratch buffers.
- It already has pinned H2D and async allocation paths.

That means this file is already optimization-aware. The goal is not to redesign it. The goal is to remove the remaining synchronization and copy waste.

### Confirmed issue A: resident uploads finalize too often
`ensure_resident_market_series` rebuilds the resident cache lazily, but each upload helper (`upload_resident_series_f64`) creates and finalizes its own pinned-upload guard. That means the rebuild path can synchronize once per field instead of once per batch.

### Why it matters
This directly hits:
- first warm run after dataset switch
- first request that needs `high/low`
- first request that needs `volume`
- first request that needs non-`close` source overrides

### Recommended change
Introduce a batched resident-upload helper so all newly-needed series use the same guard and one final synchronization.

### Representative code edit

```rust
struct ResidentUploadBatch<'a> {
    stream: &'a Stream,
    guard: AsyncPinnedUploadGuard<'a>,
    use_pinned: bool,
}

impl<'a> ResidentUploadBatch<'a> {
    #[inline]
    fn new(stream: &'a Stream) -> Self {
        Self {
            stream,
            guard: AsyncPinnedUploadGuard::new(stream),
            use_pinned: kernel_pinned_h2d_enabled(),
        }
    }

    #[inline]
    fn upload_f64(&mut self, src: &[f64]) -> Result<DeviceBuffer<f32>, String> {
        upload_f32_source_to_device(
            self.stream,
            HostF32Source::F64(src),
            self.use_pinned,
            &mut self.guard,
        )
    }

    #[inline]
    fn upload_f32(&mut self, src: &[f32]) -> Result<DeviceBuffer<f32>, String> {
        upload_f32_source_to_device(
            self.stream,
            HostF32Source::F32(src),
            self.use_pinned,
            &mut self.guard,
        )
    }

    #[inline]
    fn finish(self) -> Result<(), String> {
        self.guard.finalize()
    }
}
```

Then use it in `ensure_resident_market_series`:

```rust
fn ensure_resident_market_series(
    stream: &Stream,
    resident: &mut Option<ResidentMarketSeriesCache>,
    candles: &Candles,
    close: &[f64],
    ma_source: &str,
    ma_prices: Option<&[f64]>,
    needs_ma_prices: bool,
    needs_high_low: bool,
    high: Option<&[f64]>,
    low: Option<&[f64]>,
    needs_volume: bool,
    volume: Option<&[f64]>,
) -> Result<(), String> {
    let signature = candle_dataset_signature(candles);
    let source_key = normalize_market_source_key(ma_source);

    let should_reset = resident
        .as_ref()
        .map(|cache| cache.signature != signature)
        .unwrap_or(true);

    if should_reset {
        reset_resident_market_cache(stream, resident)?;
        let mut batch = ResidentUploadBatch::new(stream);
        let mut cache = ResidentMarketSeriesCache {
            signature,
            close: batch.upload_f64(close)?,
            high: None,
            low: None,
            volume: None,
            source_overrides: HashMap::new(),
        };

        if needs_high_low {
            cache.high = Some(batch.upload_f64(
                high.ok_or_else(|| "high prices required for resident upload".to_string())?
            )?);
            cache.low = Some(batch.upload_f64(
                low.ok_or_else(|| "low prices required for resident upload".to_string())?
            )?);
        }

        if needs_volume {
            cache.volume = Some(batch.upload_f64(
                volume.ok_or_else(|| "volume prices required for resident upload".to_string())?
            )?);
        }

        if needs_ma_prices && source_key != "close" {
            cache.source_overrides.insert(
                source_key.clone(),
                batch.upload_f64(
                    ma_prices.ok_or_else(|| format!("ma prices required for ma_source={source_key}"))?
                )?,
            );
        }

        batch.finish()?;
        *resident = Some(cache);
        return Ok(());
    }

    let cache = resident
        .as_mut()
        .ok_or_else(|| "resident market cache unavailable".to_string())?;

    let mut batch = ResidentUploadBatch::new(stream);

    if needs_high_low {
        if cache.high.is_none() {
            cache.high = Some(batch.upload_f64(
                high.ok_or_else(|| "high prices required for resident upload".to_string())?
            )?);
        }
        if cache.low.is_none() {
            cache.low = Some(batch.upload_f64(
                low.ok_or_else(|| "low prices required for resident upload".to_string())?
            )?);
        }
    }

    if needs_volume && cache.volume.is_none() {
        cache.volume = Some(batch.upload_f64(
            volume.ok_or_else(|| "volume prices required for resident upload".to_string())?
        )?);
    }

    if needs_ma_prices && source_key != "close" && !cache.source_overrides.contains_key(&source_key) {
        cache.source_overrides.insert(
            source_key.clone(),
            batch.upload_f64(
                ma_prices.ok_or_else(|| format!("ma prices required for ma_source={source_key}"))?
            )?,
        );
    }

    batch.finish()?;
    Ok(())
}
```

### Expected benefit
- lower cold-switch latency
- lower first-hit latency for optional fields
- fewer avoidable stream-wide barriers
- better overlap potential if later kernels get more async

---

### Likely issue B: single resident-market entry is too narrow
`CudaKernelRuntime` currently stores:

```rust
resident_market: Option<ResidentMarketSeriesCache>
```

That means alternating between:
- base dataset and resampled dataset
- two recent symbols
- two timeframes

will thrash the resident market cache.

### Why it matters
Your `AppState` also caps active retention aggressively. A single resident entry amplifies warm/cold bimodality.

### Recommended change
Replace the single entry with a tiny LRU, ideally capacity 2 by default, optionally env-configurable. Keep this small. This is not a “cache everything” proposal.

### Representative code edit

```rust
#[derive(Default)]
struct ResidentMarketCacheLru {
    order: std::collections::VecDeque<CandleDatasetSignature>,
    map: HashMap<CandleDatasetSignature, ResidentMarketSeriesCache>,
    capacity: usize,
}

impl ResidentMarketCacheLru {
    fn new(capacity: usize) -> Self {
        Self {
            order: std::collections::VecDeque::new(),
            map: HashMap::new(),
            capacity: capacity.max(1),
        }
    }

    fn touch(&mut self, key: CandleDatasetSignature) {
        if let Some(pos) = self.order.iter().position(|k| *k == key) {
            self.order.remove(pos);
        }
        self.order.push_back(key);
    }

    fn get_mut(&mut self, key: &CandleDatasetSignature) -> Option<&mut ResidentMarketSeriesCache> {
        if self.map.contains_key(key) {
            self.touch(*key);
        }
        self.map.get_mut(key)
    }

    fn insert(&mut self, value: ResidentMarketSeriesCache) -> Option<ResidentMarketSeriesCache> {
        let key = value.signature;
        self.map.insert(key, value);
        self.touch(key);

        if self.map.len() > self.capacity {
            if let Some(evict_key) = self.order.pop_front() {
                if evict_key != key {
                    return self.map.remove(&evict_key);
                }
            }
        }
        None
    }
}
```

Then in `CudaKernelRuntime`:

```rust
resident_market: ResidentMarketCacheLru,
```

### Risk
Medium, because eviction and device-buffer lifetime need careful handling. Keep eviction synchronous at first if needed, then optimize drop behavior later.

---

### Likely issue C: `reset_resident_market_cache` does a full stream synchronize on eviction
That is safe, but it makes dataset switches “spiky.”

### Recommended change
If you keep the single-entry design, leave this as-is for correctness. If you move to a multi-entry LRU, avoid calling `stream.synchronize()` for every logical switch. Instead:
- synchronize only when evicting an actually-live buffer that could still be in-flight
- or keep the conservative synchronization only on true eviction, not on every different-signature access

### Suggested approach
Treat this as phase 2 after batched uploads land.

---

### Optimization opportunity D: use reusable uninitialized conversion helpers
There are still several conversion paths that allocate initialized vectors or collect through iterators.

Examples worth tightening:
- `simple_returns_from_log_returns_f32`
- non-pinned `HostF64 -> DeviceBuffer<f32>` path in `upload_f32_source_to_device`
- other small conversion helpers in this file

### Representative code edit

```rust
#[inline]
fn cast_f64_slice_to_f32_vec(src: &[f64]) -> Vec<f32> {
    let mut out = alloc_uninit_f32(src.len());
    for (dst, &v) in out.iter_mut().zip(src.iter()) {
        *dst = v as f32;
    }
    out
}

#[inline]
fn simple_returns_from_log_returns_f32(lr: &[f32]) -> Vec<f32> {
    let mut out = alloc_uninit_f32(lr.len());
    for (dst, &v) in out.iter_mut().zip(lr.iter()) {
        *dst = v.exp_m1();
    }
    out
}
```

Then in `upload_f32_source_to_device`:

```rust
HostF32Source::F64(slice) => {
    let tmp = cast_f64_slice_to_f32_vec(slice);
    DeviceBuffer::from_slice(&tmp).map_err(|e| e.to_string())
}
```

### Expected benefit
Small to medium. The real win is not CPU cycles alone, but avoiding redundant init/fill patterns in paths that still run often.

---

### Optimization opportunity E: reuse pinned staging buffers
Right now each pinned upload allocates a fresh `LockedBuffer<f32>`. That is better than pageable staging, but it still pays allocation churn.

### Recommended change
Add reusable pinned staging buffers to `KernelScratch`, keyed by required length bucket. Reuse them for:
- resident uploads
- ephemeral uploads
- dynamic-program batch uploads if you later convert those too

### Representative sketch

```rust
#[derive(Default)]
struct PinnedStageBuffers {
    f32_stage: Option<LockedBuffer<f32>>,
    len: usize,
}

impl PinnedStageBuffers {
    fn ensure_len(&mut self, len: usize) -> Result<&mut LockedBuffer<f32>, String> {
        let needs_new = self.f32_stage.is_none() || self.len < len;
        if needs_new {
            self.f32_stage = Some(
                unsafe { LockedBuffer::<f32>::uninitialized(len) }.map_err(|e| e.to_string())?
            );
            self.len = len;
        }
        Ok(self.f32_stage.as_mut().expect("stage just initialized"))
    }
}
```

### Risk
Medium. `LockedBuffer` lifetime must not outlive any in-flight async copy that still needs it. The safest first version still keeps the batch guard semantics and only reuses the stage after finalization.

---

### Optimization opportunity F: lower-priority cleanup in dynamic-program batch upload
`upload_double_ma_dynamic_program_batch` still:
- transposes into host temporary vectors
- computes `ret1` into a fresh vector
- uploads via `DeviceBuffer::from_slice`

That is acceptable, but if this path is hot enough, it can benefit from the same staged-upload infrastructure.

### Suggested edits
- convert `simple_returns_from_log_returns_f32` to uninitialized fill
- if profiling says worthwhile, let this path use `ResidentUploadBatch` or a generic async upload batch helper
- do not prioritize this ahead of resident market reuse and dynamic prep dedupe

---

### Open question G: thread-local cache scope
The main CUDA runtime cache and last-kernel timing store are thread-local. That is fine for a blocking same-thread benchmark. It may be less fine if Tauri request handling hops threads.

### Recommendation
Do not redesign this first. Instead:
- add warm-path cache-hit counters
- verify whether real optimize calls stay on the same worker thread
- only then decide whether a broader cache scope is worth the complexity

---

### Tests to add for this file
1. **resident rebuild uploads once-per-batch**
   - assert one synchronization barrier for a multi-field rebuild if you can mock/track stream sync calls
2. **high/low first-use is cached**
   - second request needing `high/low` should not upload them again
3. **non-close source override is cached**
   - second request with same `ma_source` should reuse the override
4. **alternating between two dataset signatures with capacity=2 stays warm**
   - h2d upload time should drop after the first touches
5. **single-entry fallback still correct**
   - ensure no stale field survives signature change

---

## 2. `crates/app_core/src/dynamic_indicator_adapter.rs`

## Current strengths
- The file already centralizes prepared CUDA inputs.
- It already caches by device and dataset signature.
- It already lazily materializes source overrides.

The next step is to make the prepared cache *truly minimal*.

### Confirmed issue A: rebuild eagerly converts/uploads all OHLCV fields
On rebuild, `from_candles_on_device` converts:
- `open`
- `high`
- `low`
- `close`
- `volume`

into host `Vec<f32>` and uploads all five to device, even when only a subset is required.

### Why it matters
This is unnecessary work for indicators that only need:
- `close`
- `source + volume`
- `high/low`
- `ohlc` but not `volume`

### Recommended change
Make field preparation lazy.

### Target structure
Instead of always storing fully prepared fields, store per-field state and materialize only on demand.

### Representative code edit

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum PreparedFieldKind {
    Open,
    High,
    Low,
    Close,
    Volume,
}

#[derive(Debug)]
struct PreparedFieldBuffers {
    host: Option<Arc<Vec<f32>>>,
    device: Option<Arc<CudaDeviceVectorF32>>,
}

impl PreparedFieldBuffers {
    fn empty() -> Self {
        Self {
            host: None,
            device: None,
        }
    }
}

#[derive(Debug)]
struct ResidentCudaIndicatorDataset {
    signature: PreparedCudaDatasetSignature,
    runtime: Arc<CudaRuntime>,
    open: PreparedFieldBuffers,
    high: PreparedFieldBuffers,
    low: PreparedFieldBuffers,
    close: PreparedFieldBuffers,
    volume: PreparedFieldBuffers,
    source_overrides: HashMap<String, Arc<Vec<f32>>>,
    d_source_overrides: HashMap<String, Arc<CudaDeviceVectorF32>>,
    first_valid_cache: HashMap<PreparedFirstValidKey, usize>,
}
```

Then add a helper:

```rust
impl ResidentCudaIndicatorDataset {
    fn ensure_field_uploaded(
        &mut self,
        candles: &Candles,
        field: PreparedFieldKind,
    ) -> Result<(), DynamicIndicatorAdapterError> {
        let slot = match field {
            PreparedFieldKind::Open => &mut self.open,
            PreparedFieldKind::High => &mut self.high,
            PreparedFieldKind::Low => &mut self.low,
            PreparedFieldKind::Close => &mut self.close,
            PreparedFieldKind::Volume => &mut self.volume,
        };

        if slot.device.is_some() {
            return Ok(());
        }

        let src_f64: &[f64] = match field {
            PreparedFieldKind::Open => candles.open.as_slice(),
            PreparedFieldKind::High => candles.high.as_slice(),
            PreparedFieldKind::Low => candles.low.as_slice(),
            PreparedFieldKind::Close => candles.close.as_slice(),
            PreparedFieldKind::Volume => candles.volume.as_slice(),
        };

        let host = slot
            .host
            .get_or_insert_with(|| Arc::new(cast_f64_slice_to_f32_vec(src_f64)))
            .clone();

        let device = Arc::new(
            self.runtime
                .upload_f32(host.as_slice())
                .map_err(|e| DynamicIndicatorAdapterError::CudaPreparation {
                    details: e.to_string(),
                })?,
        );

        slot.device = Some(device);
        Ok(())
    }
}
```

### Expected benefit
Substantial when the dynamic indicator set only touches a subset of OHLCV.

---

### Confirmed issue B: the cache is only one dataset per device
`PREPARED_CUDA_INPUT_CACHE` is currently:

```rust
thread_local! {
    static PREPARED_CUDA_INPUT_CACHE: RefCell<HashMap<usize, ResidentCudaIndicatorDataset>>
}
```

That means one prepared dataset per device, not a small per-device set.

### Recommended change
Use a small per-device LRU keyed by dataset signature.

### Representative structure

```rust
struct PreparedCudaDatasetLru {
    order: std::collections::VecDeque<PreparedCudaDatasetSignature>,
    map: HashMap<PreparedCudaDatasetSignature, ResidentCudaIndicatorDataset>,
    capacity: usize,
}

thread_local! {
    static PREPARED_CUDA_INPUT_CACHE: RefCell<HashMap<usize, PreparedCudaDatasetLru>> =
        RefCell::new(HashMap::new());
}
```

### Why it matters
If the app alternates between:
- base and derived datasets
- two timeframes
- two symbols

prepared input reuse otherwise collapses.

---

### Likely issue C: repeated `first_valid` scans
The prepared-input object recomputes:
- `source_first_valid`
- `high_low_first_valid`
- `hlc_first_valid`
- `ohlc_first_valid`
- `ohlcv_first_valid`
- `close_volume_first_valid`

by scanning arrays each time.

### Recommended change
Cache first-valid results per input kind and, where relevant, per source override key.

### Representative code edit

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum PreparedFirstValidKey {
    Source,
    HighLow,
    Hlc,
    Ohlc,
    Ohlcv,
    CloseVolume,
    SourceOverride(u64),
}

impl ResidentCudaIndicatorDataset {
    fn cached_first_valid<F>(
        &mut self,
        key: PreparedFirstValidKey,
        compute: F,
    ) -> usize
    where
        F: FnOnce(&Self) -> usize,
    {
        if let Some(&v) = self.first_valid_cache.get(&key) {
            return v;
        }
        let v = compute(self);
        self.first_valid_cache.insert(key, v);
        v
    }
}
```

### Expected benefit
Small to medium per call, but good cumulative payoff for repeated indicator staging.

---

### Likely issue D: prepared-device failure falls back too quietly
`compute_dynamic_indicator_cuda_with_inputs` tries `compute_cuda_device(...)`, but on failure it falls back to host-backed `compute_cuda(...)` without preserving much attribution.

### Why it matters
This can erase the value of prepared-device staging while still returning a successful result. That makes performance debugging much harder.

### Recommended change
Keep the fallback for parity, but make it explicit.

### Suggested approach
Return or log a structured fallback reason when:
- device data preparation is unavailable
- device kernel path rejects the request
- the prepared path had to degrade to host-backed CUDA

### Representative pattern

```rust
pub enum PreparedCudaPathUsed {
    DevicePrepared,
    HostFallbackFromPrepared { reason: String },
}

pub struct DynamicIndicatorCudaComputeOutput {
    pub output_id: String,
    pub series: DynamicIndicatorCudaSeries,
    pub warmup: usize,
    pub rows: usize,
    pub cols: usize,
    pub pattern_ids: Option<Vec<i32>>,
    pub path_used: PreparedCudaPathUsed,
}
```

Then propagate this into dynamic-strategy metrics or artifact output.

### Risk
Low. This is mostly attribution.

---

### Optimization opportunity E: add field mask to `PreparedCudaDatasetSignature`
The market-series signature in `gpu_backtest_kernel.rs` includes a field mask. The prepared CUDA signature does not.

### Recommended change
Mirror that approach here:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
struct PreparedCudaDatasetSignature {
    len: usize,
    timestamp_ptr: usize,
    open_ptr: usize,
    high_ptr: usize,
    low_ptr: usize,
    close_ptr: usize,
    volume_ptr: usize,
    field_mask: u8,
}
```

This is mostly correctness hygiene and parity with the other residency surface.

---

### Optimization opportunity F: use existing uninitialized-vec policy for f64->f32 host conversions
Right now rebuild uses repeated `.iter().map(|v| *v as f32).collect::<Vec<f32>>()`.

Replace that with a shared helper.

```rust
#[inline]
fn cast_f64_slice_to_f32_vec(src: &[f64]) -> Vec<f32> {
    let mut out = crate::alloc_policy::alloc_uninit_f32(src.len());
    for (dst, &v) in out.iter_mut().zip(src.iter()) {
        *dst = v as f32;
    }
    out
}
```

Use it for:
- base OHLCV host staging
- source overrides

---

### Tests to add for this file
1. **close-only indicators do not upload `open/high/low/volume`**
2. **prepared dataset cache retains two signatures when capacity=2**
3. **first_valid cache is reused**
4. **source override is cached per normalized source key**
5. **prepared-device failure surfaces explicit fallback attribution**
6. **field-mask change invalidates cache correctly**

---

## 3. `crates/app_core/src/strategy_dynamic.rs`

This file is probably the highest pure host-side copy waste among the uploaded files.

## Current strengths
- It already separates preview, compile, and runnable-plan preparation.
- It already has dynamic indicator slot staging helpers.
- It already has env-controlled experimental and shadow behavior.
- It already has optional artifact logging.

The main problem is that it still prepares too much and copies too much.

### Confirmed issue A: local dynamic indicator cache clones large outputs
The local staging cache is:

```rust
HashMap<String, DynamicIndicatorComputeOutput>
```

and on miss it does:

- compute output
- `cache.insert(cache_key, computed.clone())`
- `slot_outputs.insert(slot.indicator_id.clone(), computed)`

and on hit it does:

- `slot_outputs.insert(slot.indicator_id.clone(), cached.clone())`

### Why it matters
If a cached dynamic indicator output contains a large vector, the cache still duplicates that vector in memory. That defeats part of the point of the dedupe cache.

### Recommended change
Use shared ownership for slot outputs.

### Representative code edit

```rust
use std::sync::Arc;

pub struct PreparedDynamicCpuContext {
    pub lr: Vec<f64>,
    pub close: Vec<f64>,
    pub ma_source_prices: Vec<f64>,
    pub halftrend_filter: Option<Vec<f64>>,
    pub composite_halftrend: Option<Vec<f64>>,
    pub macz_hist: Option<Vec<f64>>,
    pub ott_line: Option<Vec<f64>>,
    pub dynamic_slot_series: HashMap<String, Arc<DynamicIndicatorComputeOutput>>,
    kaufman_stops: Option<KaufmanStopSeries>,
}
```

Local cache key and value:

```rust
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
struct DynamicIndicatorLocalCacheKey {
    route: crate::strategy_plan::IndicatorRouteKind,
    source_indicator_id: String,
    output_id: Option<String>,
    params_hash: u64,
}

type SharedDynamicOutput = Arc<DynamicIndicatorComputeOutput>;
```

Usage:

```rust
let mut slot_outputs: HashMap<String, SharedDynamicOutput> = HashMap::new();
let mut cache: HashMap<DynamicIndicatorLocalCacheKey, SharedDynamicOutput> = HashMap::new();

if let Some(cached) = cache.get(&cache_key) {
    slot_outputs.insert(slot.indicator_id.clone(), Arc::clone(cached));
    continue;
}

let computed = Arc::new(compute_dynamic_indicator_cpu(...)?);
cache.insert(cache_key, Arc::clone(&computed));
slot_outputs.insert(slot.indicator_id.clone(), computed);
```

### Expected benefit
High for plans with duplicated indicator expressions or repeated source refs.

---

### Confirmed issue B: cache-key construction over-serializes
The local cache key currently uses:
- a dataset-signature string
- route stringification
- normalized ids
- `serde_json::to_string(params)`

for a cache that lives only for the duration of one preparation call.

### Why it matters
That is more allocation and formatting than necessary.

### Recommended change
Use a structured key with a cheap stable param hash.

### Representative code edit

```rust
fn hash_indicator_params(params: &[crate::indicator_catalog::IndicatorParamKV]) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for p in params {
        p.key.hash(&mut h);
        p.value.to_string().hash(&mut h);
    }
    h.finish()
}
```

Then:

```rust
let cache_key = DynamicIndicatorLocalCacheKey {
    route: slot.route,
    source_indicator_id: source_indicator_id.to_string(),
    output_id: slot.output_id.clone(),
    params_hash: hash_indicator_params(slot.params.as_slice()),
};
```

### Important note
Because this cache is local to a single `prepare_dynamic_indicator_slot_series_*` call, the dataset signature does not need to be part of the key at all.

### Expected benefit
Medium. It cuts allocation overhead and simplifies the code.

---

### Confirmed issue C: unconditional CPU context prep
`prepare_runnable_dynamic_double_ma_plan` currently:
- previews the plan
- compiles a CPU plan
- prepares the full CPU context
- optionally compiles a GPU plan and handle

That means even a GPU-resolved plan pays full CPU context prep up front.

### Why it matters
This can erase part of the benefit of GPU routing, especially when:
- dynamic slot staging is expensive
- the CPU context is only needed for fallback or shadow sampling
- shadow mode is disabled

### Recommended change
Make CPU context lazy.

### Representative code edit

```rust
use std::sync::OnceLock;

pub struct RunnableDynamicDoubleMaPlan {
    pub ir: crate::strategy_ir::StrategyIr,
    pub preview: DynamicDoubleMaStrategyIrPreview,
    pub cpu_plan: CompiledStrategyPlan,
    cpu_context: OnceLock<PreparedDynamicCpuContext>,
    pub gpu_handle: Option<StrategyHandle>,
}

impl RunnableDynamicDoubleMaPlan {
    pub fn cpu_context(&self, candles: &Candles) -> Result<&PreparedDynamicCpuContext, String> {
        self.cpu_context.get_or_try_init(|| {
            prepare_dynamic_cpu_context(&self.cpu_plan, candles, self.ir.ma_source.as_str())
        })
    }
}
```

Then:

```rust
pub fn prepare_runnable_dynamic_double_ma_plan(
    ir: crate::strategy_ir::StrategyIr,
    requested_backend: StrategyPlanBackend,
    objective: ta_optimizer::ObjectiveKind,
    top_k: usize,
    include_all: bool,
    candles: &Candles,
) -> Result<RunnableDynamicDoubleMaPlan, String> {
    let preview = preview_dynamic_double_ma_strategy_ir(
        &ir,
        requested_backend,
        objective,
        top_k,
        include_all,
    )?;

    let cpu_plan = compile_strategy_plan(&ir, StrategyPlanBackend::Cpu, None)?;

    let gpu_handle = if preview.resolved_backend == StrategyPlanBackend::GpuUnified {
        let gpu_plan = compile_strategy_plan(&ir, StrategyPlanBackend::GpuUnified, None)?;
        Some(StrategyHandle::from_plan(
            gpu_plan,
            preview.capability.clone(),
        )?)
    } else {
        None
    };

    let runnable = RunnableDynamicDoubleMaPlan {
        ir,
        preview,
        cpu_plan,
        cpu_context: OnceLock::new(),
        gpu_handle,
    };

    if runnable.preview.resolved_backend == StrategyPlanBackend::Cpu {
        let _ = runnable.cpu_context(candles)?;
    }

    Ok(runnable)
}
```

### Expected benefit
Potentially large for GPU-routed dynamic plans.

### Risk
Low to medium. Callers that assume `cpu_context` is always present need a small API adjustment.

---

### Likely issue D: preview/prepare path recompiles more than needed
`preview_dynamic_double_ma_strategy_ir` can compile plans and create a `StrategyHandle` just to validate GPU support, then `prepare_runnable_dynamic_double_ma_plan` compiles again.

### Recommended change
Return more from preview, or add a dedicated compile bundle.

### Suggested shape

```rust
pub struct DynamicPlanCompileBundle {
    pub preview: DynamicDoubleMaStrategyIrPreview,
    pub cpu_plan: CompiledStrategyPlan,
    pub gpu_plan: Option<CompiledStrategyPlan>,
}
```

Then `prepare_runnable_dynamic_double_ma_plan` can build from the bundle instead of recompiling.

### Risk
Low. This is mostly refactoring for reuse.

---

### Likely issue E: GPU slot staging still comes back to host and inflates to `f64`
`cast_cuda_output_to_cpu_series` converts `HostF32` results into `Vec<f64>`. That is correct for the current CPU-context representation, but it means the GPU staging path still pays:
- D2H
- host allocation
- `f32 -> f64` expansion

### Recommended change
Do not try to fix this first by redesigning the entire dynamic evaluation pipeline. Start with:
- lazy CPU context
- `Arc`-shared outputs
- prepared input efficiency

Then, if profiling still says dynamic slot staging dominates, consider a split representation:

```rust
enum DynamicPreparedSeries {
    HostF64(Vec<f64>),
    HostF32(Vec<f32>),
    DeviceF32(Arc<CudaDeviceVectorF32>),
}
```

That would be a phase-3 change, not phase 1.

---

### Optimization opportunity F: artifact logging can distort perf if enabled
The file includes append-on-each-record CSV artifact logic controlled by env vars.

### Recommended change
When perf testing:
- disable artifact logging by default
- or batch artifact writes in memory and flush at end-of-run
- or explicitly mark artifacts-on runs as diagnostic, not benchmark runs

This is benchmark hygiene, not a code-path redesign.

---

### Tests to add for this file
1. **duplicate dynamic slot requests share one `Arc` output**
2. **GPU-resolved plan does not build CPU context until requested**
3. **CPU-resolved plan still eagerly builds CPU context**
4. **cache-key hashing preserves dedupe behavior**
5. **shadow-disabled path does not force CPU context prep**
6. **artifact logging off/on does not affect correctness**

---

## 4. `crates/app_core/src/strategy_plan.rs`

This file is not the main copy hotspot. Its main role in this plan is to stop repeated compilation and avoid route-policy ambiguity.

### Likely issue A: repeated compile work is paid outside the file
`compile_strategy_plan` itself is straightforward, but other files call it multiple times during preview and prepare.

### Recommended change
Do not aggressively micro-optimize the body of `compile_strategy_plan` first. Instead, reduce repeated calls from `strategy_dynamic.rs`.

### Safe edit
Add a tiny compile-bundle helper:

```rust
pub struct CompiledStrategyPlanPair {
    pub cpu: CompiledStrategyPlan,
    pub gpu_unified: Option<CompiledStrategyPlan>,
}

pub fn compile_strategy_plan_pair(
    ir: &StrategyIr,
    need_gpu: bool,
) -> Result<CompiledStrategyPlanPair, String> {
    let cpu = compile_strategy_plan(ir, StrategyPlanBackend::Cpu, None)?;
    let gpu_unified = if need_gpu {
        Some(compile_strategy_plan(ir, StrategyPlanBackend::GpuUnified, None)?)
    } else {
        None
    };
    Ok(CompiledStrategyPlanPair { cpu, gpu_unified })
}
```

### Expected benefit
Modest, but it simplifies higher-level duplication.

---

### Open question B: default GPU route policy may be too conservative or too optimistic
`default_route_for_indicator` relies on descriptor flags:
- `prefers_gpu_by_speedup`
- `supports_gpu_kernel`
- `supports_cuda_single`
- `supports_cuda_sweep`

Without the descriptor registry, it is not safe to change policy broadly.

### Recommended change
Do not rewrite route defaults in this pass. Instead:
- add better route attribution to timing artifacts
- review the descriptor registry separately if route-matrix evidence shows drift

---

### Optimization opportunity C: plan fingerprint generation is probably low priority
If plan compilation is proven hot, you can revisit the IR fingerprint path. For now, that is not where the biggest waste is.

---

### Tests to add for this file
1. **compiled plan pair returns the same CPU plan as the standalone path**
2. **need_gpu=false does not compile GPU plan**
3. **route defaults remain unchanged after refactor**

---

## 5. `crates/app_core/src/strategy_gpu_program.rs`

This file is small enough that it is unlikely to dominate app-path latency by itself, but it is still worth tightening if repeated compile/upload is visible.

### Current assessment
- vector allocations are already bounded by plan size
- shapes are relatively small
- likely not the first bottleneck

### Likely issue A: repeated small allocations if recompilation happens often
Both `compile_gpu_predicate_program` and `compile_gpu_predicate_program_abi` build fresh vectors each time.

### Recommended change
Only optimize this after:
- resident upload batching
- prepared input laziness
- dynamic output de-cloning
- lazy CPU context

### Safe optimization
Add “compile-into scratch” helpers so repeated plan compiles can reuse capacity.

### Representative code edit

```rust
#[derive(Default)]
pub struct GpuProgramAbiScratch {
    indicator_slots: Vec<GpuIndicatorSlotAbi>,
    conditions: Vec<GpuConditionAbi>,
    groups: Vec<GpuConditionGroupAbi>,
    group_condition_indexes: Vec<u32>,
}

pub fn compile_gpu_predicate_program_abi_into(
    program: &CompiledGpuPredicateProgram,
    scratch: &mut GpuProgramAbiScratch,
) -> CompiledGpuPredicateProgramAbi {
    scratch.indicator_slots.clear();
    scratch.conditions.clear();
    scratch.groups.clear();
    scratch.group_condition_indexes.clear();

    scratch.indicator_slots.extend(program.indicator_slots.iter().map(|slot| GpuIndicatorSlotAbi {
        indicator_index: slot.indicator_index,
        route: encode_indicator_route(slot.route),
    }));

    scratch.conditions.extend(program.conditions.iter().map(|condition| GpuConditionAbi {
        op: encode_condition_op(condition.op),
        lhs: GpuValueOperandAbi {
            source: encode_value_source_kind(condition.lhs.source),
            index: condition.lhs.index,
            constant_bits: condition.lhs.constant_bits,
        },
        rhs: GpuValueOperandAbi {
            source: encode_value_source_kind(condition.rhs.source),
            index: condition.rhs.index,
            constant_bits: condition.rhs.constant_bits,
        },
    }));

    scratch.groups.extend(program.groups.iter().map(|group| GpuConditionGroupAbi {
        mode: encode_group_mode(group.mode),
        condition_offset: group.condition_offset,
        condition_count: group.condition_count,
    }));

    scratch
        .group_condition_indexes
        .extend_from_slice(&program.group_condition_indexes);

    CompiledGpuPredicateProgramAbi {
        abi_version: STRATEGY_GPU_EXECUTABLE_ABI_VERSION,
        plan_fingerprint_hex: program.plan_fingerprint_hex.clone(),
        indicator_slots: scratch.indicator_slots.clone(),
        conditions: scratch.conditions.clone(),
        groups: scratch.groups.clone(),
        group_condition_indexes: scratch.group_condition_indexes.clone(),
        signals: GpuSignalBindingsAbi {
            long_entry_group: encode_optional_group_binding(program.signals.long_entry_group),
            long_exit_group: encode_optional_group_binding(program.signals.long_exit_group),
            short_entry_group: encode_optional_group_binding(program.signals.short_entry_group),
            short_exit_group: encode_optional_group_binding(program.signals.short_exit_group),
        },
    }
}
```

### Important note
This still clones to produce the returned owned ABI. The real payoff only comes if the scratch structure is also what gets uploaded or cached in the missing runtime layer.

### Open question
Without `strategy_gpu_runtime.rs`, it is not possible to tell whether repeated device upload is already cached by `plan_fingerprint_hex`.

---

### Tests to add for this file
1. **compile-into scratch produces identical ABI**
2. **scratch reuse does not retain stale conditions/groups**
3. **plan fingerprint is preserved**

---

## 6. `crates/app_core/src/vram_ma.rs`

This file is intentionally thin. It only re-exports:

- `VramMaInputs`
- `supports_vram_kernel_ma`
- `VramMaComputer`

### Recommendation
Do not change this file in the current pass.

### Why
Any meaningful performance work here belongs either:
- in `vector_ta` internals
- or at the call sites in `gpu_backtest_kernel.rs`

### Only possible change if you want a stronger ownership boundary later
If, later, you want to centralize reusable VRAM-prepared inputs or MA-kernel-specific residency helpers, this file could become the seam for that. That is not necessary now.

---

## 7. `src-tauri/src/state.rs`

This file is central to whether warm-path GPU reuse is actually possible in a real app workflow.

## Current strengths
- It tracks dataset versions.
- It reuses derived/resampled datasets when the parent version is unchanged.
- It has explicit retention logic.

### Confirmed good behavior
`resample_loaded_data` already reuses a cached derived dataset when:
- parent id matches
- timeframe matches
- `source_version` still matches the current base version

Keep that behavior.

---

### Likely issue A: repeated base loads do not reuse existing in-memory datasets
`load_price_data` always:
- calls `data_cache::load_or_build`
- generates a new dataset id
- inserts a new `Arc<Candles>`

even if the same file with the same source identity was just loaded.

### Why it matters
The GPU/runtime signatures are pointer-based. A fresh `Arc<Candles>` means fresh pointers, which means:
- resident market cache misses
- prepared CUDA input cache misses
- warm-path loss after a logically identical load

### Recommended change
Search for an already-loaded matching base dataset first.

### Required cross-file support
`data_cache::SourceIdentity` should derive `PartialEq, Eq`, and `LoadedDataContext` should store it.

### Representative code edit

```rust
#[derive(Debug, Clone)]
pub struct LoadedDataContext {
    pub csv_path: String,
    pub label: String,
    pub timeframe: Option<String>,
    pub source_identity: Option<data_cache::SourceIdentity>,
}
```

Then in `load_price_data`:

```rust
pub fn load_price_data(
    &self,
    path: &str,
    app_cache_dir: &Path,
) -> Result<LoadPriceDataResponse, String> {
    let pathbuf = Path::new(path);
    let source_identity = data_cache::identity_for_source(pathbuf)?;

    if let Some(existing) = self.find_matching_loaded_base(path, source_identity)? {
        return Ok(existing);
    }

    let outcome = data_cache::load_or_build_with_identity(pathbuf, app_cache_dir, source_identity)?;
    let candles = outcome.candles;
    let meta = build_price_data_meta(
        &candles,
        infer_timeframe_label(&candles.timestamp),
        Some(outcome.source_identity.size),
        Some(outcome.source_identity.mtime_utc_ms),
    );

    let id = self.next_dataset_id();
    let access_seq = self.next_access_seq();

    let evicted_ids = {
        let mut map = self
            .candles
            .lock()
            .map_err(|_| "failed to lock candles map".to_string())?;
        map.insert(
            id.clone(),
            DatasetEntry {
                candles: Arc::new(candles),
                version_meta: DatasetVersionMeta {
                    version: 1,
                    kind: DatasetKind::Base,
                    source_version: None,
                },
                last_access_seq: access_seq,
            },
        );
        let protect_ids = HashSet::from([id.clone()]);
        enforce_dataset_retention_locked(&mut map, &protect_ids)
    };

    self.remove_loaded_context_entries(&evicted_ids)?;

    let mut ctx_map = self
        .loaded_data_context
        .lock()
        .map_err(|_| "failed to lock loaded-data context map".to_string())?;
    ctx_map.insert(
        id.clone(),
        LoadedDataContext {
            csv_path: path.to_string(),
            label: dataset_label_from_path(path),
            timeframe: meta.timeframe.clone(),
            source_identity: Some(outcome.source_identity),
        },
    );

    Ok(LoadPriceDataResponse {
        id,
        meta,
        cache_hit: outcome.cache_hit,
        cache_path: outcome
            .cache_path
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default(),
    })
}
```

And helper:

```rust
fn find_matching_loaded_base(
    &self,
    path: &str,
    source_identity: data_cache::SourceIdentity,
) -> Result<Option<LoadPriceDataResponse>, String> {
    let access_seq = self.next_access_seq();

    let ctx_map = self
        .loaded_data_context
        .lock()
        .map_err(|_| "failed to lock loaded-data context map".to_string())?;

    let candidate_id = ctx_map.iter().find_map(|(id, ctx)| {
        if ctx.timeframe.is_none()
            && ctx.csv_path == path
            && ctx.source_identity == Some(source_identity)
        {
            Some(id.clone())
        } else {
            None
        }
    });

    drop(ctx_map);

    let Some(id) = candidate_id else {
        return Ok(None);
    };

    let mut map = self
        .candles
        .lock()
        .map_err(|_| "failed to lock candles map".to_string())?;

    let Some(entry) = map.get_mut(&id) else {
        return Ok(None);
    };

    entry.last_access_seq = access_seq;

    let meta = build_price_data_meta(
        entry.candles.as_ref(),
        infer_timeframe_label(&entry.candles.timestamp),
        Some(source_identity.size),
        Some(source_identity.mtime_utc_ms),
    );

    Ok(Some(LoadPriceDataResponse {
        id,
        meta,
        cache_hit: true,
        cache_path: String::new(),
    }))
}
```

### Expected benefit
Potentially large in real app workflows that reopen the same CSV or revisit the same dataset after a UI refresh.

---

### Likely issue B: retention capacity of 1 base + 1 derived is too aggressive
This is a product choice, but from a warm-path perspective it is narrow.

### Recommended change
Make retention counts configurable, and consider defaulting to 2/2 for desktop-class machines if memory allows.

### Representative code edit

```rust
fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|v| v.trim().parse::<usize>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(default)
}

fn max_active_base_datasets() -> usize {
    env_usize("VECTORBT_MAX_ACTIVE_BASE_DATASETS", 1)
}

fn max_active_derived_datasets() -> usize {
    env_usize("VECTORBT_MAX_ACTIVE_DERIVED_DATASETS", 1)
}
```

Then use those functions instead of fixed constants.

### Why this matters
If you also add a 2-entry GPU resident LRU and a 2-entry prepared-input LRU, aligning the app-state retention window with them gives warm reuse a chance to survive normal user alternation.

### Risk
Memory footprint grows. Keep it configurable.

---

### Optimization opportunity C: `eviction_candidate_locked` sorts everything
The current eviction path sorts candidates. For tiny retention windows this is fine. If you widen capacity and datasets grow in count, a one-pass min selection would be cleaner.

### Recommendation
Do not prioritize this. It is secondary.

---

### Tests to add for this file
1. **identical base source load reuses existing id**
2. **matching base load updates access seq**
3. **derived dataset reuse still works**
4. **retention env overrides are honored**
5. **eviction does not remove protected datasets**

---

## 8. `src-tauri/src/data_cache.rs`

This file is already the closest to your stated preference.

## What this file already does right
- It uses `MaybeUninit` to decode binary vectors without zero-fill.
- It writes atomically.
- It separates source identity from content fingerprinting.
- It keeps the disk cache concern isolated.

### Recommendation
Treat this file as the model for “safe uninitialized memory where justified.”

### Confirmed good pattern
`read_le_vec_i64` and `read_le_vec_f64` already use uninitialized allocation carefully and are worth preserving.

---

### Recommended change A: make `SourceIdentity` comparable
This is needed so `state.rs` can reuse already-loaded base datasets without manual field comparisons.

### Code edit

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceIdentity {
    pub size: u64,
    pub mtime_utc_ms: i64,
}
```

---

### Recommended change B: add `load_or_build_with_identity`
If `state.rs` checks source identity before load, do not immediately recompute it again inside `load_or_build`.

### Representative code edit

```rust
pub fn load_or_build_with_identity(
    csv_path: &Path,
    app_cache_dir: &Path,
    source_identity: SourceIdentity,
) -> Result<LoadOutcome, String> {
    let cache_dir = app_data_cache_dir(app_cache_dir);
    fs::create_dir_all(&cache_dir).map_err(|e| e.to_string())?;

    let content_fp = fingerprint_source(csv_path)?;
    let cache_path = cache_path_for_identity(&cache_dir, source_identity, content_fp);

    if cache_path.exists() {
        let candles = read_cache(&cache_path)?;
        return Ok(LoadOutcome {
            candles,
            cache_hit: true,
            cache_path: Some(cache_path),
            source_identity,
        });
    }

    let candles = Candles::from_csv_path(csv_path).map_err(|e| e.to_string())?;
    write_cache(&cache_path, &candles, source_identity, content_fp)?;
    Ok(LoadOutcome {
        candles,
        cache_hit: false,
        cache_path: Some(cache_path),
        source_identity,
    })
}

pub fn load_or_build(csv_path: &Path, app_cache_dir: &Path) -> Result<LoadOutcome, String> {
    let source_identity = identity_for_source(csv_path)?;
    load_or_build_with_identity(csv_path, app_cache_dir, source_identity)
}
```

### Expected benefit
Small on its own, but important when combined with `state.rs` reuse.

---

### Optimization opportunity C: explicitly size readers/writers
Large cache files can benefit from larger buffer capacities.

### Representative code edit

```rust
const IO_BUF_CAPACITY: usize = 1 << 20; // 1 MiB

fn read_cache(path: &Path) -> Result<Candles, String> {
    let f = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut r = std::io::BufReader::with_capacity(IO_BUF_CAPACITY, f);
    // ...
}

fn write_cache(
    path: &Path,
    candles: &Candles,
    source_identity: SourceIdentity,
    content_fp: u64,
) -> Result<(), String> {
    let tmp = path.with_extension("tmp");
    let f = fs::File::create(&tmp).map_err(|e| e.to_string())?;
    let mut w = std::io::BufWriter::with_capacity(IO_BUF_CAPACITY, f);
    // ...
}
```

### Expected benefit
Usually modest, but essentially free and clean.

---

### Tests to add for this file
1. **`SourceIdentity` equality works**
2. **`load_or_build_with_identity` matches `load_or_build`**
3. **buffer-capacity change does not affect cache correctness**
4. **existing `MaybeUninit` readers still round-trip correctly**

---

## 9. `src-tauri/src/bin/ta_app_path_faceoff.rs`

This file is the benchmark-boundary anchor for the app-path side of section 3.

## What it already does well
- separates load timing and optional resample timing
- performs warmup trials before measured trials
- runs through `AppState` and the blocking optimization call
- records the returned backend result

That is already useful. The main improvement here is better diagnostic granularity.

### Confirmed limitation A: trials only record wall-clock runtime
Current per-trial output is:

```rust
struct TrialArtifact {
    trial_index: usize,
    runtime_ms_wall: f64,
    result: ta_app_core::BackendOptimizationResult,
}
```

That makes it hard to tell whether a speedup came from:
- data lookup
- route change
- GPU indicator prep
- H2D upload reduction
- something else

### Recommended change
Split trial timing into:
- total wall time
- engine-only wall time
- optional kernel sub-timings if available

### Representative code edit

```rust
#[derive(Debug, Serialize)]
struct TrialTimingArtifact {
    total_wall_ms: f64,
    state_lookup_ms: f64,
    engine_wall_ms: f64,
    gpu_indicator_prep_ms: Option<u64>,
    h2d_upload_ms: Option<u64>,
}

#[derive(Debug, Serialize)]
struct TrialArtifact {
    trial_index: usize,
    timing: TrialTimingArtifact,
    result: ta_app_core::BackendOptimizationResult,
}
```

Then:

```rust
fn run_trial(
    req: &DoubleMaRequest,
    state: &AppState,
    data_id: &str,
    trial_index: usize,
) -> Result<TrialArtifact, String> {
    let total_started = Instant::now();

    let lookup_started = Instant::now();
    let candles = state.get_candles(data_id)?;
    let state_lookup_ms = lookup_started.elapsed().as_secs_f64() * 1000.0;

    let cancel = AtomicBool::new(false);
    let engine_started = Instant::now();
    let result = ta_app_core::run_double_ma_optimization_blocking_with_candles(
        req.clone(),
        candles.as_ref(),
        &cancel,
        None,
    )?;
    let engine_wall_ms = engine_started.elapsed().as_secs_f64() * 1000.0;

    let total_wall_ms = total_started.elapsed().as_secs_f64() * 1000.0;

    Ok(TrialArtifact {
        trial_index,
        timing: TrialTimingArtifact {
            total_wall_ms,
            state_lookup_ms,
            engine_wall_ms,
            gpu_indicator_prep_ms: None,
            h2d_upload_ms: None,
        },
        result,
    })
}
```

### Expected benefit
Not a runtime speedup, but a large gain in timing-boundary quality.

---

### Recommended change B: expose kernel sub-timings when possible
`gpu_backtest_kernel.rs` already tracks:
- `gpu_indicator_prep_ms`
- `h2d_upload_ms`

If `ta_app_core` can expose them after each optimization run, feed them into the faceoff artifact.

### Important note
This requires a tiny adjacent-file change outside the uploaded set, because the timing accessor in `gpu_backtest_kernel.rs` is `pub(crate)` and thread-local.

### Suggested path
- keep the timing local to the same thread
- expose a small `ta_app_core` helper that reads it immediately after the blocking optimization call returns
- attach it to the backend result or return it alongside the result

---

### Likely issue C: same-thread harness may overstate real warm reuse
Because this is a simple blocking binary, it may hit the same thread-local caches repeatedly more reliably than the real Tauri request path.

### Recommended change
Do not invalidate the harness. Instead:
- keep it as the canonical same-thread warm-path measurement
- explicitly document that cache reuse depends on thread affinity
- optionally add a second harness later if you need cross-thread realism

---

### Optimization opportunity D: include summary stats in the artifact
At the end of the run, include:
- median measured trial time
- p95 if trial count is high enough
- median `h2d_upload_ms`
- median `gpu_indicator_prep_ms`

This makes regressions easier to spot in CI artifacts.

### Representative code edit

```rust
#[derive(Debug, Serialize)]
struct FaceoffSummary {
    measured_median_wall_ms: f64,
    measured_min_wall_ms: f64,
    measured_max_wall_ms: f64,
}

fn summarize_trials(trials: &[TrialArtifact]) -> FaceoffSummary {
    let mut xs: Vec<f64> = trials.iter().map(|t| t.timing.total_wall_ms).collect();
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = xs.get(xs.len() / 2).copied().unwrap_or(0.0);
    FaceoffSummary {
        measured_median_wall_ms: median,
        measured_min_wall_ms: xs.first().copied().unwrap_or(0.0),
        measured_max_wall_ms: xs.last().copied().unwrap_or(0.0),
    }
}
```

---

### Tests to add for this file
1. **artifact schema round-trips with new timing fields**
2. **summary stats are correct**
3. **load/resample timings remain separate**
4. **warmup/measured trial counts are unchanged**

---

# Cross-file implementation plan

## Wave 1: remove the most obvious memory-copy and sync waste

### 1. Add shared `f64 -> f32` uninitialized conversion helpers
Use the existing allocation helpers rather than ad hoc `collect()` calls.

Apply to:
- `gpu_backtest_kernel.rs`
- `dynamic_indicator_adapter.rs`

### 2. Batch resident market uploads
Implement `ResidentUploadBatch` and update `ensure_resident_market_series`.

### 3. Make prepared CUDA fields lazy
Do not convert/upload all OHLCV fields at rebuild time. Prepare only on demand.

### 4. Change dynamic slot cache values to `Arc`
This is a straightforward host-side memory-copy reduction.

### 5. Make CPU context lazy
Do not compute it for GPU-routed plans unless actually needed.

---

## Wave 2: preserve warm-path identity and improve app-path measurement

### 6. Make `SourceIdentity` comparable and reuse loaded base datasets
Implement:
- `PartialEq, Eq` on `SourceIdentity`
- `load_or_build_with_identity`
- `LoadedDataContext.source_identity`
- `find_matching_loaded_base`

### 7. Widen retention slightly or make it configurable
At minimum, make it an env/config decision.

### 8. Improve `ta_app_path_faceoff`
Add:
- total vs engine timing split
- summary stats
- kernel sub-timings if adjacent exports are added

---

## Wave 3: tighten remaining warm/cold bimodality sources

### 9. Small LRU for resident market and prepared CUDA datasets
Keep capacities tiny:
- 2 by default is enough to start
- maybe 3 or 4 if profiling proves it

### 10. Cache `first_valid`
Cheap, safe, cumulative payoff.

### 11. Bundle repeated plan compilation
Refactor preview/prepare flow to stop redoing work.

---

## Wave 4: only if measurements still demand it

### 12. Revisit thread-local cache scope
Only after proving that request thread hopping is materially harming warm reuse.

### 13. Revisit GPU program/ABI compile/upload reuse
Only after inspecting the missing runtime cache layer.

### 14. Revisit host `f64` CPU-context representation
Only if dynamic GPU slot staging still dominates after the easier wins land.

---

# Suggested benchmark plan

## Primary benchmark boundary
Use `ta_app_path_faceoff` as the truth source for this surface.

### Baseline scenarios
1. **same dataset, same route, repeated warm runs**
2. **base dataset then derived dataset alternating**
3. **same dataset, `close` source only**
4. **same dataset, non-`close` source override**
5. **dynamic strategy with duplicated indicator expressions**
6. **dynamic strategy GPU-routed with shadow disabled**
7. **dynamic strategy GPU-routed with shadow enabled**

## Metrics to track
- total wall runtime
- engine wall runtime
- `gpu_indicator_prep_ms`
- `h2d_upload_ms`
- resident market cache hit/miss counts
- prepared CUDA dataset hit/miss counts
- source override hit/miss counts
- whether prepared-device path degraded to host fallback

## Success criteria
### For `gpu_backtest_kernel.rs`
- lower first-hit `h2d_upload_ms`
- less cold/warm variance after dataset switches
- lower first-use penalty for optional fields

### For `dynamic_indicator_adapter.rs`
- lower indicator prep time when only `close` is needed
- lower repeated dynamic staging time on same dataset
- visible path attribution when device-prepared path is lost

### For `strategy_dynamic.rs`
- lower host allocation traffic
- lower dynamic prep wall time
- no CPU-context prep in GPU-only runs unless needed

### For `state.rs`
- same-source reload should preserve warm behavior
- fewer avoidable cache invalidations in repeated workflow loops

---

# Detailed risk notes

## Low-risk changes
- uninitialized conversion helpers using existing allocation utilities
- `Arc`-based dynamic output dedupe
- `SourceIdentity` equality
- `load_or_build_with_identity`
- faceoff artifact timing expansion
- `first_valid` caching
- buffer-capacity tuning in `data_cache.rs`

## Medium-risk changes
- batched resident uploads
- lazy prepared CUDA fields
- lazy CPU context
- two-entry retention windows
- small LRU caches for resident/prepared datasets

## Higher-risk changes
- changing cache scope away from thread-local
- changing host/device series representation in dynamic prep
- changing ABI/program upload ownership without seeing the runtime cache layer

---

# Minimal code-edit checklist

## `gpu_backtest_kernel.rs`
- [ ] add `ResidentUploadBatch`
- [ ] change `ensure_resident_market_series` to batch newly-needed uploads
- [ ] add reusable `cast_f64_slice_to_f32_vec`
- [ ] change `simple_returns_from_log_returns_f32` to uninitialized fill
- [ ] optionally add tiny resident-market LRU
- [ ] add cache hit/miss counters if practical

## `dynamic_indicator_adapter.rs`
- [ ] add `field_mask` to `PreparedCudaDatasetSignature`
- [ ] convert prepared dataset cache to per-device small LRU
- [ ] lazy-host/lazy-device field prep
- [ ] shared uninitialized `f64 -> f32` helper
- [ ] add `first_valid_cache`
- [ ] make prepared-device fallback explicit in attribution

## `strategy_dynamic.rs`
- [ ] change dynamic slot series values to `Arc<DynamicIndicatorComputeOutput>`
- [ ] replace stringified local cache keys with a structured hashed key
- [ ] make CPU context lazy with `OnceLock`
- [ ] reduce repeated `compile_strategy_plan` calls
- [ ] keep artifact logging out of perf runs

## `strategy_plan.rs`
- [ ] add compile bundle helper
- [ ] keep route defaults unchanged in this pass

## `strategy_gpu_program.rs`
- [ ] optional reusable ABI scratch
- [ ] defer deeper caching until runtime layer is visible

## `vram_ma.rs`
- [ ] no change

## `state.rs`
- [ ] store `source_identity` in `LoadedDataContext`
- [ ] add `find_matching_loaded_base`
- [ ] reuse identical base loads
- [ ] make retention counts configurable

## `data_cache.rs`
- [ ] derive `PartialEq, Eq` for `SourceIdentity`
- [ ] add `load_or_build_with_identity`
- [ ] optionally increase reader/writer buffer capacity

## `ta_app_path_faceoff.rs`
- [ ] split total vs engine timing
- [ ] add summary stats
- [ ] if adjacent export is available, include kernel sub-timings

---

# Final recommendation

If you want the highest return for the least risk, implement in this order:

1. `strategy_dynamic.rs`: **remove output clones + make CPU context lazy**
2. `gpu_backtest_kernel.rs`: **batch resident uploads**
3. `dynamic_indicator_adapter.rs`: **lazy field prep + two-entry prepared cache + first-valid cache**
4. `state.rs` + `data_cache.rs`: **reuse identical base loads**
5. `ta_app_path_faceoff.rs`: **measure total vs engine vs kernel sub-timings**
6. `gpu_backtest_kernel.rs` + `dynamic_indicator_adapter.rs`: **small LRUs**
7. only then revisit `strategy_gpu_program.rs`, cache scope, and deeper representation changes

That sequence keeps the work grounded, measurable, and compatible with your stated goals:
- fewer unnecessary memory copies
- more reuse of already-prepared data
- more safe use of uninitialized memory
- better warm-path behavior
- better attribution when something still falls back or goes cold
