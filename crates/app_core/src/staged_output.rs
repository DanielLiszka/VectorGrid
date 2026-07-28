use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static OUTPUT_TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Streams an output into a same-directory temporary file and publishes it atomically.
///
/// Dropping an unpublished writer removes its temporary file and leaves any existing target
/// untouched.
pub(crate) struct AtomicFileWriter {
    target_path: PathBuf,
    staged_path: PathBuf,
    writer: Option<BufWriter<File>>,
    published: bool,
}

impl AtomicFileWriter {
    pub(crate) fn create(target_path: impl AsRef<Path>) -> Result<Self, String> {
        let target_path = target_path.as_ref().to_path_buf();
        let parent = target_path
            .parent()
            .filter(|entry| !entry.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let (staged_path, file) = create_staged_file(parent, &target_path)?;
        Ok(Self {
            target_path,
            staged_path,
            writer: Some(BufWriter::new(file)),
            published: false,
        })
    }

    pub(crate) fn publish(mut self) -> Result<(), String> {
        let mut writer = self
            .writer
            .take()
            .ok_or_else(|| "staged output writer is already closed".to_string())?;
        writer.flush().map_err(|error| {
            format!(
                "flush staged output {}: {error}",
                self.staged_path.display()
            )
        })?;
        writer.get_ref().sync_all().map_err(|error| {
            format!("sync staged output {}: {error}", self.staged_path.display())
        })?;
        drop(writer);

        let target_exists = existing_regular_target(&self.target_path)?;
        publish_staged_file(&self.staged_path, &self.target_path, target_exists)?;
        self.published = true;
        Ok(())
    }
}

impl Write for AtomicFileWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.writer
            .as_mut()
            .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "staged writer is closed"))?
            .write(buffer)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer
            .as_mut()
            .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "staged writer is closed"))?
            .flush()
    }
}

impl Drop for AtomicFileWriter {
    fn drop(&mut self) {
        self.writer.take();
        if !self.published {
            let _ = fs::remove_file(&self.staged_path);
        }
    }
}

fn create_staged_file(parent: &Path, target: &Path) -> Result<(PathBuf, File), String> {
    for _ in 0..64 {
        let staged_path = staged_path_for(parent, target)?;
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&staged_path)
        {
            Ok(file) => return Ok((staged_path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(format!(
                    "create staged output {}: {error}",
                    staged_path.display()
                ));
            }
        }
    }

    Err(format!(
        "failed to allocate a unique staged output path for {}",
        target.display()
    ))
}

fn staged_path_for(parent: &Path, target: &Path) -> Result<PathBuf, String> {
    let file_name = target
        .file_name()
        .ok_or_else(|| format!("output path has no file name: {}", target.display()))?;
    let counter = OUTPUT_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut staged_name = OsString::from(".");
    staged_name.push(file_name);
    staged_name.push(format!(".{}.{}.tmp", std::process::id(), counter));
    Ok(parent.join(staged_name))
}

fn existing_regular_target(path: &Path) -> Result<bool, String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            let file_type = metadata.file_type();
            if file_type.is_symlink() {
                return Err(format!(
                    "refusing to replace symlink output target {}",
                    path.display()
                ));
            }
            if !metadata.is_file() {
                return Err(format!(
                    "refusing to replace non-file output target {}",
                    path.display()
                ));
            }
            Ok(true)
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(format!("stat output target {}: {error}", path.display())),
    }
}

fn publish_staged_file(staged: &Path, target: &Path, target_exists: bool) -> Result<(), String> {
    if target_exists {
        replace_existing_file_with_staged(staged, target)
    } else {
        fs::rename(staged, target).map_err(|error| {
            format!(
                "publish staged output {} to {}: {error}",
                staged.display(),
                target.display()
            )
        })
    }
}

#[cfg(windows)]
fn replace_existing_file_with_staged(staged: &Path, target: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::ReplaceFileW;

    let target_wide: Vec<u16> = target.as_os_str().encode_wide().chain(Some(0)).collect();
    let staged_wide: Vec<u16> = staged.as_os_str().encode_wide().chain(Some(0)).collect();
    let replaced = unsafe {
        ReplaceFileW(
            target_wide.as_ptr(),
            staged_wide.as_ptr(),
            std::ptr::null(),
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if replaced == 0 {
        Err(format!(
            "replace output target {} with {}: {}",
            target.display(),
            staged.display(),
            io::Error::last_os_error()
        ))
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn replace_existing_file_with_staged(staged: &Path, target: &Path) -> Result<(), String> {
    fs::rename(staged, target).map_err(|error| {
        format!(
            "replace output target {} with {}: {error}",
            target.display(),
            staged.display()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::AtomicFileWriter;
    use std::fs;
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

    static TEST_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

    struct TestDir(PathBuf);

    impl TestDir {
        fn new(label: &str) -> Self {
            let counter = TEST_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "vectorgrid_staged_output_{label}_{}_{}",
                std::process::id(),
                counter
            ));
            fs::create_dir(&path).expect("create isolated test directory");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn staged_siblings(target: &Path) -> Vec<PathBuf> {
        let file_name = target
            .file_name()
            .expect("target file name")
            .to_string_lossy();
        let prefix = format!(".{file_name}.");
        fs::read_dir(target.parent().expect("target parent"))
            .expect("read target parent")
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                let name = path
                    .file_name()
                    .map(|name| name.to_string_lossy())
                    .unwrap_or_default();
                name.starts_with(&prefix) && name.ends_with(".tmp")
            })
            .collect()
    }

    #[test]
    fn pre_cancelled_export_preserves_existing_target_without_staging() {
        let dir = TestDir::new("pre_cancel");
        let target = dir.path().join("all_pairs.csv");
        fs::write(&target, b"sentinel\n").expect("write sentinel");
        let cancel = AtomicBool::new(true);

        let result = (|| -> Result<(), String> {
            if cancel.load(Ordering::Relaxed) {
                return Err("cancelled".to_string());
            }
            let _writer = AtomicFileWriter::create(&target)?;
            Ok(())
        })();

        assert_eq!(result.expect_err("export must cancel"), "cancelled");
        assert_eq!(fs::read(&target).expect("read sentinel"), b"sentinel\n");
        assert!(staged_siblings(&target).is_empty());
    }

    #[test]
    fn failed_export_cleans_staged_file_and_preserves_existing_target() {
        let dir = TestDir::new("failure");
        let target = dir.path().join("all_pairs.csv");
        fs::write(&target, b"sentinel\n").expect("write sentinel");

        let result = (|| -> Result<(), String> {
            let mut writer = AtomicFileWriter::create(&target)?;
            writer
                .write_all(b"partial replacement\n")
                .map_err(|error| error.to_string())?;
            assert_eq!(staged_siblings(&target).len(), 1);
            Err("simulated kernel failure".to_string())
        })();

        assert_eq!(
            result.expect_err("export must fail"),
            "simulated kernel failure"
        );
        assert_eq!(fs::read(&target).expect("read sentinel"), b"sentinel\n");
        assert!(staged_siblings(&target).is_empty());
    }

    #[test]
    fn successful_export_atomically_replaces_existing_target() {
        let dir = TestDir::new("success");
        let target = dir.path().join("all_pairs.csv");
        fs::write(&target, b"sentinel\n").expect("write sentinel");

        let mut writer = AtomicFileWriter::create(&target).expect("create staged writer");
        writer
            .write_all(b"complete replacement\n")
            .expect("write replacement");
        writer.publish().expect("publish replacement");

        assert_eq!(
            fs::read(&target).expect("read replacement"),
            b"complete replacement\n"
        );
        assert!(staged_siblings(&target).is_empty());
    }
}
