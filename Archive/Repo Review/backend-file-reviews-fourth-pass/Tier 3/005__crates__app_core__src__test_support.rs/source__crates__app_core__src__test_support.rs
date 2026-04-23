use std::sync::{Mutex, MutexGuard, OnceLock};

#[cfg(test)]
pub(crate) fn lock_test_env() -> MutexGuard<'static, ()> {
    static TEST_ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    TEST_ENV_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("test env lock poisoned")
}
