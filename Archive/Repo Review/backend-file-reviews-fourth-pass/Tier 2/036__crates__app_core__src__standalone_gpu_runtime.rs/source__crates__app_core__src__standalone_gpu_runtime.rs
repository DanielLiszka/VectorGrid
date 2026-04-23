#![cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]

use crate::alloc_policy::{alloc_uninit_copy, resize_uninit_f32};
use cust::memory::{CopyDestination, DeviceBuffer, DeviceCopy};
use cust::stream::Stream;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct CloseSeriesKey {
    pub(crate) len: usize,
    pub(crate) host_ptr: usize,
    pub(crate) device_ptr: u64,
}

impl CloseSeriesKey {
    pub(crate) fn from_host_and_device(close_f32: &[f32], close_device_ptr: u64) -> Self {
        Self {
            len: close_f32.len(),
            host_ptr: close_f32.as_ptr() as usize,
            device_ptr: close_device_ptr,
        }
    }
}

pub(crate) struct ResidentCloseReturns {
    pub(crate) key: CloseSeriesKey,
    pub(crate) close_ptr: u64,
    pub(crate) d_close_owner: Option<DeviceBuffer<f32>>,
    pub(crate) d_lr: DeviceBuffer<f32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct UploadSignature {
    len: usize,
    hash: u64,
}

#[inline]
pub(crate) fn checked_i32(label: &str, value: usize) -> Result<i32, String> {
    i32::try_from(value).map_err(|_| format!("{label} exceeds i32 range"))
}

#[inline]
pub(crate) fn slice_signature<T: Hash>(values: &[T]) -> UploadSignature {
    let mut hasher = DefaultHasher::new();
    values.hash(&mut hasher);
    UploadSignature {
        len: values.len(),
        hash: hasher.finish(),
    }
}

pub(crate) fn ensure_device_buffer_len<T: DeviceCopy>(
    buf: &mut Option<DeviceBuffer<T>>,
    required: usize,
    stream: &Stream,
) -> Result<(), String> {
    let needs_alloc = buf
        .as_ref()
        .map(|current| current.len() < required)
        .unwrap_or(true);
    if needs_alloc {
        if let Some(old) = buf.take() {
            old.drop_async(stream).map_err(|e| e.to_string())?;
        }
        *buf =
            Some(unsafe { DeviceBuffer::<T>::uninitialized(required) }.map_err(|e| e.to_string())?);
    }
    Ok(())
}

pub(crate) fn copy_host_to_device_prefix<T: DeviceCopy>(
    buf: &mut DeviceBuffer<T>,
    values: &[T],
) -> Result<(), String> {
    let mut prefix = buf.index(..values.len());
    prefix.copy_from(values).map_err(|e| e.to_string())
}

pub(crate) fn copy_device_to_host_prefix<T: DeviceCopy>(
    buf: &DeviceBuffer<T>,
    values: &mut [T],
) -> Result<(), String> {
    let prefix = buf.index(..values.len());
    prefix.copy_to(values).map_err(|e| e.to_string())
}

pub(crate) fn ensure_host_f32_len(buf: &mut Vec<f32>, required: usize) {
    if buf.capacity() < required {
        *buf = alloc_uninit_copy(required);
    } else {
        resize_uninit_f32(buf, required);
    }
}
