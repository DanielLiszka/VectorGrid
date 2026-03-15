use crate::indicator_catalog::{
    find_dynamic_indicator_descriptor, normalize_and_validate_indicator_output_ref,
    normalize_and_validate_indicator_params, IndicatorInputKind, IndicatorParamKV,
    IndicatorParamValue,
};
#[cfg(feature = "cuda")]
use std::cell::RefCell;
#[cfg(feature = "cuda")]
use std::collections::HashMap;
#[cfg(feature = "cuda")]
use std::sync::Arc;
use vector_ta::indicators::dispatch::{
    compute_cpu, IndicatorComputeRequest, IndicatorDataRef, IndicatorSeries, ParamKV, ParamValue,
};
use vector_ta::utilities::data_loader::{source_type, Candles};
use vector_ta::utilities::enums::Kernel;

#[cfg(feature = "cuda")]
use vector_ta::cuda::{
    CudaDeviceCloseVolumeRef, CudaDeviceHighLowRef, CudaDeviceOhlcRef, CudaDeviceOhlcvRef,
    CudaDeviceSliceF32Ref, CudaDeviceVectorF32, CudaRuntime,
};
#[cfg(feature = "cuda")]
use vector_ta::indicators::dispatch::{
    compute_cuda, compute_cuda_device, IndicatorCudaDataRef, IndicatorCudaDeviceDataRef,
    IndicatorCudaDeviceRequest, IndicatorCudaRequest, IndicatorCudaSeries,
};

#[derive(Clone, Debug, PartialEq)]
pub enum DynamicIndicatorSeries {
    F64(Vec<f64>),
    I32(Vec<i32>),
    Bool(Vec<bool>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DynamicIndicatorAdapterError {
    UnsupportedIndicator {
        indicator_id: String,
    },
    CudaPreparation {
        details: String,
    },
    ParamValidation {
        indicator_id: String,
        details: String,
    },
    OutputValidation {
        indicator_id: String,
        details: String,
    },
    ComputeFailure {
        indicator_id: String,
        details: String,
    },
}

impl std::fmt::Display for DynamicIndicatorAdapterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DynamicIndicatorAdapterError::UnsupportedIndicator { indicator_id } => write!(
                f,
                "unsupported_indicator: '{}' is not in dynamic indicator registry",
                indicator_id
            ),
            DynamicIndicatorAdapterError::CudaPreparation { details } => {
                write!(f, "cuda preparation failed: {}", details)
            }
            DynamicIndicatorAdapterError::ParamValidation {
                indicator_id,
                details,
            } => write!(
                f,
                "indicator '{}' parameter validation failed: {}",
                indicator_id, details
            ),
            DynamicIndicatorAdapterError::OutputValidation {
                indicator_id,
                details,
            } => write!(
                f,
                "indicator '{}' output validation failed: {}",
                indicator_id, details
            ),
            DynamicIndicatorAdapterError::ComputeFailure {
                indicator_id,
                details,
            } => write!(
                f,
                "indicator '{}' compute failed: {}",
                indicator_id, details
            ),
        }
    }
}

impl std::error::Error for DynamicIndicatorAdapterError {}

#[derive(Clone, Debug, PartialEq)]
pub struct DynamicIndicatorComputeOutput {
    pub output_id: String,
    pub series: DynamicIndicatorSeries,
    pub warmup: Option<usize>,
    pub rows: usize,
    pub cols: usize,
    pub pattern_ids: Option<Vec<String>>,
}

#[cfg(feature = "cuda")]
#[derive(Clone, Debug, PartialEq)]
pub enum DynamicIndicatorCudaSeries {
    DeviceF32(vector_ta::indicators::dispatch::DeviceMatrixF32),
    HostF32(Vec<f32>),
}

#[cfg(feature = "cuda")]
#[derive(Clone, Debug, PartialEq)]
pub struct DynamicIndicatorCudaComputeOutput {
    pub output_id: String,
    pub series: DynamicIndicatorCudaSeries,
    pub warmup: Option<usize>,
    pub rows: usize,
    pub cols: usize,
    pub pattern_ids: Option<Vec<String>>,
}

fn map_param_value<'a>(value: &'a IndicatorParamValue) -> ParamValue<'a> {
    match value {
        IndicatorParamValue::Int(v) => ParamValue::Int(*v),
        IndicatorParamValue::Float(v) => ParamValue::Float(*v),
        IndicatorParamValue::Bool(v) => ParamValue::Bool(*v),
        IndicatorParamValue::EnumString(v) => ParamValue::EnumString(v.as_str()),
    }
}

fn to_dispatch_params<'a>(params: &'a [IndicatorParamKV]) -> Vec<ParamKV<'a>> {
    params
        .iter()
        .map(|kv| ParamKV {
            key: kv.key.as_str(),
            value: map_param_value(&kv.value),
        })
        .collect()
}

fn data_ref_for_input_kind<'a>(
    candles: &'a Candles,
    ma_source: &'a str,
    input_kind: IndicatorInputKind,
) -> IndicatorDataRef<'a> {
    match input_kind {
        IndicatorInputKind::Slice => IndicatorDataRef::Slice {
            values: source_type(candles, ma_source),
        },
        IndicatorInputKind::Candles => IndicatorDataRef::Candles {
            candles,
            source: Some(ma_source),
        },
        IndicatorInputKind::Ohlc => IndicatorDataRef::Ohlc {
            open: candles.open.as_slice(),
            high: candles.high.as_slice(),
            low: candles.low.as_slice(),
            close: candles.close.as_slice(),
        },
        IndicatorInputKind::Ohlcv => IndicatorDataRef::Ohlcv {
            open: candles.open.as_slice(),
            high: candles.high.as_slice(),
            low: candles.low.as_slice(),
            close: candles.close.as_slice(),
            volume: candles.volume.as_slice(),
        },
        IndicatorInputKind::HighLow => IndicatorDataRef::HighLow {
            high: candles.high.as_slice(),
            low: candles.low.as_slice(),
        },
        IndicatorInputKind::CloseVolume => IndicatorDataRef::CloseVolume {
            close: source_type(candles, ma_source),
            volume: candles.volume.as_slice(),
        },
    }
}

pub fn compute_dynamic_indicator_cpu(
    candles: &Candles,
    ma_source: &str,
    indicator_id: &str,
    output_id: Option<&str>,
    params: &[IndicatorParamKV],
    kernel: Kernel,
) -> Result<DynamicIndicatorComputeOutput, DynamicIndicatorAdapterError> {
    let descriptor = find_dynamic_indicator_descriptor(indicator_id).ok_or_else(|| {
        DynamicIndicatorAdapterError::UnsupportedIndicator {
            indicator_id: indicator_id.to_string(),
        }
    })?;
    let normalized_id = descriptor.id.as_str();
    let normalized_params = normalize_and_validate_indicator_params(normalized_id, params)
        .map_err(|details| DynamicIndicatorAdapterError::ParamValidation {
            indicator_id: normalized_id.to_string(),
            details,
        })?;
    let normalized_output_id =
        normalize_and_validate_indicator_output_ref(normalized_id, output_id).map_err(
            |details| DynamicIndicatorAdapterError::OutputValidation {
                indicator_id: normalized_id.to_string(),
                details,
            },
        )?;
    let dispatch_params = to_dispatch_params(normalized_params.as_slice());
    let data = data_ref_for_input_kind(candles, ma_source, descriptor.input_kind);

    let out = compute_cpu(IndicatorComputeRequest {
        indicator_id: normalized_id,
        output_id: Some(normalized_output_id.as_str()),
        data,
        params: dispatch_params.as_slice(),
        kernel,
    })
    .map_err(|e| DynamicIndicatorAdapterError::ComputeFailure {
        indicator_id: normalized_id.to_string(),
        details: e.to_string(),
    })?;

    let series = match out.series {
        IndicatorSeries::F64(v) => DynamicIndicatorSeries::F64(v),
        IndicatorSeries::I32(v) => DynamicIndicatorSeries::I32(v),
        IndicatorSeries::Bool(v) => DynamicIndicatorSeries::Bool(v),
    };

    Ok(DynamicIndicatorComputeOutput {
        output_id: out.output_id,
        series,
        warmup: out.warmup,
        rows: out.rows,
        cols: out.cols,
        pattern_ids: out.pattern_ids,
    })
}

#[cfg(feature = "cuda")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PreparedCudaSourceKind {
    Open,
    High,
    Low,
    Close,
    Volume,
    Derived,
}

#[cfg(feature = "cuda")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PreparedCudaDatasetSignature {
    len: usize,
    timestamp_ptr: usize,
    open_ptr: usize,
    high_ptr: usize,
    low_ptr: usize,
    close_ptr: usize,
    volume_ptr: usize,
}

#[cfg(feature = "cuda")]
#[derive(Debug)]
struct ResidentCudaIndicatorDataset {
    signature: PreparedCudaDatasetSignature,
    runtime: Arc<CudaRuntime>,
    open: Arc<Vec<f32>>,
    high: Arc<Vec<f32>>,
    low: Arc<Vec<f32>>,
    close: Arc<Vec<f32>>,
    volume: Arc<Vec<f32>>,
    d_open: Arc<CudaDeviceVectorF32>,
    d_high: Arc<CudaDeviceVectorF32>,
    d_low: Arc<CudaDeviceVectorF32>,
    d_close: Arc<CudaDeviceVectorF32>,
    d_volume: Arc<CudaDeviceVectorF32>,
    source_overrides: HashMap<String, Arc<Vec<f32>>>,
    d_source_overrides: HashMap<String, Arc<CudaDeviceVectorF32>>,
}

#[cfg(feature = "cuda")]
thread_local! {
    static PREPARED_CUDA_INPUT_CACHE: RefCell<HashMap<usize, ResidentCudaIndicatorDataset>> =
        RefCell::new(HashMap::new());
}

#[cfg(all(feature = "cuda", test))]
pub(crate) fn reset_test_cuda_input_cache() {
    PREPARED_CUDA_INPUT_CACHE.with(|cell| {
        cell.borrow_mut().clear();
    });
}

#[cfg(feature = "cuda")]
#[inline]
fn prepared_cuda_dataset_signature(candles: &Candles) -> PreparedCudaDatasetSignature {
    PreparedCudaDatasetSignature {
        len: candles.close.len(),
        timestamp_ptr: candles.timestamp.as_ptr() as usize,
        open_ptr: candles.open.as_ptr() as usize,
        high_ptr: candles.high.as_ptr() as usize,
        low_ptr: candles.low.as_ptr() as usize,
        close_ptr: candles.close.as_ptr() as usize,
        volume_ptr: candles.volume.as_ptr() as usize,
    }
}

#[cfg(feature = "cuda")]
#[inline]
fn normalize_prepared_cuda_source_key(ma_source: &str) -> String {
    let trimmed = ma_source.trim();
    if trimmed.is_empty() {
        return "close".to_string();
    }
    trimmed.to_ascii_lowercase()
}

#[cfg(feature = "cuda")]
#[derive(Debug)]
pub struct PreparedCudaIndicatorInputs {
    source_kind: PreparedCudaSourceKind,
    source_override: Option<Arc<Vec<f32>>>,
    open: Arc<Vec<f32>>,
    high: Arc<Vec<f32>>,
    low: Arc<Vec<f32>>,
    close: Arc<Vec<f32>>,
    volume: Arc<Vec<f32>>,
    runtime: Arc<CudaRuntime>,
    d_source_override: Option<Arc<CudaDeviceVectorF32>>,
    d_open: Arc<CudaDeviceVectorF32>,
    d_high: Arc<CudaDeviceVectorF32>,
    d_low: Arc<CudaDeviceVectorF32>,
    d_close: Arc<CudaDeviceVectorF32>,
    d_volume: Arc<CudaDeviceVectorF32>,
}

#[cfg(feature = "cuda")]
impl PreparedCudaIndicatorInputs {
    pub fn from_candles(
        candles: &Candles,
        ma_source: &str,
    ) -> Result<Self, DynamicIndicatorAdapterError> {
        Self::from_candles_on_device(candles, ma_source, 0)
    }

    pub fn from_candles_on_device(
        candles: &Candles,
        ma_source: &str,
        device_id: usize,
    ) -> Result<Self, DynamicIndicatorAdapterError> {
        #[cfg(test)]
        let _gpu_guard = crate::gpu_backtest_kernel::lock_gpu_test();
        let signature = prepared_cuda_dataset_signature(candles);
        let source_key = normalize_prepared_cuda_source_key(ma_source);

        PREPARED_CUDA_INPUT_CACHE.with(|cell| {
            let mut cache = cell.borrow_mut();
            let rebuild = cache
                .get(&device_id)
                .map(|entry| entry.signature != signature)
                .unwrap_or(true);
            if rebuild {
                let runtime = Arc::new(CudaRuntime::new(device_id).map_err(|e| {
                    DynamicIndicatorAdapterError::CudaPreparation {
                        details: e.to_string(),
                    }
                })?);
                let open = Arc::new(candles.open.iter().map(|v| *v as f32).collect::<Vec<f32>>());
                let high = Arc::new(candles.high.iter().map(|v| *v as f32).collect::<Vec<f32>>());
                let low = Arc::new(candles.low.iter().map(|v| *v as f32).collect::<Vec<f32>>());
                let close = Arc::new(
                    candles
                        .close
                        .iter()
                        .map(|v| *v as f32)
                        .collect::<Vec<f32>>(),
                );
                let volume = Arc::new(
                    candles
                        .volume
                        .iter()
                        .map(|v| *v as f32)
                        .collect::<Vec<f32>>(),
                );
                let d_open = Arc::new(runtime.upload_f32(open.as_slice()).map_err(|e| {
                    DynamicIndicatorAdapterError::CudaPreparation {
                        details: e.to_string(),
                    }
                })?);
                let d_high = Arc::new(runtime.upload_f32(high.as_slice()).map_err(|e| {
                    DynamicIndicatorAdapterError::CudaPreparation {
                        details: e.to_string(),
                    }
                })?);
                let d_low = Arc::new(runtime.upload_f32(low.as_slice()).map_err(|e| {
                    DynamicIndicatorAdapterError::CudaPreparation {
                        details: e.to_string(),
                    }
                })?);
                let d_close = Arc::new(runtime.upload_f32(close.as_slice()).map_err(|e| {
                    DynamicIndicatorAdapterError::CudaPreparation {
                        details: e.to_string(),
                    }
                })?);
                let d_volume = Arc::new(runtime.upload_f32(volume.as_slice()).map_err(|e| {
                    DynamicIndicatorAdapterError::CudaPreparation {
                        details: e.to_string(),
                    }
                })?);
                cache.insert(
                    device_id,
                    ResidentCudaIndicatorDataset {
                        signature,
                        runtime,
                        open,
                        high,
                        low,
                        close,
                        volume,
                        d_open,
                        d_high,
                        d_low,
                        d_close,
                        d_volume,
                        source_overrides: HashMap::new(),
                        d_source_overrides: HashMap::new(),
                    },
                );
            }

            let entry = cache.get_mut(&device_id).ok_or_else(|| {
                DynamicIndicatorAdapterError::CudaPreparation {
                    details: "prepared CUDA cache entry missing".to_string(),
                }
            })?;

            let source_kind = match source_key.as_str() {
                "open" => PreparedCudaSourceKind::Open,
                "high" => PreparedCudaSourceKind::High,
                "low" => PreparedCudaSourceKind::Low,
                "close" => PreparedCudaSourceKind::Close,
                "volume" => PreparedCudaSourceKind::Volume,
                _ => PreparedCudaSourceKind::Derived,
            };

            if source_kind == PreparedCudaSourceKind::Derived
                && !entry.source_overrides.contains_key(&source_key)
            {
                let source_override = Arc::new(
                    source_type(candles, ma_source)
                        .iter()
                        .map(|v| *v as f32)
                        .collect::<Vec<f32>>(),
                );
                let d_source_override = Arc::new(
                    entry
                        .runtime
                        .upload_f32(source_override.as_slice())
                        .map_err(|e| DynamicIndicatorAdapterError::CudaPreparation {
                            details: e.to_string(),
                        })?,
                );
                entry
                    .source_overrides
                    .insert(source_key.clone(), source_override);
                entry
                    .d_source_overrides
                    .insert(source_key.clone(), d_source_override);
            }

            Ok(Self {
                source_kind,
                source_override: entry.source_overrides.get(&source_key).cloned(),
                open: entry.open.clone(),
                high: entry.high.clone(),
                low: entry.low.clone(),
                close: entry.close.clone(),
                volume: entry.volume.clone(),
                runtime: entry.runtime.clone(),
                d_source_override: entry.d_source_overrides.get(&source_key).cloned(),
                d_open: entry.d_open.clone(),
                d_high: entry.d_high.clone(),
                d_low: entry.d_low.clone(),
                d_close: entry.d_close.clone(),
                d_volume: entry.d_volume.clone(),
            })
        })
    }

    fn source_slice(&self) -> &[f32] {
        match self.source_kind {
            PreparedCudaSourceKind::Open => self.open.as_slice(),
            PreparedCudaSourceKind::High => self.high.as_slice(),
            PreparedCudaSourceKind::Low => self.low.as_slice(),
            PreparedCudaSourceKind::Close => self.close.as_slice(),
            PreparedCudaSourceKind::Volume => self.volume.as_slice(),
            PreparedCudaSourceKind::Derived => self
                .source_override
                .as_ref()
                .expect("derived source kind requires source override")
                .as_slice(),
        }
    }

    pub fn source(&self) -> &[f32] {
        self.source_slice()
    }

    pub fn open(&self) -> &[f32] {
        self.open.as_slice()
    }

    pub fn high(&self) -> &[f32] {
        self.high.as_slice()
    }

    pub fn low(&self) -> &[f32] {
        self.low.as_slice()
    }

    pub fn close(&self) -> &[f32] {
        self.close.as_slice()
    }

    pub fn volume(&self) -> &[f32] {
        self.volume.as_slice()
    }

    pub fn runtime(&self) -> &CudaRuntime {
        self.runtime.as_ref()
    }

    pub fn len(&self) -> usize {
        self.close.len()
    }

    pub fn source_device(&self) -> &CudaDeviceVectorF32 {
        match self.source_kind {
            PreparedCudaSourceKind::Open => &self.d_open,
            PreparedCudaSourceKind::High => &self.d_high,
            PreparedCudaSourceKind::Low => &self.d_low,
            PreparedCudaSourceKind::Close => &self.d_close,
            PreparedCudaSourceKind::Volume => &self.d_volume,
            PreparedCudaSourceKind::Derived => self
                .d_source_override
                .as_ref()
                .expect("derived source kind requires source override"),
        }
    }

    pub fn open_device(&self) -> &CudaDeviceVectorF32 {
        &self.d_open
    }

    pub fn high_device(&self) -> &CudaDeviceVectorF32 {
        &self.d_high
    }

    pub fn low_device(&self) -> &CudaDeviceVectorF32 {
        &self.d_low
    }

    pub fn close_device(&self) -> &CudaDeviceVectorF32 {
        &self.d_close
    }

    pub fn volume_device(&self) -> &CudaDeviceVectorF32 {
        &self.d_volume
    }

    pub fn source_first_valid(&self) -> usize {
        first_valid_in_slice_f32(self.source())
    }

    pub fn high_low_first_valid(&self) -> usize {
        first_valid_in_pair_f32(self.high(), self.low())
    }

    pub fn hlc_first_valid(&self) -> usize {
        first_valid_in_triple_f32(self.high(), self.low(), self.close())
    }

    pub fn ohlc_first_valid(&self) -> usize {
        self.open()
            .iter()
            .zip(self.high().iter())
            .zip(self.low().iter())
            .zip(self.close().iter())
            .position(|(((open, high), low), close)| {
                open.is_finite() && high.is_finite() && low.is_finite() && close.is_finite()
            })
            .unwrap_or(self.len())
    }

    pub fn ohlcv_first_valid(&self) -> usize {
        self.open()
            .iter()
            .zip(self.high().iter())
            .zip(self.low().iter())
            .zip(self.close().iter())
            .zip(self.volume().iter())
            .position(|((((open, high), low), close), volume)| {
                open.is_finite()
                    && high.is_finite()
                    && low.is_finite()
                    && close.is_finite()
                    && volume.is_finite()
            })
            .unwrap_or(self.len())
    }

    pub fn close_volume_first_valid(&self) -> usize {
        first_valid_in_pair_f32(self.source(), self.volume())
    }

    pub fn first_valid_for_input_kind(&self, input_kind: IndicatorInputKind) -> usize {
        match input_kind {
            IndicatorInputKind::Slice => self.source_first_valid(),
            IndicatorInputKind::Candles | IndicatorInputKind::Ohlc => self.ohlc_first_valid(),
            IndicatorInputKind::Ohlcv => self.ohlcv_first_valid(),
            IndicatorInputKind::HighLow => self.high_low_first_valid(),
            IndicatorInputKind::CloseVolume => self.close_volume_first_valid(),
        }
    }
}

#[cfg(feature = "cuda")]
fn first_valid_in_slice_f32(values: &[f32]) -> usize {
    values
        .iter()
        .position(|v| v.is_finite())
        .unwrap_or(values.len())
}

#[cfg(feature = "cuda")]
fn first_valid_in_pair_f32(lhs: &[f32], rhs: &[f32]) -> usize {
    lhs.iter()
        .zip(rhs.iter())
        .position(|(a, b)| a.is_finite() && b.is_finite())
        .unwrap_or(lhs.len().min(rhs.len()))
}

#[cfg(feature = "cuda")]
fn first_valid_in_triple_f32(a: &[f32], b: &[f32], c: &[f32]) -> usize {
    a.iter()
        .zip(b.iter())
        .zip(c.iter())
        .position(|((x, y), z)| x.is_finite() && y.is_finite() && z.is_finite())
        .unwrap_or(a.len().min(b.len()).min(c.len()))
}

#[cfg(feature = "cuda")]
fn cuda_data_ref_for_input_kind<'a>(
    buffers: &'a PreparedCudaIndicatorInputs,
    input_kind: IndicatorInputKind,
) -> IndicatorCudaDataRef<'a> {
    match input_kind {
        IndicatorInputKind::Slice => IndicatorCudaDataRef::Slice {
            values: buffers.source_slice(),
        },
        IndicatorInputKind::Candles | IndicatorInputKind::Ohlc => IndicatorCudaDataRef::Ohlc {
            open: buffers.open.as_slice(),
            high: buffers.high.as_slice(),
            low: buffers.low.as_slice(),
            close: buffers.close.as_slice(),
            source: Some(buffers.source_slice()),
        },
        IndicatorInputKind::Ohlcv => IndicatorCudaDataRef::Ohlcv {
            timestamp: None,
            open: buffers.open.as_slice(),
            high: buffers.high.as_slice(),
            low: buffers.low.as_slice(),
            close: buffers.close.as_slice(),
            volume: buffers.volume.as_slice(),
            source: Some(buffers.source_slice()),
        },
        IndicatorInputKind::HighLow => IndicatorCudaDataRef::HighLow {
            high: buffers.high.as_slice(),
            low: buffers.low.as_slice(),
        },
        IndicatorInputKind::CloseVolume => IndicatorCudaDataRef::CloseVolume {
            close: buffers.source_slice(),
            volume: buffers.volume.as_slice(),
        },
    }
}

#[cfg(feature = "cuda")]
fn cuda_device_data_ref_for_input_kind(
    buffers: &PreparedCudaIndicatorInputs,
    input_kind: IndicatorInputKind,
) -> Result<IndicatorCudaDeviceDataRef, DynamicIndicatorAdapterError> {
    match input_kind {
        IndicatorInputKind::Slice => Ok(IndicatorCudaDeviceDataRef::Slice {
            values: buffers.source_device().as_view(),
        }),
        IndicatorInputKind::Candles | IndicatorInputKind::Ohlc => {
            Ok(IndicatorCudaDeviceDataRef::Ohlc(
                CudaDeviceOhlcRef::new(
                    buffers.open_device().as_view(),
                    buffers.high_device().as_view(),
                    buffers.low_device().as_view(),
                    buffers.close_device().as_view(),
                    Some(buffers.source_device().as_view()),
                )
                .map_err(|e| DynamicIndicatorAdapterError::CudaPreparation {
                    details: e.to_string(),
                })?,
            ))
        }
        IndicatorInputKind::Ohlcv => Ok(IndicatorCudaDeviceDataRef::Ohlcv(
            CudaDeviceOhlcvRef::new(
                None,
                buffers.open_device().as_view(),
                buffers.high_device().as_view(),
                buffers.low_device().as_view(),
                buffers.close_device().as_view(),
                buffers.volume_device().as_view(),
                Some(buffers.source_device().as_view()),
            )
            .map_err(|e| DynamicIndicatorAdapterError::CudaPreparation {
                details: e.to_string(),
            })?,
        )),
        IndicatorInputKind::HighLow => Ok(IndicatorCudaDeviceDataRef::HighLow(
            CudaDeviceHighLowRef::new(
                buffers.high_device().as_view(),
                buffers.low_device().as_view(),
            )
            .map_err(|e| DynamicIndicatorAdapterError::CudaPreparation {
                details: e.to_string(),
            })?,
        )),
        IndicatorInputKind::CloseVolume => Ok(IndicatorCudaDeviceDataRef::CloseVolume(
            CudaDeviceCloseVolumeRef::new(
                buffers.source_device().as_view(),
                buffers.volume_device().as_view(),
            )
            .map_err(|e| DynamicIndicatorAdapterError::CudaPreparation {
                details: e.to_string(),
            })?,
        )),
    }
}

#[cfg(feature = "cuda")]
fn dispatch_params_with_first_valid<'a>(
    params: &'a [IndicatorParamKV],
    first_valid: usize,
) -> Vec<ParamKV<'a>> {
    let mut dispatch_params = to_dispatch_params(params);
    dispatch_params.retain(|param| !param.key.eq_ignore_ascii_case("first_valid"));
    dispatch_params.push(ParamKV {
        key: "first_valid",
        value: ParamValue::Int(first_valid as i64),
    });
    dispatch_params
}

#[cfg(feature = "cuda")]
pub(crate) fn device_slice_ref_from_matrix_row(
    matrix: &vector_ta::indicators::dispatch::DeviceMatrixF32,
    row: usize,
) -> Result<CudaDeviceSliceF32Ref, DynamicIndicatorAdapterError> {
    if row >= matrix.rows {
        return Err(DynamicIndicatorAdapterError::CudaPreparation {
            details: format!(
                "device matrix row {row} out of range (rows={})",
                matrix.rows
            ),
        });
    }
    let row_len = matrix.cols;
    let byte_offset = row
        .checked_mul(row_len)
        .and_then(|items| items.checked_mul(std::mem::size_of::<f32>()))
        .ok_or_else(|| DynamicIndicatorAdapterError::CudaPreparation {
            details: "device matrix row offset overflow".to_string(),
        })?;
    let device_ptr = matrix
        .device_ptr
        .checked_add(byte_offset as u64)
        .ok_or_else(|| DynamicIndicatorAdapterError::CudaPreparation {
            details: "device matrix pointer overflow".to_string(),
        })?;
    unsafe { CudaDeviceSliceF32Ref::from_raw_parts(device_ptr, row_len, matrix.device_id) }.map_err(
        |e| DynamicIndicatorAdapterError::CudaPreparation {
            details: e.to_string(),
        },
    )
}

#[cfg(feature = "cuda")]
pub fn compute_dynamic_indicator_cuda_from_device_matrix_row(
    indicator_id: &str,
    output_id: Option<&str>,
    matrix: &vector_ta::indicators::dispatch::DeviceMatrixF32,
    row: usize,
    first_valid: usize,
    params: &[IndicatorParamKV],
    kernel: Kernel,
    target: vector_ta::indicators::dispatch::CudaOutputTarget,
) -> Result<DynamicIndicatorCudaComputeOutput, DynamicIndicatorAdapterError> {
    let descriptor = find_dynamic_indicator_descriptor(indicator_id).ok_or_else(|| {
        DynamicIndicatorAdapterError::UnsupportedIndicator {
            indicator_id: indicator_id.to_string(),
        }
    })?;
    let normalized_id = descriptor.id.as_str();
    if descriptor.input_kind != IndicatorInputKind::Slice {
        return Err(DynamicIndicatorAdapterError::CudaPreparation {
            details: format!(
                "indicator '{}' requires {:?} input and cannot be chained from a device slice",
                normalized_id, descriptor.input_kind
            ),
        });
    }
    let normalized_params = normalize_and_validate_indicator_params(normalized_id, params)
        .map_err(|details| DynamicIndicatorAdapterError::ParamValidation {
            indicator_id: normalized_id.to_string(),
            details,
        })?;
    let normalized_output_id =
        normalize_and_validate_indicator_output_ref(normalized_id, output_id).map_err(
            |details| DynamicIndicatorAdapterError::OutputValidation {
                indicator_id: normalized_id.to_string(),
                details,
            },
        )?;
    let dispatch_params =
        dispatch_params_with_first_valid(normalized_params.as_slice(), first_valid);
    let values = device_slice_ref_from_matrix_row(matrix, row)?;
    let out = compute_cuda_device(IndicatorCudaDeviceRequest {
        indicator_id: normalized_id,
        output_id: Some(normalized_output_id.as_str()),
        data: IndicatorCudaDeviceDataRef::Slice { values },
        params: dispatch_params.as_slice(),
        kernel,
        target,
    })
    .map_err(|e| DynamicIndicatorAdapterError::ComputeFailure {
        indicator_id: normalized_id.to_string(),
        details: e.to_string(),
    })?;

    let series = match out.series {
        IndicatorCudaSeries::DeviceF32(v) => DynamicIndicatorCudaSeries::DeviceF32(v),
        IndicatorCudaSeries::HostF32(v) => DynamicIndicatorCudaSeries::HostF32(v),
    };

    Ok(DynamicIndicatorCudaComputeOutput {
        output_id: out.output_id,
        series,
        warmup: out.warmup,
        rows: out.rows,
        cols: out.cols,
        pattern_ids: out.pattern_ids,
    })
}

#[cfg(feature = "cuda")]
pub fn compute_dynamic_indicator_cuda_with_inputs(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    indicator_id: &str,
    output_id: Option<&str>,
    params: &[IndicatorParamKV],
    kernel: Kernel,
    target: vector_ta::indicators::dispatch::CudaOutputTarget,
) -> Result<DynamicIndicatorCudaComputeOutput, DynamicIndicatorAdapterError> {
    #[cfg(test)]
    let _gpu_guard = crate::gpu_backtest_kernel::lock_gpu_test();
    let descriptor = find_dynamic_indicator_descriptor(indicator_id).ok_or_else(|| {
        DynamicIndicatorAdapterError::UnsupportedIndicator {
            indicator_id: indicator_id.to_string(),
        }
    })?;
    let normalized_id = descriptor.id.as_str();
    let normalized_params = normalize_and_validate_indicator_params(normalized_id, params)
        .map_err(|details| DynamicIndicatorAdapterError::ParamValidation {
            indicator_id: normalized_id.to_string(),
            details,
        })?;
    let normalized_output_id =
        normalize_and_validate_indicator_output_ref(normalized_id, output_id).map_err(
            |details| DynamicIndicatorAdapterError::OutputValidation {
                indicator_id: normalized_id.to_string(),
                details,
            },
        )?;
    let first_valid = prepared_inputs.first_valid_for_input_kind(descriptor.input_kind);
    let dispatch_params =
        dispatch_params_with_first_valid(normalized_params.as_slice(), first_valid);
    let data = cuda_data_ref_for_input_kind(prepared_inputs, descriptor.input_kind);
    let out = match cuda_device_data_ref_for_input_kind(prepared_inputs, descriptor.input_kind)
        .and_then(|device_data| {
            compute_cuda_device(IndicatorCudaDeviceRequest {
                indicator_id: normalized_id,
                output_id: Some(normalized_output_id.as_str()),
                data: device_data,
                params: dispatch_params.as_slice(),
                kernel,
                target,
            })
            .map_err(|e| DynamicIndicatorAdapterError::ComputeFailure {
                indicator_id: normalized_id.to_string(),
                details: e.to_string(),
            })
        }) {
        Ok(out) => out,
        Err(_) => compute_cuda(IndicatorCudaRequest {
            indicator_id: normalized_id,
            output_id: Some(normalized_output_id.as_str()),
            data,
            params: dispatch_params.as_slice(),
            kernel,
            target,
        })
        .map_err(|e| DynamicIndicatorAdapterError::ComputeFailure {
            indicator_id: normalized_id.to_string(),
            details: e.to_string(),
        })?,
    };

    let series = match out.series {
        IndicatorCudaSeries::DeviceF32(v) => DynamicIndicatorCudaSeries::DeviceF32(v),
        IndicatorCudaSeries::HostF32(v) => DynamicIndicatorCudaSeries::HostF32(v),
    };

    Ok(DynamicIndicatorCudaComputeOutput {
        output_id: out.output_id,
        series,
        warmup: out.warmup,
        rows: out.rows,
        cols: out.cols,
        pattern_ids: out.pattern_ids,
    })
}

#[cfg(feature = "cuda")]
pub fn compute_dynamic_indicator_cuda_direct_with_inputs(
    prepared_inputs: &PreparedCudaIndicatorInputs,
    indicator_id: &str,
    output_id: Option<&str>,
    input_kind: IndicatorInputKind,
    params: &[IndicatorParamKV],
    kernel: Kernel,
    target: vector_ta::indicators::dispatch::CudaOutputTarget,
) -> Result<DynamicIndicatorCudaComputeOutput, DynamicIndicatorAdapterError> {
    #[cfg(test)]
    let _gpu_guard = crate::gpu_backtest_kernel::lock_gpu_test();
    let normalized_id = indicator_id.trim();
    if normalized_id.is_empty() {
        return Err(DynamicIndicatorAdapterError::UnsupportedIndicator {
            indicator_id: indicator_id.to_string(),
        });
    }

    let first_valid = prepared_inputs.first_valid_for_input_kind(input_kind);
    let dispatch_params = dispatch_params_with_first_valid(params, first_valid);
    let data = cuda_data_ref_for_input_kind(prepared_inputs, input_kind);
    let out = match cuda_device_data_ref_for_input_kind(prepared_inputs, input_kind).and_then(
        |device_data| {
            compute_cuda_device(IndicatorCudaDeviceRequest {
                indicator_id: normalized_id,
                output_id,
                data: device_data,
                params: dispatch_params.as_slice(),
                kernel,
                target,
            })
            .map_err(|e| DynamicIndicatorAdapterError::ComputeFailure {
                indicator_id: normalized_id.to_string(),
                details: e.to_string(),
            })
        },
    ) {
        Ok(out) => out,
        Err(_) => compute_cuda(IndicatorCudaRequest {
            indicator_id: normalized_id,
            output_id,
            data,
            params: dispatch_params.as_slice(),
            kernel,
            target,
        })
        .map_err(|e| DynamicIndicatorAdapterError::ComputeFailure {
            indicator_id: normalized_id.to_string(),
            details: e.to_string(),
        })?,
    };

    let series = match out.series {
        IndicatorCudaSeries::DeviceF32(v) => DynamicIndicatorCudaSeries::DeviceF32(v),
        IndicatorCudaSeries::HostF32(v) => DynamicIndicatorCudaSeries::HostF32(v),
    };

    Ok(DynamicIndicatorCudaComputeOutput {
        output_id: out.output_id,
        series,
        warmup: out.warmup,
        rows: out.rows,
        cols: out.cols,
        pattern_ids: out.pattern_ids,
    })
}

#[cfg(feature = "cuda")]
pub fn compute_dynamic_indicator_cuda(
    candles: &Candles,
    ma_source: &str,
    indicator_id: &str,
    output_id: Option<&str>,
    params: &[IndicatorParamKV],
    kernel: Kernel,
    target: vector_ta::indicators::dispatch::CudaOutputTarget,
) -> Result<DynamicIndicatorCudaComputeOutput, DynamicIndicatorAdapterError> {
    let prepared_inputs = PreparedCudaIndicatorInputs::from_candles(candles, ma_source)?;
    compute_dynamic_indicator_cuda_with_inputs(
        &prepared_inputs,
        indicator_id,
        output_id,
        params,
        kernel,
        target,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_trending_candles(len: usize) -> Candles {
        let mut ts = Vec::with_capacity(len);
        let mut open = Vec::with_capacity(len);
        let mut high = Vec::with_capacity(len);
        let mut low = Vec::with_capacity(len);
        let mut close = Vec::with_capacity(len);
        let mut vol: Vec<f64> = Vec::with_capacity(len);

        for i in 0..len {
            ts.push(i as i64);
            let base = 100.0 + i as f64 * 0.05;
            open.push(base);
            high.push(base + 0.75);
            low.push(base - 0.75);
            close.push(base + ((i % 11) as f64 - 5.0) * 0.01);
            vol.push(10_000.0 + ((i % 17) as f64) * 10.0);
        }
        Candles::new(ts, open, high, low, close, vol)
    }

    #[test]
    fn cpu_adapter_computes_single_output_ma_like_indicator() {
        let candles = make_trending_candles(1024);
        let out = compute_dynamic_indicator_cpu(
            &candles,
            "close",
            "sma",
            Some("value"),
            &[IndicatorParamKV {
                key: "period".to_string(),
                value: IndicatorParamValue::Int(20),
            }],
            Kernel::Auto,
        )
        .expect("sma should compute");
        assert_eq!(out.output_id, "value");
        assert_eq!(out.cols, candles.close.len());
        assert!(matches!(out.series, DynamicIndicatorSeries::F64(_)));
    }

    #[test]
    fn cpu_adapter_computes_multi_output_indicator_when_output_selected() {
        let candles = make_trending_candles(1024);
        let out = compute_dynamic_indicator_cpu(
            &candles,
            "close",
            "macd",
            Some("hist"),
            &[],
            Kernel::Auto,
        )
        .expect("macd hist should compute");
        assert_eq!(out.output_id, "hist");
        assert_eq!(out.cols, candles.close.len());
        assert!(matches!(out.series, DynamicIndicatorSeries::F64(_)));
    }

    #[test]
    fn cpu_adapter_supports_candles_required_indicators() {
        let candles = make_trending_candles(1024);
        let out = compute_dynamic_indicator_cpu(
            &candles,
            "close",
            "atr",
            Some("value"),
            &[],
            Kernel::Auto,
        )
        .expect("atr should compute");
        assert_eq!(out.output_id, "value");
        assert_eq!(out.cols, candles.close.len());
        assert!(matches!(out.series, DynamicIndicatorSeries::F64(_)));
    }

    #[test]
    fn cpu_adapter_handles_cpu_only_indicators() {
        let descriptor = crate::indicator_catalog::list_dynamic_indicator_descriptors_cached()
            .iter()
            .find(|d| d.supports_cpu_single && !d.supports_cuda_single && !d.outputs.is_empty())
            .expect("expected at least one CPU-only dynamic indicator");
        let params = descriptor
            .params
            .iter()
            .filter_map(|p| {
                p.default.clone().map(|value| IndicatorParamKV {
                    key: p.key.clone(),
                    value,
                })
            })
            .collect::<Vec<_>>();
        let candles = make_trending_candles(1024);
        let out = compute_dynamic_indicator_cpu(
            &candles,
            "close",
            descriptor.id.as_str(),
            Some(descriptor.outputs[0].id.as_str()),
            params.as_slice(),
            Kernel::Auto,
        )
        .expect("CPU-only indicator should compute via cpu adapter");
        assert_eq!(out.cols, candles.close.len());
    }

    #[cfg(feature = "cuda")]
    fn assert_prepared_cuda_matches_cpu(
        indicator_id: &str,
        output_id: &str,
        params: &[IndicatorParamKV],
    ) {
        if !vector_ta::cuda::cuda_available() {
            return;
        }
        let candles = make_trending_candles(1024);
        let cpu = compute_dynamic_indicator_cpu(
            &candles,
            "close",
            indicator_id,
            Some(output_id),
            params,
            Kernel::Auto,
        )
        .expect("cpu compute should succeed");
        let prepared_inputs = PreparedCudaIndicatorInputs::from_candles(&candles, "close")
            .expect("prepared inputs should upload to device");
        let cuda = compute_dynamic_indicator_cuda_with_inputs(
            &prepared_inputs,
            indicator_id,
            Some(output_id),
            params,
            Kernel::Auto,
            vector_ta::indicators::dispatch::CudaOutputTarget::HostF32,
        )
        .expect("prepared-input cuda compute should succeed");
        match (cpu.series, cuda.series) {
            (DynamicIndicatorSeries::F64(lhs), DynamicIndicatorCudaSeries::HostF32(rhs)) => {
                assert_eq!(lhs.len(), rhs.len());
                for (l, r) in lhs.iter().zip(rhs.iter()) {
                    if l.is_nan() {
                        continue;
                    }
                    assert!((l - (*r as f64)).abs() <= 1e-3);
                }
            }
            _ => panic!("unexpected series type for cpu/cuda comparison"),
        }
    }

    #[cfg(feature = "cuda")]
    fn assert_cuda_direct_matches_typed_cpu(
        indicator_id: &str,
        output_id: &str,
        params: &[IndicatorParamKV],
    ) {
        if !vector_ta::cuda::cuda_available() {
            return;
        }
        let candles = make_trending_candles(1024);
        let prepared_inputs = PreparedCudaIndicatorInputs::from_candles(&candles, "close")
            .expect("prepared inputs should upload to device");
        let cuda = compute_dynamic_indicator_cuda_direct_with_inputs(
            &prepared_inputs,
            indicator_id,
            Some(output_id),
            IndicatorInputKind::Slice,
            params,
            Kernel::Auto,
            vector_ta::indicators::dispatch::CudaOutputTarget::HostF32,
        )
        .expect("direct cuda compute should succeed");

        let cpu = match indicator_id {
            "mama" => {
                let mut builder = vector_ta::indicators::moving_averages::mama::MamaBuilder::new()
                    .kernel(Kernel::Auto);
                for param in params {
                    match param.key.as_str() {
                        "fast_limit" => {
                            if let IndicatorParamValue::Float(v) = param.value {
                                builder = builder.fast_limit(v);
                            }
                        }
                        "slow_limit" => {
                            if let IndicatorParamValue::Float(v) = param.value {
                                builder = builder.slow_limit(v);
                            }
                        }
                        _ => {}
                    }
                }
                let out = builder
                    .apply_slice(source_type(&candles, "close"))
                    .expect("typed cpu mama should succeed");
                if output_id.eq_ignore_ascii_case("fama") {
                    out.fama_values
                } else {
                    out.mama_values
                }
            }
            "decycler" => {
                let mut builder =
                    vector_ta::indicators::decycler::DecyclerBuilder::new().kernel(Kernel::Auto);
                for param in params {
                    match param.key.as_str() {
                        "hp_period" => {
                            if let IndicatorParamValue::Int(v) = param.value {
                                builder = builder.hp_period(v as usize);
                            }
                        }
                        "k" => {
                            if let IndicatorParamValue::Float(v) = param.value {
                                builder = builder.k(v);
                            }
                        }
                        _ => {}
                    }
                }
                builder
                    .apply_slice(source_type(&candles, "close"))
                    .expect("typed cpu decycler should succeed")
                    .values
            }
            _ => panic!("unsupported typed cpu comparison for {indicator_id}"),
        };

        match cuda.series {
            DynamicIndicatorCudaSeries::HostF32(rhs) => {
                assert_eq!(cpu.len(), rhs.len());
                for (l, r) in cpu.iter().zip(rhs.iter()) {
                    if l.is_nan() {
                        continue;
                    }
                    assert!((l - (*r as f64)).abs() <= 1e-3);
                }
            }
            _ => panic!("unexpected direct cuda series type"),
        }
    }

    #[cfg(feature = "cuda")]
    #[test]
    fn cuda_adapter_parity_matches_cpu_for_single_output_indicator() {
        if !vector_ta::cuda::cuda_available() {
            return;
        }
        let candles = make_trending_candles(1024);
        let params = vec![IndicatorParamKV {
            key: "period".to_string(),
            value: IndicatorParamValue::Int(20),
        }];
        let cpu = compute_dynamic_indicator_cpu(
            &candles,
            "close",
            "sma",
            Some("value"),
            params.as_slice(),
            Kernel::Auto,
        )
        .expect("cpu compute should succeed");
        let cuda = compute_dynamic_indicator_cuda(
            &candles,
            "close",
            "sma",
            Some("value"),
            params.as_slice(),
            Kernel::Auto,
            vector_ta::indicators::dispatch::CudaOutputTarget::HostF32,
        )
        .expect("cuda compute should succeed");
        match (cpu.series, cuda.series) {
            (DynamicIndicatorSeries::F64(lhs), DynamicIndicatorCudaSeries::HostF32(rhs)) => {
                assert_eq!(lhs.len(), rhs.len());
                for (l, r) in lhs.iter().zip(rhs.iter()) {
                    if l.is_nan() {
                        continue;
                    }
                    assert!((l - (*r as f64)).abs() <= 1e-3);
                }
            }
            _ => panic!("unexpected series type for sma parity comparison"),
        }
    }

    #[cfg(feature = "cuda")]
    #[test]
    fn cuda_adapter_with_prepared_inputs_matches_standard_cuda_adapter() {
        if !vector_ta::cuda::cuda_available() {
            return;
        }
        let candles = make_trending_candles(1024);
        let params = vec![IndicatorParamKV {
            key: "period".to_string(),
            value: IndicatorParamValue::Int(20),
        }];
        let prepared_inputs = PreparedCudaIndicatorInputs::from_candles(&candles, "close")
            .expect("prepared inputs should upload to device");
        let from_prepared = compute_dynamic_indicator_cuda_with_inputs(
            &prepared_inputs,
            "sma",
            Some("value"),
            params.as_slice(),
            Kernel::Auto,
            vector_ta::indicators::dispatch::CudaOutputTarget::HostF32,
        )
        .expect("cuda compute with prepared inputs should succeed");
        let standard = compute_dynamic_indicator_cuda(
            &candles,
            "close",
            "sma",
            Some("value"),
            params.as_slice(),
            Kernel::Auto,
            vector_ta::indicators::dispatch::CudaOutputTarget::HostF32,
        )
        .expect("standard cuda compute should succeed");

        match (from_prepared.series, standard.series) {
            (
                DynamicIndicatorCudaSeries::HostF32(lhs),
                DynamicIndicatorCudaSeries::HostF32(rhs),
            ) => {
                assert_eq!(lhs.len(), rhs.len());
                for (l, r) in lhs.iter().zip(rhs.iter()) {
                    if l.is_nan() {
                        continue;
                    }
                    assert!((*l - *r).abs() <= 1e-6);
                }
            }
            _ => panic!("expected host f32 output for prepared-input parity test"),
        }
    }

    #[cfg(feature = "cuda")]
    #[test]
    fn cuda_adapter_supports_device_slice_chaining() {
        if !vector_ta::cuda::cuda_available() {
            return;
        }
        let candles = make_trending_candles(1024);
        let prepared_inputs = PreparedCudaIndicatorInputs::from_candles(&candles, "close")
            .expect("prepared inputs should upload to device");
        let sma_params = vec![IndicatorParamKV {
            key: "period".to_string(),
            value: IndicatorParamValue::Int(20),
        }];
        let sma_device = compute_dynamic_indicator_cuda_with_inputs(
            &prepared_inputs,
            "sma",
            Some("value"),
            sma_params.as_slice(),
            Kernel::Auto,
            vector_ta::indicators::dispatch::CudaOutputTarget::DeviceF32,
        )
        .expect("device sma should succeed");
        let sma_matrix = match sma_device.series {
            DynamicIndicatorCudaSeries::DeviceF32(matrix) => matrix,
            _ => panic!("expected device f32 output for chained sma"),
        };

        let ema_params = vec![IndicatorParamKV {
            key: "period".to_string(),
            value: IndicatorParamValue::Int(10),
        }];
        let ema_cuda = compute_dynamic_indicator_cuda_from_device_matrix_row(
            "ema",
            Some("value"),
            &sma_matrix,
            0,
            sma_device.warmup.unwrap_or(0),
            ema_params.as_slice(),
            Kernel::Auto,
            vector_ta::indicators::dispatch::CudaOutputTarget::HostF32,
        )
        .expect("device chained ema should succeed");

        let sma_host = compute_dynamic_indicator_cuda_with_inputs(
            &prepared_inputs,
            "sma",
            Some("value"),
            sma_params.as_slice(),
            Kernel::Auto,
            vector_ta::indicators::dispatch::CudaOutputTarget::HostF32,
        )
        .expect("host sma should succeed");
        let sma_host_values = match sma_host.series {
            DynamicIndicatorCudaSeries::HostF32(values) => values,
            _ => panic!("expected host f32 output for reference sma"),
        };
        let sma_host_f64: Vec<f64> = sma_host_values.iter().map(|v| *v as f64).collect();
        let ema_cpu =
            vector_ta::indicators::ema::ema(&vector_ta::indicators::ema::EmaInput::from_slice(
                sma_host_f64.as_slice(),
                vector_ta::indicators::ema::EmaParams { period: Some(10) },
            ))
            .expect("cpu ema on chained sma should succeed")
            .values;

        match ema_cuda.series {
            DynamicIndicatorCudaSeries::HostF32(rhs) => {
                assert_eq!(ema_cpu.len(), rhs.len());
                for (l, r) in ema_cpu.iter().zip(rhs.iter()) {
                    if l.is_nan() {
                        continue;
                    }
                    assert!((l - (*r as f64)).abs() <= 1e-3);
                }
            }
            _ => panic!("expected host f32 output for chained ema"),
        }
    }

    #[cfg(feature = "cuda")]
    #[test]
    fn cuda_adapter_parity_matches_cpu_for_multi_output_indicator() {
        if !vector_ta::cuda::cuda_available() {
            return;
        }
        let candles = make_trending_candles(1024);
        let cpu = compute_dynamic_indicator_cpu(
            &candles,
            "close",
            "macd",
            Some("hist"),
            &[],
            Kernel::Auto,
        )
        .expect("cpu macd hist should succeed");
        let cuda = compute_dynamic_indicator_cuda(
            &candles,
            "close",
            "macd",
            Some("hist"),
            &[],
            Kernel::Auto,
            vector_ta::indicators::dispatch::CudaOutputTarget::HostF32,
        )
        .expect("cuda macd hist should succeed");
        match (cpu.series, cuda.series) {
            (DynamicIndicatorSeries::F64(lhs), DynamicIndicatorCudaSeries::HostF32(rhs)) => {
                assert_eq!(lhs.len(), rhs.len());
                for (l, r) in lhs.iter().zip(rhs.iter()) {
                    if l.is_nan() {
                        continue;
                    }
                    assert!((l - (*r as f64)).abs() <= 1e-3);
                }
            }
            _ => panic!("unexpected series type for macd parity comparison"),
        }
    }

    #[cfg(feature = "cuda")]
    #[test]
    fn cuda_adapter_supports_qqe_fast_and_slow_outputs_with_prepared_inputs() {
        let params = vec![
            IndicatorParamKV {
                key: "rsi_period".to_string(),
                value: IndicatorParamValue::Int(14),
            },
            IndicatorParamKV {
                key: "smoothing_factor".to_string(),
                value: IndicatorParamValue::Int(5),
            },
            IndicatorParamKV {
                key: "fast_factor".to_string(),
                value: IndicatorParamValue::Float(4.236),
            },
        ];
        assert_prepared_cuda_matches_cpu("qqe", "fast", params.as_slice());
        assert_prepared_cuda_matches_cpu("qqe", "slow", params.as_slice());
    }

    #[cfg(feature = "cuda")]
    #[test]
    fn cuda_adapter_supports_mama_and_fama_outputs_with_prepared_inputs() {
        let params = vec![
            IndicatorParamKV {
                key: "fast_limit".to_string(),
                value: IndicatorParamValue::Float(0.5),
            },
            IndicatorParamKV {
                key: "slow_limit".to_string(),
                value: IndicatorParamValue::Float(0.05),
            },
        ];
        assert_cuda_direct_matches_typed_cpu("mama", "mama", params.as_slice());
        assert_cuda_direct_matches_typed_cpu("mama", "fama", params.as_slice());
    }

    #[cfg(feature = "cuda")]
    #[test]
    fn cuda_adapter_supports_decycler_with_prepared_inputs() {
        let params = vec![
            IndicatorParamKV {
                key: "hp_period".to_string(),
                value: IndicatorParamValue::Int(20),
            },
            IndicatorParamKV {
                key: "k".to_string(),
                value: IndicatorParamValue::Float(0.707),
            },
        ];
        assert_cuda_direct_matches_typed_cpu("decycler", "value", params.as_slice());
    }
}
