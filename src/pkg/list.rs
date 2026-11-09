use crate::alpm::handle::AlpmHandle;
use crate::display::{style, table};
use anyhow::Result;

pub const UNKNOWN_REPO: &str = "aur";


pub fn upgrade_line(repo: &str, name: &str, old: &str, new: &str) -> String {
    format!(
        " {}/{} {} \u{2192} {}",
        style::bold(repo),
        style::color(name, style::Color::Green),
        style::color(old, style::Color::Blue),
        style::color(new, style::Color::Blue),
    )
}

fn repo_of(pkg: &alpm::Package) -> &str {
    pkg.db().map(|db| db.name()).unwrap_or(UNKNOWN_REPO)
}

pub fn run_list(handle: &AlpmHandle, installed: bool, upgradable: bool, patterns: &[String]) -> Result<()> {
    let local = handle.handle.localdb();

    fn repo_for(syncdbs: &[&alpm::Db], pkg: &alpm::Package) -> String {
        syncdbs.iter()
            .find(|db| db.pkg(pkg.name()).is_ok())
            .map(|db| db.name().to_string())
            .unwrap_or_else(|| UNKNOWN_REPO.to_string())
    }

    if upgradable {
        let mut results: Vec<(&alpm::Package, &alpm::Package)> = Vec::new();
        for pkg in local.pkgs() {
            let name = pkg.name().to_lowercase();
            if !patterns.is_empty() && !patterns.iter().any(|p| name.contains(&p.to_lowercase())) {
                continue;
            }
            if let Some(sync_pkg) = pkg.sync_new_version(handle.handle.syncdbs()) {
                results.push((pkg, sync_pkg));
            }
        }
        results.sort_by(|a, b| repo_of(a.1).cmp(repo_of(b.1)).then(a.0.name().cmp(b.0.name())));

        if results.is_empty() {
            println!(" {} all packages are up to date", style::green("\u{2713}"));
            return Ok(());
        }

        for (local_pkg, sync_pkg) in &results {
            println!("{}", upgrade_line(
                repo_of(sync_pkg),
                local_pkg.name(),
                local_pkg.version().as_str(),
                sync_pkg.version().as_str(),
            ));
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
            return Err(anyhow::anyhow!("no packages found"));
        }

        for (pkg, repo) in &results {
            let version = pkg.version();
            println!(" {}/{} {}", style::bold(repo), style::color(pkg.name(), style::Color::Green), style::color(version.as_str(), style::Color::Blue));
        }
        println!(" {} {} {} found.", style::green("\u{2713}"), results.len(), table::plural("package", results.len()));
    }

    Ok(())
}
