use serde_json::Value;
use tauri::Manager;

pub(crate) fn quantile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    if sorted.len() == 1 {
        return sorted[0];
    }
    let qq = q.clamp(0.0, 1.0);
    let idx = (qq * (sorted.len().saturating_sub(1) as f64)).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

pub(crate) fn reveal_path_blocking(path: String) -> Result<(), String> {
    use std::path::PathBuf;
    use std::process::Command;

    let path = PathBuf::from(path);
    if !path.exists() {
        return Err(format!("path does not exist: {}", path.to_string_lossy()));
    }

    #[cfg(target_os = "windows")]
    {
        if path.is_dir() {
            Command::new("explorer")
                .arg(path.as_os_str())
                .spawn()
                .map_err(|e| e.to_string())?;
        } else {
            let arg = format!("/select,\"{}\"", path.to_string_lossy());
            Command::new("explorer")
                .arg(arg)
                .spawn()
                .map_err(|e| e.to_string())?;
        }
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        if path.is_dir() {
            Command::new("open")
                .arg(path.as_os_str())
                .spawn()
                .map_err(|e| e.to_string())?;
        } else {
            Command::new("open")
                .arg("-R")
                .arg(path.as_os_str())
                .spawn()
                .map_err(|e| e.to_string())?;
        }
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    {
        let target = if path.is_dir() {
            path
        } else {
            path.parent()
                .unwrap_or_else(|| std::path::Path::new("."))
                .to_path_buf()
        };
        Command::new("xdg-open")
            .arg(target.as_os_str())
            .spawn()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }

    #[allow(unreachable_code)]
    Ok(())
}

pub(crate) fn open_path_blocking(path: String) -> Result<(), String> {
    use std::path::PathBuf;
    use std::process::Command;

    let path = PathBuf::from(path);
    if !path.exists() {
        return Err(format!("path does not exist: {}", path.to_string_lossy()));
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .arg("/C")
            .arg("start")
            .arg("")
            .arg(path.as_os_str())
            .spawn()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(path.as_os_str())
            .spawn()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(path.as_os_str())
            .spawn()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }

    #[allow(unreachable_code)]
    Ok(())
}

pub(crate) fn sanitize_filename_component(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' => out.push(ch),
            ':' => out.push('-'),
            _ => out.push('_'),
        }
    }
    if out.is_empty() {
        "unknown".to_string()
    } else {
        // Avoid excessively long filenames on Windows.
        out.chars().take(96).collect()
    }
}

pub(super) fn suggest_report_path(
    report: &Value,
    app: &tauri::AppHandle,
) -> Result<std::path::PathBuf, String> {
    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let dir = base.join("reports");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let ts = report
        .get("ts")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let safe_ts = sanitize_filename_component(ts);

    let base_name = format!("vectorgrid_report_{safe_ts}");
    let mut path = dir.join(format!("{base_name}.xlsx"));
    if !path.exists() {
        return Ok(path);
    }
    for i in 2..=999u32 {
        let p = dir.join(format!("{base_name}_{i}.xlsx"));
        if !p.exists() {
            path = p;
            break;
        }
    }
    Ok(path)
}
