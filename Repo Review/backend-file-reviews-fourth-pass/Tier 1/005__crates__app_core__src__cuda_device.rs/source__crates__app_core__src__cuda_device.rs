#[cfg(feature = "cuda")]
use std::sync::OnceLock;

#[derive(Clone, Debug)]
pub struct PreferredCudaDevice {
    pub device_id: u32,
    pub name: String,
    pub total_memory_bytes: u64,
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

            let mut best_device_id: Option<u32> = None;
            let mut best_total_memory_bytes = 0u64;
            for idx in 0..count {
                let device = Device::get_device(idx).map_err(|e| e.to_string())?;
                let total_memory_bytes = device.total_memory().map_err(|e| e.to_string())? as u64;
                if best_device_id.is_none() || total_memory_bytes > best_total_memory_bytes {
                    best_device_id = Some(idx as u32);
                    best_total_memory_bytes = total_memory_bytes;
                }
            }
            Ok(best_device_id)
        })
        .clone()
}

#[cfg(feature = "cuda")]
pub fn preferred_cuda_device_id_result() -> Result<Option<u32>, String> {
    resolve_preferred_cuda_device_id()
}

#[cfg(feature = "cuda")]
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
pub fn preferred_cuda_device() -> Result<Option<PreferredCudaDevice>, String> {
    Ok(None)
}

#[cfg(not(feature = "cuda"))]
pub fn preferred_cuda_device_id_result() -> Result<Option<u32>, String> {
    Ok(None)
}

pub fn preferred_cuda_device_id() -> Option<u32> {
    preferred_cuda_device_id_result().ok().flatten()
}
