use crate::alpm::{callbacks, handle::AlpmHandle, transaction};
use crate::commands::common::{commit_epilogue, detail_from_prep_err};
use crate::config::Config;
use crate::display::summary::show_upgrade_summary;
use crate::display::{db, render, style};
use crate::sys::require_root;
use anyhow::Result;
use std::sync::{Arc, Mutex};

pub fn run(assume_yes: bool, autoremove: bool) -> Result<()> {
    require_root()?;
    let config = Config::load()?;
    let assume_yes = assume_yes || config.waka.assume_yes;
    render::require_confirmation_possible(assume_yes)?;
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

    transaction::trans_init(h, alpm::TransFlag::NONE)?;
    // Install the question callback before sysupgrade.
    let title = "Upgrade".to_string();
    let shared = callbacks::callbacks(h, &title, assume_yes);
    h.sync_sysupgrade(false).map_err(|e| anyhow::anyhow!("sysupgrade: {e}"))?;
    let add_count = h.trans_add().len();
    if add_count == 0 {
        transaction::end_trans(h);
        shared.lock().unwrap().done();
        println!("{} All packages are up to date", style::green("\u{2713}"));
        return Ok(());
    }
    shared.lock().unwrap().title = format!("Upgrade ({} packages)", add_count);
    shared.lock().unwrap().render();

    let prep_err = match h.trans_prepare() {
        Ok(_) => None,
        Err(e) => Some((e.error().to_string(), detail_from_prep_err(e))),
    };
    if let Some((msg, detail)) = prep_err {
        transaction::end_trans(h);
        shared.lock().unwrap().done();
        if let Some(d) = detail { return Err(anyhow::anyhow!("{}: {}", msg, d)); }
        return Err(anyhow::anyhow!("{}", msg));
    }

    let add = h.trans_add();
    let rm = h.trans_remove();
    let local = h.localdb();
    show_upgrade_summary(&add, &rm, local);

    if !assume_yes && !render::confirm(&style::bold("\n Do you want to continue? [Y/n] ")) {
        transaction::end_trans(h);
        shared.lock().unwrap().done();
        eprintln!(" {}", style::red("Aborted."));
        return Err(crate::aborted());
    }

    {
        let mut state = shared.lock().unwrap();
        state.total_pkgs = add.len();
        for pkg in add.iter() {
            let size = pkg.download_size();
            state.total_bytes += size;
            state.pkg_sizes.insert(pkg.name().to_string(), size);
        }
        state.render();
    }
    let commit_result = h.trans_commit();
    commit_epilogue(&shared, h, commit_result, "Downloading packages")?;

    if autoremove {
        super::autoremove::run(assume_yes)?;
    }

    Ok(())
}
