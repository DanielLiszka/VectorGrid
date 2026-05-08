use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static OUTPUT_TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn write_str(path: &Path, text: &str) -> Result<(), String> {
    write_bytes(path, text.as_bytes())
}

pub fn write_bytes(path: &Path, bytes: impl AsRef<[u8]>) -> Result<(), String> {
    write_with(path, |file| {
        file.write_all(bytes.as_ref())
            .map_err(|e| format!("write staged output {}: {e}", path.display()))
    })
}

pub fn write_with(
    path: &Path,
    write_contents: impl FnOnce(&mut fs::File) -> Result<(), String>,
) -> Result<(), String> {
    let parent = path
        .parent()
        .filter(|entry| !entry.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)
        .map_err(|e| format!("create output parent {}: {e}", parent.display()))?;

    let target_exists = existing_regular_target(path)?;
    let (staged_path, mut staged_file) = create_staged_file(parent, path)?;
    let write_result = (|| {
        write_contents(&mut staged_file)?;
        staged_file
            .sync_all()
            .map_err(|e| format!("sync staged output {}: {e}", staged_path.display()))?;
        Ok::<(), String>(())
    })();
    drop(staged_file);

    if let Err(error) = write_result {
        let _ = fs::remove_file(&staged_path);
        return Err(error);
    }

    if let Err(error) = publish_staged_file(&staged_path, path, target_exists) {
        let _ = fs::remove_file(&staged_path);
        return Err(error);
    }
    Ok(())
}

fn create_staged_file(parent: &Path, target: &Path) -> Result<(PathBuf, fs::File), String> {
    for _ in 0..64 {
        let temp_path = temp_path_for(parent, target)?;
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
        {
            Ok(file) => return Ok((temp_path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(format!(
                    "create staged output {}: {error}",
                    temp_path.display()
                ));
            }
        }
    }

    Err(format!(
        "failed to allocate a unique staged output path for {}",
        target.display()
    ))
}

fn temp_path_for(parent: &Path, target: &Path) -> Result<PathBuf, String> {
    let file_name = target
        .file_name()
        .ok_or_else(|| format!("output path has no file name: {}", target.display()))?;
    let counter = OUTPUT_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut temp_name = OsString::from(".");
    temp_name.push(file_name);
    temp_name.push(format!(".{}.{}.tmp", std::process::id(), counter));
    Ok(parent.join(temp_name))
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
        fs::rename(staged, target).map_err(|e| {
            format!(
                "publish staged output {} to {}: {e}",
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
    let ok = unsafe {
        ReplaceFileW(
            target_wide.as_ptr(),
            staged_wide.as_ptr(),
            std::ptr::null(),
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if ok == 0 {
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
    fs::rename(staged, target).map_err(|e| {
        format!(
            "replace output target {} with {}: {e}",
            target.display(),
            staged.display()
        )
    })
}
