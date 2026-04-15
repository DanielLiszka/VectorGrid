//! Preferred CUDA device selection and sticky process-lifetime caches.
//!
//! This module intentionally exposes two independent cached views of the
//! preferred CUDA device on CUDA builds:
//!
//! - the preferred device id via [`preferred_cuda_device_id_result`]
//! - the preferred device metadata via [`preferred_cuda_device`]
//!
//! Each cache is sticky for the process lifetime. On CPU-only builds both
//! public result-returning helpers resolve to `Ok(None)`.

#[cfg(feature = "cuda")]
use std::sync::OnceLock;

/// Metadata for the currently selected preferred CUDA device.
#[derive(Clone, Debug)]
pub struct PreferredCudaDevice {
    pub device_id: u32,
    pub name: String,
    pub total_memory_bytes: u64,
}

#[cfg(feature = "cuda")]
fn select_preferred_cuda_device_id_by_total_memory<I>(devices: I) -> Option<u32>
where
    I: IntoIterator<Item = (u32, u64)>,
{
    let mut best_device_id: Option<u32> = None;
    let mut best_total_memory_bytes = 0u64;
    for (device_id, total_memory_bytes) in devices {
        if best_device_id.is_none() || total_memory_bytes > best_total_memory_bytes {
            best_device_id = Some(device_id);
            best_total_memory_bytes = total_memory_bytes;
        }
    }
    best_device_id
}

#[cfg(feature = "cuda")]
fn resolve_preferred_cuda_device_id() -> Result<Option<u32>, String> {
    static PREFERRED_CUDA_DEVICE_ID: OnceLock<Result<Option<u32>, String>> = OnceLock::new();
    PREFERRED_CUDA_DEVICE_ID
        .get_or_init(|| {
            use cust::{device::Device, prelude::CudaFlags};

            cust::init(CudaFlags::empty()).map_err(|e| e.to_string())?;

            let count = Device::num_devices().map_err(|e| e.to_string())?;
            if count == 0 {
                return Ok(None);
            }
            if count == 1 {
                return Ok(Some(0));
            }

            let devices = (0..count)
                .map(|idx| {
                    let device = Device::get_device(idx).map_err(|e| e.to_string())?;
                    let total_memory_bytes = device.total_memory().map_err(|e| e.to_string())?;
                    Ok((idx as u32, total_memory_bytes as u64))
                })
                .collect::<Result<Vec<_>, String>>()?;
            Ok(select_preferred_cuda_device_id_by_total_memory(devices))
        })
        .clone()
}

#[cfg(feature = "cuda")]
/// Returns the cached preferred CUDA device id.
///
/// The first successful or failed probe result is cached for the lifetime of
/// the current process. The current selection policy is:
///
/// - `Ok(None)` when CUDA enumerates no devices
/// - `Ok(Some(0))` when exactly one device is visible
/// - otherwise the visible device with the largest total VRAM wins, with ties
///   broken by first enumeration order
pub fn preferred_cuda_device_id_result() -> Result<Option<u32>, String> {
    resolve_preferred_cuda_device_id()
}

#[cfg(feature = "cuda")]
/// Returns cached metadata for the preferred CUDA device.
///
/// This metadata cache is independent from the device-id cache used by
/// [`preferred_cuda_device_id_result`], but both caches are sticky for the
/// lifetime of the current process.
pub fn preferred_cuda_device() -> Result<Option<PreferredCudaDevice>, String> {
    static PREFERRED_CUDA_DEVICE: OnceLock<Result<Option<PreferredCudaDevice>, String>> =
        OnceLock::new();
    PREFERRED_CUDA_DEVICE
        .get_or_init(|| {
            use cust::{device::Device, prelude::CudaFlags};

            let Some(device_id) = resolve_preferred_cuda_device_id()? else {
                return Ok(None);
            };

            cust::init(CudaFlags::empty()).map_err(|e| e.to_string())?;
            let device = Device::get_device(device_id).map_err(|e| e.to_string())?;
            Ok(Some(PreferredCudaDevice {
                device_id,
                name: device.name().map_err(|e| e.to_string())?,
                total_memory_bytes: device.total_memory().map_err(|e| e.to_string())? as u64,
            }))
        })
        .clone()
}

#[cfg(not(feature = "cuda"))]
/// Returns `Ok(None)` on CPU-only builds.
pub fn preferred_cuda_device() -> Result<Option<PreferredCudaDevice>, String> {
    Ok(None)
}

#[cfg(not(feature = "cuda"))]
/// Returns `Ok(None)` on CPU-only builds.
pub fn preferred_cuda_device_id_result() -> Result<Option<u32>, String> {
    Ok(None)
}

/// Lossy convenience wrapper over [`preferred_cuda_device_id_result`].
///
/// This helper collapses both probe failures and the "no visible device" case
/// into `None`, so callers that need to differentiate those outcomes should use
/// the result-returning API instead.
pub fn preferred_cuda_device_id() -> Option<u32> {
    preferred_cuda_device_id_result().ok().flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "cuda")]
    #[test]
    fn preferred_cuda_device_selection_prefers_largest_total_memory() {
        assert_eq!(
            select_preferred_cuda_device_id_by_total_memory([(0, 8), (1, 24), (2, 16)]),
            Some(1)
        );
    }

    #[cfg(feature = "cuda")]
    #[test]
    fn preferred_cuda_device_selection_breaks_ties_by_first_enumerated_device() {
        assert_eq!(
            select_preferred_cuda_device_id_by_total_memory([(4, 12), (7, 12), (9, 10)]),
            Some(4)
        );
    }

    #[cfg(not(feature = "cuda"))]
    #[test]
    fn preferred_cuda_device_stubs_return_none_without_cuda() {
        assert_eq!(preferred_cuda_device_id_result().expect("cpu build"), None);
        assert_eq!(preferred_cuda_device_id(), None);
        assert!(preferred_cuda_device().expect("cpu build").is_none());
    }
}
