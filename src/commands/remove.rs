use crate::alpm::{callbacks, handle::AlpmHandle, transaction};
use crate::commands::common::{commit_epilogue, detail_from_prep_err};
use crate::config::Config;
use crate::display::{draw, format as fmt, render, style, table};
use crate::sys::require_root;
use anyhow::Result;

/// `-Rcns`: remove the targets, packages that depend on them, unneeded
/// dependencies, and config files.
pub fn remove_flags() -> alpm::TransFlag {
    alpm::TransFlag::CASCADE | alpm::TransFlag::RECURSE | alpm::TransFlag::NO_SAVE
}

pub fn run(packages: &[String], assume_yes: bool) -> Result<()> {
    if packages.is_empty() { return Err(anyhow::anyhow!("no packages specified")); }
    require_root()?;
    let config = Config::load()?;
    let assume_yes = assume_yes || config.waka.assume_yes;
    render::require_confirmation_possible(assume_yes)?;
    let mut handle = AlpmHandle::new()?;
    let h = &mut handle.handle;
    let mut targets: Vec<String> = Vec::new();
    let mut missing: Vec<String> = Vec::new();
    {
        let local = h.localdb();
        for name in packages {
            match local.pkg(name.as_str()) {
                Ok(_) => targets.push(name.clone()),
                Err(_) => missing.push(name.clone()),
            }
        }
    }
    if !missing.is_empty() {
        return Err(anyhow::anyhow!("target not found: {}", missing.join(", ")));
    }
    if targets.is_empty() { return Err(anyhow::anyhow!("no packages to remove")); }

    let shared = callbacks::callbacks(h, "Remove", assume_yes);
    let flags = remove_flags();
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

    // Do not let the commit log erase the summary that is printed next. The
    // prepare box stays on screen; only its row count is reset.
    shared.lock().unwrap().rendered = 0;

    let rm = h.trans_remove();
    if let Some(held) = rm
        .iter()
        .find(|pkg| handle.hold_pkgs.iter().any(|pattern| crate::sys::glob_match(pattern, pkg.name())))
        .map(|pkg| pkg.name().to_string())
    {
        transaction::end_trans(h);
        shared.lock().unwrap().done();
        return Err(anyhow::anyhow!(
            "refusing to remove held package '{held}' (HoldPkg in pacman.conf)"
        ));
    }
    println!();
    table::write_summary_row(&style::bold("Package"), &style::bold("Old Ver"), &style::bold("New Ver"), &style::bold("Net Change"), &style::bold("Size"));
    table::write_separator();
    let mut total_freed: i64 = 0;
    let mut dep_count = 0;
    for pkg in rm.iter() {
        if pkg.reason() == alpm::PackageReason::Depend {
            dep_count += 1;
        }
        table::write_summary_row(&style::red(&draw::sanitize(pkg.name())), &draw::sanitize(pkg.version().as_str()), "", &fmt::format_size(-pkg.isize()), "0 B");
        total_freed += pkg.isize();
    }
    table::write_separator();
    println!(" {} {} \u{00b7} {} {} as dependencies",
        rm.len(), table::plural("package", rm.len()),
        dep_count, table::plural("package", dep_count));
    println!();
    table::write_summary_footer(&[
        ("Total Removed Size", &fmt::format_size(total_freed)),
        ("Net Upgrade Size", &fmt::format_size(-total_freed)),
    ]);

    if !assume_yes && !render::confirm(&style::bold("\n Do you want to continue? [Y/n] ")) {
        transaction::end_trans(h);
        shared.lock().unwrap().done();
        eprintln!(" {}", style::red("Aborted."));
        return Err(crate::aborted());
    }

    {
        let mut state = shared.lock().unwrap();
        state.log.push(format!("{}...", style::cyan("Removing packages")));
    }
    transaction::begin_commit(h);
    let commit_result = h.trans_commit();
    commit_epilogue(&shared, h, commit_result, "Downloading packages")?;
    Ok(())
}

