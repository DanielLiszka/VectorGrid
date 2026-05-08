use serde_json::Value;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const REPORT_FILENAME_COLLISION_LIMIT: u32 = 999;
static REPORT_TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[cfg(test)]
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

#[cfg(test)]
pub(crate) fn existing_path(path: String) -> Result<PathBuf, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("path must not be empty".to_string());
    }

    let path = PathBuf::from(trimmed);
    if !path.exists() {
        return Err(format!("path does not exist: {}", path.to_string_lossy()));
    }

    Ok(path)
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
        out.truncate(96);
        out
    }
}

pub(super) fn reports_dir_for_base(base: &Path) -> Result<PathBuf, String> {
    let dir = base.join("reports");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

#[cfg(test)]
pub(super) fn suggest_report_path_for_base(report: &Value, base: &Path) -> Result<PathBuf, String> {
    let dir = reports_dir_for_base(base)?;
    suggest_report_path_in_dir(report, &dir, REPORT_FILENAME_COLLISION_LIMIT)
}

fn report_base_name(report: &Value) -> String {
    let ts = report
        .get("ts")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let safe_ts = sanitize_filename_component(ts);
    format!("vectorgrid_report_{safe_ts}")
}

fn report_path_for_slot(dir: &Path, base_name: &str, slot: u32) -> PathBuf {
    if slot <= 1 {
        dir.join(format!("{base_name}.xlsx"))
    } else {
        dir.join(format!("{base_name}_{slot}.xlsx"))
    }
}

#[cfg(test)]
fn suggest_report_path_in_dir(
    report: &Value,
    dir: &Path,
    max_suffix: u32,
) -> Result<PathBuf, String> {
    let base_name = report_base_name(report);
    let path = report_path_for_slot(dir, &base_name, 1);
    if !path.exists() {
        return Ok(path);
    }

    for i in 2..=max_suffix {
        let candidate = report_path_for_slot(dir, &base_name, i);
        if !candidate.exists() {
            return Ok(candidate);
        }
    }

    Err(format!(
        "no available report filename slots for {}",
        dir.join(format!("{base_name}.xlsx")).to_string_lossy()
    ))
}

fn report_temp_path(dir: &Path, base_name: &str) -> PathBuf {
    let seq = REPORT_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    dir.join(format!(
        ".{base_name}.{}.{}.{}.tmp",
        std::process::id(),
        nonce,
        seq
    ))
}

fn write_report_temp_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| {
            format!(
                "failed to create temp report workbook {}: {e}",
                path.display()
            )
        })?;
    file.write_all(bytes).map_err(|e| {
        format!(
            "failed to write temp report workbook {}: {e}",
            path.display()
        )
    })?;
    file.sync_all().map_err(|e| {
        format!(
            "failed to sync temp report workbook {}: {e}",
            path.display()
        )
    })
}

pub(super) fn persist_report_bytes_for_base(
    report: &Value,
    base: &Path,
    bytes: &[u8],
) -> Result<PathBuf, String> {
    let dir = reports_dir_for_base(base)?;
    let base_name = report_base_name(report);
    let temp_path = report_temp_path(&dir, &base_name);
    write_report_temp_file(&temp_path, bytes)?;

    for slot in 1..=REPORT_FILENAME_COLLISION_LIMIT {
        let candidate = report_path_for_slot(&dir, &base_name, slot);
        match std::fs::hard_link(&temp_path, &candidate) {
            Ok(()) => {
                let _ = std::fs::remove_file(&temp_path);
                return Ok(candidate);
            }
            Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
            Err(error) => {
                let _ = std::fs::remove_file(&temp_path);
                return Err(format!(
                    "failed to persist report workbook at {}: {error}",
                    candidate.display()
                ));
            }
        }
    }

    let _ = std::fs::remove_file(&temp_path);
    Err(format!(
        "no available report filename slots for {}",
        report_path_for_slot(&dir, &base_name, 1).to_string_lossy()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Barrier};

    fn make_temp_dir(prefix: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        path.push(format!("{prefix}_{}_{}", std::process::id(), unique));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("create temp dir");
        path
    }

    #[test]
    fn reports_dir_for_base_creates_reports_subdir() {
        let base = make_temp_dir("reports_dir_for_base");
        let reports = reports_dir_for_base(&base).expect("reports dir");
        assert_eq!(reports, base.join("reports"));
        assert!(reports.is_dir());
    }

    #[test]
    fn existing_path_rejects_missing_paths_before_shell_launch() {
        let missing = make_temp_dir("open_missing").join("missing.xlsx");
        let err = existing_path(missing.to_string_lossy().to_string()).expect_err("missing path");
        assert!(err.contains("path does not exist"));
        assert!(err.contains("missing.xlsx"));
    }

    #[test]
    fn existing_path_rejects_whitespace_only_input() {
        let err = existing_path("   \n\t  ".to_string()).expect_err("whitespace-only path");
        assert_eq!(err, "path must not be empty");
    }

    #[test]
    fn existing_path_trims_outer_whitespace_before_resolution() {
        let dir = make_temp_dir("existing_path_preserve_bytes");
        let file = dir.join("report.xlsx");
        std::fs::write(&file, b"report").expect("seed report");

        let padded = format!(" {} ", file.to_string_lossy());
        let resolved = existing_path(padded).expect("padded path should resolve after trimming");
        assert_eq!(resolved, file);
    }

    #[test]
    fn quantile_preserves_current_edge_case_semantics() {
        let sorted = [10.0, 20.0, 30.0, 40.0];

        assert_eq!(quantile(&[], 0.5), 0.0);
        assert_eq!(quantile(&[7.5], 0.5), 7.5);
        assert_eq!(quantile(&sorted, -1.0), 10.0);
        assert_eq!(quantile(&sorted, 2.0), 40.0);
        assert_eq!(quantile(&sorted, f64::NAN), 10.0);
    }

    #[test]
    fn suggest_report_path_for_base_uses_first_available_collision_suffix() {
        let base = make_temp_dir("suggest_report_path_collision");
        let report = serde_json::json!({ "ts": "2026-04-10T23:59:59Z" });
        let reports = reports_dir_for_base(&base).expect("reports dir");
        let base_name = report_base_name(&report);
        let seed = reports.join(format!("{base_name}.xlsx"));
        std::fs::write(&seed, b"taken").expect("seed collision");

        let suggested = suggest_report_path_for_base(&report, &base).expect("suggested path");
        assert_eq!(suggested, reports.join(format!("{base_name}_2.xlsx")));
    }

    #[test]
    fn suggest_report_path_in_dir_errors_when_collision_slots_are_exhausted() {
        let dir = make_temp_dir("suggest_report_path_exhausted");
        let report = serde_json::json!({ "ts": "2026-04-10T23:59:59Z" });
        let base_name = report_base_name(&report);

        std::fs::write(dir.join(format!("{base_name}.xlsx")), b"taken").expect("seed base");
        std::fs::write(dir.join(format!("{base_name}_2.xlsx")), b"taken").expect("seed slot 2");
        std::fs::write(dir.join(format!("{base_name}_3.xlsx")), b"taken").expect("seed slot 3");

        let err = suggest_report_path_in_dir(&report, &dir, 3).expect_err("slots should exhaust");
        assert!(err.contains("no available report filename slots"));
        assert!(err.contains(&base_name));
    }

    #[test]
    fn persist_report_bytes_for_base_does_not_overwrite_existing_report() {
        let base = make_temp_dir("persist_report_no_overwrite");
        let report = serde_json::json!({ "ts": "2026-04-10T23:59:59Z" });
        let reports = reports_dir_for_base(&base).expect("reports dir");
        let base_name = report_base_name(&report);
        let seed = reports.join(format!("{base_name}.xlsx"));
        std::fs::write(&seed, b"existing").expect("seed existing report");

        let saved =
            persist_report_bytes_for_base(&report, &base, b"new workbook").expect("persist report");

        assert_eq!(saved, reports.join(format!("{base_name}_2.xlsx")));
        assert_eq!(std::fs::read(&seed).expect("read seed"), b"existing");
        assert_eq!(std::fs::read(saved).expect("read saved"), b"new workbook");
    }

    #[test]
    fn persist_report_bytes_for_base_concurrent_same_timestamp_uses_distinct_files() {
        let base = Arc::new(make_temp_dir("persist_report_concurrent"));
        let report = Arc::new(serde_json::json!({ "ts": "2026-04-10T23:59:59Z" }));
        let barrier = Arc::new(Barrier::new(2));

        let handles = (0..2)
            .map(|idx| {
                let base = Arc::clone(&base);
                let report = Arc::clone(&report);
                let barrier = Arc::clone(&barrier);
                std::thread::spawn(move || {
                    barrier.wait();
                    persist_report_bytes_for_base(
                        &report,
                        &base,
                        format!("workbook-{idx}").as_bytes(),
                    )
                    .expect("persist report")
                })
            })
            .collect::<Vec<_>>();

        let mut paths = handles
            .into_iter()
            .map(|handle| handle.join().expect("worker should not panic"))
            .collect::<Vec<_>>();
        paths.sort();
        paths.dedup();

        assert_eq!(paths.len(), 2);
        for path in paths {
            assert!(path.is_file(), "report should exist: {}", path.display());
            let bytes = std::fs::read(path).expect("read report");
            assert!(bytes == b"workbook-0" || bytes == b"workbook-1");
        }
    }

    #[test]
    fn reports_dir_for_base_errors_when_reports_path_is_a_file() {
        let base = make_temp_dir("reports_dir_for_base_file");
        std::fs::write(base.join("reports"), b"not a directory").expect("reports file");

        let err = reports_dir_for_base(&base).expect_err("reports file should block dir creation");
        assert!(!err.is_empty());
    }
}
