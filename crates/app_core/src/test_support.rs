use std::sync::{Mutex, MutexGuard, OnceLock};

/// Test-only coordination lock for repo-owned process-global env/state mutation.
///
/// This reduces interference among known writers, but it is not a general proof
/// that environment mutation is safe under parallel multithreaded test execution.
#[cfg(test)]
pub(crate) fn lock_test_env() -> MutexGuard<'static, ()> {
    static TEST_ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    TEST_ENV_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

#[cfg(test)]
mod tests {
    use super::lock_test_env;

    #[test]
    fn lock_test_env_recovers_after_poison() {
        let join = std::thread::spawn(|| {
            let _guard = lock_test_env();
            panic!("intentional poison for recovery test");
        })
        .join();
        assert!(join.is_err());

        let _guard = lock_test_env();
    }
}
