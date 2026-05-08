use std::path::{Path, PathBuf};
use tauri::Manager;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellOpenPurpose {
    GenericOpen,
    GenericReveal,
    ReportOpen,
    RunArtifactReveal,
    MarketDataFolderOpen,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedLocalPath {
    pub canonical: PathBuf,
    pub original: PathBuf,
}

pub fn canonical_existing_path(raw: &str) -> Result<ValidatedLocalPath, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("path must not be empty".to_string());
    }
    if looks_url_like(trimmed) {
        return Err("path must be a local filesystem path, not a URL".to_string());
    }

    let original = PathBuf::from(trimmed);
    let canonical = original
        .canonicalize()
        .map_err(|_| format!("path does not exist: {}", original.to_string_lossy()))?;
    Ok(ValidatedLocalPath {
        canonical,
        original,
    })
}

pub fn validate_shell_open_path<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    raw: &str,
    purpose: ShellOpenPurpose,
) -> Result<PathBuf, String> {
    let validated = canonical_existing_path(raw)?;
    let roots = allowed_shell_roots(app, purpose)?;
    if roots
        .iter()
        .any(|root| validated.canonical == *root || validated.canonical.starts_with(root))
    {
        validate_shell_open_shape(&validated.canonical, purpose)?;
        return Ok(validated.canonical);
    }

    Err(format!(
        "path is outside allowed VectorGrid roots for {:?}: {}",
        purpose,
        validated.original.to_string_lossy()
    ))
}

pub fn ensure_csv_extension(path: &Path) -> Result<(), String> {
    ensure_extension(path, "csv")
}

pub fn ensure_xlsx_extension(path: &Path) -> Result<(), String> {
    ensure_extension(path, "xlsx")
}

pub fn ensure_json_extension(path: &Path) -> Result<(), String> {
    ensure_extension(path, "json")
}

fn ensure_extension(path: &Path, expected: &str) -> Result<(), String> {
    let actual = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if actual.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        Err(format!(
            "path must use .{expected} extension: {}",
            path.to_string_lossy()
        ))
    }
}

fn allowed_shell_roots<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    purpose: ShellOpenPurpose,
) -> Result<Vec<PathBuf>, String> {
    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let reports = base.join("reports");
    let runs = base.join("runs");
    let data_cache = crate::data_cache::app_data_cache_dir(&base);
    let market_root = crate::settings::resolve_market_data_root_dir(&base)?;
    let polygon_cache = crate::polygon::polygon_cache_dir(&market_root);

    let candidates = match purpose {
        ShellOpenPurpose::GenericOpen | ShellOpenPurpose::GenericReveal => {
            vec![reports, runs, data_cache]
        }
        ShellOpenPurpose::ReportOpen => vec![reports],
        ShellOpenPurpose::RunArtifactReveal => vec![runs],
        ShellOpenPurpose::MarketDataFolderOpen => vec![market_root, polygon_cache],
    };

    Ok(canonical_existing_roots(candidates))
}

fn validate_shell_open_shape(path: &Path, purpose: ShellOpenPurpose) -> Result<(), String> {
    match purpose {
        ShellOpenPurpose::GenericOpen | ShellOpenPurpose::GenericReveal => {
            ensure_regular_file(path, purpose)?;
            ensure_allowed_generic_open_extension(path)
        }
        ShellOpenPurpose::ReportOpen => {
            ensure_regular_file(path, purpose)?;
            ensure_xlsx_extension(path)
        }
        ShellOpenPurpose::RunArtifactReveal => {
            ensure_regular_file(path, purpose)?;
            ensure_json_extension(path)
        }
        ShellOpenPurpose::MarketDataFolderOpen => ensure_directory(path, purpose),
    }
}

fn ensure_regular_file(path: &Path, purpose: ShellOpenPurpose) -> Result<(), String> {
    let metadata = std::fs::metadata(path).map_err(|e| e.to_string())?;
    if metadata.is_file() {
        Ok(())
    } else {
        Err(format!(
            "path must be a file for {:?}: {}",
            purpose,
            path.to_string_lossy()
        ))
    }
}

fn ensure_directory(path: &Path, purpose: ShellOpenPurpose) -> Result<(), String> {
    let metadata = std::fs::metadata(path).map_err(|e| e.to_string())?;
    if metadata.is_dir() {
        Ok(())
    } else {
        Err(format!(
            "path must be a directory for {:?}: {}",
            purpose,
            path.to_string_lossy()
        ))
    }
}

fn ensure_allowed_generic_open_extension(path: &Path) -> Result<(), String> {
    let actual = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if matches!(
        actual.to_ascii_lowercase().as_str(),
        "csv" | "json" | "log" | "md" | "txt" | "xlsx"
    ) {
        return Ok(());
    }
    Err(format!(
        "path extension is not allowed for shell open/reveal: {}",
        path.to_string_lossy()
    ))
}

fn canonical_existing_roots(roots: Vec<PathBuf>) -> Vec<PathBuf> {
    roots
        .into_iter()
        .filter_map(|root| root.canonicalize().ok())
        .fold(Vec::new(), |mut out, root| {
            if !out.iter().any(|existing| existing == &root) {
                out.push(root);
            }
            out
        })
}

fn looks_url_like(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.contains("://")
        || lower.starts_with("file:")
        || lower.starts_with("javascript:")
        || lower.starts_with("data:")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tauri::Manager;

    fn make_target_dir(prefix: &str) -> PathBuf {
        let mut path = std::env::current_dir()
            .expect("current dir")
            .join("target")
            .join("local_path_policy_tests");
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        path.push(format!("{prefix}_{}_{}", std::process::id(), unique));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("create target test dir");
        path
    }

    fn build_mock_app() -> tauri::App<tauri::test::MockRuntime> {
        tauri::test::mock_builder()
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .expect("failed to build tauri mock app")
    }

    #[test]
    fn canonical_existing_path_rejects_url_like_values() {
        let err = canonical_existing_path("https://example.invalid/report.xlsx")
            .expect_err("URL-like path should fail");
        assert!(err.contains("local filesystem path"));
    }

    #[test]
    fn validate_shell_open_path_accepts_app_reports_and_runs_roots() {
        let app = build_mock_app();
        let handle = app.handle();
        let base = handle.path().app_local_data_dir().expect("app data dir");
        let reports = base.join("reports");
        let runs = base.join("runs");
        std::fs::create_dir_all(&reports).expect("reports dir");
        std::fs::create_dir_all(&runs).expect("runs dir");
        let report = reports.join("summary.xlsx");
        let run = runs.join("vectorgrid_run_test.json");
        std::fs::write(&report, b"report").expect("report file");
        std::fs::write(&run, b"run").expect("run file");

        let opened = validate_shell_open_path(
            handle,
            report.to_string_lossy().as_ref(),
            ShellOpenPurpose::GenericOpen,
        )
        .expect("report path accepted");
        assert_eq!(opened, report.canonicalize().expect("canonical report"));

        let revealed = validate_shell_open_path(
            handle,
            run.to_string_lossy().as_ref(),
            ShellOpenPurpose::GenericReveal,
        )
        .expect("run path accepted");
        assert_eq!(revealed, run.canonicalize().expect("canonical run"));
    }

    #[test]
    fn validate_shell_open_path_rejects_arbitrary_external_files() {
        let app = build_mock_app();
        let external = make_target_dir("vectorgrid_external_open").join("external.txt");
        std::fs::write(&external, b"outside").expect("external file");

        let err = validate_shell_open_path(
            app.handle(),
            external.to_string_lossy().as_ref(),
            ShellOpenPurpose::GenericOpen,
        )
        .expect_err("external file should fail");
        assert!(err.contains("outside allowed VectorGrid roots"));
    }

    #[test]
    fn validate_shell_open_path_rejects_market_root_files_for_generic_open() {
        let app = build_mock_app();
        let handle = app.handle();
        let base = handle.path().app_local_data_dir().expect("app data dir");
        let market_root = make_target_dir("vectorgrid_market_root");
        crate::settings::set_market_data_root_dir(&base, Some(&market_root))
            .expect("set market root");
        let market_file = market_root.join("prices.csv");
        std::fs::write(&market_file, b"date,close\n").expect("market data file");

        let err = validate_shell_open_path(
            handle,
            market_file.to_string_lossy().as_ref(),
            ShellOpenPurpose::GenericOpen,
        )
        .expect_err("generic open should not accept market root files");
        assert!(err.contains("outside allowed VectorGrid roots"));
    }

    #[test]
    fn validate_shell_open_path_accepts_market_root_directory_only_for_market_open() {
        let app = build_mock_app();
        let handle = app.handle();
        let base = handle.path().app_local_data_dir().expect("app data dir");
        let market_root = make_target_dir("vectorgrid_market_dir");
        crate::settings::set_market_data_root_dir(&base, Some(&market_root))
            .expect("set market root");
        let market_file = market_root.join("prices.csv");
        std::fs::write(&market_file, b"date,close\n").expect("market data file");

        let opened = validate_shell_open_path(
            handle,
            market_root.to_string_lossy().as_ref(),
            ShellOpenPurpose::MarketDataFolderOpen,
        )
        .expect("market root directory accepted");
        assert_eq!(opened, market_root.canonicalize().expect("market root"));

        let err = validate_shell_open_path(
            handle,
            market_file.to_string_lossy().as_ref(),
            ShellOpenPurpose::MarketDataFolderOpen,
        )
        .expect_err("market folder open should reject files");
        assert!(err.contains("must be a directory"));
    }

    #[test]
    fn extension_helpers_are_case_insensitive_and_fail_closed() {
        ensure_csv_extension(Path::new("C:/data/SAMPLE.CSV")).expect("csv");
        ensure_xlsx_extension(Path::new("C:/reports/report.xlsx")).expect("xlsx");
        ensure_json_extension(Path::new("C:/runs/run.JSON")).expect("json");

        let err = ensure_csv_extension(Path::new("C:/data/not_csv.txt"))
            .expect_err("wrong extension should fail");
        assert!(err.contains(".csv"));
    }

    #[test]
    fn validate_shell_open_path_rejects_symlink_escape_when_supported() {
        #[cfg(unix)]
        fn symlink_file(src: &Path, dst: &Path) -> std::io::Result<()> {
            std::os::unix::fs::symlink(src, dst)
        }

        #[cfg(windows)]
        fn symlink_file(src: &Path, dst: &Path) -> std::io::Result<()> {
            std::os::windows::fs::symlink_file(src, dst)
        }

        let app = build_mock_app();
        let handle = app.handle();
        let base = handle.path().app_local_data_dir().expect("app data dir");
        let reports = base.join("reports");
        std::fs::create_dir_all(&reports).expect("reports dir");
        let external = make_target_dir("vectorgrid_symlink_escape").join("external.xlsx");
        std::fs::write(&external, b"outside").expect("external file");
        let link = reports.join("escape.xlsx");
        if symlink_file(&external, &link).is_err() {
            return;
        }

        let err = validate_shell_open_path(
            handle,
            link.to_string_lossy().as_ref(),
            ShellOpenPurpose::GenericOpen,
        )
        .expect_err("symlink escape should fail");
        assert!(err.contains("outside allowed VectorGrid roots"));
    }
}
