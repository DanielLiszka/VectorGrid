use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

pub struct BenchmarkRunGuard {
    path: PathBuf,
    _file: File,
}

pub fn acquire(label: &str) -> Result<BenchmarkRunGuard, String> {
    let path = std::env::temp_dir().join("vectorgrid_benchmark.lock");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                format!(
                    "another VectorGrid benchmark/profiling run appears active (lock: {}). Stop it or remove the stale lock before rerunning.",
                    path.display()
                )
            } else {
                format!("failed to create benchmark lock {}: {error}", path.display())
            }
        })?;
    writeln!(
        file,
        "label={label}\npid={}\nstarted_at_unix_ms={}",
        std::process::id(),
        unix_epoch_ms()
    )
    .map_err(|error| format!("failed to write benchmark lock {}: {error}", path.display()))?;
    file.sync_all()
        .map_err(|error| format!("failed to sync benchmark lock {}: {error}", path.display()))?;
    Ok(BenchmarkRunGuard { path, _file: file })
}

fn unix_epoch_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

impl Drop for BenchmarkRunGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}
