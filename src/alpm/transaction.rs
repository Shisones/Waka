use anyhow::Context;
use std::sync::atomic::{AtomicPtr, Ordering};

/// Pointer to the handle with an in-flight transaction. The signal handler
/// uses it to call `alpm_trans_interrupt` instead of killing the process.
static ACTIVE: AtomicPtr<alpm::Alpm> = AtomicPtr::new(std::ptr::null_mut());

pub fn interrupt_active() -> bool {
    let ptr = ACTIVE.load(Ordering::SeqCst);
    if ptr.is_null() {
        return false;
    }
    // The transaction is already owned by the main thread. libalpm's interrupt
    // is a flag, so calling it from the signal thread is the least-bad option.
    unsafe {
        let _ = (*ptr).trans_interrupt();
    }
    true
}

pub fn trans_init(h: &mut alpm::Alpm, flags: alpm::TransFlag) -> anyhow::Result<()> {
    h.trans_init(flags).map_err(|e| anyhow::anyhow!("init transaction: {e}"))?;
    // Only claim the lock after libalpm created it, so a failed init never
    // leaves us owning someone else's lock file.
    ACTIVE.store(h as *mut alpm::Alpm, Ordering::SeqCst);
    let lock = crate::constants::lock_file();
    let lock_str = lock.to_str().context("lock file path contains non-UTF8 characters")?;
    crate::set_lockfile(lock_str);
    Ok(())
}

pub fn end_trans(h: &mut alpm::Alpm) {
    ACTIVE.store(std::ptr::null_mut(), Ordering::SeqCst);
    if let Err(e) = h.trans_release() {
        eprintln!("  {} trans_release failed: {}", crate::display::style::yellow("warning:"), e);
    }
    crate::clear_lockfile();
}
