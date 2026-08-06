use crate::alpm::handle::AlpmHandle;
use crate::display::{style, table};
use anyhow::Result;

pub fn search_packages(handle: &AlpmHandle, query: &str) -> Result<()> {
    if query.is_empty() {
        return Err(anyhow::anyhow!("no search terms specified"));
    }
    let q = query.to_lowercase();
    let mut results: Vec<&alpm::Package> = Vec::new();

    for db in handle.handle.syncdbs() {
        for pkg in db.pkgs() {
            if pkg.name().to_lowercase().contains(&q) || pkg.desc().unwrap_or("").to_lowercase().contains(&q) {
                results.push(pkg);
            }
        }
    }
    results.sort_by(|a, b| a.name().cmp(b.name()));

    if results.is_empty() {
        eprintln!(" {}: no packages found matching '{}'", style::yellow("warn"), style::color(query, style::Color::Green));
        return Ok(());
    }

    for pkg in &results {
        let installed = handle.find_local_pkg(pkg.name()).is_some();
        print_pkg_search(pkg, installed);
    }
    println!("{} {} {} found.", style::green("\u{2713}"), results.len(), table::plural("package", results.len()));
    Ok(())
}

fn print_pkg_search(pkg: &alpm::Package, installed: bool) {
    let name = pkg.name();
    let version = pkg.version();
    let desc = pkg.desc().unwrap_or("").to_string();
    let repo = pkg.db()
        .map(|db| style::bold(db.name()))
        .unwrap_or_else(|| style::bold("any"));

    let tag = if installed {
        format!(" {}", style::color("[installed]", style::Color::Yellow))
    } else {
        String::new()
    };
    let name_ver = format!("{repo}/{name} {ver}{tag}",
        repo = repo, name = style::color(name, style::Color::Green), ver = style::color(version, style::Color::Blue));
    println!("{}", name_ver);

    if !desc.is_empty() {
        println!("  {}", desc);
    }

    let size_str = crate::display::format::format_size(pkg.size());
    println!("  {}", style::dim(&size_str));
}
