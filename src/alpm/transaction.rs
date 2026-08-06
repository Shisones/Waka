use anyhow::Context;

pub fn trans_init(h: &mut alpm::Alpm, flags: alpm::TransFlag) -> anyhow::Result<()> {
    let lock = crate::constants::lock_file();
    let lock_str = lock.to_str().context("lock file path contains non-UTF8 characters")?;
    crate::set_lockfile(lock_str);
    h.trans_init(flags).map_err(|e| anyhow::anyhow!("init transaction: {e}"))
}

pub fn end_trans(h: &mut alpm::Alpm) {
    if let Err(e) = h.trans_release() {
        eprintln!("  {} trans_release failed: {}", crate::display::style::yellow("warning:"), e);
    }
    crate::clear_lockfile();
}
