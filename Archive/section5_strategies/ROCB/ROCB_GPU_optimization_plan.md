# ROCB strategy-local GPU-native optimization and validation plan

Prepared against `gpt54_feature_review_map.md`, section **`## 5. Strategy-local GPU-native optimization and validation routes`**, and limited to the ROCB strategy surface.

Covered files:

- `rocb.rs`
- `rocb_gpu_kernel.rs`
- `gpu_native_expansion_opt.rs`
- `composite_validation_workflow.rs`

This document follows the section-5 review contract from the map:

- one strategy at a time
- strategy-local hot-path review
- CPU/GPU payoff review
- staging/fallback review
- per-strategy benchmark review
- validation-throughput review, not just optimizer throughput

It stays inside the repo’s optimization-first, parity-preserving scope. It does **not** add new ROCB features or new user-facing knobs.

---

## Executive summary

The highest-value ROCB work is **not** changing the kernel math first. The highest-value work is shrinking the repeated staging boundary around the kernel.

Today, `rocb_gpu_kernel.rs` has a thread-local runtime, but the hot path still does all of the following **every evaluation call**:

- uploads `close_f32` from host to device
- allocates a fresh `d_lr`
- recomputes log returns on device
- uploads combo vectors (`channel_row_index`, `atr_row_index`, `start_t`, `end_t`)
- allocates a fresh `d_out`
- allocates and zero-fills a fresh host readback vector

That means ROCB is already partly GPU-native, but not yet *resident* in the way needed for strong warm-path payoff.

At the same time, `rocb.rs` already contains the correct building blocks for device-native prep (`compute_rocb_donchian_output_sweep_cuda_device`, `compute_rocb_atr_sweep_cuda_device`), but the scalar helper path still prefers `HostF32` readback and silently falls back to CPU. That can erase GPU benefit if the wrong prep path is used by the optimizer/validator caller.

The plan below is designed to:

1. upload market data once per candle set
2. compute log returns once per candle set
3. upload combo batches once per candidate batch
4. reuse output/scratch buffers across relaunches
5. keep host readback safe and reuse-based, not unsound
6. surface prep fallback reasons explicitly
7. add ROCB-specific optimizer and validation benchmarks before deeper tuning

---

## Current state by file

### `rocb.rs`

Relevant current areas:

- `compute_indicator_period_sweep_cuda_device` at `rocb.rs:310-360`
- `compute_rocb_donchian_output_sweep_cuda_device` at `rocb.rs:363-382`
- `compute_rocb_atr_sweep_cuda_device` at `rocb.rs:385-403`
- `compute_indicator_f64` at `rocb.rs:405-454`
- `compute_rocb_breakout_signals` at `rocb.rs:456-506`
- `compute_rocb_risk_signals` at `rocb.rs:508-534`
- `build_rocb_desired_positions` at `rocb.rs:557-677`

What is already good:

- ROCB already has GPU device sweep helpers for Donchian and ATR.
- ROCB already resolves period-to-row mapping through `RocbPeriodSweepDevice::row_index_for_period`.
- The CPU reference logic is simple and auditable.

What is currently weak:

- `compute_indicator_f64` tries CUDA host readback and silently falls back to CPU on failure or missing prep.
- `compute_rocb_breakout_signals` computes Donchian upper and lower separately; that is acceptable for correctness, but it means two indicator dispatches and two host allocations on that path.
- There is no prep-route diagnostic summary, so the caller cannot easily tell whether ROCB remained GPU-backed.
- `build_rocb_desired_positions` allocates a fresh `Vec<i32>` every call, which prevents buffer reuse if the caller evaluates many CPU-reference batches.

### `rocb_gpu_kernel.rs`

Relevant current area:

- `eval_rocb_metrics_gpu_kernel_device_rows` at `rocb_gpu_kernel.rs:173-318`

What is already good:

- There is already a per-device thread-local runtime.
- `d_lr` and `d_out` already use device-side uninitialized allocation, which is correct and desirable.
- The kernel accepts precomputed Donchian and ATR matrices by device pointer; this is the right shape.

What is currently weak:

- `close_f32` is uploaded every call.
- log returns are recomputed every call.
- combo vectors are uploaded every call.
- `d_out` is allocated every call.
- a zero-filled host vector is allocated every call.
- runtime setup still repeats some launch-shape/device-cap work.

### `gpu_native_expansion_opt.rs`

Relevant current issue:

- ROCB is not benchmarked at all in this file.

Implication:

- there is no ROCB-specific optimizer throughput evidence here
- there is no ROCB CPU-vs-GPU benchmark pair here
- there is no ROCB regression target on the optimizer timing boundary

### `composite_validation_workflow.rs`

Relevant current issue:

- ROCB exists only in the generic profile bench at `50_000` candles / `8` candidates
- there is no dedicated heavier ROCB GPU validation case similar to the ASR-heavy case
- CPU and GPU heavy ROCB validation comparison is under-specified

Implication:

- ROCB validation throughput and scaling are under-measured
- ROCB could still have a good optimizer story but a weak validation story, and the current benches would not show that clearly

---

## Prioritized optimization goals

### P0: Eliminate repeated market uploads and repeated log-return computation

This is the most obvious hot-path waste in `rocb_gpu_kernel.rs`.

### P0: Eliminate repeated combo uploads where batches are reused

This matters for validation and any repeated finalist scoring loop.

### P0: Reuse device output buffers and host readback buffers across launches

This reduces repeated allocations and repeated memset/zero-fill work.

### P1: Surface ROCB prep route and fallback reasons

This is necessary so that optimization wins are not accidentally lost behind a silent CPU fallback.

### P1: Add ROCB-specific optimizer and validation benches

Without this, tuning ROCB remains guesswork.

### P2: Add in-place CPU-reference builders for reusable buffers

This matters most if ROCB falls back to CPU or when parity/reference tests are run in large batches.

### P3: Only pursue host-side uninitialized readback when done through a sound raw-copy helper

The attached files do **not** show such a helper, so the first safe step is buffer reuse, not unsafe host-uninit tricks.

---

## Soundness rules for uninitialized memory

This matters because you explicitly want to use uninitialized memory where possible.

### Safe to use now

These are already good candidates and should stay that way:

- `DeviceBuffer::<f32>::uninitialized(signals.cols)` for `d_lr`
- `DeviceBuffer::<f32>::uninitialized(out_len)` for `d_out`
- any reusable device scratch that is fully overwritten before read

### Safe with reuse

For host metrics, the safest immediate optimization is:

- allocate a host `Vec<f32>` once
- reuse it across repeated calls
- only grow it when needed
- copy into `&mut host_metrics[..out_len]`

That avoids most repeated allocation and zero-fill on the hot path after the first growth.

### Do **not** do this

Do **not** use this pattern for host readback:

```rust
let mut v: Vec<f32> = Vec::with_capacity(out_len);
unsafe { v.set_len(out_len); }
d_out.copy_to(v.as_mut_slice())?;
```

That pattern is not something I recommend here. It creates a `&mut [f32]` over memory that Rust has not initialized, which is not the same as device-side uninitialized allocation.

### When host-side uninitialized readback is acceptable

Only do host uninitialized readback if your repo already has one of these:

- a repo-local `alloc_policy` helper that safely wraps raw device-to-host copy into `MaybeUninit`
- a raw CUDA copy helper that writes to a raw pointer and only materializes `Vec<f32>` after the copy is complete

Because those helpers are not in the attached files, the plan below treats **host buffer reuse** as the primary change and **host uninit** as optional follow-up.

---

# 1) `rocb_gpu_kernel.rs`

## Hot-path problems

At `rocb_gpu_kernel.rs:209-221`, the current function does all uploads and device allocations every call.

At `rocb_gpu_kernel.rs:307-311`, the current function synchronizes and then allocates a fresh zeroed host vector for readback.

These costs are exactly the kind of strategy-local staging waste called out by the section-5 prompt.

## Recommended design

Introduce three reusable concepts:

1. **Uploaded market**: owns `d_close` and `d_lr` for one candle set
2. **Uploaded combos**: owns all combo vectors for one candidate batch
3. **Runtime scratch**: owns reusable `d_out` and reusable host metrics buffer

Keep the existing entrypoint as a convenience wrapper, but add a new hot-path entrypoint that accepts uploaded inputs.

## Concrete edits

### 1.1 Add a small runtime helper

Add a helper so every call site shares the same runtime-lookup pattern.

```rust
fn with_runtime<T>(
    device_id: u32,
    f: impl FnOnce(&mut RocbCudaRuntime) -> Result<T, String>,
) -> Result<T, String> {
    ROCB_GPU_RUNTIME.with(|cell| {
        let mut runtimes = cell.borrow_mut();
        if !runtimes.contains_key(&device_id) {
            runtimes.insert(device_id, RocbCudaRuntime::new(device_id)?);
        }
        let rt = runtimes
            .get_mut(&device_id)
            .ok_or_else(|| "ROCB GPU runtime missing".to_string())?;
        rt.set_current()?;
        f(rt)
    })
}
```

This is not a huge speedup by itself. It just makes the larger refactor simpler and less error-prone.

### 1.2 Add reusable runtime scratch

```rust
struct RocbGpuScratch {
    d_out: Option<DeviceBuffer<f32>>,
    d_out_capacity: usize,
    host_metrics: Vec<f32>,
}

impl Default for RocbGpuScratch {
    fn default() -> Self {
        Self {
            d_out: None,
            d_out_capacity: 0,
            host_metrics: Vec::new(),
        }
    }
}

impl RocbGpuScratch {
    fn ensure_out_capacity(&mut self, len: usize) -> Result<(), String> {
        if self.d_out_capacity < len {
            self.d_out = Some(
                unsafe { DeviceBuffer::<f32>::uninitialized(len) }
                    .map_err(|e| e.to_string())?,
            );
            self.d_out_capacity = len;
        }
        if self.host_metrics.len() < len {
            self.host_metrics.resize(len, 0.0);
        }
        Ok(())
    }

    fn d_out_ptr_raw(&mut self) -> Result<u64, String> {
        let d_out = self
            .d_out
            .as_mut()
            .ok_or_else(|| "ROCB GPU output scratch missing".to_string())?;
        Ok(d_out.as_device_ptr().as_raw())
    }

    fn copy_out_to_host(&mut self, len: usize) -> Result<&[f32], String> {
        let d_out = self
            .d_out
            .as_ref()
            .ok_or_else(|| "ROCB GPU output scratch missing".to_string())?;
        d_out
            .copy_to(&mut self.host_metrics[..len])
            .map_err(|e| e.to_string())?;
        Ok(&self.host_metrics[..len])
    }
}
```

Then add it to the runtime:

```rust
struct RocbCudaRuntime {
    context: Arc<Context>,
    module: Module,
    stream: Stream,
    scratch: RocbGpuScratch,
}

impl RocbCudaRuntime {
    fn new(device_id: u32) -> Result<Self, String> {
        cust::init(CudaFlags::empty()).map_err(|e| e.to_string())?;
        let device = Device::get_device(device_id).map_err(|e| e.to_string())?;
        let context = Arc::new(Context::new(device).map_err(|e| e.to_string())?);

        let ptx: &str = include_str!(concat!(env!("OUT_DIR"), "/double_crossover.ptx"));
        let jit_opts = &[
            ModuleJitOption::DetermineTargetFromContext,
            ModuleJitOption::OptLevel(cust::module::OptLevel::O2),
        ];
        let module = match Module::from_ptx(ptx, jit_opts) {
            Ok(m) => m,
            Err(_) => Module::from_ptx(ptx, &[ModuleJitOption::DetermineTargetFromContext])
                .or_else(|_| Module::from_ptx(ptx, &[]))
                .map_err(|e| e.to_string())?,
        };

        let stream = Stream::new(StreamFlags::NON_BLOCKING, None).map_err(|e| e.to_string())?;

        Ok(Self {
            context,
            module,
            stream,
            scratch: RocbGpuScratch::default(),
        })
    }
}
```

Why this helps:

- `d_out` becomes grow-only instead of per-call
- host readback buffer becomes grow-only instead of per-call
- repeated calls with similar batch sizes stop paying repeated alloc costs

### 1.3 Add uploaded market and uploaded combo objects

```rust
pub struct RocbGpuUploadedMarket {
    device_id: u32,
    cols: usize,
    d_close: DeviceBuffer<f32>,
    d_lr: DeviceBuffer<f32>,
}

pub struct RocbGpuUploadedCombos {
    device_id: u32,
    len: usize,
    d_channel_row_index: DeviceBuffer<i32>,
    d_atr_row_index: DeviceBuffer<i32>,
    d_start_t: DeviceBuffer<i32>,
    d_end_t: DeviceBuffer<i32>,
}

pub fn upload_rocb_market_to_device(
    device_id: u32,
    close_f32: &[f32],
) -> Result<RocbGpuUploadedMarket, String> {
    if close_f32.is_empty() {
        return Err("ROCB GPU market upload requires non-empty close series".to_string());
    }

    with_runtime(device_id, |rt| {
        let d_close = DeviceBuffer::from_slice(close_f32).map_err(|e| e.to_string())?;
        let mut d_lr = unsafe { DeviceBuffer::<f32>::uninitialized(close_f32.len()) }
            .map_err(|e| e.to_string())?;
        launch_compute_log_returns_f32(rt, device_id, &d_close, &mut d_lr, close_f32.len())?;
        rt.stream.synchronize().map_err(|e| e.to_string())?;

        Ok(RocbGpuUploadedMarket {
            device_id,
            cols: close_f32.len(),
            d_close,
            d_lr,
        })
    })
}

pub fn upload_rocb_combo_batch_to_device(
    device_id: u32,
    combos: &RocbGpuComboBatch<'_>,
) -> Result<RocbGpuUploadedCombos, String> {
    let len = combos.channel_row_index.len();
    if combos.atr_row_index.len() != len
        || combos.start_t.len() != len
        || combos.end_t.len() != len
    {
        return Err("ROCB GPU combo vector length mismatch".to_string());
    }

    with_runtime(device_id, |_rt| {
        Ok(RocbGpuUploadedCombos {
            device_id,
            len,
            d_channel_row_index: DeviceBuffer::from_slice(combos.channel_row_index)
                .map_err(|e| e.to_string())?,
            d_atr_row_index: DeviceBuffer::from_slice(combos.atr_row_index)
                .map_err(|e| e.to_string())?,
            d_start_t: DeviceBuffer::from_slice(combos.start_t)
                .map_err(|e| e.to_string())?,
            d_end_t: DeviceBuffer::from_slice(combos.end_t)
                .map_err(|e| e.to_string())?,
        })
    })
}
```

Why this helps:

- one candle set can be reused across many ROCB launches
- one combo batch can be reused across validation loops or repeated finalist scoring
- the expensive repeated `close -> d_close -> d_lr` setup disappears from the warm path

### 1.4 Add a new uploaded-input hot-path entrypoint

```rust
pub fn eval_rocb_metrics_gpu_kernel_uploaded(
    device_id: u32,
    market: &RocbGpuUploadedMarket,
    signals: &RocbGpuDeviceSignalMatrices<'_>,
    combos: &RocbGpuUploadedCombos,
    strategy: &StrategyConfig,
    rocb: &RocbStrategyParams,
) -> Result<Vec<Metrics>, String> {
    if market.device_id != device_id {
        return Err("ROCB uploaded market device mismatch".to_string());
    }
    if combos.device_id != device_id {
        return Err("ROCB uploaded combo batch device mismatch".to_string());
    }
    if market.cols != signals.cols {
        return Err("ROCB uploaded market length does not match signal columns".to_string());
    }
    if strategy.uses_execution_overlays()
        || strategy.stop_loss_pct().is_some()
        || strategy.take_profit_pct().is_some()
    {
        return Err(
            "ROCB GPU kernel currently supports strategy execution without overlays/stop-loss/take-profit."
                .to_string(),
        );
    }

    let combo_count = combos.len;
    if combo_count == 0 {
        return Ok(Vec::new());
    }

    let out_len = combo_count
        .checked_mul(ROCB_METRICS_STRIDE)
        .ok_or_else(|| "ROCB GPU metrics buffer overflow".to_string())?;

    with_runtime(device_id, |rt| {
        rt.scratch.ensure_out_capacity(out_len)?;
        let out_ptr_raw = rt.scratch.d_out_ptr_raw()?;

        let kernel = rt
            .module
            .get_function("rocb_backtest_metrics_f32")
            .map_err(|e| e.to_string())?;
        let device = Device::get_device(device_id).map_err(|e| e.to_string())?;
        let device_caps = detect_device_launch_caps(device);
        let launch = choose_launch_shape_1d_from_candidates(
            combo_count,
            device_caps,
            detect_kernel_launch_caps(&kernel, device_caps),
            &[32u32, 64, 128, 256],
        );

        let mut flags: u32 = 0;
        if strategy.long_only {
            flags |= 1u32 << 0;
        }
        if !strategy.allow_flip {
            flags |= 1u32 << 1;
        }
        if strategy.trade_on_next_bar {
            flags |= 1u32 << 2;
        }
        flags |= 1u32 << 4;

        let commission = strategy.effective_commission().clamp(0.0, 0.95) as f32;

        let mut close_ptr = market.d_close.as_device_ptr().as_raw();
        let mut lr_ptr = market.d_lr.as_device_ptr().as_raw();
        let mut upper_ptr = signals.upper_row_major_ptr;
        let mut lower_ptr = signals.lower_row_major_ptr;
        let mut channel_rows_i = signals.channel_rows as i32;
        let mut channel_stride_i = signals.channel_stride as i32;
        let mut atr_ptr = signals.atr_row_major_ptr;
        let mut atr_rows_i = signals.atr_rows as i32;
        let mut atr_stride_i = signals.atr_stride as i32;
        let mut combo_channel_ptr = combos.d_channel_row_index.as_device_ptr().as_raw();
        let mut combo_atr_ptr = combos.d_atr_row_index.as_device_ptr().as_raw();
        let mut combo_start_ptr = combos.d_start_t.as_device_ptr().as_raw();
        let mut combo_end_ptr = combos.d_end_t.as_device_ptr().as_raw();
        let mut combo_count_i = combo_count as i32;
        let mut t_i = signals.cols as i32;
        let mut flags_u = flags;
        let mut commission_f = commission;
        let mut atr_stop_mult_f = rocb.atr_stop_mult as f32;
        let mut max_trade_bars_i = i32::from(rocb.max_trade_bars.max(1));
        let mut out_stride_i = ROCB_METRICS_STRIDE as i32;
        let mut out_ptr = out_ptr_raw;

        unsafe {
            let args: &mut [*mut c_void] = &mut [
                &mut close_ptr as *mut _ as *mut c_void,
                &mut lr_ptr as *mut _ as *mut c_void,
                &mut t_i as *mut _ as *mut c_void,
                &mut upper_ptr as *mut _ as *mut c_void,
                &mut lower_ptr as *mut _ as *mut c_void,
                &mut channel_rows_i as *mut _ as *mut c_void,
                &mut channel_stride_i as *mut _ as *mut c_void,
                &mut atr_ptr as *mut _ as *mut c_void,
                &mut atr_rows_i as *mut _ as *mut c_void,
                &mut atr_stride_i as *mut _ as *mut c_void,
                &mut combo_channel_ptr as *mut _ as *mut c_void,
                &mut combo_atr_ptr as *mut _ as *mut c_void,
                &mut combo_start_ptr as *mut _ as *mut c_void,
                &mut combo_end_ptr as *mut _ as *mut c_void,
                &mut combo_count_i as *mut _ as *mut c_void,
                &mut flags_u as *mut _ as *mut c_void,
                &mut commission_f as *mut _ as *mut c_void,
                &mut atr_stop_mult_f as *mut _ as *mut c_void,
                &mut max_trade_bars_i as *mut _ as *mut c_void,
                &mut out_stride_i as *mut _ as *mut c_void,
                &mut out_ptr as *mut _ as *mut c_void,
            ];
            rt.stream
                .launch(
                    &kernel,
                    (launch.grid_x.max(1), 1, 1),
                    (launch.block_x.max(1), 1, 1),
                    0,
                    args,
                )
                .map_err(|e| e.to_string())?;
        }

        rt.stream.synchronize().map_err(|e| e.to_string())?;
        let host_metrics = rt.scratch.copy_out_to_host(out_len)?;

        let mut out = Vec::with_capacity(combo_count);
        for row in host_metrics.chunks_exact(ROCB_METRICS_STRIDE) {
            out.push(decode_metrics_row(row));
        }
        Ok(out)
    })
}
```

### 1.5 Keep the old entrypoint as a compatibility wrapper

This lets you migrate callers incrementally.

```rust
pub fn eval_rocb_metrics_gpu_kernel_device_rows(
    device_id: u32,
    signals: &RocbGpuDeviceSignalMatrices<'_>,
    combos: &RocbGpuComboBatch<'_>,
    strategy: &StrategyConfig,
    rocb: &RocbStrategyParams,
) -> Result<Vec<Metrics>, String> {
    validate_inputs(signals, combos)?;

    let market = upload_rocb_market_to_device(device_id, signals.close_f32)?;
    let uploaded_combos = upload_rocb_combo_batch_to_device(device_id, combos)?;
    eval_rocb_metrics_gpu_kernel_uploaded(
        device_id,
        &market,
        signals,
        &uploaded_combos,
        strategy,
        rocb,
    )
}
```

This wrapper remains allocation-heavy, but it preserves current behavior while you wire the real callers over to the uploaded path.

### 1.6 Optional but worthwhile cleanup: cache small repeated launch metadata

You currently call `Device::get_device(device_id)` and `detect_device_launch_caps(device)` in both log-return launch and main kernel launch.

This is a small cost compared to copies, but if the surrounding hot path is cleaned up, it becomes more visible.

Recommended after the larger refactor:

- cache device caps in `RocbCudaRuntime` if the type is easy to store
- cache function handles if your local `cust` usage allows it cleanly

This is **not** the first change to make.

## Expected payoff from `rocb_gpu_kernel.rs`

Most likely benefit ordering:

1. uploaded market reuse (`d_close` + `d_lr`)
2. uploaded combo reuse
3. reusable `d_out`
4. reusable host readback buffer
5. cached small runtime metadata

---

# 2) `rocb.rs`

## Hot-path problems

The current file already has the right GPU sweep primitives, but it lacks two things that matter operationally:

- explicit prep-route attribution
- an in-place CPU-reference builder for reuse

`compute_indicator_f64` currently hides whether CUDA was actually used or whether it silently fell back to CPU.

## Recommended design

### 2.1 Add ROCB prep diagnostics

Add a small route/fallback data structure.

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RocbPrepRoute {
    Cpu,
    CudaHostReadback,
    CudaFallbackToCpu,
    CudaDeviceSweep,
}

#[derive(Clone, Debug, Default)]
pub struct RocbPrepDiagnostics {
    pub breakout_upper_route: Option<RocbPrepRoute>,
    pub breakout_lower_route: Option<RocbPrepRoute>,
    pub atr_route: Option<RocbPrepRoute>,
    pub fallback_reasons: Vec<String>,
}
```

This gives the optimizer/validator caller a concrete place to record whether ROCB prep stayed on the intended route.

### 2.2 Add a traced helper beside `compute_indicator_f64`

Do **not** remove the existing helper immediately. Add a traced variant and have the existing helper call through it.

```rust
fn compute_indicator_f64_traced(
    candles: &Candles,
    ma_source: &str,
    backend: &Backend,
    indicator_id: &str,
    output_id: &str,
    params: &[IndicatorParamKV],
    #[cfg(feature = "cuda")] prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<(Vec<f64>, RocbPrepRoute, Option<String>), String> {
    #[cfg(not(feature = "cuda"))]
    {
        let out = compute_dynamic_indicator_cpu(
            candles,
            ma_source,
            indicator_id,
            Some(output_id),
            params,
            Kernel::Auto,
        )
        .map_err(|e| e.to_string())?;
        return Ok((series_to_f64(out), RocbPrepRoute::Cpu, None));
    }

    #[cfg(feature = "cuda")]
    {
        let descriptor = find_dynamic_indicator_descriptor(indicator_id).ok_or_else(|| {
            format!("ROCB indicator '{indicator_id}' is not registered for dynamic dispatch")
        })?;

        let prefer_gpu = backend_prefers_gpu(backend)
            && descriptor.prefers_gpu_by_speedup
            && (descriptor.supports_cuda_single || descriptor.supports_cuda_sweep);

        if prefer_gpu {
            match prepared_cuda_inputs {
                Some(prepared) => {
                    match compute_dynamic_indicator_cuda_with_inputs(
                        prepared,
                        indicator_id,
                        Some(output_id),
                        params,
                        Kernel::Auto,
                        vector_ta::indicators::dispatch::CudaOutputTarget::HostF32,
                    ) {
                        Ok(values) => {
                            return Ok((
                                cuda_series_to_f64(values)?,
                                RocbPrepRoute::CudaHostReadback,
                                None,
                            ));
                        }
                        Err(err) => {
                            let cpu = compute_dynamic_indicator_cpu(
                                candles,
                                ma_source,
                                indicator_id,
                                Some(output_id),
                                params,
                                Kernel::Auto,
                            )
                            .map_err(|e| e.to_string())?;
                            return Ok((
                                series_to_f64(cpu),
                                RocbPrepRoute::CudaFallbackToCpu,
                                Some(err.to_string()),
                            ));
                        }
                    }
                }
                None => {
                    let cpu = compute_dynamic_indicator_cpu(
                        candles,
                        ma_source,
                        indicator_id,
                        Some(output_id),
                        params,
                        Kernel::Auto,
                    )
                    .map_err(|e| e.to_string())?;
                    return Ok((
                        series_to_f64(cpu),
                        RocbPrepRoute::CudaFallbackToCpu,
                        Some("prepared CUDA inputs missing".to_string()),
                    ));
                }
            }
        }

        let cpu = compute_dynamic_indicator_cpu(
            candles,
            ma_source,
            indicator_id,
            Some(output_id),
            params,
            Kernel::Auto,
        )
        .map_err(|e| e.to_string())?;
        Ok((series_to_f64(cpu), RocbPrepRoute::Cpu, None))
    }
}
```

Then keep the current API by delegating to the traced helper:

```rust
fn compute_indicator_f64(
    candles: &Candles,
    ma_source: &str,
    backend: &Backend,
    indicator_id: &str,
    output_id: &str,
    params: &[IndicatorParamKV],
    #[cfg(feature = "cuda")] prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<Vec<f64>, String> {
    let (values, _route, _fallback) = compute_indicator_f64_traced(
        candles,
        ma_source,
        backend,
        indicator_id,
        output_id,
        params,
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    Ok(values)
}
```

### 2.3 Add traced breakout/risk helpers

These preserve the current public surface while giving callers a way to inspect the route.

```rust
pub fn compute_rocb_breakout_signals_traced(
    candles: &Candles,
    ma_source: &str,
    backend: &Backend,
    channel_period: u16,
    #[cfg(feature = "cuda")] prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<(RocbBreakoutSignals, RocbPrepDiagnostics), String> {
    let period_key = resolve_param_key("donchian", &["period", "length", "timeperiod"])?;
    let params = vec![IndicatorParamKV {
        key: period_key,
        value: IndicatorParamValue::Int(i64::from(channel_period)),
    }];
    let upper_output = resolve_output_id("donchian", &["upperband", "upper", "value"])?;
    let lower_output = resolve_output_id("donchian", &["lowerband", "lower"])?;

    let (upper, upper_route, upper_fallback) = compute_indicator_f64_traced(
        candles,
        ma_source,
        backend,
        "donchian",
        upper_output.as_str(),
        params.as_slice(),
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let (lower, lower_route, lower_fallback) = compute_indicator_f64_traced(
        candles,
        ma_source,
        backend,
        "donchian",
        lower_output.as_str(),
        params.as_slice(),
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;

    if upper.len() != lower.len() {
        return Err("ROCB donchian output length mismatch".to_string());
    }

    let first_valid = first_finite_idx_local(&upper)
        .zip(first_finite_idx_local(&lower))
        .map(|(a, b)| a.max(b))
        .ok_or_else(|| "ROCB donchian outputs are all non-finite".to_string())?;

    let mut diag = RocbPrepDiagnostics {
        breakout_upper_route: Some(upper_route),
        breakout_lower_route: Some(lower_route),
        atr_route: None,
        fallback_reasons: Vec::new(),
    };
    if let Some(reason) = upper_fallback {
        diag.fallback_reasons.push(format!("donchian-upper: {reason}"));
    }
    if let Some(reason) = lower_fallback {
        diag.fallback_reasons.push(format!("donchian-lower: {reason}"));
    }

    Ok((
        RocbBreakoutSignals {
            upper,
            lower,
            first_valid,
        },
        diag,
    ))
}

pub fn compute_rocb_risk_signals_traced(
    candles: &Candles,
    ma_source: &str,
    backend: &Backend,
    atr_period: u16,
    #[cfg(feature = "cuda")] prepared_cuda_inputs: Option<&PreparedCudaIndicatorInputs>,
) -> Result<(RocbRiskSignals, RocbPrepDiagnostics), String> {
    let period_key = resolve_param_key("atr", &["period", "length", "timeperiod"])?;
    let params = vec![IndicatorParamKV {
        key: period_key,
        value: IndicatorParamValue::Int(i64::from(atr_period)),
    }];
    let output_id = resolve_output_id("atr", &["value", "atr"])?;
    let (atr, route, fallback) = compute_indicator_f64_traced(
        candles,
        ma_source,
        backend,
        "atr",
        output_id.as_str(),
        params.as_slice(),
        #[cfg(feature = "cuda")]
        prepared_cuda_inputs,
    )?;
    let first_valid = first_finite_idx_local(&atr)
        .ok_or_else(|| "ROCB ATR output is all non-finite".to_string())?;

    let mut diag = RocbPrepDiagnostics {
        breakout_upper_route: None,
        breakout_lower_route: None,
        atr_route: Some(route),
        fallback_reasons: Vec::new(),
    };
    if let Some(reason) = fallback {
        diag.fallback_reasons.push(format!("atr: {reason}"));
    }

    Ok((RocbRiskSignals { atr, first_valid }, diag))
}
```

Then keep the current public functions as thin wrappers if you want backward compatibility.

### 2.4 Add a bundled GPU sweep prep helper

This is the ROCB-native prep helper the caller should prefer for optimization and validation paths.

```rust
#[cfg(feature = "cuda")]
pub struct RocbGpuPreparedSweeps {
    pub upper: RocbPeriodSweepDevice,
    pub lower: RocbPeriodSweepDevice,
    pub atr: RocbPeriodSweepDevice,
}

#[cfg(feature = "cuda")]
pub fn prepare_rocb_gpu_sweeps(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    channel_period_start: u16,
    channel_period_end: u16,
    channel_period_step: u16,
    atr_period_start: u16,
    atr_period_end: u16,
    atr_period_step: u16,
) -> Result<RocbGpuPreparedSweeps, String> {
    let upper = compute_rocb_donchian_output_sweep_cuda_device(
        prepared_inputs,
        &["upperband", "upper", "value"],
        channel_period_start,
        channel_period_end,
        channel_period_step,
    )?;
    let lower = compute_rocb_donchian_output_sweep_cuda_device(
        prepared_inputs,
        &["lowerband", "lower"],
        channel_period_start,
        channel_period_end,
        channel_period_step,
    )?;
    let atr = compute_rocb_atr_sweep_cuda_device(
        prepared_inputs,
        atr_period_start,
        atr_period_end,
        atr_period_step,
    )?;

    debug_assert_eq!(upper.matrix.cols, lower.matrix.cols);
    debug_assert_eq!(upper.matrix.cols, atr.matrix.cols);
    debug_assert_eq!(upper.matrix.device_id, lower.matrix.device_id);
    debug_assert_eq!(upper.matrix.device_id, atr.matrix.device_id);

    Ok(RocbGpuPreparedSweeps { upper, lower, atr })
}
```

Why this matters:

- it makes the intended ROCB GPU-native prep path explicit
- it keeps Donchian upper/lower/ATR sweeps together
- it reduces the chance that an optimization route accidentally uses the scalar host-readback path

### 2.5 Add an in-place CPU desired-position builder

This is a small but useful CPU-side allocation improvement.

```rust
pub fn build_rocb_desired_positions_into(
    out: &mut Vec<i32>,
    close: &[f64],
    breakout: &RocbBreakoutSignals,
    risk: &RocbRiskSignals,
    start_t: usize,
    strategy: &StrategyConfig,
    cfg: &RocbStrategyParams,
) {
    let n = close.len();
    out.clear();
    out.resize(n, 0);

    if start_t >= n {
        return;
    }

    let trade_offset = if strategy.trade_on_next_bar { 1usize } else { 0usize };
    let max_trade_bars = usize::from(cfg.max_trade_bars.max(1));
    let mut pos: i32 = 0;
    let mut bars_in_trade: usize = 0;
    let mut entry_price = f64::NAN;

    for t in start_t..n {
        let sig_t = t.saturating_sub(trade_offset);
        let prev_t = sig_t.saturating_sub(1);

        let close_now = close.get(sig_t).copied().unwrap_or(f64::NAN);
        let upper_prev = breakout.upper.get(prev_t).copied().unwrap_or(f64::NAN);
        let lower_prev = breakout.lower.get(prev_t).copied().unwrap_or(f64::NAN);
        let atr_now = risk.atr.get(sig_t).copied().unwrap_or(f64::NAN);

        let long_breakout =
            close_now.is_finite() && upper_prev.is_finite() && close_now > upper_prev;
        let short_breakout =
            close_now.is_finite() && lower_prev.is_finite() && close_now < lower_prev;

        if pos == 0 {
            bars_in_trade = 0;
            entry_price = f64::NAN;
            if long_breakout {
                pos = 1;
                bars_in_trade = 1;
                entry_price = close_now;
            } else if !strategy.long_only && short_breakout {
                pos = -1;
                bars_in_trade = 1;
                entry_price = close_now;
            }
        } else if pos > 0 {
            let stop_dist = if atr_now.is_finite() {
                cfg.atr_stop_mult * atr_now
            } else {
                f64::NAN
            };
            let stop_hit = entry_price.is_finite()
                && stop_dist.is_finite()
                && close_now.is_finite()
                && close_now <= entry_price - stop_dist;
            let channel_exit =
                close_now.is_finite() && upper_prev.is_finite() && close_now <= upper_prev;
            let mut exit_now = stop_hit || channel_exit || bars_in_trade >= max_trade_bars;

            if !strategy.long_only && short_breakout {
                if strategy.allow_flip {
                    pos = -1;
                    bars_in_trade = 1;
                    entry_price = close_now;
                    out[t] = pos;
                    continue;
                }
                exit_now = true;
            }

            if exit_now {
                pos = 0;
                bars_in_trade = 0;
                entry_price = f64::NAN;
            } else {
                bars_in_trade = bars_in_trade.saturating_add(1);
            }
        } else {
            let stop_dist = if atr_now.is_finite() {
                cfg.atr_stop_mult * atr_now
            } else {
                f64::NAN
            };
            let stop_hit = entry_price.is_finite()
                && stop_dist.is_finite()
                && close_now.is_finite()
                && close_now >= entry_price + stop_dist;
            let channel_exit =
                close_now.is_finite() && lower_prev.is_finite() && close_now >= lower_prev;
            let mut exit_now = stop_hit || channel_exit || bars_in_trade >= max_trade_bars;

            if long_breakout {
                if strategy.allow_flip {
                    pos = 1;
                    bars_in_trade = 1;
                    entry_price = close_now;
                    out[t] = pos;
                    continue;
                }
                exit_now = true;
            }

            if exit_now {
                pos = 0;
                bars_in_trade = 0;
                entry_price = f64::NAN;
            } else {
                bars_in_trade = bars_in_trade.saturating_add(1);
            }
        }

        out[t] = pos;
    }
}

pub fn build_rocb_desired_positions(
    close: &[f64],
    breakout: &RocbBreakoutSignals,
    risk: &RocbRiskSignals,
    start_t: usize,
    strategy: &StrategyConfig,
    cfg: &RocbStrategyParams,
) -> Vec<i32> {
    let mut out = Vec::with_capacity(close.len());
    build_rocb_desired_positions_into(&mut out, close, breakout, risk, start_t, strategy, cfg);
    out
}
```

Why this matters:

- it avoids repeated CPU vector allocation when the caller reuses a scratch buffer
- it gives parity/reference harnesses a cheaper allocation model
- it is a zero-risk semantic refactor because the original API stays intact

## Additional `rocb.rs` notes

### Keep scalar helper path, but do not let it be the optimization default

The traced helper is mainly a visibility fix. The real optimization route should prefer the new sweep-bundle helper.

### Do not chase `Vec<f32>` rewrites in this file first

`rocb.rs` is mostly prep/orchestration and CPU-reference logic. The largest wins are in staging, not in converting all CPU reference code to `f32`.

---

# 3) `gpu_native_expansion_opt.rs`

## Current gap

This file currently benchmarks KQSR, CARM, KVDF, and CDMC, but not ROCB. That means the section-5 “CPU/GPU payoff review” is incomplete for ROCB on the optimizer boundary.

## Goal

Add:

- ROCB CPU bench
- ROCB GPU-kernel bench
- a ROCB-friendly synthetic candle generator
- ROCB request builder
- ROCB entries in `criterion_group!`

## Concrete edits

### 3.1 Update imports

Change the import block to include ROCB params:

```rust
use ta_app_core::{
    Backend, BackendUsed, CarmStrategyParams, CdmcStrategyParams, CompositeCpuRequest,
    CompositeSignalMode, KqsrStrategyParams, KvdfStrategyParams, RocbStrategyParams,
};
```

### 3.2 Add a ROCB-friendly synthetic candle series

ROCB needs real breakouts and ATR variation. A flat or overly smooth series will understate real work.

```rust
fn make_rocb_breakout_candles(len: usize) -> Candles {
    let len = len.max(320);
    let mut ts = Vec::with_capacity(len);
    let mut open = Vec::with_capacity(len);
    let mut high = Vec::with_capacity(len);
    let mut low = Vec::with_capacity(len);
    let mut close = Vec::with_capacity(len);
    let mut vol: Vec<f64> = Vec::with_capacity(len);

    let mut px = 100.0f64;
    for i in 0..len {
        ts.push(i as i64 * 60_000);

        let drift = if i < len / 4 {
            0.025
        } else if i < len / 2 {
            0.16
        } else if i < (3 * len) / 4 {
            -0.11
        } else {
            0.12
        };
        let wave = ((i as f64) * 0.17).sin() * 0.9 + ((i as f64) * 0.041).cos() * 0.35;
        px += drift + wave * 0.05;

        let range = 0.75 + ((i as f64) * 0.13).sin().abs() * 0.65;
        let o = px - wave * 0.08;
        let c = px;
        open.push(o);
        high.push(o.max(c) + range);
        low.push(o.min(c) - range);
        close.push(c);
        vol.push(1_200.0 + (i % 23) as f64 * 40.0);
    }

    Candles::new(ts, open, high, low, close, vol)
}
```

### 3.3 Add ROCB request builder

```rust
fn make_rocb_req(
    backend: Backend,
    fast_range: (u32, u32, u32),
    slow_range: (u32, u32, u32),
) -> CompositeCpuRequest {
    let mut req = base_req(backend);
    req.rocb = Some(RocbStrategyParams::default());
    req.fast_range = fast_range;
    req.slow_range = slow_range;
    req
}
```

### 3.4 Add ROCB CPU and GPU bench functions

```rust
fn bench_rocb_cpu(c: &mut Criterion) {
    if !bench_target_enabled("rocb") {
        return;
    }
    let candles = make_rocb_breakout_candles(200_000);
    bench_cpu_cases(c, "rocb/cpu/200k", &candles, make_rocb_req);
}

fn bench_rocb_gpu_kernel(c: &mut Criterion) {
    if !bench_target_enabled("rocb") {
        return;
    }
    let candles = make_rocb_breakout_candles(200_000);
    bench_gpu_cases(
        c,
        "rocb/gpu_kernel/200k",
        "rocb/gpu_kernel/200k",
        &candles,
        make_rocb_req,
    );
}
```

### 3.5 Add ROCB to the criterion group

```rust
criterion_group!(
    gpu_native_expansion_benches,
    bench_kqsr_cpu,
    bench_kqsr_gpu_kernel,
    bench_carm_cpu,
    bench_carm_gpu_kernel,
    bench_kvdf_cpu,
    bench_kvdf_gpu_kernel,
    bench_cdmc_cpu,
    bench_cdmc_gpu_kernel,
    bench_rocb_cpu,
    bench_rocb_gpu_kernel
);
```

## Why these bench edits matter

They give you:

- ROCB optimizer throughput on CPU
- ROCB optimizer throughput on GPU
- the route/fallback probe log on ROCB specifically
- a ROCB regression target after the residency refactor

## What to watch when reading the results

If ROCB is still not clearly ahead on GPU after the residency fixes, the likely causes are:

- prep path still falling back to CPU
- outer route still re-uploading too much work outside `rocb_gpu_kernel.rs`
- candidate counts too small to amortize the GPU setup
- the current workload genuinely favors CPU for ROCB at small sizes

That is exactly why the ROCB-specific benchmark needs to exist before claiming victory.

---

# 4) `composite_validation_workflow.rs`

## Current gap

ROCB appears only in the generic profile benches at `50k/8` and is missing from the heavier dedicated GPU validation workflow case.

That means you do not yet have a ROCB-focused view of validation scaling.

## Goal

Add:

- a larger ROCB CPU validation profile
- a larger ROCB GPU validation profile
- a dedicated heavier ROCB GPU workflow case similar to the ASR-heavy pattern

## Concrete edits

### 4.1 Extend the existing CPU profile coverage

Near the existing calls at the bottom of `bench_composite_validation_workflow`, add ROCB to the larger set too:

```rust
bench_cpu_profile("rocb", 100_000, make_rocb_candidates(32));
```

So the tail becomes:

```rust
bench_cpu_profile("base_composite", 50_000, make_candidates(8));
bench_cpu_profile("asr", 50_000, make_asr_candidates(8));
bench_cpu_profile("vcc", 50_000, make_vcc_candidates(8));
bench_cpu_profile("vemr", 50_000, make_vemr_candidates(8));
bench_cpu_profile("rocb", 50_000, make_rocb_candidates(8));
bench_cpu_profile("mkcs", 50_000, make_mkcs_candidates(8));
bench_cpu_profile("kqsr", 50_000, make_kqsr_candidates(8));
bench_cpu_profile("carm", 50_000, make_carm_candidates(8));
bench_cpu_profile("kvdf", 50_000, make_kvdf_candidates(8));
bench_cpu_profile("cdmc", 50_000, make_cdmc_candidates(8));
bench_cpu_profile("rocb", 100_000, make_rocb_candidates(32));
bench_cpu_profile("carm", 100_000, make_carm_candidates(32));
bench_cpu_profile("cdmc", 100_000, make_cdmc_candidates(32));
```

### 4.2 Add heavier ROCB GPU profile coverage

At the bottom of `bench_composite_validation_gpu_workflow`, add:

```rust
bench_profile("rocb", 100_000, make_rocb_candidates(32));
```

So the profile tail becomes:

```rust
bench_profile("base_composite", 50_000, make_candidates(8));
bench_profile("asr", 50_000, make_asr_candidates(8));
bench_profile("vcc", 50_000, make_vcc_candidates(8));
bench_profile("vemr", 50_000, make_vemr_candidates(8));
bench_profile("rocb", 50_000, make_rocb_candidates(8));
bench_profile("mkcs", 50_000, make_mkcs_candidates(8));
bench_profile("kqsr", 50_000, make_kqsr_candidates(8));
bench_profile("carm", 50_000, make_carm_candidates(8));
bench_profile("kvdf", 50_000, make_kvdf_candidates(8));
bench_profile("cdmc", 50_000, make_cdmc_candidates(8));
bench_profile("rocb", 100_000, make_rocb_candidates(32));
bench_profile("carm", 100_000, make_carm_candidates(32));
bench_profile("cdmc", 100_000, make_cdmc_candidates(32));
```

This alone already improves ROCB coverage.

### 4.3 Add a dedicated heavy ROCB GPU workflow block

Mirror the ASR-focused heavy case with a ROCB-specific one.

```rust
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn bench_rocb_validation_gpu_workflow(c: &mut Criterion) {
    let candles = make_synth_candles(100_000);
    let cancel = AtomicBool::new(false);
    let candidates = make_rocb_candidates(64);

    let mut group = c.benchmark_group("composite_validate/gpu_kernel/rocb/100k/64candidates");
    group.measurement_time(Duration::from_secs(8));
    group.sample_size(10);

    let base = CompositeValidationRequest {
        data_id: "bench-gpu-rocb".to_string(),
        backend: ta_app_core::Backend::GpuKernel { device_id: 0 },
        ma_source: "close".to_string(),
        fast_ma_params: None,
        slow_ma_params: None,
        candidates: candidates.clone(),
        walk_forward: Some(WalkForwardConfigOptional {
            window_bars: 20_000,
            step_bars: 20_000,
        }),
        reopt_walk_forward: None,
        spa: None,
        perturb: None,
        trade_bootstrap: None,
        cross_ticker: None,
        cscv_pbo: Some(CscvPboConfig {
            slices: 8,
            max_combinations: 2_000,
            threshold: 0.0,
        }),
        seed: 123,
        max_candidates: candidates.len(),
        validation_engine: ta_app_core::ValidationEngine::Vectorized,
        iterative: ta_app_core::IterativeSimConfig::default(),
        min_psr: None,
        min_dsr: None,
        min_cscv_oos_rank_frac: None,
        min_cscv_oos_metric: None,
        max_pbo: None,
        validation_context: None,
    };

    let route_probe = ta_app_core::run_composite_validation_blocking_with_candles(
        base.clone(),
        &candles,
        &cancel,
        None,
    )
    .expect("rocb gpu validation route probe");
    println!(
        "benchmark route composite_validate/gpu_kernel/rocb/100k/64candidates: backend_used={:?} backend_route={} fallback={}",
        route_probe.backend_used,
        route_probe.backend_route.as_deref().unwrap_or("<none>"),
        route_probe.backend_fallback_reason.as_deref().unwrap_or("<none>")
    );

    group.bench_function("wf_cscv", |b| {
        b.iter(|| {
            let _ = ta_app_core::run_composite_validation_blocking_with_candles(
                base.clone(),
                &candles,
                &cancel,
                None,
            )
            .expect("rocb gpu wf+cscv validate");
        })
    });

    let mut vram_heavy = base.clone();
    vram_heavy.candidates = make_rocb_candidates(192);
    vram_heavy.max_candidates = vram_heavy.candidates.len();
    vram_heavy.walk_forward = None;
    vram_heavy.cscv_pbo = None;

    group.bench_function("in_sample_192candidates", |b| {
        b.iter(|| {
            let _ = ta_app_core::run_composite_validation_blocking_with_candles(
                vram_heavy.clone(),
                &candles,
                &cancel,
                None,
            )
            .expect("rocb gpu in-sample validate");
        })
    });

    group.finish();
}
```

Then add it to the criterion group:

```rust
criterion_group!(
    benches,
    bench_composite_validation_workflow,
    bench_composite_validation_gpu_workflow,
    bench_rocb_validation_gpu_workflow
);
```

If you prefer to avoid another top-level function, you can also inline the ROCB-heavy block into `bench_composite_validation_gpu_workflow`. I prefer a separate function because it gives ROCB a clearly named benchmark group.

## Why these bench edits matter

They let you answer the section-5 question that broad composite reviews often hide:

- ROCB optimizer throughput might improve while ROCB validation throughput still bottlenecks
- ROCB may have a different candidate-scaling cliff than ASR
- ROCB may show stronger GPU payoff only at 32+ or 64+ candidates

---

# 5) Wiring expectations outside the attached files

The attached files are enough to define the strategy-local refactor, but the full payoff comes only when the outer route adopts the new uploaded API.

## Target call pattern for the real hot path

The ideal call pattern for ROCB GPU evaluation becomes:

1. prepare CUDA indicator inputs once for the candle set
2. call `prepare_rocb_gpu_sweeps(...)` once per candle set / period-range request
3. call `upload_rocb_market_to_device(...)` once per candle set
4. build combo row-index arrays once per candidate batch
5. call `upload_rocb_combo_batch_to_device(...)` once per candidate batch
6. call `eval_rocb_metrics_gpu_kernel_uploaded(...)` for repeated scoring
7. reuse runtime scratch across launches automatically

Pseudo-flow:

```rust
#[cfg(feature = "cuda")]
fn score_rocb_batch_gpu(
    prepared: &PreparedCudaIndicatorInputs,
    close_f32: &[f32],
    channel_range: (u16, u16, u16),
    atr_range: (u16, u16, u16),
    combo_batch: RocbGpuComboBatch<'_>,
    strategy: &StrategyConfig,
    rocb: &RocbStrategyParams,
) -> Result<Vec<Metrics>, String> {
    let sweeps = prepare_rocb_gpu_sweeps(
        prepared,
        channel_range.0,
        channel_range.1,
        channel_range.2,
        atr_range.0,
        atr_range.1,
        atr_range.2,
    )?;

    let market = upload_rocb_market_to_device(sweeps.upper.matrix.device_id, close_f32)?;
    let uploaded_combos =
        upload_rocb_combo_batch_to_device(sweeps.upper.matrix.device_id, &combo_batch)?;

    let signals = RocbGpuDeviceSignalMatrices {
        close_f32,
        cols: sweeps.upper.matrix.cols,
        upper_row_major_ptr: sweeps.upper.matrix.device_ptr,
        lower_row_major_ptr: sweeps.lower.matrix.device_ptr,
        channel_rows: sweeps.upper.matrix.rows,
        channel_stride: sweeps.upper.matrix.cols,
        atr_row_major_ptr: sweeps.atr.matrix.device_ptr,
        atr_rows: sweeps.atr.matrix.rows,
        atr_stride: sweeps.atr.matrix.cols,
    };

    eval_rocb_metrics_gpu_kernel_uploaded(
        sweeps.upper.matrix.device_id,
        &market,
        &signals,
        &uploaded_combos,
        strategy,
        rocb,
    )
}
```

This is the route that actually realizes the residency improvements.

---

# 6) Test plan and parity plan

You asked for a comprehensive plan, so the performance plan needs an equally explicit safety net.

## A. ROCB GPU-vs-CPU metric parity

Add at least these tests:

1. **single-combo parity**
   - one Donchian period
   - one ATR period
   - no flips disabled surprises
   - compare `Metrics` fields within tolerance

2. **multi-combo parity**
   - several channel periods
   - several ATR periods
   - varying `start_t` and `end_t`
   - compare full output vectors

3. **trade-on-next-bar parity**
   - same combos, same candles
   - compare with `strategy.trade_on_next_bar = true`

4. **long-only parity**
   - same combos, same candles
   - compare with `strategy.long_only = true`

## B. Fallback-route tests in `rocb.rs`

Add tests that verify the diagnostics, even if exact route details vary by feature flag:

- missing prepared CUDA inputs records a fallback reason
- CPU-only backend reports `RocbPrepRoute::Cpu`
- successful CUDA host-readback path reports `RocbPrepRoute::CudaHostReadback`

## C. Allocation-preserving tests

For `build_rocb_desired_positions_into`:

- output matches `build_rocb_desired_positions`
- calling it twice on the same buffer yields correct results and no stale values

Suggested test:

```rust
#[test]
fn rocb_desired_positions_into_matches_allocating_builder() {
    let close = vec![100.0, 101.0, 103.0, 104.0, 101.0, 99.0];
    let breakout = RocbBreakoutSignals {
        upper: vec![f64::NAN, 100.5, 102.0, 103.0, 102.5, 101.0],
        lower: vec![f64::NAN, 99.5, 99.0, 100.0, 100.5, 98.0],
        first_valid: 1,
    };
    let risk = RocbRiskSignals {
        atr: vec![f64::NAN, 1.0, 1.0, 1.0, 1.0, 1.0],
        first_valid: 1,
    };
    let strategy = StrategyConfig::default();
    let cfg = RocbStrategyParams::default();

    let expected =
        build_rocb_desired_positions(close.as_slice(), &breakout, &risk, 1, &strategy, &cfg);

    let mut out = Vec::new();
    build_rocb_desired_positions_into(
        &mut out,
        close.as_slice(),
        &breakout,
        &risk,
        1,
        &strategy,
        &cfg,
    );

    assert_eq!(out, expected);
}
```

## D. GPU blocker tests

The current ROCB GPU kernel explicitly rejects overlays / stop-loss / take-profit. Keep that behavior locked down with tests once you know the exact `StrategyConfig` constructors/setters available in your crate.

The test should assert that those unsupported routes fail *before* any expensive runtime work is attempted.

---

# 7) Rollout order

This is the order I would implement.

## Phase 1: Benchmark coverage and observability

1. add ROCB optimizer benches in `gpu_native_expansion_opt.rs`
2. add ROCB heavier validation benches in `composite_validation_workflow.rs`
3. add ROCB prep diagnostics in `rocb.rs`

Why first:
- you need the before/after record
- you need route visibility before you trust any timing numbers

## Phase 2: Safe hot-path staging fixes

1. add `with_runtime`
2. add `RocbGpuScratch`
3. add `RocbGpuUploadedMarket`
4. add `RocbGpuUploadedCombos`
5. add `eval_rocb_metrics_gpu_kernel_uploaded`
6. keep the old wrapper API

Why second:
- these are the highest-confidence wins
- they do not change kernel math
- they are easiest to validate with parity tests

## Phase 3: CPU-reference allocation cleanup

1. add `build_rocb_desired_positions_into`
2. keep old API as wrapper
3. add output-reuse tests

Why third:
- smaller payoff than GPU staging
- trivial risk once tests are in place

## Phase 4: Optional follow-up work

1. cache small launch metadata in runtime
2. consider host-side uninitialized readback only if you already have a safe raw-copy helper
3. inspect whether any outer ROCB route still reconstructs upload objects too often

---

# 8) Expected performance impact by change

This ranking reflects likely ROI, not certainty.

## Highest expected payoff

### `upload_rocb_market_to_device` + reuse of `RocbGpuUploadedMarket`

This removes repeated close upload and repeated log-return computation. For repeated scoring over the same candle set, that should be the biggest ROCB win.

### `upload_rocb_combo_batch_to_device` + reuse of `RocbGpuUploadedCombos`

This matters most in validation and finalist rescoring where the same candidate batch is reused.

### `RocbGpuScratch`

This reduces repeated device alloc and host alloc churn.

## Moderate expected payoff

### `prepare_rocb_gpu_sweeps`

This is more about preventing route regressions than speeding up a single kernel launch. It ensures the optimizer/validator can stay truly GPU-native.

### `build_rocb_desired_positions_into`

Useful for CPU fallback and parity harnesses, but lower ROI than the GPU staging work.

## Lower expected payoff

### caching launch metadata / function handles

Worth doing after the big staging wins, but not first.

---

# 9) Acceptance criteria

Do not consider the work complete until all of these are true.

## Functional

- ROCB GPU metrics match CPU reference within the agreed tolerance on representative test cases.
- ROCB prep diagnostics correctly report CPU, CUDA host-readback, or fallback routes.
- Legacy wrapper API still behaves as before.

## Performance

- repeated ROCB scoring on the same candle set no longer re-uploads `close_f32`
- repeated ROCB scoring on the same candle set no longer recomputes log returns every call
- repeated ROCB scoring on the same candidate batch no longer re-uploads combo vectors every call
- repeated ROCB scoring no longer allocates `d_out` every call
- repeated ROCB scoring no longer allocates a new host readback vector every call

## Benchmarking

- `gpu_native_expansion_opt.rs` includes ROCB CPU and GPU benches
- `composite_validation_workflow.rs` includes ROCB at `100k/32`
- ROCB has a dedicated heavier GPU validation workflow case

## Safety

- no unsound host uninitialized readback was introduced
- all new reuse objects validate device id / length consistency

---

# 10) Implementation checklist

## `rocb_gpu_kernel.rs`

- [ ] add `with_runtime(...)`
- [ ] add `RocbGpuScratch`
- [ ] add `scratch` field to `RocbCudaRuntime`
- [ ] change `RocbCudaRuntime::new(...)` to initialize scratch
- [ ] add `RocbGpuUploadedMarket`
- [ ] add `RocbGpuUploadedCombos`
- [ ] add `upload_rocb_market_to_device(...)`
- [ ] add `upload_rocb_combo_batch_to_device(...)`
- [ ] add `eval_rocb_metrics_gpu_kernel_uploaded(...)`
- [ ] keep `eval_rocb_metrics_gpu_kernel_device_rows(...)` as a wrapper during migration
- [ ] reuse `d_out` through scratch instead of allocating it every call
- [ ] reuse host metrics buffer through scratch instead of `vec![0.0; out_len]` every call
- [ ] validate device-id mismatch errors for uploaded market/combos
- [ ] validate signal-length mismatch errors for uploaded market
- [ ] keep existing overlay / stop-loss / take-profit blocker behavior

## `rocb.rs`

- [ ] add `RocbPrepRoute`
- [ ] add `RocbPrepDiagnostics`
- [ ] add `compute_indicator_f64_traced(...)`
- [ ] keep `compute_indicator_f64(...)` as a wrapper over the traced helper
- [ ] add `compute_rocb_breakout_signals_traced(...)`
- [ ] add `compute_rocb_risk_signals_traced(...)`
- [ ] add `RocbGpuPreparedSweeps`
- [ ] add `prepare_rocb_gpu_sweeps(...)`
- [ ] add `build_rocb_desired_positions_into(...)`
- [ ] keep `build_rocb_desired_positions(...)` as a wrapper over the in-place builder
- [ ] ensure fallback reasons are recorded when prepared CUDA inputs are missing
- [ ] ensure fallback reasons are recorded when CUDA indicator execution fails

## `gpu_native_expansion_opt.rs`

- [ ] import `RocbStrategyParams`
- [ ] add `make_rocb_breakout_candles(...)`
- [ ] add `make_rocb_req(...)`
- [ ] add `bench_rocb_cpu(...)`
- [ ] add `bench_rocb_gpu_kernel(...)`
- [ ] add ROCB entries to `criterion_group!(...)`
- [ ] verify the ROCB route probe prints backend route and fallback reason
- [ ] verify the ROCB synthetic candles actually generate non-degenerate breakout activity

## `composite_validation_workflow.rs`

- [ ] add `bench_cpu_profile("rocb", 100_000, make_rocb_candidates(32));`
- [ ] add `bench_profile("rocb", 100_000, make_rocb_candidates(32));`
- [ ] add dedicated `bench_rocb_validation_gpu_workflow(...)`
- [ ] add ROCB-heavy validation bench to `criterion_group!(...)`
- [ ] verify ROCB route probe prints backend route and fallback reason
- [ ] verify ROCB-heavy case uses enough candidates to expose staging issues

## Tests

- [ ] add `build_rocb_desired_positions_into` equivalence test
- [ ] add repeated-call buffer reuse test for `build_rocb_desired_positions_into`
- [ ] add ROCB single-combo GPU-vs-CPU parity test
- [ ] add ROCB multi-combo GPU-vs-CPU parity test
- [ ] add ROCB trade-on-next-bar parity test
- [ ] add ROCB long-only parity test
- [ ] add prep fallback-diagnostics tests
- [ ] add GPU blocker tests for unsupported overlays / stop-loss / take-profit

## Benchmark validation

- [ ] record baseline ROCB optimizer CPU timings
- [ ] record baseline ROCB optimizer GPU timings
- [ ] record baseline ROCB validation CPU timings
- [ ] record baseline ROCB validation GPU timings
- [ ] re-run all ROCB benches after Phase 2
- [ ] compare route probe output before and after
- [ ] verify GPU path still reports `BackendUsed::Gpu`
- [ ] verify no new fallback reason appeared on the intended ROCB GPU route

## Safety / review sign-off

- [ ] confirm no unsound host `set_len` + `copy_to` pattern was introduced
- [ ] confirm all device buffers are fully written before any read
- [ ] confirm output buffers only grow and are reused safely
- [ ] confirm uploaded market/combos are not reused across mismatched device ids
- [ ] confirm uploaded market/combos are not reused across mismatched series lengths
- [ ] confirm parity results are stable enough for fp32 ROCB GPU scoring
- [ ] confirm the optimization path prefers the sweep-device route over scalar host-readback prep

---

## Recommended first implementation slice

If you want the smallest high-payoff slice first, do this exact sequence:

1. add ROCB benches in both benchmark files  
2. add `RocbGpuScratch`  
3. add `RocbGpuUploadedMarket`  
4. add `RocbGpuUploadedCombos`  
5. add `eval_rocb_metrics_gpu_kernel_uploaded(...)`  
6. switch the real ROCB GPU caller to use the uploaded path  
7. add ROCB prep diagnostics in `rocb.rs`  

That sequence gives you observable ROI fast, without changing ROCB trading semantics.
