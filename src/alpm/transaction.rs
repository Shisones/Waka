use anyhow::Context;

pub fn trans_init(h: &mut alpm::Alpm, flags: alpm::TransFlag) -> anyhow::Result<()> {
    h.trans_init(flags).map_err(|e| anyhow::anyhow!("init transaction: {e}"))?;
    // Only claim the lock after libalpm created it, so a failed init never
    // leaves us owning someone else's lock file.
    let lock = crate::constants::lock_file();
    let lock_str = lock.to_str().context("lock file path contains non-UTF8 characters")?;
    crate::set_lockfile(lock_str);
    Ok(())
}

pub fn end_trans(h: &mut alpm::Alpm) {
    if let Err(e) = h.trans_release() {
        eprintln!("  {} trans_release failed: {}", crate::display::style::yellow("warning:"), e);
    }
    crate::clear_lockfile();
}
