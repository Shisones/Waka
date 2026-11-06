use crate::alpm::handle::AlpmHandle;
use crate::display::{db, table};
use crate::sys::require_root;
use anyhow::Result;
use std::sync::{Arc, Mutex};

pub fn run() -> Result<()> {
    require_root()?;
    let mut handle = AlpmHandle::new()?;
    let h = &mut handle.handle;

    let state = Arc::new(Mutex::new(db::DbSyncState::default()));
    {
        let mut s = state.lock().unwrap();
        for db in h.syncdbs() {
            s.bytes.insert(db.name().to_string(), (0, 0));
        }
    }
    let cb = Arc::clone(&state);
    h.set_dl_cb(cb, |fname, event, arc| {
        if let Ok(mut s) = arc.lock() {
            match event.event() {
                alpm::DownloadEvent::Init(_) => s.update(fname, 0, 0),
                alpm::DownloadEvent::Progress(p) => s.update(fname, p.downloaded, p.total),
                alpm::DownloadEvent::Completed(c) => s.update(fname, c.total, c.total),
                _ => {}
            }
        }
    });

    crate::commands::common::with_db_lock(|| h.syncdbs_mut().update(false))
        .map_err(|e| anyhow::anyhow!("database update failed: {}", e))?;

    {
        let names: Vec<String> = h.syncdbs().iter().map(|db| db.name().to_string()).collect();
        let mut b = state.lock().unwrap();
        b.done();
        for line in db::db_sync_lines(&b.bytes, &names) {
            println!("{line}");
        }
    }

    let local = h.localdb();
    let upgradable: usize = h.syncdbs().iter()
        .flat_map(|db| db.pkgs())
        .filter(|p| {
            local.pkg(p.name())
                .ok()
                .map(|l| alpm::vercmp(p.version().as_str(), l.version().as_str()) == std::cmp::Ordering::Greater)
                .unwrap_or(false)
        })
        .map(|p| p.name())
        .collect::<std::collections::HashSet<&str>>()
        .len();

    if upgradable > 0 {
        println!(" {} {} can be upgraded. Run 'waka upgrade' to see them.", upgradable, table::plural("package", upgradable));
    }

    Ok(())
}

