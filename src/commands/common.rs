use crate::alpm::transaction;
use crate::display::{render, style};
use anyhow::Result;
use std::sync::{Arc, Mutex};

/// libalpm takes the database lock inside sync updates too. Claim it for the
/// signal handler while that call is running.
pub(crate) fn with_db_lock<T>(f: impl FnOnce() -> T) -> T {
    let lock = crate::constants::lock_file();
    if let Some(path) = lock.to_str() {
        crate::set_lockfile(path);
    }
    let out = f();
    crate::clear_lockfile();
    out
}

pub(crate) fn detail_from_prep_err(e: alpm::PrepareError<'_>) -> Option<String> {
    match e.data() {
        Some(alpm::PrepareData::UnsatisfiedDeps(deps)) => Some(deps.iter().map(|d| format!("{} (needs {})", d.target(), d.depend().name())).collect::<Vec<_>>().join(", ")),
        Some(alpm::PrepareData::ConflictingDeps(deps)) => Some(deps.iter().map(|d| format!("{} conflicts {}", d.package1().name(), d.package2().name())).collect::<Vec<_>>().join(", ")),
        Some(alpm::PrepareData::PkgInvalidArch(pkgs)) => Some(format!(
            "invalid architecture: {}",
            pkgs.iter().map(|p| p.name()).collect::<Vec<_>>().join(", ")
        )),
        _ => None,
    }
}

fn commit_detail(e: &alpm::CommitError) -> String {
    match e.data() {
        Some(alpm::CommitData::FileConflict(list)) => format!(
            "file conflict: {}",
            list.iter()
                .map(|c| format!("{} and {}", c.package1().name(), c.package2().name()))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Some(alpm::CommitData::PkgInvalid(list)) => {
            format!("invalid package: {}", list.iter().collect::<Vec<_>>().join(", "))
        }
        _ => e.to_string(),
    }
}


pub(crate) fn commit_epilogue(
    shared: &Arc<Mutex<render::DownloadState>>,
    h: &mut alpm::Alpm,
    commit_result: Result<(), alpm::CommitError>,
    filter: &str,
) -> Result<()> {
    transaction::end_trans(h);
    {
        let mut state = shared.lock().unwrap();
        state.phase = true;
        state.log.retain(|l| !l.contains(filter));
        match &commit_result {
            Ok(_) => state.log.push(style::green("Transaction completed successfully").to_string()),
            Err(e) => state.log.push(format!("{}: {}", style::red("Failed"), commit_detail(e))),
        }
        state.render();
    }
    commit_result.map_err(|e| anyhow::anyhow!("commit failed: {}", commit_detail(&e)))?;
    Ok(())
}

