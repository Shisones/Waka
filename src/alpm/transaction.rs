use anyhow::Context;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};

/// Pointer to the handle with an in-flight transaction. The signal handler
/// uses it to call `alpm_trans_interrupt`.
static ACTIVE: AtomicPtr<alpm::Alpm> = AtomicPtr::new(std::ptr::null_mut());
static INTERRUPTED: AtomicBool = AtomicBool::new(false);

pub fn interrupt_active() -> bool {
    let ptr = ACTIVE.load(Ordering::SeqCst);
    if ptr.is_null() {
        return false;
    }
    // libalpm's interrupt is a flag. Call it from the signal thread.
    INTERRUPTED.store(true, Ordering::SeqCst);
    unsafe {
        let _ = (*ptr).trans_interrupt();
    }
    true
}

pub fn trans_init(h: &mut alpm::Alpm, flags: alpm::TransFlag) -> anyhow::Result<()> {
    INTERRUPTED.store(false, Ordering::SeqCst);
    h.trans_init(flags).map_err(|e| anyhow::anyhow!("init transaction: {e}"))?;
    // Claim the lock only after libalpm creates it.
    ACTIVE.store(h as *mut alpm::Alpm, Ordering::SeqCst);
    let lock = crate::constants::lock_file();
    let lock_str = lock.to_str().context("lock file path contains non-UTF8 characters")?;
    crate::set_lockfile(lock_str);
    Ok(())
}

pub fn was_interrupted() -> bool {
    INTERRUPTED.load(Ordering::SeqCst)
}

pub fn end_trans(h: &mut alpm::Alpm) {
    ACTIVE.store(std::ptr::null_mut(), Ordering::SeqCst);
    if let Err(e) = h.trans_release() {
        eprintln!("  {} trans_release failed: {}", crate::display::style::yellow("warning:"), e);
    }
    crate::clear_lockfile();
}
