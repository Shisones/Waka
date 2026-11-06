use crate::alpm::{callbacks, handle::AlpmHandle, transaction};
use crate::commands::common::{commit_epilogue, detail_from_prep_err};
use crate::config::Config;
use crate::display::{format as fmt, render, style, table};
use crate::sys::require_root;
use anyhow::Result;

pub fn run(assume_yes: bool) -> Result<()> {
    require_root()?;
    let config = Config::load()?;
    let assume_yes = assume_yes || config.waka.assume_yes;
    render::require_confirmation_possible(assume_yes)?;
    let mut handle = AlpmHandle::new()?;
    let h = &mut handle.handle;

    let orphans: Vec<(String, String, i64)> = {
        let local = h.localdb();
        local.pkgs().iter()
            .filter(|p| p.reason() == alpm::PackageReason::Depend)
            .filter(|p| p.required_by().is_empty())
            .map(|p| (p.name().to_string(), p.version().to_string(), p.isize()))
            .collect()
    };

    if orphans.is_empty() {
        println!(" {} no orphaned packages found", style::green("\u{2713}"));
        return Ok(());
    }

    eprintln!(" {} {} {}", style::yellow("warn"), orphans.len(), table::plural("orphaned package", orphans.len()));
    for (name, ver, _) in &orphans {
        println!("  {} ({})", style::red(name), style::dim(ver));
    }
    let freed: i64 = orphans.iter().map(|(_, _, size)| *size).sum();
    println!("  {} will be freed", fmt::format_size(freed));

    eprintln!(" {} {} This will remove orphaned packages. This action can delete critical dependencies if they are no longer required.",
        style::red("WARNING"), style::bold("PROCEED WITH CAUTION"));

    if !assume_yes && !render::confirm(&style::bold("\n Do you want to continue? [Y/n] ")) {
        eprintln!(" {}", style::red("Aborted."));
        return Err(crate::aborted());
    }

    let shared = callbacks::callbacks(h, "Autoremove", assume_yes);
    shared.lock().unwrap().render();

    transaction::trans_init(h, alpm::TransFlag::CASCADE | alpm::TransFlag::RECURSE)?;
    for (name, _, _) in &orphans {
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

    if let Some(held) = h
        .trans_remove()
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

    let requested: std::collections::HashSet<&str> =
        orphans.iter().map(|(name, _, _)| name.as_str()).collect();
    let extras: Vec<String> = h
        .trans_remove()
        .iter()
        .filter(|pkg| !requested.contains(pkg.name()))
        .map(|pkg| pkg.name().to_string())
        .collect();
    if !extras.is_empty() && !assume_yes {
        shared.lock().unwrap().done();
        println!();
        println!(" Additional packages will be removed:");
        for name in &extras {
            println!("  {}", style::red(name));
        }
        if !render::confirm(&style::bold("\n Remove them too? [Y/n] ")) {
            transaction::end_trans(h);
            eprintln!(" {}", style::red("Aborted."));
            return Err(crate::aborted());
        }
    }

    {
        let mut state = shared.lock().unwrap();
        state.log.push(format!("{}...", style::cyan("Removing orphaned packages")));
    }
    let commit_result = h.trans_commit();
    commit_epilogue(&shared, h, commit_result, "Removing orphaned packages")?;
    Ok(())
}

