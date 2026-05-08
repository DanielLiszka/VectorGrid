use std::env;

const LOCK_CONTEXT_ENV: &str = "TA_DESKTOP_DEMO_BENCHMARK_LOCK_CONTEXT";
const LOCK_HELD_ENV: &str = "TA_DESKTOP_DEMO_BENCHMARK_LOCK_HELD";
const LOCK_OWNER_PID_ENV: &str = "TA_DESKTOP_DEMO_BENCHMARK_LOCK_OWNER_PID";
const LOCK_TOKEN_ENV: &str = "TA_DESKTOP_DEMO_BENCHMARK_LOCK_TOKEN";
const LOCK_TIMEOUT_MS: u32 = 900_000;

pub struct BenchmarkRunGuard {
    lock: Option<PlatformLock>,
    previous_env: Option<BenchmarkLockEnv>,
}

#[derive(Clone)]
struct BenchmarkLockEnv {
    held: Option<String>,
    owner_pid: Option<String>,
    context: Option<String>,
    token: Option<String>,
}

pub fn acquire(context: &str) -> Result<BenchmarkRunGuard, String> {
    if env::var(LOCK_HELD_ENV).ok().as_deref() == Some("1") {
        validate_inherited_lock(context)?;
        return Ok(BenchmarkRunGuard {
            lock: None,
            previous_env: None,
        });
    }

    let lock = PlatformLock::acquire(context)?;
    let previous_env = BenchmarkLockEnv::capture();
    env::set_var(LOCK_HELD_ENV, "1");
    env::set_var(LOCK_OWNER_PID_ENV, std::process::id().to_string());
    env::set_var(LOCK_CONTEXT_ENV, context);
    env::set_var(LOCK_TOKEN_ENV, new_token());
    Ok(BenchmarkRunGuard {
        lock: Some(lock),
        previous_env: Some(previous_env),
    })
}

fn validate_inherited_lock(context: &str) -> Result<(), String> {
    let owner_pid_text = non_empty_env(LOCK_OWNER_PID_ENV).ok_or_else(|| {
        format!("benchmark lock inheritance marker is incomplete; refusing to run {context}")
    })?;
    let _owner_context = non_empty_env(LOCK_CONTEXT_ENV).ok_or_else(|| {
        format!("benchmark lock inheritance marker is incomplete; refusing to run {context}")
    })?;
    let _owner_token = non_empty_env(LOCK_TOKEN_ENV).ok_or_else(|| {
        format!("benchmark lock inheritance marker is incomplete; refusing to run {context}")
    })?;
    let owner_pid = owner_pid_text
        .parse::<u32>()
        .map_err(|_| format!("benchmark lock owner PID is invalid: {owner_pid_text}"))?;
    if owner_pid != std::process::id() && !process_is_running(owner_pid) {
        return Err(format!(
            "benchmark lock owner PID {owner_pid} is no longer running; refusing to inherit lock for {context}"
        ));
    }
    Ok(())
}

fn non_empty_env(name: &str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn new_token() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{:x}{:x}", std::process::id(), nanos)
}

impl BenchmarkLockEnv {
    fn capture() -> Self {
        Self {
            held: env::var(LOCK_HELD_ENV).ok(),
            owner_pid: env::var(LOCK_OWNER_PID_ENV).ok(),
            context: env::var(LOCK_CONTEXT_ENV).ok(),
            token: env::var(LOCK_TOKEN_ENV).ok(),
        }
    }

    fn restore(self) {
        restore_env(LOCK_HELD_ENV, self.held);
        restore_env(LOCK_OWNER_PID_ENV, self.owner_pid);
        restore_env(LOCK_CONTEXT_ENV, self.context);
        restore_env(LOCK_TOKEN_ENV, self.token);
    }
}

fn restore_env(name: &str, value: Option<String>) {
    match value {
        Some(value) if !value.trim().is_empty() => env::set_var(name, value),
        _ => env::remove_var(name),
    }
}

impl Drop for BenchmarkRunGuard {
    fn drop(&mut self) {
        drop(self.lock.take());
        if let Some(previous_env) = self.previous_env.take() {
            previous_env.restore();
        }
    }
}

#[cfg(windows)]
struct PlatformLock(windows_sys::Win32::Foundation::HANDLE);

#[cfg(windows)]
impl PlatformLock {
    fn acquire(context: &str) -> Result<Self, String> {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Foundation::{
            CloseHandle, WAIT_ABANDONED, WAIT_FAILED, WAIT_OBJECT_0, WAIT_TIMEOUT,
        };
        use windows_sys::Win32::System::Threading::{CreateMutexW, WaitForSingleObject};

        let name: Vec<u16> = std::ffi::OsStr::new("Local\\ta_desktop_demo_benchmark_gate")
            .encode_wide()
            .chain(Some(0))
            .collect();
        let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
        if handle.is_null() {
            return Err(format!(
                "failed to create benchmark mutex for {context}: {}",
                std::io::Error::last_os_error()
            ));
        }

        match unsafe { WaitForSingleObject(handle, LOCK_TIMEOUT_MS) } {
            WAIT_OBJECT_0 | WAIT_ABANDONED => Ok(Self(handle)),
            WAIT_TIMEOUT => {
                unsafe {
                    CloseHandle(handle);
                }
                Err(format!(
                    "timed out waiting for benchmark lock after {}s ({context}); another benchmark/profiler may still be running",
                    LOCK_TIMEOUT_MS / 1000
                ))
            }
            WAIT_FAILED => {
                let error = std::io::Error::last_os_error();
                unsafe {
                    CloseHandle(handle);
                }
                Err(format!(
                    "failed while waiting for benchmark lock ({context}): {error}"
                ))
            }
            other => {
                unsafe {
                    CloseHandle(handle);
                }
                Err(format!(
                    "unexpected benchmark mutex wait result {other} ({context})"
                ))
            }
        }
    }
}

#[cfg(windows)]
impl Drop for PlatformLock {
    fn drop(&mut self) {
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::Threading::ReleaseMutex;

        unsafe {
            ReleaseMutex(self.0);
            CloseHandle(self.0);
        }
    }
}

#[cfg(not(windows))]
struct PlatformLock {
    path: std::path::PathBuf,
    _file: std::fs::File,
}

#[cfg(not(windows))]
impl PlatformLock {
    fn acquire(context: &str) -> Result<Self, String> {
        use std::fs::{self, OpenOptions};
        use std::io::Write;

        let path = env::temp_dir().join("ta_desktop_demo_benchmark_gate.lock");
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|error| {
                if error.kind() == std::io::ErrorKind::AlreadyExists {
                    format!(
                        "another benchmark/profiling run appears active (lock: {}); stop it or remove the stale lock before rerunning {context}",
                        path.display()
                    )
                } else {
                    format!("failed to create benchmark lock {}: {error}", path.display())
                }
            })?;
        writeln!(
            file,
            "context={context}\npid={}\nstarted_at_unix_ms={}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        )
        .map_err(|error| format!("failed to write benchmark lock {}: {error}", path.display()))?;
        file.sync_all().map_err(|error| {
            format!("failed to sync benchmark lock {}: {error}", path.display())
        })?;
        Ok(Self { path, _file: file })
    }
}

#[cfg(not(windows))]
impl Drop for PlatformLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

#[cfg(windows)]
fn process_is_running(pid: u32) -> bool {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};

    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if handle.is_null() {
        false
    } else {
        unsafe {
            CloseHandle(handle);
        }
        true
    }
}

#[cfg(not(windows))]
fn process_is_running(_pid: u32) -> bool {
    true
}
