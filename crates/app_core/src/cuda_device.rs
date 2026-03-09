#[derive(Clone, Debug)]
pub struct PreferredCudaDevice {
    pub device_id: u32,
    pub name: String,
    pub total_memory_bytes: u64,
}

#[cfg(feature = "cuda")]
pub fn preferred_cuda_device() -> Result<Option<PreferredCudaDevice>, String> {
    if !vector_ta::cuda::cuda_available() {
        return Ok(None);
    }

    use cust::{device::Device, prelude::CudaFlags};

    cust::init(CudaFlags::empty()).map_err(|e| e.to_string())?;

    let count = Device::num_devices().map_err(|e| e.to_string())?;
    let mut best: Option<PreferredCudaDevice> = None;
    for idx in 0..count {
        let device = Device::get_device(idx).map_err(|e| e.to_string())?;
        let total_memory_bytes = device.total_memory().map_err(|e| e.to_string())? as u64;
        let replace = best
            .as_ref()
            .map(|current| total_memory_bytes > current.total_memory_bytes)
            .unwrap_or(true);
        if replace {
            best = Some(PreferredCudaDevice {
                device_id: idx as u32,
                name: device.name().map_err(|e| e.to_string())?,
                total_memory_bytes,
            });
        }
    }
    Ok(best)
}

#[cfg(not(feature = "cuda"))]
pub fn preferred_cuda_device() -> Result<Option<PreferredCudaDevice>, String> {
    Ok(None)
}

pub fn preferred_cuda_device_id() -> Option<u32> {
    preferred_cuda_device()
        .ok()
        .flatten()
        .map(|device| device.device_id)
}
