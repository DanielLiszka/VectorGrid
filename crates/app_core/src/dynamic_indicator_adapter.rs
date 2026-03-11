use crate::indicator_catalog::{
    find_dynamic_indicator_descriptor, normalize_and_validate_indicator_output_ref,
    normalize_and_validate_indicator_params, IndicatorInputKind, IndicatorParamKV,
    IndicatorParamValue,
};
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
#[derive(Debug)]
pub struct PreparedCudaIndicatorInputs {
    source_kind: PreparedCudaSourceKind,
    source_override: Option<Vec<f32>>,
    open: Vec<f32>,
    high: Vec<f32>,
    low: Vec<f32>,
    close: Vec<f32>,
    volume: Vec<f32>,
    runtime: CudaRuntime,
    d_source_override: Option<CudaDeviceVectorF32>,
    d_open: CudaDeviceVectorF32,
    d_high: CudaDeviceVectorF32,
    d_low: CudaDeviceVectorF32,
    d_close: CudaDeviceVectorF32,
    d_volume: CudaDeviceVectorF32,
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
        let open: Vec<f32> = candles.open.iter().map(|v| *v as f32).collect();
        let high: Vec<f32> = candles.high.iter().map(|v| *v as f32).collect();
        let low: Vec<f32> = candles.low.iter().map(|v| *v as f32).collect();
        let close: Vec<f32> = candles.close.iter().map(|v| *v as f32).collect();
        let volume: Vec<f32> = candles.volume.iter().map(|v| *v as f32).collect();

        let source_kind = match ma_source.trim().to_ascii_lowercase().as_str() {
            "open" => PreparedCudaSourceKind::Open,
            "high" => PreparedCudaSourceKind::High,
            "low" => PreparedCudaSourceKind::Low,
            "close" => PreparedCudaSourceKind::Close,
            "volume" => PreparedCudaSourceKind::Volume,
            _ => PreparedCudaSourceKind::Derived,
        };

        let source_override = if source_kind == PreparedCudaSourceKind::Derived {
            Some(
                source_type(candles, ma_source)
                    .iter()
                    .map(|v| *v as f32)
                    .collect::<Vec<f32>>(),
            )
        } else {
            None
        };

        let runtime = CudaRuntime::new(device_id).map_err(|e| {
            DynamicIndicatorAdapterError::CudaPreparation {
                details: e.to_string(),
            }
        })?;
        let d_open = runtime.upload_f32(open.as_slice()).map_err(|e| {
            DynamicIndicatorAdapterError::CudaPreparation {
                details: e.to_string(),
            }
        })?;
        let d_high = runtime.upload_f32(high.as_slice()).map_err(|e| {
            DynamicIndicatorAdapterError::CudaPreparation {
                details: e.to_string(),
            }
        })?;
        let d_low = runtime.upload_f32(low.as_slice()).map_err(|e| {
            DynamicIndicatorAdapterError::CudaPreparation {
                details: e.to_string(),
            }
        })?;
        let d_close = runtime.upload_f32(close.as_slice()).map_err(|e| {
            DynamicIndicatorAdapterError::CudaPreparation {
                details: e.to_string(),
            }
        })?;
        let d_volume = runtime.upload_f32(volume.as_slice()).map_err(|e| {
            DynamicIndicatorAdapterError::CudaPreparation {
                details: e.to_string(),
            }
        })?;
        let d_source_override = match source_override.as_ref() {
            Some(values) => Some(runtime.upload_f32(values.as_slice()).map_err(|e| {
                DynamicIndicatorAdapterError::CudaPreparation {
                    details: e.to_string(),
                }
            })?),
            None => None,
        };

        Ok(Self {
            source_kind,
            source_override,
            open,
            high,
            low,
            close,
            volume,
            runtime,
            d_source_override,
            d_open,
            d_high,
            d_low,
            d_close,
            d_volume,
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
        &self.runtime
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
}
