use crate::alpm::handle::AlpmHandle;
use crate::display::{style, table};
use anyhow::Result;


pub fn run_list(handle: &AlpmHandle, installed: bool, upgradable: bool, patterns: &[String]) -> Result<()> {
    let local = handle.handle.localdb();

    fn repo_for(syncdbs: &[&alpm::Db], pkg: &alpm::Package) -> String {
        syncdbs.iter()
            .find(|db| db.pkg(pkg.name()).is_ok())
            .map(|db| db.name().to_string())
            .unwrap_or_else(|| "aur".to_string())
    }

    if upgradable {
        let mut results: Vec<(&alpm::Package, String)> = Vec::new();
        for pkg in local.pkgs() {
            let name = pkg.name().to_lowercase();
            if !patterns.is_empty() && !patterns.iter().any(|p| name.contains(&p.to_lowercase())) {
                continue;
            }
            if let Some(sync_pkg) = pkg.sync_new_version(handle.handle.syncdbs()) {
                let repo = sync_pkg.db().map(|db| db.name().to_string()).unwrap_or_default();
                results.push((pkg, repo));
            }
        }
        results.sort_by(|a, b| a.1.cmp(&b.1).then(a.0.name().cmp(b.0.name())));

        if results.is_empty() {
            println!(" {} all packages are up to date", style::green("\u{2713}"));
            return Ok(());
        }

        for (pkg, repo) in &results {
            let version = pkg.version();
            println!(" {}/{} {}", style::bold(repo), style::color(pkg.name(), style::Color::Green), style::color(version.as_str(), style::Color::Blue));
        }
        println!(" {} {} {} found.", style::green("\u{2713}"), results.len(), table::plural("package", results.len()));
    } else {
        let mut results: Vec<(&alpm::Package, String)> = Vec::new();
        for pkg in local.pkgs() {
            let name = pkg.name().to_lowercase();
            if !patterns.is_empty() && !patterns.iter().any(|p| name.contains(&p.to_lowercase())) {
                continue;
            }
            if !installed && pkg.reason() != alpm::PackageReason::Explicit {
                continue;
            }
            let syncdbs: Vec<&alpm::Db> = handle.handle.syncdbs().iter().collect();
            let repo = repo_for(&syncdbs, pkg);
            results.push((pkg, repo));
        }
        results.sort_by(|a, b| a.1.cmp(&b.1).then(a.0.name().cmp(b.0.name())));

        if results.is_empty() {
            eprintln!(" {}: no packages found", style::yellow("warn"));
            return Ok(());
        }

        for (pkg, repo) in &results {
            let version = pkg.version();
            println!(" {}/{} {}", style::bold(repo), style::color(pkg.name(), style::Color::Green), style::color(version.as_str(), style::Color::Blue));
        }
        println!(" {} {} {} found.", style::green("\u{2713}"), results.len(), table::plural("package", results.len()));
    }

    Ok(())
}
