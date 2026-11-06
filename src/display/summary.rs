use crate::display::{format as fmt, style, table};
use std::collections::HashSet;

pub(crate) fn show_upgrade_summary(add: &alpm::AlpmList<&alpm::Package>, rm: &alpm::AlpmList<&alpm::Package>, local: &alpm::Db) {
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
        let old_ver = old.map(|o| o.version().to_string()).unwrap_or_default();
        let old_size = old.map(|o| o.isize()).unwrap_or(0);
        let net_change = pkg.isize() - old_size;
        table::write_summary_row(&table::package_cell(pkg.name(), dependency), &old_ver, pkg.version(), &fmt::format_size(net_change), &fmt::format_size(pkg.download_size()));
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


pub(crate) fn show_install_summary(add: &alpm::AlpmList<&alpm::Package>, rm: &alpm::AlpmList<&alpm::Package>, explicit: &[String]) {
    let add: Vec<&alpm::Package> = add.iter().collect();
    let rm: Vec<&alpm::Package> = rm.iter().collect();
    render_install_summary(&add, &rm, explicit);
}

fn render_install_summary(add: &[&alpm::Package], rm: &[&alpm::Package], explicit: &[String]) {
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
        table::write_summary_row(&table::package_cell(pkg.name(), false), "", pkg.version(), &fmt::format_size(pkg.isize()), &fmt::format_size(pkg.download_size()));
        total_dl += pkg.download_size();
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
                table::write_summary_row(&table::package_cell(name, true), "", dep.version(), &fmt::format_size(dep.isize()), &fmt::format_size(dep.download_size()));
                total_dl += dep.download_size();
                total_isize += dep.isize();
            }
        }
    }

    let mut remaining: Vec<&alpm::Package> = add
        .iter()
        .copied()
        .filter(|p| !explicit.iter().any(|e| e == p.name()) && !assigned.contains(p.name()))
        .collect();
    remaining.sort_by(|a, b| a.name().cmp(b.name()));
    for pkg in remaining {
        dep_count += 1;
        table::write_summary_row(
            &table::package_cell(pkg.name(), true),
            "",
            pkg.version(),
            &fmt::format_size(pkg.isize()),
            &fmt::format_size(pkg.download_size()),
        );
        total_dl += pkg.download_size();
        total_isize += pkg.isize();
    }

    let mut total_freed: i64 = 0;
    if !rm.is_empty() {
        println!();
        table::write_summary_row(&style::bold("Package"), &style::bold("Old Ver"), &style::bold("New Ver"), &style::bold("Net Change"), &style::bold("Size"));
        table::write_separator();
        for pkg in rm {
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
