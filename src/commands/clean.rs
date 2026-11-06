use crate::alpm::handle::AlpmHandle;
use crate::constants;
use crate::display::{format as fmt, render, style, table};
use crate::sys::require_root;
use anyhow::Result;
use std::collections::HashSet;

/// True only for a sync database file whose repo is no longer configured.
pub fn unused_sync_db(fname: &str, configured: &HashSet<String>) -> bool {
    for suffix in [".db.sig", ".files.sig", ".db.part", ".files.part", ".db", ".files"] {
        if let Some(repo) = fname.strip_suffix(suffix) {
            return !configured.contains(repo);
        }
    }
    false
}

pub fn should_remove(fname: &str, all: bool, installed: &HashSet<String>) -> bool {
    if fname.ends_with(".part") {
        return true;
    }
    if fname.ends_with(".db") || fname.ends_with(".db.sig") {
        return all;
    }
    if let Some(base) = fname.strip_suffix(".sig") {
        return should_remove(base, all, installed);
    }
    let base = match fname.split(".pkg.tar.").next() {
        Some(base) => base,
        None => return false,
    };
    let without_arch = base.rsplit_once('-').map(|(p, _)| p).unwrap_or(base);
    !installed.contains(without_arch)
}


pub fn run(all: bool, assume_yes: bool) -> Result<()> {
    use std::fs;

    require_root()?;
    let config = crate::config::Config::load()?;
    let assume_yes = assume_yes || config.waka.assume_yes;
    render::require_confirmation_possible(assume_yes)?;
    let handle = AlpmHandle::new()?;
    let local = handle.handle.localdb();
    let installed: HashSet<String> = local.pkgs().iter()
        .map(|p| format!("{}-{}", p.name(), p.version()))
        .collect();

    let mut caches: Vec<std::path::PathBuf> = handle
        .handle
        .cachedirs()
        .iter()
        .map(std::path::PathBuf::from)
        .collect();
    if caches.is_empty() {
        caches.push(constants::cache_dir());
    }

    let mut doomed: Vec<(std::path::PathBuf, u64)> = Vec::new();
    for cache in &caches {
        if !cache.exists() {
            continue;
        }
        for entry in fs::read_dir(cache)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() { continue; }
            let fname = path.file_name().unwrap().to_string_lossy().to_string();
            if should_remove(&fname, all, &installed) {
                let len = entry.metadata().map(|m| m.len()).unwrap_or(0);
                doomed.push((path, len));
            }
        }
    }

    if all {
        let configured: HashSet<String> = handle
            .handle
            .syncdbs()
            .iter()
            .map(|db| db.name().to_string())
            .collect();
        let sync = constants::db_path().join("sync");
        if sync.is_dir() {
            for entry in fs::read_dir(&sync)? {
                let entry = entry?;
                let path = entry.path();
                let fname = entry.file_name().to_string_lossy().to_string();
                if path.is_file()
                    && !entry.file_type()?.is_symlink()
                    && unused_sync_db(&fname, &configured)
                {
                    let len = entry.metadata().map(|m| m.len()).unwrap_or(0);
                    doomed.push((path, len));
                }
            }
        }
    }

    if doomed.is_empty() {
        println!(" {} cache is already clean", style::green("\u{2713}"));
        return Ok(());
    }

    let freed: i64 = doomed.iter().map(|(_, len)| *len as i64).sum();
    println!(" {} {} {} \u{00b7} {} will be freed",
        style::cyan("info"), doomed.len(), table::plural("file", doomed.len()), fmt::format_size(freed));

    println!();

    if !assume_yes && !render::confirm(&style::bold("\n Clear package cache? [Y/n] ")) {
        eprintln!(" {}", style::red("Aborted."));
        return Err(crate::aborted());
    }

    let mut removed = 0u32;
    for (path, _) in &doomed {
        match fs::remove_file(path) {
            Ok(_) => removed += 1,
            Err(e) => eprintln!("  {} {}: {}", style::yellow("warn"), path.display(), e),
        }
    }

    println!(" {} removed {} {} from cache", style::green("\u{2713}"), removed, table::plural("file", removed as usize));
    Ok(())
}
