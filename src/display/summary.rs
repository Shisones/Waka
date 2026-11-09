use crate::display::{draw, format as fmt, style, table};
use std::collections::HashSet;

pub(crate) fn show_upgrade_summary(add: &alpm::AlpmList<&alpm::Package>, rm: &alpm::AlpmList<&alpm::Package>, local: &alpm::Db, cachedirs: &[String]) {
    println!();
    table::write_summary_row(&style::bold("Package"), &style::bold("Old Ver"), &style::bold("New Ver"), &style::bold("Net Change"), &style::bold("Size"));
    table::write_separator();
    let mut total_dl: i64 = 0;
    let mut total_isize: i64 = 0;
    let mut net_upgrade: i64 = 0;
    let mut dep_count = 0;
    for pkg in add.iter() {
        let dependency = pkg.reason() == alpm::PackageReason::Depend;
        if dependency {
            dep_count += 1;
        }
        let old = local.pkg(pkg.name()).ok();
        let old_ver = old.map(|o| draw::sanitize(o.version().as_str())).unwrap_or_default();
        let old_size = old.map(|o| o.isize()).unwrap_or(0);
        let net_change = pkg.isize() - old_size;
        let remaining = remaining_download(pkg, cachedirs);
        table::write_summary_row(&table::package_cell(pkg.name(), dependency), &old_ver, &draw::sanitize(pkg.version().as_str()), &fmt::format_size(net_change), &fmt::format_size(remaining));
        total_dl += remaining;
        total_isize += pkg.isize();
        net_upgrade += net_change;
    }
    for pkg in rm.iter() {
        table::write_summary_row(&style::red(&draw::sanitize(pkg.name())), &draw::sanitize(pkg.version().as_str()), "", &fmt::format_size(-pkg.isize()), "0 B");
        net_upgrade -= pkg.isize();
    }
    table::write_separator();
    println!("{}", count_line(add.len(), dep_count, "upgrade"));
    println!();
    table::write_summary_footer(&[
        ("Total Download Size", &fmt::format_size(total_dl)),
        ("Total Installed Size", &fmt::format_size(total_isize)),
        ("Net Upgrade Size", &fmt::format_size(net_upgrade)),
    ]);
}


pub(crate) fn show_install_summary(add: &alpm::AlpmList<&alpm::Package>, rm: &alpm::AlpmList<&alpm::Package>, explicit: &[String], cachedirs: &[String]) {
    let add: Vec<&alpm::Package> = add.iter().collect();
    let rm: Vec<&alpm::Package> = rm.iter().collect();
    render_install_summary(&add, &rm, explicit, cachedirs);
}

fn render_install_summary(add: &[&alpm::Package], rm: &[&alpm::Package], explicit: &[String], cachedirs: &[String]) {
    println!();
    table::write_summary_row(&style::bold("Package"), &style::bold("Old Ver"), &style::bold("New Ver"), &style::bold("Net Change"), &style::bold("Size"));
    table::write_separator();
    let mut total_dl: i64 = 0;
    let mut total_isize: i64 = 0;
    let mut dep_count = 0;
    let mut assigned: HashSet<String> = HashSet::new();

    let mut explicit_pkgs: Vec<&alpm::Package> = add.iter().copied().filter(|p| explicit.iter().any(|e| e == p.name())).collect();
    explicit_pkgs.sort_by(|a, b| a.name().cmp(b.name()));

    for pkg in &explicit_pkgs {
        let remaining = remaining_download(pkg, cachedirs);
        table::write_summary_row(&table::package_cell(pkg.name(), false), "", &draw::sanitize(pkg.version().as_str()), &fmt::format_size(pkg.isize()), &fmt::format_size(remaining));
        total_dl += remaining;
        total_isize += pkg.isize();

        let mut deps: Vec<&str> = Vec::new();
        for dep in pkg.depends() {
            let name = dep.name();
            if assigned.contains(name) {
                continue;
            }
            if add.iter().any(|p| p.name() == name) && !explicit.iter().any(|e| e == name) {
                deps.push(name);
                assigned.insert(name.to_string());
            }
        }
        deps.sort();
        for name in deps {
            dep_count += 1;
            if let Some(dep) = add.iter().find(|p| p.name() == name) {
                let remaining = remaining_download(dep, cachedirs);
                table::write_summary_row(&table::package_cell(name, true), "", &draw::sanitize(dep.version().as_str()), &fmt::format_size(dep.isize()), &fmt::format_size(remaining));
                total_dl += remaining;
                total_isize += dep.isize();
            }
        }
    }

    let names: Vec<&str> = add.iter().map(|pkg| pkg.name()).collect();
    for name in remaining_install_names(&names, explicit, &assigned) {
        dep_count += 1;
        if let Some(pkg) = add.iter().find(|pkg| pkg.name() == name) {
            let remaining = remaining_download(pkg, cachedirs);
            table::write_summary_row(
                &table::package_cell(name, true),
                "",
                &draw::sanitize(pkg.version().as_str()),
                &fmt::format_size(pkg.isize()),
                &fmt::format_size(remaining),
            );
            total_dl += remaining;
            total_isize += pkg.isize();
        }
    }

    let mut total_freed: i64 = 0;
    if !rm.is_empty() {
        println!();
        table::write_summary_row(&style::bold("Package"), &style::bold("Old Ver"), &style::bold("New Ver"), &style::bold("Net Change"), &style::bold("Size"));
        table::write_separator();
        for pkg in rm {
            table::write_summary_row(&style::red(&draw::sanitize(pkg.name())), &draw::sanitize(pkg.version().as_str()), "", &fmt::format_size(-pkg.isize()), "0 B");
            total_freed += pkg.isize();
        }
        table::write_separator();
    } else {
        table::write_separator();
    }
    println!("{}", count_line(add.len(), dep_count, "package"));
    println!();
    table::write_summary_footer(&[
        ("Total Download Size", &fmt::format_size(total_dl)),
        ("Total Installed Size", &fmt::format_size(total_isize)),
        ("Net Upgrade Size", &fmt::format_size(total_isize - total_freed)),
    ]);
}

/// Bytes still missing from the cache for this package.
pub(crate) fn remaining_download(pkg: &alpm::Package, cachedirs: &[String]) -> i64 {
    let total = pkg.download_size();
    if total <= 0 {
        return total;
    }
    let version = pkg
        .version()
        .as_str()
        .split_once(':')
        .map(|(_, value)| value)
        .unwrap_or(pkg.version().as_str());
    let arch = pkg.arch().unwrap_or("any");
    let prefix = format!("{}-{}-{}.pkg.tar.", pkg.name(), version, arch);
    let mut part = 0u64;
    let mut complete = false;
    for dir in cachedirs {
        let Ok(entries) = std::fs::read_dir(dir) else { continue; };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with(&prefix) || name.ends_with(".sig") {
                continue;
            }
            if name.ends_with(".part") {
                part = part.max(entry.metadata().map(|m| m.len()).unwrap_or(0));
            } else {
                complete = true;
            }
        }
    }
    remaining_bytes(total, part, complete)
}

pub(crate) fn count_line(total: usize, dependencies: usize, noun: &str) -> String {
    if dependencies == 0 {
        format!(" {} {}", total, table::plural(noun, total))
    } else {
        format!(
            " {} {} \u{00b7} {} {} as dependencies",
            total,
            table::plural(noun, total),
            dependencies,
            table::plural("package", dependencies)
        )
    }
}

pub(crate) fn remaining_install_names<'a>(
    names: &[&'a str],
    explicit: &[String],
    assigned: &HashSet<String>,
) -> Vec<&'a str> {
    let mut remaining: Vec<&str> = names
        .iter()
        .copied()
        .filter(|name| !explicit.iter().any(|e| e == name) && !assigned.contains(*name))
        .collect();
    remaining.sort_unstable();
    remaining
}

pub(crate) fn remaining_bytes(total: i64, part: u64, complete: bool) -> i64 {
    if total <= 0 {
        return total;
    }
    if complete {
        0
    } else {
        (total - part as i64).max(0)
    }
}

#[cfg(test)]
mod tests {
    use super::{count_line, remaining_bytes, remaining_install_names};
    use std::collections::HashSet;

    #[test]
    fn cached_archive_has_zero_remaining_download() {
        assert_eq!(remaining_bytes(100, 0, true), 0);
    }

    #[test]
    fn partial_archive_subtracts_cached_bytes() {
        assert_eq!(remaining_bytes(100, 40, false), 60);
        assert_eq!(remaining_bytes(100, 140, false), 0);
    }

    #[test]
    fn uncached_archive_keeps_full_size() {
        assert_eq!(remaining_bytes(100, 0, false), 100);
    }

    #[test]
    fn zero_dependencies_are_not_mentioned() {
        assert_eq!(count_line(1, 0, "package"), " 1 package");
    }

    #[test]
    fn dependencies_are_counted() {
        assert_eq!(
            count_line(2, 1, "upgrade"),
            " 2 upgrades \u{00b7} 1 package as dependencies"
        );
    }

    #[test]
    fn transitive_packages_are_listed() {
        let names = ["a", "b", "c"];
        let explicit = vec!["a".to_string()];
        let mut assigned = HashSet::new();
        assigned.insert("b".to_string());
        assert_eq!(remaining_install_names(&names, &explicit, &assigned), vec!["c"]);
    }
}
