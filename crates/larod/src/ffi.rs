use std::sync::Mutex;

static LAROD_LOCK: Mutex<()> = Mutex::new(());

pub(crate) fn with_larod<T>(f: impl FnOnce() -> T) -> T {
    let _guard = LAROD_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    f()
}

#[cfg(test)]
pub(crate) fn try_with_larod<T>(f: impl FnOnce() -> T) -> Option<T> {
    let _guard = match LAROD_LOCK.try_lock() {
        Ok(guard) => guard,
        Err(std::sync::TryLockError::WouldBlock) => return None,
        Err(std::sync::TryLockError::Poisoned(error)) => error.into_inner(),
    };
    Some(f())
}
