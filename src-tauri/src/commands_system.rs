use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct GpuStatus {
    pub built_with_cuda: bool,
    pub built_with_cuda_backtest_kernel: bool,
    pub cuda_available: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CudaDeviceInfo {
    pub device_id: u32,
    pub name: String,
    pub total_memory_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ListCudaDevicesResponse {
    pub built_with_cuda: bool,
    pub cuda_available: bool,
    pub devices: Vec<CudaDeviceInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommended_device_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[cfg(feature = "cuda")]
fn cuda_available_now() -> bool {
    vector_ta::cuda::cuda_available()
}

#[cfg(not(feature = "cuda"))]
fn cuda_available_now() -> bool {
    false
}

#[tauri::command]
pub fn get_gpu_status() -> Result<GpuStatus, String> {
    Ok(GpuStatus {
        built_with_cuda: cfg!(feature = "cuda"),
        built_with_cuda_backtest_kernel: cfg!(feature = "cuda-backtest-kernel"),
        cuda_available: cuda_available_now(),
    })
}

#[cfg(feature = "cuda")]
fn list_cuda_devices_impl() -> Result<ListCudaDevicesResponse, String> {
    if !cuda_available_now() {
        return Ok(ListCudaDevicesResponse {
            built_with_cuda: true,
            cuda_available: false,
            devices: Vec::new(),
            recommended_device_id: None,
            error: Some("CUDA runtime not available (cuda_available() == false)".to_string()),
        });
    }

    use cust::{device::Device, prelude::CudaFlags};

    cust::init(CudaFlags::empty()).map_err(|e| e.to_string())?;

    let n = Device::num_devices().map_err(|e| e.to_string())?;
    let mut devices: Vec<CudaDeviceInfo> = Vec::with_capacity(n as usize);
    for i in 0..n {
        let dev = Device::get_device(i).map_err(|e| e.to_string())?;
        let name = dev.name().map_err(|e| e.to_string())?;
        let mem = dev.total_memory().map_err(|e| e.to_string())?;
        devices.push(CudaDeviceInfo {
            device_id: i as u32,
            name,
            total_memory_bytes: mem as u64,
        });
    }

    let mut recommended: Option<u32> = None;
    let mut best_mem: u64 = 0;
    for d in &devices {
        if recommended.is_none() || d.total_memory_bytes > best_mem {
            recommended = Some(d.device_id);
            best_mem = d.total_memory_bytes;
        }
    }

    Ok(ListCudaDevicesResponse {
        built_with_cuda: true,
        cuda_available: true,
        devices,
        recommended_device_id: recommended,
        error: None,
    })
}

#[cfg(not(feature = "cuda"))]
fn list_cuda_devices_impl() -> Result<ListCudaDevicesResponse, String> {
    Ok(ListCudaDevicesResponse {
        built_with_cuda: false,
        cuda_available: false,
        devices: Vec::new(),
        recommended_device_id: None,
        error: Some("App not built with CUDA feature".to_string()),
    })
}

#[tauri::command]
pub fn list_cuda_devices() -> Result<ListCudaDevicesResponse, String> {
    list_cuda_devices_impl()
}

#[tauri::command]
pub fn set_window_effect(
    effect: Option<String>,
    dark: Option<bool>,
    window: tauri::WebviewWindow,
) -> Result<(), String> {
    let eff = effect
        .unwrap_or_else(|| "none".to_string())
        .trim()
        .to_lowercase();

    #[cfg(target_os = "windows")]
    {
        use window_vibrancy::{
            apply_acrylic, apply_blur, apply_mica, clear_acrylic, clear_blur, clear_mica,
        };

        let _ = clear_mica(&window);
        let _ = clear_acrylic(&window);
        let _ = clear_blur(&window);

        let tint = match dark {
            Some(true) => Some((18u8, 18u8, 18u8, 180u8)),
            Some(false) => Some((245u8, 245u8, 245u8, 180u8)),
            None => None,
        };

        match eff.as_str() {
            "" | "none" => Ok(()),
            "mica" => apply_mica(&window, dark).map_err(|e| e.to_string()),
            "acrylic" => apply_acrylic(&window, tint).map_err(|e| e.to_string()),
            "blur" => apply_blur(&window, tint).map_err(|e| e.to_string()),
            _ => Err(format!("Unknown window effect: {eff}")),
        }?;
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        use window_vibrancy::{
            apply_vibrancy, clear_vibrancy, NSVisualEffectBlendingMode, NSVisualEffectMaterial,
            NSVisualEffectState,
        };

        let _ = clear_vibrancy(&window);
        match eff.as_str() {
            "" | "none" => Ok(()),
            "vibrancy" => apply_vibrancy(
                &window,
                NSVisualEffectMaterial::Sidebar,
                NSVisualEffectBlendingMode::BehindWindow,
                NSVisualEffectState::Active,
            )
            .map_err(|e| e.to_string()),
            _ => Err(format!("Unknown window effect: {eff}")),
        }?;
        return Ok(());
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let _ = (eff, dark, window);
        Ok(())
    }
}
