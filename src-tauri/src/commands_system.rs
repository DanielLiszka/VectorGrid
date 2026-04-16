use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct GpuStatus {
    pub built_with_cuda: bool,
    pub built_with_cuda_backtest_kernel: bool,
    pub cuda_available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_device_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_device_memory_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_device_error: Option<String>,
}

#[cfg(feature = "cuda")]
fn cuda_available_now() -> bool {
    vector_ta::cuda::cuda_available()
}

#[cfg(not(feature = "cuda"))]
fn cuda_available_now() -> bool {
    false
}

fn normalized_window_effect(effect: Option<String>) -> String {
    effect
        .unwrap_or_else(|| "none".to_string())
        .trim()
        .to_lowercase()
}

#[cfg(target_os = "windows")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowsWindowEffect {
    None,
    Mica,
    Acrylic,
    Blur,
}

#[cfg(target_os = "windows")]
fn parse_windows_window_effect(effect: &str) -> Result<WindowsWindowEffect, String> {
    match effect {
        "" | "none" => Ok(WindowsWindowEffect::None),
        "mica" => Ok(WindowsWindowEffect::Mica),
        "acrylic" => Ok(WindowsWindowEffect::Acrylic),
        "blur" => Ok(WindowsWindowEffect::Blur),
        _ => Err(format!("Unknown window effect: {effect}")),
    }
}

#[cfg(target_os = "macos")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MacosWindowEffect {
    None,
    Vibrancy,
}

#[cfg(target_os = "macos")]
fn parse_macos_window_effect(effect: &str) -> Result<MacosWindowEffect, String> {
    match effect {
        "" | "none" => Ok(MacosWindowEffect::None),
        "vibrancy" => Ok(MacosWindowEffect::Vibrancy),
        _ => Err(format!("Unknown window effect: {effect}")),
    }
}

#[tauri::command]
pub fn get_gpu_status() -> Result<GpuStatus, String> {
    let (auto_device_name, auto_device_memory_bytes, auto_device_error) =
        match ta_app_core::preferred_cuda_device() {
            Ok(Some(device)) => (Some(device.name), Some(device.total_memory_bytes), None),
            Ok(None) => (None, None, None),
            Err(error) => (None, None, Some(error)),
        };
    Ok(GpuStatus {
        built_with_cuda: cfg!(feature = "cuda"),
        built_with_cuda_backtest_kernel: cfg!(feature = "cuda-backtest-kernel"),
        cuda_available: cuda_available_now(),
        auto_device_name,
        auto_device_memory_bytes,
        auto_device_error,
    })
}

#[tauri::command]
pub fn exit_application(app: tauri::AppHandle) -> Result<(), String> {
    app.exit(0);
    Ok(())
}

#[tauri::command]
pub fn set_window_effect(
    effect: Option<String>,
    dark: Option<bool>,
    window: tauri::WebviewWindow,
) -> Result<(), String> {
    let eff = normalized_window_effect(effect);

    #[cfg(target_os = "windows")]
    {
        use window_vibrancy::{
            apply_acrylic, apply_blur, apply_mica, clear_acrylic, clear_blur, clear_mica,
        };
        let planned_effect = parse_windows_window_effect(&eff)?;

        let _ = clear_mica(&window);
        let _ = clear_acrylic(&window);
        let _ = clear_blur(&window);

        let tint = match dark {
            Some(true) => Some((18u8, 18u8, 18u8, 180u8)),
            Some(false) => Some((245u8, 245u8, 245u8, 180u8)),
            None => None,
        };

        match planned_effect {
            WindowsWindowEffect::None => Ok(()),
            WindowsWindowEffect::Mica => apply_mica(&window, dark).map_err(|e| e.to_string()),
            WindowsWindowEffect::Acrylic => apply_acrylic(&window, tint).map_err(|e| e.to_string()),
            WindowsWindowEffect::Blur => apply_blur(&window, tint).map_err(|e| e.to_string()),
        }?;
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        use window_vibrancy::{
            apply_vibrancy, clear_vibrancy, NSVisualEffectBlendingMode, NSVisualEffectMaterial,
            NSVisualEffectState,
        };
        let planned_effect = parse_macos_window_effect(&eff)?;

        let _ = clear_vibrancy(&window);
        match planned_effect {
            MacosWindowEffect::None => Ok(()),
            MacosWindowEffect::Vibrancy => apply_vibrancy(
                &window,
                NSVisualEffectMaterial::Sidebar,
                NSVisualEffectBlendingMode::BehindWindow,
                NSVisualEffectState::Active,
            )
            .map_err(|e| e.to_string()),
        }?;
        return Ok(());
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let _ = (eff, dark, window);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{get_gpu_status, normalized_window_effect};

    #[test]
    fn get_gpu_status_reports_consistent_diagnostics_shape() {
        let status = get_gpu_status().expect("gpu diagnostics should not fail");

        assert_eq!(status.built_with_cuda, cfg!(feature = "cuda"));
        assert_eq!(
            status.built_with_cuda_backtest_kernel,
            cfg!(feature = "cuda-backtest-kernel")
        );

        if !status.built_with_cuda {
            assert!(!status.cuda_available);
        }

        match (
            status.auto_device_name.as_ref(),
            status.auto_device_memory_bytes,
            status.auto_device_error.as_ref(),
        ) {
            (Some(_), Some(memory_bytes), None) => {
                assert!(memory_bytes > 0);
            }
            (None, None, Some(error)) => {
                assert!(!error.trim().is_empty());
            }
            (None, None, None) => {}
            other => panic!("unexpected gpu diagnostics shape: {:?}", other),
        }
    }

    #[test]
    fn normalized_window_effect_trims_and_lowercases() {
        assert_eq!(
            normalized_window_effect(Some("  Mica  ".to_string())),
            "mica"
        );
        assert_eq!(normalized_window_effect(None), "none");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn parse_windows_window_effect_rejects_unknown_values_before_side_effects() {
        use super::{parse_windows_window_effect, WindowsWindowEffect};

        assert_eq!(
            parse_windows_window_effect("mica").expect("mica should parse"),
            WindowsWindowEffect::Mica
        );
        assert!(parse_windows_window_effect("glass").is_err());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn parse_macos_window_effect_rejects_unknown_values_before_side_effects() {
        use super::{parse_macos_window_effect, MacosWindowEffect};

        assert_eq!(
            parse_macos_window_effect("vibrancy").expect("vibrancy should parse"),
            MacosWindowEffect::Vibrancy
        );
        assert!(parse_macos_window_effect("glass").is_err());
    }
}
