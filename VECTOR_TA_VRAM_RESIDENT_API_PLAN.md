# VectorTA VRAM-Resident CUDA API Plan

## Recommendation

Build a new additive device-native API layer inside `vector-ta`.

Do not start by refactoring or replacing the current host-slice CUDA API.

That means:

- keep the current host-driven API working,
- add a new API path whose contract is "device buffers in, device buffers out",
- migrate performance-sensitive callers like VEMR to the new path first,
- optionally reimplement the old host API on top of the new layer later.

This is the safest path because the current API is already consumed in multiple places, and a superficial wrapper around the current host API would not solve the repeated host-to-device upload problem.

## Core Design Goal

The device-native API must let callers:

1. Upload market data once.
2. Keep OHLC or OHLCV plus optional source series resident in VRAM.
3. Run multiple indicator sweeps against those same device buffers without re-uploading the price data.
4. Keep indicator outputs on device and feed them directly into downstream GPU kernels.
5. Download only the final requested results.

For VEMR specifically, that means:

- upload `open/high/low/close` once,
- optionally upload `source` once if `ma_source != close`,
- compute `zscore`, Bollinger, and ATR sweeps from those device buffers,
- pass the output matrices directly into the backtest kernel,
- copy back only top-k or final metrics at the end.

## Important Constraint

If you want exact VEMR parity, uploading only a single `close_f32` series is not enough.

VEMR's ATR leg requires `high`, `low`, and `close`.

So the real minimum upload shape is:

- `OHLC`, or
- `OHLC + source`, or
- `OHLCV + source` for a generic reusable market-data container.

## Current Blockers In VectorTA

These are the current library boundaries that prevent end-to-end VRAM residency:

- `IndicatorCudaDataRef` still takes host slices only in `src/indicators/dispatch/types.rs`.
- `compute_cuda(...)` and `cuda_data_from_req(...)` in `src/indicators/dispatch/cuda.rs` only map host-backed inputs into the CUDA layer.
- `CudaMaData::to_prices_f32()` in `src/cuda/moving_averages/ma_selector.rs` materializes fresh `Vec<f32>` host buffers.
- Multiple wrappers still do fresh `DeviceBuffer::from_slice(...)` uploads from host buffers on each call:
  - `src/cuda/zscore_wrapper.rs`
  - `src/cuda/bollinger_bands_wrapper.rs`
  - `src/cuda/atr_wrapper.rs`

There is already useful low-level machinery in the library:

- device-returning output types exist,
- DLPack export exists,
- ATR already has a low-level `atr_batch_device_with_tr(...)` entrypoint,
- many wrappers already return device-owned output buffers.

So this is mostly an API and wrapper redesign, not a full kernel rewrite.

## Recommended Layering

Add a new device-native layer with this shape:

1. Runtime/session layer
2. Device input/view layer
3. Device-native indicator execution layer
4. Optional compatibility bridge back to host APIs

Do not implement a new app-side wrapper around the existing host APIs.

That would preserve the same repeated upload bottleneck.

## Required Behavioral Contract

Every new `*_device*` API must obey these rules:

1. No implicit host-to-device copy.
   If a function name contains `device`, it must not accept host slices and then secretly upload them.

2. No implicit device-to-host copy.
   Device-native calls must return device-owned or device-borrowed outputs unless the caller explicitly asks to download.

3. No implicit synchronize.
   Device-native calls should launch work on the provided runtime stream and return.
   Synchronization should be explicit.

4. Same-device validation.
   All device inputs passed into one call must be on the same CUDA device.

5. Explicit ownership model.
   Borrowed device refs must not own memory.
   Owned device arrays or matrices must keep memory alive.

6. Reusable plans and scratch.
   Parameter buffers and scratch allocations should be reusable across repeated calls.

7. Upload/download are explicit API boundaries.
   The only functions allowed to cross PCIe should be explicit `upload_*` and `download_*` helpers.

## Proposed Public Types

These are the types I would add.

```rust
pub struct CudaRuntime {
    pub device_id: u32,
    // owns context, stream, allocator, and optional scratch pool
}

pub struct DeviceSliceF32Ref {
    pub device_ptr: u64,
    pub len: usize,
    pub device_id: u32,
}

pub struct DeviceSliceI32Ref {
    pub device_ptr: u64,
    pub len: usize,
    pub device_id: u32,
}

pub struct DeviceSeriesF32 {
    pub device_ptr: u64,
    pub len: usize,
    pub device_id: u32,
    // owns underlying device allocation
}

pub struct DeviceMatrixF32 {
    pub device_ptr: u64,
    pub rows: usize,
    pub cols: usize,
    pub device_id: u32,
    // owns underlying device allocation
}

pub struct DeviceMarketDataF32 {
    pub open: DeviceSeriesF32,
    pub high: DeviceSeriesF32,
    pub low: DeviceSeriesF32,
    pub close: DeviceSeriesF32,
    pub volume: Option<DeviceSeriesF32>,
    pub source: Option<DeviceSeriesF32>,
}

pub enum IndicatorCudaDeviceDataRef<'a> {
    SliceDevice {
        values: DeviceSliceF32Ref,
    },
    OhlcDevice {
        open: DeviceSliceF32Ref,
        high: DeviceSliceF32Ref,
        low: DeviceSliceF32Ref,
        close: DeviceSliceF32Ref,
        source: Option<DeviceSliceF32Ref>,
    },
    OhlcvDevice {
        open: DeviceSliceF32Ref,
        high: DeviceSliceF32Ref,
        low: DeviceSliceF32Ref,
        close: DeviceSliceF32Ref,
        volume: DeviceSliceF32Ref,
        source: Option<DeviceSliceF32Ref>,
    },
    HighLowDevice {
        high: DeviceSliceF32Ref,
        low: DeviceSliceF32Ref,
    },
    CloseVolumeDevice {
        close: DeviceSliceF32Ref,
        volume: DeviceSliceF32Ref,
    },
}

pub struct IndicatorCudaDeviceRequest<'a> {
    pub indicator_id: &'a str,
    pub output_id: Option<&'a str>,
    pub data: IndicatorCudaDeviceDataRef<'a>,
    pub params: &'a [ParamKV<'a>],
    pub kernel: Kernel,
}
```

## Runtime API

The runtime should own stream and reusable CUDA state.

```rust
impl CudaRuntime {
    pub fn new(device_id: usize) -> Result<Self, CudaError>;
    pub fn synchronize(&self) -> Result<(), CudaError>;

    pub fn upload_f32(&self, host: &[f32]) -> Result<DeviceSeriesF32, CudaError>;
    pub fn upload_i32(&self, host: &[i32]) -> Result<DeviceBufferI32, CudaError>;

    pub fn upload_ohlc(
        &self,
        open: &[f32],
        high: &[f32],
        low: &[f32],
        close: &[f32],
        source: Option<&[f32]>,
    ) -> Result<DeviceMarketDataF32, CudaError>;

    pub fn upload_ohlcv(
        &self,
        open: &[f32],
        high: &[f32],
        low: &[f32],
        close: &[f32],
        volume: &[f32],
        source: Option<&[f32]>,
    ) -> Result<DeviceMarketDataF32, CudaError>;
}
```

Behavior:

- upload once per dataset,
- reuse the returned device handles across many indicator calls,
- keep stream ownership centralized.

## Concrete Rust Shape Using Alma As The Template

The abstract types above describe the contract.

This section shows how I would make that contract look in Rust if you use the existing ALMA CUDA wrapper style as the baseline.

The important design change is this:

- today, `CudaAlma` owns its own `Context` and `Stream`,
- in the new device-native layer, wrappers like `CudaAlma`, `CudaZscore`, `CudaBollinger`, and `CudaAtr` should share one `CudaRuntime`.

That lets one uploaded dataset stay resident and flow through many wrappers without crossing PCIe again.

### 1. Shared runtime

This should live in `src/cuda/runtime.rs`.

```rust
use cust::context::Context;
use cust::device::Device;
use cust::memory::{AsyncCopyDestination, DeviceBuffer, LockedBuffer};
use cust::prelude::*;
use cust::stream::{Stream, StreamFlags};
use std::sync::Arc;

pub struct CudaRuntime {
    context: Arc<Context>,
    stream: Stream,
    device_id: u32,
}

impl CudaRuntime {
    pub fn new(device_id: usize) -> Result<Self, CudaError> {
        cust::init(CudaFlags::empty())?;
        let device = Device::get_device(device_id as u32)?;
        let context = Arc::new(Context::new(device)?);
        let stream = Stream::new(StreamFlags::NON_BLOCKING, None)?;

        Ok(Self {
            context,
            stream,
            device_id: device_id as u32,
        })
    }

    #[inline]
    pub fn stream(&self) -> &Stream {
        &self.stream
    }

    #[inline]
    pub fn context_arc(&self) -> Arc<Context> {
        self.context.clone()
    }

    #[inline]
    pub fn device_id(&self) -> u32 {
        self.device_id
    }

    #[inline]
    pub fn synchronize(&self) -> Result<(), CudaError> {
        self.stream.synchronize()
    }

    pub fn upload_f32(&self, host: &[f32]) -> Result<DeviceSeriesF32, CudaError> {
        let buf = unsafe { DeviceBuffer::from_slice_async(host, &self.stream)? };
        Ok(DeviceSeriesF32 {
            buf,
            len: host.len(),
            device_id: self.device_id,
        })
    }

    pub fn upload_ohlc(
        &self,
        open: &[f32],
        high: &[f32],
        low: &[f32],
        close: &[f32],
        source: Option<&[f32]>,
    ) -> Result<DeviceMarketDataF32, CudaError> {
        Ok(DeviceMarketDataF32 {
            open: self.upload_f32(open)?,
            high: self.upload_f32(high)?,
            low: self.upload_f32(low)?,
            close: self.upload_f32(close)?,
            volume: None,
            source: match source {
                Some(src) => Some(self.upload_f32(src)?),
                None => None,
            },
        })
    }

    pub fn download_series_f32(&self, src: &DeviceSeriesF32) -> Result<Vec<f32>, CudaError> {
        let mut pinned: LockedBuffer<f32> = unsafe { LockedBuffer::uninitialized(src.len)? };
        unsafe {
            src.buf.async_copy_to(pinned.as_mut_slice(), &self.stream)?;
        }
        self.stream.synchronize()?;
        Ok(pinned.as_slice().to_vec())
    }
}
```

### 2. Owned device buffers plus borrowed device views

This should live in `src/cuda/device_types.rs`.

This is intentionally close to the existing `DeviceArrayF32` style in the ALMA wrapper.

```rust
use cust::memory::{DeviceBuffer, DevicePointer};
use std::marker::PhantomData;

pub struct DeviceSeriesF32 {
    pub buf: DeviceBuffer<f32>,
    pub len: usize,
    pub device_id: u32,
}

pub struct DeviceMatrixF32 {
    pub buf: DeviceBuffer<f32>,
    pub rows: usize,
    pub cols: usize,
    pub device_id: u32,
}

#[derive(Clone, Copy)]
pub struct DeviceSliceF32Ref<'a> {
    pub ptr: DevicePointer<f32>,
    pub len: usize,
    pub device_id: u32,
    pub _marker: PhantomData<&'a f32>,
}

pub struct DeviceMarketDataF32 {
    pub open: DeviceSeriesF32,
    pub high: DeviceSeriesF32,
    pub low: DeviceSeriesF32,
    pub close: DeviceSeriesF32,
    pub volume: Option<DeviceSeriesF32>,
    pub source: Option<DeviceSeriesF32>,
}

impl DeviceSeriesF32 {
    #[inline]
    pub fn as_ref(&self) -> DeviceSliceF32Ref<'_> {
        DeviceSliceF32Ref {
            ptr: self.buf.as_device_ptr(),
            len: self.len,
            device_id: self.device_id,
            _marker: PhantomData,
        }
    }
}

impl DeviceMarketDataF32 {
    #[inline]
    pub fn source_or_close(&self) -> DeviceSliceF32Ref<'_> {
        match &self.source {
            Some(src) => src.as_ref(),
            None => self.close.as_ref(),
        }
    }
}
```

### 3. ALMA should be the first concrete example of the new pattern

The wrapper should still look like `CudaAlma`, but it should stop owning the stream and context itself.

This is the shape I would want in `src/cuda/moving_averages/alma_wrapper.rs`.

```rust
use crate::cuda::device_types::{DeviceMatrixF32, DeviceSeriesF32, DeviceSliceF32Ref};
use crate::cuda::runtime::CudaRuntime;
use crate::indicators::moving_averages::alma::{AlmaBatchRange, AlmaParams};
use cust::module::{Module, ModuleJitOption, OptLevel};
use std::sync::Arc;

pub struct AlmaBatchPlanDevice {
    pub periods: DeviceBuffer<i32>,
    pub weights: Option<DeviceBuffer<f32>>,
    pub inv_norms: Option<DeviceBuffer<f32>>,
    pub offsets: Option<DeviceBuffer<f32>>,
    pub sigmas: Option<DeviceBuffer<f32>>,
    pub combos: Vec<AlmaParams>,
    pub max_period: usize,
    pub device_id: u32,
}

pub struct CudaAlma {
    rt: Arc<CudaRuntime>,
    module: Module,
    policy: CudaAlmaPolicy,
    last_batch: Option<BatchKernelSelected>,
    last_many: Option<ManySeriesKernelSelected>,
}

impl CudaAlma {
    pub fn new(rt: Arc<CudaRuntime>) -> Result<Self, CudaAlmaError> {
        let ptx: &str = include_str!(concat!(env!("OUT_DIR"), "/alma_kernel.ptx"));
        let jit_opts = &[
            ModuleJitOption::DetermineTargetFromContext,
            ModuleJitOption::OptLevel(OptLevel::O2),
        ];

        let module = Module::from_ptx(ptx, jit_opts)
            .or_else(|_| Module::from_ptx(ptx, &[ModuleJitOption::DetermineTargetFromContext]))
            .or_else(|_| Module::from_ptx(ptx, &[]))?;

        Ok(Self {
            rt,
            module,
            policy: CudaAlmaPolicy::default(),
            last_batch: None,
            last_many: None,
        })
    }

    #[inline]
    pub fn device_id(&self) -> u32 {
        self.rt.device_id()
    }

    #[inline]
    pub fn synchronize(&self) -> Result<(), CudaAlmaError> {
        self.rt.synchronize().map_err(Into::into)
    }

    pub fn alma_batch_from_device_prices(
        &self,
        prices: DeviceSliceF32Ref<'_>,
        first_valid: usize,
        sweep: &AlmaBatchRange,
        plan: Option<&AlmaBatchPlanDevice>,
    ) -> Result<DeviceMatrixF32, CudaAlmaError> {
        validate_same_device(self.rt.device_id(), &[prices.device_id])?;

        let (combos, max_period) =
            Self::prepare_batch_inputs_device(prices.len, first_valid, sweep)?;

        let mut out = DeviceMatrixF32 {
            buf: unsafe {
                DeviceBuffer::uninitialized_async(
                    combos.len() * prices.len,
                    self.rt.stream(),
                )?
            },
            rows: combos.len(),
            cols: prices.len,
            device_id: self.rt.device_id(),
        };

        self.alma_batch_into_device_from_device_prices(
            prices,
            first_valid,
            sweep,
            plan,
            &mut out,
        )?;

        Ok(out)
    }

    pub fn alma_batch_into_device_from_device_prices(
        &self,
        prices: DeviceSliceF32Ref<'_>,
        first_valid: usize,
        sweep: &AlmaBatchRange,
        plan: Option<&AlmaBatchPlanDevice>,
        out: &mut DeviceMatrixF32,
    ) -> Result<(), CudaAlmaError> {
        validate_same_device(self.rt.device_id(), &[prices.device_id, out.device_id])?;

        let launch_plan = match plan {
            Some(existing) => existing,
            None => return Err(CudaAlmaError::InvalidInput("missing plan".into())),
        };

        self.launch_batch_kernel_precomputed_from_device(
            prices,
            &launch_plan.periods,
            launch_plan.weights.as_ref(),
            launch_plan.inv_norms.as_ref(),
            launch_plan.max_period,
            first_valid,
            out,
        )
    }
}
```

### 4. The old ALMA host API should become a compatibility bridge

The old entrypoints can remain public, but they should become thin host helpers on top of the new device-native path.

This is the migration pattern I want repeated everywhere else.

```rust
impl CudaAlma {
    pub fn alma_batch_dev(
        &self,
        data_f32: &[f32],
        sweep: &AlmaBatchRange,
    ) -> Result<DeviceMatrixF32, CudaAlmaError> {
        let first_valid = data_f32
            .iter()
            .position(|x| !x.is_nan())
            .ok_or_else(|| CudaAlmaError::InvalidInput("all values are NaN".into()))?;

        let prices = self.rt.upload_f32(data_f32)?;
        let plan = self.build_alma_batch_plan(sweep)?;

        self.alma_batch_from_device_prices(prices.as_ref(), first_valid, sweep, Some(&plan))
    }

    pub fn alma_batch_into_host_f32(
        &self,
        data_f32: &[f32],
        sweep: &AlmaBatchRange,
        out: &mut [f32],
    ) -> Result<(usize, usize, Vec<AlmaParams>), CudaAlmaError> {
        let arr = self.alma_batch_dev(data_f32, sweep)?;
        let host = download_matrix_f32(self.rt.as_ref(), &arr)?;
        out.copy_from_slice(&host);
        Ok((arr.rows, arr.cols, self.expand_grid(sweep)?))
    }
}
```

That is the right layering:

- host helper uploads explicitly,
- device-native path never uploads,
- host helper downloads explicitly,
- device-native path never downloads.

### 5. Other wrappers should copy the ALMA pattern exactly

Once ALMA is in this shape, every other indicator wrapper should follow the same naming and ownership rules.

```rust
pub struct CudaZscore {
    rt: Arc<CudaRuntime>,
    module: Module,
}

impl CudaZscore {
    pub fn zscore_batch_from_device_prices(
        &self,
        prices: DeviceSliceF32Ref<'_>,
        first_valid: usize,
        sweep: &ZscoreBatchRange,
        plan: Option<&ZscoreSweepPlanDevice>,
    ) -> Result<DeviceMatrixF32, CudaZscoreError>;
}

pub struct CudaBollinger {
    rt: Arc<CudaRuntime>,
    module: Module,
}

impl CudaBollinger {
    pub fn bollinger_bands_batch_from_device_prices(
        &self,
        prices: DeviceSliceF32Ref<'_>,
        first_valid: usize,
        sweep: &BollingerBandsBatchRange,
        plan: Option<&BollingerSweepPlanDevice>,
    ) -> Result<(DeviceMatrixF32, DeviceMatrixF32, DeviceMatrixF32), CudaBollingerError>;
}

pub struct CudaAtr {
    rt: Arc<CudaRuntime>,
    module: Module,
}

impl CudaAtr {
    pub fn atr_batch_from_ohlc_device(
        &self,
        high: DeviceSliceF32Ref<'_>,
        low: DeviceSliceF32Ref<'_>,
        close: DeviceSliceF32Ref<'_>,
        first_valid: usize,
        sweep: &AtrBatchRange,
        plan: Option<&AtrSweepPlanDevice>,
    ) -> Result<DeviceMatrixF32, CudaAtrError>;
}
```

That gives you one consistent mental model across the whole CUDA side of `vector-ta`.

## Device-Native Dispatch API

Add a new dispatch entrypoint instead of overloading the current one.

```rust
pub fn compute_cuda_device(
    rt: &CudaRuntime,
    req: IndicatorCudaDeviceRequest<'_>,
) -> Result<IndicatorCudaOutput, IndicatorDispatchError>;
```

Behavior:

- accepts only device-backed inputs,
- returns device-backed outputs by default,
- never downloads unless an explicit helper is called later.

Keep the current `compute_cuda(...)` intact for host-backed callers.

## CudaMaSelector Changes

`CudaMaSelector` is where most of the host-materialization damage happens today.

Required changes:

1. Add a device-native input enum.

```rust
pub enum CudaMaDataDevice<'a> {
    SliceDevice(DeviceSliceF32Ref),
    OhlcDevice { ... },
    OhlcvDevice { ... },
}
```

2. Add device-native execution methods.

```rust
pub fn ma_to_device_from_device(
    &self,
    ma_type: &str,
    data: CudaMaDataDevice<'_>,
    period: usize,
) -> Result<DeviceArrayF32, CudaMaSelectorError>;

pub fn ma_sweep_to_device_from_device_with_typed_params(
    &self,
    ma_type: &str,
    data: CudaMaDataDevice<'_>,
    start: usize,
    end: usize,
    step: usize,
    params: &[CudaMaParamKV<'_>],
) -> Result<DeviceArrayF32, CudaMaSelectorError>;
```

3. Remove host materialization from the device path.

Specifically, the device-native path must not call anything like:

- `to_vec()`
- `to_prices_f32()`
- `DeviceBuffer::from_slice(...)` for market data

inside the execution path.

It may still upload small parameter vectors if they are not yet planned or cached.

## Wrapper Changes Needed For VEMR

### Z-score

Add:

```rust
pub fn zscore_batch_dev_from_device(
    &self,
    data: DeviceSliceF32Ref,
    sweep: &ZscoreBatchRange,
    plan: Option<&ZscoreSweepPlanDevice>,
) -> Result<DeviceArrayF32, CudaZscoreError>;
```

Behavior:

- input series is already in VRAM,
- no fresh upload of the price series,
- prefix buffers should be computed on device or reused from a plan.

### Bollinger Bands

Add:

```rust
pub fn bollinger_bands_batch_dev_from_device(
    &self,
    data: DeviceSliceF32Ref,
    sweep: &BollingerBandsBatchRange,
    plan: Option<&BollingerSweepPlanDevice>,
) -> Result<(DeviceArrayF32, DeviceArrayF32, DeviceArrayF32), CudaBollingerError>;
```

Behavior:

- input series already on device,
- no fresh price upload,
- reusable parameter buffers and scratch.

### ATR

Add:

```rust
pub fn compute_tr_device(
    &self,
    high: DeviceSliceF32Ref,
    low: DeviceSliceF32Ref,
    close: DeviceSliceF32Ref,
    out_tr: &mut DeviceSeriesF32,
) -> Result<(), CudaAtrError>;

pub fn atr_batch_dev_from_ohlc_device(
    &self,
    high: DeviceSliceF32Ref,
    low: DeviceSliceF32Ref,
    close: DeviceSliceF32Ref,
    sweep: &AtrBatchRange,
    plan: Option<&AtrSweepPlanDevice>,
) -> Result<DeviceArrayF32Atr, CudaAtrError>;
```

The existing `atr_batch_device_with_tr(...)` should remain and become the internal low-level building block for the higher-level device-native ATR path.

## Reusable Plans And Scratch

To avoid reallocating helper buffers on every call, add explicit reusable plans:

```rust
pub struct ZscoreSweepPlanDevice { ... }
pub struct BollingerSweepPlanDevice { ... }
pub struct AtrSweepPlanDevice { ... }
```

These should hold:

- period arrays,
- warmup arrays,
- devup/devdn arrays,
- any scratch or prefix buffers that can be reused,
- optional precomputed first-valid metadata.

Behavior:

- plans are device-specific,
- plans are reusable across many runs on the same device,
- plans are invalid if device changes.

## Explicit Download API

Add download helpers instead of downloading inside execution methods.

```rust
pub fn download_series_f32(rt: &CudaRuntime, src: &DeviceSeriesF32) -> Result<Vec<f32>, CudaError>;
pub fn download_matrix_f32(rt: &CudaRuntime, src: &DeviceMatrixF32) -> Result<Vec<f32>, CudaError>;
```

If the caller never invokes these, no device-to-host copy should occur.

## Exact Naming Rule

Use naming to make behavior obvious:

- `upload_*` = host to device
- `download_*` = device to host
- `*_from_device` = accepts device-resident inputs, no implicit upload
- `*_into_device` = writes into caller-provided device output buffers
- `*_dev` may remain for compatibility, but new code should prefer explicit names

## VEMR Target Flow

This is the desired end-state flow for VEMR:

```rust
let rt = CudaRuntime::new(device_id)?;

let market = rt.upload_ohlc(
    open_f32,
    high_f32,
    low_f32,
    close_f32,
    source_f32_opt,
)?;

let z = compute_zscore_sweep_from_device(&rt, market.source_or_close(), z_plan)?;
let bb = compute_bollinger_sweep_from_device(&rt, market.source_or_close(), bb_plan)?;
let atr = compute_atr_sweep_from_device(&rt, market.high(), market.low(), market.close(), atr_plan)?;

let metrics = vemr_backtest_metrics_f32(
    market.close(),
    z,
    bb,
    atr,
    combo_params_device,
);

let topk = reduce_metrics_topk_on_device(metrics);
let host = download_topk(topk)?;
```

Allowed per-run uploads:

- combo metadata,
- parameter ranges if not already planned,
- tiny control buffers.

Disallowed per-run uploads:

- full price history,
- full OHLC arrays,
- full source series,
- already-uploaded indicator inputs.

## File-Level Change List In VectorTA

### Add new core modules

- `src/cuda/runtime.rs`
- `src/cuda/device_types.rs`
- optionally `src/cuda/scratch.rs`

### Extend dispatch types

- `src/indicators/dispatch/types.rs`
  - add device-input variants and device ref types

### Add device-native dispatch entrypoint

- `src/indicators/dispatch/cuda.rs`
  - add `compute_cuda_device(...)`
  - add `cuda_data_from_req_device(...)`

### Extend selector

- `src/cuda/moving_averages/ma_selector.rs`
  - add `CudaMaDataDevice`
  - add `ma_to_device_from_device(...)`
  - add `ma_sweep_to_device_from_device_with_typed_params(...)`
  - keep old host path intact

### Extend indicator wrappers

- `src/cuda/zscore_wrapper.rs`
- `src/cuda/bollinger_bands_wrapper.rs`
- `src/cuda/atr_wrapper.rs`

### Optional interop

- `src/utilities/dlpack_cuda.rs`
  - keep export support
  - optionally add import support later if Python interop matters

## Migration Strategy

Phase 1:

- add `CudaRuntime`,
- add device ref types,
- add explicit upload/download helpers,
- add device-native dispatch entrypoint.

Phase 2:

- implement device-native `zscore`, Bollinger, and ATR wrappers,
- add reusable plan objects.

Phase 3:

- switch VEMR backend to the new API,
- confirm one-time OHLC upload and no repeated price re-upload.

Phase 4:

- migrate other strategies selectively,
- optionally make old host APIs thin bridges over the new device-native path.

## Final Recommendation

Yes, add a new wrapper/API layer rather than trying to refactor the current host API in place.

But make sure that new layer is truly device-native.

Bad version:

- new wrapper in the app repo calling the old host-slice `vector-ta` APIs

Good version:

- new device-native API layer inside `vector-ta`,
- old host APIs stay,
- performance-sensitive code adopts the new device-native layer.

That gives you:

- low migration risk,
- no immediate ecosystem breakage,
- a clean path to true VRAM-resident indicator pipelines.
