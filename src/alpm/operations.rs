use crate::alpm::{callbacks, handle::AlpmHandle, transaction};
use crate::config::Config;
use crate::constants;
use crate::display::{format as fmt, render, style, table};
use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

fn detail_from_prep_err(e: alpm::PrepareError<'_>) -> Option<String> {
    match e.data() {
        Some(alpm::PrepareData::UnsatisfiedDeps(deps)) => Some(deps.iter().map(|d| format!("{} (needs {})", d.target(), d.depend().name())).collect::<Vec<_>>().join(", ")),
        Some(alpm::PrepareData::ConflictingDeps(deps)) => Some(deps.iter().map(|d| format!("{} conflicts {}", d.package1().name(), d.package2().name())).collect::<Vec<_>>().join(", ")),
        _ => None,
    }
}

fn commit_epilogue(
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
            Err(e) => state.log.push(format!("{}: {}", style::red("Failed"), e)),
        }
        state.render();
    }
    commit_result.map_err(|e| anyhow::anyhow!("commit failed: {}", e))?;
    Ok(())
}

pub fn run_update() -> Result<()> {
    let mut handle = AlpmHandle::new()?;
    let h = &mut handle.handle;

    let bytes = Arc::new(Mutex::new(HashMap::<String, (i64, i64)>::new()));
    for db in h.syncdbs() {
        bytes.lock().unwrap().insert(db.name().to_string(), (0, 0));
    }
    let cb = Arc::clone(&bytes);
    h.set_dl_cb(cb, |fname, event, bytes| {
        let fname = fname.rsplit('/').next().unwrap_or(fname);
        let repo = fname.strip_suffix(".db.part").unwrap_or(fname).strip_suffix(".db").unwrap_or(fname).to_string();
        if let Some(b) = bytes.lock().unwrap().get_mut(&repo) {
            match event.event() {
                alpm::DownloadEvent::Progress(p) => *b = (p.downloaded, p.total),
                alpm::DownloadEvent::Completed(c) => *b = (c.total, c.total),
                _ => {}
            }
        }
    });

    h.syncdbs_mut().update(false).map_err(|e| anyhow::anyhow!("database update failed: {}", e))?;

    {
        use crate::display::{draw, format as fmt};
        let w = draw::term_width();
        let b = bytes.lock().unwrap();
        let mut names: Vec<String> = h.syncdbs().iter().map(|db| db.name().to_string()).collect();
        names.sort();
        let max_nw = names.iter().map(|n| draw::display_width(n)).max().unwrap_or(1).max(1);

        let mut size_strs: Vec<String> = Vec::new();
        for name in &names {
            let (dl, _) = b.get(name).copied().unwrap_or((0, 0));
            size_strs.push(if dl > 0 { fmt::format_size(dl) } else { "0 B".to_string() });
        }
        let max_sw = size_strs.iter().map(|s| draw::display_width(s)).max().unwrap_or(3);

        for (name, size_str) in names.iter().zip(size_strs.iter()) {
            let (dl, tot) = b.get(name).copied().unwrap_or((0, 0));
            let pct = if tot > 0 { dl as f64 / tot as f64 * 100.0 } else { 100.0 };
            let padded_size = format!("{:>width$}", size_str, width = max_sw);
            let npad = " ".repeat(max_nw.saturating_sub(draw::display_width(name)));
            let name_prefix = format!(" {}{} ", name, npad);
            let max_bar = ((w as f64 * 0.25) as usize).min(50);
            let bar_w = (w.saturating_sub(draw::display_width(&name_prefix) + 3 + max_sw + 2)).min(max_bar).max(10);
            let filled = ((pct / 100.0) * bar_w as f64) as usize;
            let bar = format!("{}{}", "━".repeat(filled), "─".repeat(bar_w.saturating_sub(filled)));
            let block = format!("[{}] {} ", bar, padded_size);
            let pad = w.saturating_sub(draw::display_width(&name_prefix) + draw::display_width(&block)).max(2);
            println!("{}{}{}", name_prefix, " ".repeat(pad), block);
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

fn show_upgrade_summary(add: &alpm::AlpmList<&alpm::Package>, rm: &alpm::AlpmList<&alpm::Package>, local: &alpm::Db) {
    println!();
    table::write_summary_row(&style::bold("Package"), &style::bold("Old Ver"), &style::bold("New Ver"), &style::bold("Net Change"), &style::bold("Size"));
    table::write_separator();
    let mut total_dl: i64 = 0;
    let mut total_isize: i64 = 0;
    let mut net_upgrade: i64 = 0;
    let mut dep_count = 0;
    for pkg in add.iter() {
        if pkg.reason() == alpm::PackageReason::Depend {
            dep_count += 1;
        }
        let old = local.pkg(pkg.name()).ok();
        let old_ver = old.map(|o| o.version().to_string()).unwrap_or_default();
        let old_size = old.map(|o| o.isize()).unwrap_or(0);
        let net_change = pkg.isize() - old_size;
        table::write_summary_row(&style::green(pkg.name()), &old_ver, pkg.version(), &fmt::format_size(net_change), &fmt::format_size(pkg.download_size()));
        total_dl += pkg.download_size();
        total_isize += pkg.isize();
        net_upgrade += net_change;
    }
    for pkg in rm.iter() {
        table::write_summary_row(&style::red(pkg.name()), pkg.version(), "", &fmt::format_size(-pkg.isize()), "0 B");
        net_upgrade -= pkg.isize();
    }
    table::write_separator();
    println!(" {} {} \u{00b7} {} {} as dependencies",
        add.len(), table::plural("upgrade", add.len()),
        dep_count, table::plural("package", dep_count));
    println!();
    table::write_summary_footer(&[
        ("Total Download Size", &fmt::format_size(total_dl)),
        ("Total Installed Size", &fmt::format_size(total_isize)),
        ("Net Upgrade Size", &fmt::format_size(net_upgrade)),
    ]);
}

fn show_install_summary(add: &alpm::AlpmList<&alpm::Package>, rm: &alpm::AlpmList<&alpm::Package>, explicit: &[String]) {
    println!();
    table::write_summary_row(&style::bold("Package"), &style::bold("Old Ver"), &style::bold("New Ver"), &style::bold("Net Change"), &style::bold("Size"));
    table::write_separator();
    let mut total_dl: i64 = 0;
    let mut total_isize: i64 = 0;
    let mut dep_count = 0;
    let mut assigned: HashSet<String> = HashSet::new();

    let mut explicit_pkgs: Vec<&alpm::Package> = add.iter().filter(|p| explicit.iter().any(|e| e == p.name())).collect();
    explicit_pkgs.sort_by(|a, b| a.name().cmp(b.name()));

    for pkg in &explicit_pkgs {
        table::write_summary_row(&style::green(pkg.name()), "", pkg.version(), &fmt::format_size(pkg.isize()), &fmt::format_size(pkg.download_size()));
        total_dl += pkg.download_size();
        total_isize += pkg.isize();

        let mut deps: Vec<&str> = Vec::new();
        for dep in pkg.depends() {
            let dn = dep.name();
            if assigned.contains(dn) { continue; }
            if add.iter().any(|p| p.name() == dn) && !explicit.iter().any(|e| e == dn) {
                deps.push(dn);
                assigned.insert(dn.to_string());
            }
        }
        deps.sort();
        for dn in deps {
            dep_count += 1;
            if let Some(dp) = add.iter().find(|p| p.name() == dn) {
                table::write_summary_row(&format!(" {}", style::green(dn)), "", dp.version(), &fmt::format_size(dp.isize()), &fmt::format_size(dp.download_size()));
                total_dl += dp.download_size();
                total_isize += dp.isize();
            }
        }
    }

    let mut total_freed: i64 = 0;
    if !rm.is_empty() {
        println!();
        table::write_summary_row(&style::bold("Package"), &style::bold("Old Ver"), &style::bold("New Ver"), &style::bold("Net Change"), &style::bold("Size"));
        table::write_separator();
        for pkg in rm.iter() {
            table::write_summary_row(&style::red(pkg.name()), pkg.version(), "", &fmt::format_size(-pkg.isize()), "0 B");
            total_freed += pkg.isize();
        }
        table::write_separator();
    } else {
        table::write_separator();
    }
    println!(" {} {} \u{00b7} {} {} as dependencies",
        add.len(), table::plural("package", add.len()),
        dep_count, table::plural("package", dep_count));
    println!();
    table::write_summary_footer(&[
        ("Total Download Size", &fmt::format_size(total_dl)),
        ("Total Installed Size", &fmt::format_size(total_isize)),
        ("Net Upgrade Size", &fmt::format_size(total_isize - total_freed)),
    ]);
}

pub fn run_install(packages: &[String], assume_yes: bool) -> Result<()> {
    if packages.is_empty() { return Err(anyhow::anyhow!("no packages specified")); }
    let config = Config::load()?;
    let assume_yes = assume_yes || config.waka.assume_yes;
    let mut handle = AlpmHandle::new()?;

    let mut targets: Vec<String> = Vec::new();
    for name in packages {
        if handle.find_sync_pkg(name).is_some() { targets.push(name.clone()); }
        else { eprintln!("  {} package '{}' not found", style::yellow("warn:"), name); }
    }
    if targets.is_empty() { return Err(anyhow::anyhow!("no valid packages to install")); }

    let h = &mut handle.handle;

    transaction::trans_init(h, alpm::TransFlag::NONE)?;
    for name in &targets {
        let mut found = false;
        for db in h.syncdbs() {
            if let Ok(pkg) = db.pkg(name.as_str()) {
                h.trans_add_pkg(pkg).map_err(|e| anyhow::anyhow!("add '{name}': {}", e.error))?;
                found = true;
                break;
            }
        }
        if !found { return Err(anyhow::anyhow!("package '{name}' disappeared")); }
    }
    let shared = callbacks::callbacks(h, "Install");
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
    show_install_summary(&add, &rm, &targets);

    if !assume_yes && !render::confirm(&style::bold("\n Do you want to continue? [Y/n] ")) {
        transaction::end_trans(h);
        shared.lock().unwrap().done();
        eprintln!(" {}", style::red("Aborted."));
        return Ok(());
    }

    {
        let mut state = shared.lock().unwrap();
        for pkg in add.iter() {
            state.total_bytes += pkg.download_size();
        }
        state.render();
    }
    let commit_result = h.trans_commit();
    commit_epilogue(&shared, h, commit_result, "Downloading packages")?;
    Ok(())
}

pub fn run_remove(packages: &[String], assume_yes: bool) -> Result<()> {
    if packages.is_empty() { return Err(anyhow::anyhow!("no packages specified")); }
    let config = Config::load()?;
    let assume_yes = assume_yes || config.waka.assume_yes;
    let mut handle = AlpmHandle::new()?;
    let h = &mut handle.handle;
    let mut targets: Vec<String> = Vec::new();
    {
        let local = h.localdb();
        for name in packages {
            match local.pkg(name.as_str()) { Ok(_) => targets.push(name.clone()), Err(_) => eprintln!("  {} '{}' not installed", style::yellow("warn:"), name) }
        }
    }
    if targets.is_empty() { return Err(anyhow::anyhow!("no packages to remove")); }

    let shared = callbacks::callbacks(h, "Remove");
    let flags = alpm::TransFlag::CASCADE | alpm::TransFlag::RECURSE | alpm::TransFlag::NO_SAVE;
    transaction::trans_init(h, flags)?;
    for name in &targets {
        let pkg = h.localdb().pkg(name.as_str()).map_err(|_| anyhow::anyhow!("package '{name}' not found"))?;
        h.trans_remove_pkg(pkg).map_err(|e| anyhow::anyhow!("remove '{name}': {e}"))?;
    }
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

    let rm = h.trans_remove();
    println!();
    table::write_summary_row(&style::bold("Package"), &style::bold("Old Ver"), &style::bold("New Ver"), &style::bold("Net Change"), &style::bold("Size"));
    table::write_separator();
    let mut total_freed: i64 = 0;
    let mut dep_count = 0;
    for pkg in rm.iter() {
        if pkg.reason() == alpm::PackageReason::Depend {
            dep_count += 1;
        }
        table::write_summary_row(&style::red(pkg.name()), pkg.version(), "", &fmt::format_size(-pkg.isize()), "0 B");
        total_freed += pkg.isize();
    }
    table::write_separator();
    println!(" {} {} \u{00b7} {} {} as dependencies",
        rm.len(), table::plural("package", rm.len()),
        dep_count, table::plural("package", dep_count));
    println!();
    table::write_summary_footer(&[
        ("Total Download Size", &fmt::format_size(0)),
        ("Total Removed Size", &fmt::format_size(total_freed)),
        ("Net Upgrade Size", &fmt::format_size(-total_freed)),
    ]);

    if !assume_yes && !render::confirm(&style::bold("\n Do you want to continue? [Y/n] ")) {
        transaction::end_trans(h);
        shared.lock().unwrap().done();
        eprintln!(" {}", style::red("Aborted."));
        return Ok(());
    }

    {
        let mut state = shared.lock().unwrap();
        state.log.push(format!("{}...", style::cyan("Removing packages")));
    }
    let commit_result = h.trans_commit();
    commit_epilogue(&shared, h, commit_result, "Downloading packages")?;
    Ok(())
}

fn run_autoremove_impl(assume_yes: bool) -> Result<()> {
    let config = Config::load()?;
    let assume_yes = assume_yes || config.waka.assume_yes;
    let mut handle = AlpmHandle::new()?;
    let h = &mut handle.handle;

    let orphans: Vec<(String, String)> = {
        let local = h.localdb();
        local.pkgs().iter()
            .filter(|p| p.reason() == alpm::PackageReason::Depend)
            .filter(|p| p.required_by().is_empty())
            .map(|p| (p.name().to_string(), p.version().to_string()))
            .collect()
    };

    if orphans.is_empty() {
        println!(" {} no orphaned packages found", style::green("\u{2713}"));
        return Ok(());
    }

    eprintln!(" {} {} {}", style::yellow("warn"), orphans.len(), table::plural("orphaned package", orphans.len()));
    for (name, ver) in &orphans {
        println!("  {} ({})", style::red(name), style::dim(ver));
    }

    eprintln!(" {} {} This will remove orphaned packages. This action can delete critical dependencies if they are no longer required.",
        style::red("WARNING"), style::bold("PROCEED WITH CAUTION"));

    if !assume_yes && !render::confirm(&style::bold("\n Do you want to continue? [Y/n] ")) {
        eprintln!(" {}", style::red("Aborted."));
        return Ok(());
    }

    let shared = callbacks::callbacks(h, "Autoremove");
    shared.lock().unwrap().render();

    transaction::trans_init(h, alpm::TransFlag::CASCADE | alpm::TransFlag::RECURSE)?;
    for (name, _) in &orphans {
        let pkg = h.localdb().pkg(name.as_str())
            .map_err(|_| anyhow::anyhow!("package '{name}' not found"))?;
        h.trans_remove_pkg(pkg)
            .map_err(|e| anyhow::anyhow!("remove '{name}': {e}"))?;
    }
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

    {
        let mut state = shared.lock().unwrap();
        state.log.push(format!("{}...", style::cyan("Removing orphaned packages")));
    }
    let commit_result = h.trans_commit();
    commit_epilogue(&shared, h, commit_result, "Removing orphaned packages")?;
    Ok(())
}

pub fn run_autoremove(assume_yes: bool) -> Result<()> {
    run_autoremove_impl(assume_yes)
}

pub fn run_clean(all: bool, assume_yes: bool) -> Result<()> {
    use std::fs;

    let cache = constants::cache_dir();
    if !cache.exists() {
        eprintln!(" {} cache directory not found", style::yellow("warn"));
        return Ok(());
    }

    if !assume_yes && !render::confirm(&style::bold("\n Clear package cache? [Y/n] ")) {
        eprintln!(" {}", style::red("Aborted."));
        return Ok(());
    }

    let handle = AlpmHandle::new()?;
    let local = handle.handle.localdb();
    let installed: HashSet<String> = local.pkgs().iter()
        .map(|p| format!("{}-{}", p.name(), p.version()))
        .collect();

    let mut removed = 0u32;
    for entry in fs::read_dir(&cache)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() { continue; }
        let fname = path.file_name().unwrap().to_string_lossy().to_string();

        if fname.ends_with(".db") || fname.ends_with(".db.sig") {
            if all {
                fs::remove_file(&path)?;
                removed += 1;
            }
            continue;
        }
        if fname.ends_with(".sig") { continue; }

        if let Some(base) = fname.split(".pkg.tar.").next() {
            let without_arch = base.rsplit_once('-').map(|(p, _)| p).unwrap_or(base);
            if !installed.contains(without_arch) {
                fs::remove_file(&path)?;
                removed += 1;
            }
        }
    }

    if removed > 0 {
        println!(" {} removed {} {} from cache", style::green("\u{2713}"), removed, table::plural("file", removed as usize));
    } else {
        println!(" {} cache is already clean", style::green("\u{2713}"));
    }
    Ok(())
}

pub fn run_history(id: Option<String>) -> Result<()> {
    let log_path = constants::pacman_log();
    if !log_path.exists() {
        eprintln!(" {} no pacman log found", style::yellow("warn"));
        return Ok(());
    }
    let content = std::fs::read_to_string(&log_path)?;

    let mut entries: Vec<String> = Vec::new();
    for line in content.lines() {
        if line.contains("[ALPM]") {
            entries.push(line.to_string());
        }
    }

    if entries.is_empty() {
        eprintln!(" {} no transaction history found", style::yellow("warn"));
        return Ok(());
    }

    if let Some(tid) = id {
        let txn_entries: Vec<&str> = entries.iter()
            .filter(|l| l.contains("transaction started"))
            .map(|l| l.as_str())
            .collect();
        let idx: usize = match tid.parse() {
            Ok(n) => n,
            Err(_) => {
                eprintln!(" {}: '{}' is not a valid transaction number", style::red("error"), tid);
                return Ok(());
            }
        };
        if idx == 0 || idx > txn_entries.len() {
            eprintln!(" {} transaction id out of range (1-{})", style::red("error"), txn_entries.len());
            return Ok(());
        }
        let txn_line = txn_entries[idx - 1];
        let txn_time = txn_line.trim_start_matches('[').split(']').next().unwrap_or("");
        println!(" {} {}", style::bold("Transaction"), idx);
        println!(" {}", style::dim(txn_time));
        for line in &entries {
            if line.contains("[ALPM]") && line.as_str() >= txn_line {
                if line.contains("transaction started") && line != txn_line {
                    break;
                }
                println!(" {}", line.trim_start_matches('[').split_once(']').map(|x| x.1).unwrap_or("").trim());
            }
        }
    } else {
        let mut count = 0;
        let mut prev_date = String::new();
        for line in &entries {
            let date = line.trim_start_matches('[').split('T').next().unwrap_or("");
            if date != prev_date {
                if !prev_date.is_empty() { println!(); }
                prev_date = date.to_string();
            }
            if line.contains("transaction started") {
                count += 1;
                let date_str = line.trim_start_matches('[').split(']').next().unwrap_or("");
                println!(" {}  {} {}", style::bold(&format!("{:>3}", count)), style::dim(date_str), line.split("[ALPM] ").nth(1).unwrap_or(""));
            }
        }
        println!();
        println!(" {} {} in history", style::green("\u{2713}"), count);

        if entries.iter().any(|l| l.contains("transaction started")) {
            println!("  run 'waka history -i <id>' for details");
        }
    }

    Ok(())
}

pub fn run_fetch() -> Result<()> {
    println!(" {} fetching mirrors...", style::cyan("info"));
    println!();
    println!(" {}", style::bold("To configure mirrors, you can:"));
    println!("  {} install and run 'reflector' to auto-rank mirrors", style::dim("\u{2022}"));
    println!("  {} manually edit /etc/pacman.d/mirrorlist", style::dim("\u{2022}"));
    println!("  {} run 'waka fetch' requires root permissions", style::dim("\u{2022}"));
    println!();
    println!(" {} fetching mirrorlist from archlinux.org...", style::cyan("info"));

    let tmp_dir = std::env::temp_dir();
    let tmp_path = tmp_dir.join(format!("waka_mirrorlist.{}", std::process::id()));
    let tmp_str = tmp_path.to_string_lossy().to_string();

    let status = std::process::Command::new("/usr/bin/curl")
        .args(["-sSL", "--fail", "https://archlinux.org/mirrorlist/?country=all&protocol=https&use_mirror_status=on"])
        .arg("-o")
        .arg(&tmp_str)
        .status();

    match status {
        Ok(s) if s.success() => {
            println!(" {} downloaded to {}", style::green("\u{2713}"), &tmp_str);
            println!(" {} run 'rankmirrors -n 10 {} > /etc/pacman.d/mirrorlist' as root to apply", style::cyan("info"), &tmp_str);
            Ok(())
        }
        Ok(_) => {
            let _ = std::fs::remove_file(&tmp_path);
            Err(anyhow::anyhow!("failed to download mirrorlist (is curl installed?)"))
        }
        Err(e) => {
            Err(anyhow::anyhow!("failed to run curl: {e}"))
        }
    }
}

pub fn run_upgrade(assume_yes: bool, autoremove: bool) -> Result<()> {
    let config = Config::load()?;
    let assume_yes = assume_yes || config.waka.assume_yes;
    let mut handle = AlpmHandle::new()?;
    let h = &mut handle.handle;

    let bytes = Arc::new(Mutex::new(HashMap::<String, (i64, i64)>::new()));
    for db in h.syncdbs() {
        bytes.lock().unwrap().insert(db.name().to_string(), (0, 0));
    }
    let cb = Arc::clone(&bytes);
    h.set_dl_cb(cb, |fname, event, bytes| {
        let fname = fname.rsplit('/').next().unwrap_or(fname);
        let repo = fname.strip_suffix(".db.part").unwrap_or(fname).strip_suffix(".db").unwrap_or(fname).to_string();
        if let Some(b) = bytes.lock().unwrap().get_mut(&repo) {
            match event.event() {
                alpm::DownloadEvent::Progress(p) => *b = (p.downloaded, p.total),
                alpm::DownloadEvent::Completed(c) => *b = (c.total, c.total),
                _ => {}
            }
        }
    });

    h.syncdbs_mut().update(false).map_err(|e| anyhow::anyhow!("database update failed: {}", e))?;

    {
        use crate::display::{draw, format as fmt};
        let w = draw::term_width();
        let b = bytes.lock().unwrap();
        let mut names: Vec<String> = h.syncdbs().iter().map(|db| db.name().to_string()).collect();
        names.sort();
        let max_nw = names.iter().map(|n| draw::display_width(n)).max().unwrap_or(1).max(1);

        let mut size_strs: Vec<String> = Vec::new();
        for name in &names {
            let (dl, _) = b.get(name).copied().unwrap_or((0, 0));
            size_strs.push(if dl > 0 { fmt::format_size(dl) } else { "0 B".to_string() });
        }
        let max_sw = size_strs.iter().map(|s| draw::display_width(s)).max().unwrap_or(3);

        for (name, size_str) in names.iter().zip(size_strs.iter()) {
            let (dl, tot) = b.get(name).copied().unwrap_or((0, 0));
            let pct = if tot > 0 { dl as f64 / tot as f64 * 100.0 } else { 100.0 };
            let padded_size = format!("{:>width$}", size_str, width = max_sw);
            let npad = " ".repeat(max_nw.saturating_sub(draw::display_width(name)));
            let name_prefix = format!(" {}{} ", name, npad);
            let max_bar = ((w as f64 * 0.25) as usize).min(50);
            let bar_w = (w.saturating_sub(draw::display_width(&name_prefix) + 3 + max_sw + 2)).min(max_bar).max(10);
            let filled = ((pct / 100.0) * bar_w as f64) as usize;
            let bar = format!("{}{}", "━".repeat(filled), "─".repeat(bar_w.saturating_sub(filled)));
            let block = format!("[{}] {} ", bar, padded_size);
            let pad = w.saturating_sub(draw::display_width(&name_prefix) + draw::display_width(&block)).max(2);
            println!("{}{}{}", name_prefix, " ".repeat(pad), block);
        }
    }

    transaction::trans_init(h, alpm::TransFlag::NONE)?;
    h.sync_sysupgrade(false).map_err(|e| anyhow::anyhow!("sysupgrade: {e}"))?;
    let add_count = h.trans_add().len();
    if add_count == 0 { transaction::end_trans(h); println!("{} All packages are up to date", style::green("\u{2713}")); return Ok(()); }

    let title = format!("Upgrade ({} packages)", add_count);
    let shared = callbacks::callbacks(h, &title);
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
        return Ok(());
    }

    {
        let mut state = shared.lock().unwrap();
        for pkg in add.iter() {
            state.total_bytes += pkg.download_size();
        }
        state.render();
    }
    let commit_result = h.trans_commit();
    commit_epilogue(&shared, h, commit_result, "Downloading packages")?;

    if autoremove {
        run_autoremove_impl(true)?;
    }

    Ok(())
}
