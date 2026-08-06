use crate::alpm::handle::AlpmHandle;
use crate::display::{draw, style};
use anyhow::Result;

pub fn show_packages(handle: &AlpmHandle, packages: &[String]) -> Result<()> {
    for (i, name) in packages.iter().enumerate() {
        let pkg = handle
            .find_local_pkg(name)
            .or_else(|| handle.find_sync_pkg(name));

        match pkg {
            Some(pkg) => print_pkg_info(pkg, i),
            None => eprintln!("{}", style::color(&format!("package '{name}' not found"), style::Color::Red)),
        }
    }
    Ok(())
}

fn add_deps(lines: &mut Vec<String>, label: &str, deps: &[String]) {
    if deps.is_empty() { return; }
    let dl = dep_lines(deps);
    lines.push(format!("{} {}", label, dl[0]));
    for d in &dl[1..] {
        lines.push(format!("  {}", d));
    }
}

fn print_pkg_info(pkg: &alpm::Package, num: usize) {
    let mut lines: Vec<String> = Vec::new();

    if num > 0 {
        println!();
    }

    let name = pkg.name();
    let version = pkg.version().as_str();
    let desc = pkg.desc().unwrap_or("(no description)");
    let url = pkg.url().unwrap_or("");
    let arch = pkg.arch().unwrap_or("unknown");
    let licenses: Vec<&str> = pkg.licenses().iter().collect();
    let install_date_str = pkg.install_date().map(|ts| {
        chrono::DateTime::from_timestamp(ts, 0)
            .map(|dt| dt.format("%a %d %b %Y %H:%M:%S UTC").to_string())
            .unwrap_or_default()
    });
    let build = pkg.build_date();
    let repo = pkg.db().map(|db| db.name().to_string()).unwrap_or("now".to_string());

    lines.push(format!("{} {}", style::color("Package:", style::Color::Green), style::color(name, style::Color::Green)));
    lines.push(format!("{} {}", style::color("Version:", style::Color::Blue), style::color(version, style::Color::Blue)));
    lines.push(format!("{} {}", style::bold("Architecture:"), arch));
    if let Some(dt) = &install_date_str {
        lines.push(format!("{} {}", style::bold("Install Date:"), dt));
    }
    if !url.is_empty() && url != "." {
        lines.push(format!("{} {}", style::bold("URL:"), url));
    }
    if !licenses.is_empty() {
        lines.push(format!("{} {}", style::bold("Licenses:"), licenses.join(" ")));
    }

    let depends: Vec<String> = deps_to_strings(pkg.depends());
    add_deps(&mut lines, &style::bold("Depends:").to_string(), &depends);

    let required_by: Vec<String> = pkg.required_by().iter().map(|s| s.to_string()).collect();
    add_deps(&mut lines, &style::bold("Required By:").to_string(), &required_by);

    let optional_for: Vec<String> = pkg.optional_for().iter().map(|s| s.to_string()).collect();
    add_deps(&mut lines, &style::bold("Optional For:").to_string(), &optional_for);

    let conflicts: Vec<String> = deps_to_strings(pkg.conflicts());
    add_deps(&mut lines, &style::bold("Conflicts With:").to_string(), &conflicts);

    let provides: Vec<String> = deps_to_strings(pkg.provides());
    if !provides.is_empty() {
        lines.push(format!("{} {}", style::bold("Provides:"), provides.join(", ")));
    }

    if repo != "now" || pkg.size() > 0 {
        lines.push(format!("{} {}", style::bold("Download Size:"), crate::display::format::format_size(pkg.size())));
    }

    if pkg.isize() > 0 {
        lines.push(format!("{} {}", style::bold("Installed Size:"), crate::display::format::format_size(pkg.isize())));
    }

    if let Some(pkgr) = pkg.packager() {
        let maintainer = format_maintainer(pkgr);
        lines.push(format!("{} {}", style::bold("Packager:"), maintainer));
    }

    if build > 0 {
        let ts = chrono::DateTime::from_timestamp(build, 0)
            .map(|dt| dt.format("%a %d %b %Y").to_string())
            .unwrap_or_default();
        lines.push(format!("{} {}", style::bold("Build Date:"), ts));
    }

    lines.push(format!("{} {}", style::bold("Validated By:"), validation_str(pkg.validation())));

    lines.push(format!("{} {}", style::bold("Description:"), desc));

    println!("{}", draw::box_top(&format!(" {} ", name)));
    for l in &lines {
        println!("{}", draw::box_row(l));
    }
    println!("{}", draw::box_bottom());
}

fn dep_lines(deps: &[String]) -> Vec<String> {
    if deps.is_empty() {
        return Vec::new();
    }
    if deps.len() > 4 {
        deps.to_vec()
    } else {
        vec![deps.join(", ")]
    }
}

fn format_maintainer(raw: &str) -> String {
    if let Some(open) = raw.find('<') {
        let close = raw.find('>').unwrap_or(raw.len());
        let name_part = raw[..open].trim();
        let email_part = raw[open + 1..close].trim();
        let colored_email = style::color(email_part, style::Color::Blue);
        format!("{name_part} <{colored_email}>")
    } else {
        raw.to_string()
    }
}

fn deps_to_strings(deps: alpm::AlpmList<&alpm::Dep>) -> Vec<String> {
    let mut v: Vec<String> = deps
        .iter()
        .map(|d| {
            let name = d.name();
            match (d.version(), d.depmod()) {
                (Some(ver), alpm::DepMod::Eq) => format!("{name}={}", ver.as_str()),
                (Some(ver), alpm::DepMod::Ge) => format!("{name}>={}", ver.as_str()),
                (Some(ver), alpm::DepMod::Le) => format!("{name}<={}", ver.as_str()),
                (Some(ver), alpm::DepMod::Gt) => format!("{name}>{}", ver.as_str()),
                (Some(ver), alpm::DepMod::Lt) => format!("{name}<{}", ver.as_str()),
                _ => name.to_string(),
            }
        })
        .collect();
    v.sort();
    v
}

fn validation_str(v: alpm::PackageValidation) -> String {
    if v.is_empty() || v.contains(alpm::PackageValidation::UNKNOWN) {
        return "Unknown".to_string();
    }
    let mut parts = Vec::new();
    if v.contains(alpm::PackageValidation::MD5SUM) {
        parts.push("MD5");
    }
    if v.contains(alpm::PackageValidation::SHA256SUM) {
        parts.push("SHA256");
    }
    if v.contains(alpm::PackageValidation::SIGNATURE) {
        parts.push("Signature");
    }
    if parts.is_empty() { "Unknown".to_string() } else { parts.join(" ") }
}
