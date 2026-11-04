use crate::alpm::handle::AlpmHandle;
use crate::display::{draw, style};
use anyhow::Result;

/// Every term must appear in the name or the description.
pub fn matches_terms(name: &str, desc: &str, terms: &[String]) -> bool {
    let name = name.to_lowercase();
    let desc = desc.to_lowercase();
    terms.iter().all(|term| {
        let term = term.to_lowercase();
        name.contains(&term) || desc.contains(&term)
    })
}

pub fn search_packages(handle: &AlpmHandle, terms: &[String]) -> Result<()> {
    let terms: Vec<String> = terms.iter().filter(|t| !t.trim().is_empty()).cloned().collect();
    if terms.is_empty() {
        return Err(anyhow::anyhow!("no search terms specified"));
    }

    let mut results: Vec<(&alpm::Package, String)> = Vec::new();
    for db in handle.handle.syncdbs() {
        for pkg in db.pkgs() {
            if matches_terms(pkg.name(), pkg.desc().unwrap_or(""), &terms) {
                results.push((pkg, db.name().to_string()));
            }
        }
    }
    results.sort_by(|a, b| a.0.name().cmp(b.0.name()));

    if results.is_empty() {
        eprintln!(
            " {}: no packages found matching '{}'",
            style::yellow("warn"),
            style::color(&terms.join(" "), style::Color::Green)
        );
        return Ok(());
    }

    let rows: Vec<SearchRow> = results
        .iter()
        .map(|(pkg, repo)| {
            let size = crate::display::format::format_size(pkg.size());
            SearchRow {
                label: format!("{}/{}", style::bold(repo), style::color(pkg.name(), style::Color::Green)),
                width: draw::display_width(repo) + 1 + draw::display_width(pkg.name()),
                version: style::color(pkg.version(), style::Color::Blue),
                version_width: draw::display_width(pkg.version()),
                size_width: draw::display_width(&size),
                size: style::dim(&size),
                installed: handle.find_local_pkg(pkg.name()).is_some(),
                desc: pkg.desc().unwrap_or("").to_string(),
            }
        })
        .collect();

    for line in search_lines(&rows, draw::term_width()) {
        println!("{line}");
    }
    println!(
        "{} {} {} found.",
        style::green("\u{2713}"),
        results.len(),
        crate::display::table::plural("package", results.len())
    );
    Ok(())
}

struct SearchRow {
    label: String,
    width: usize,
    version: String,
    version_width: usize,
    size: String,
    size_width: usize,
    installed: bool,
    desc: String,
}

/// Fixed columns: ` repo/name  version  size [installed]`, description below.
fn search_lines(rows: &[SearchRow], w: usize) -> Vec<String> {
    if rows.is_empty() {
        return Vec::new();
    }
    let name_w = rows
        .iter()
        .map(|r| r.width)
        .max()
        .unwrap_or(0)
        .min(48)
        .min((w / 2).max(12));
    let version_w = rows.iter().map(|r| r.version_width).max().unwrap_or(0).min(12);
    let size_w = rows.iter().map(|r| r.size_width).max().unwrap_or(0).max(3);

    let rest = 2 + version_w + 2 + size_w;
    let name_w = name_w.min(w.saturating_sub(rest + 1)).max(10);

    let mut out = Vec::with_capacity(rows.len() * 2);
    for row in rows {
        let tag = if row.installed {
            format!(" {}", style::color("[installed]", style::Color::Yellow))
        } else {
            String::new()
        };
        let mut line = format!(
            " {}  {}  {}",
            crate::display::table::pad(&row.label, name_w, false),
            crate::display::table::pad(&row.version, version_w, false),
            crate::display::table::pad(&row.size, size_w, true)
        );
        // Keep the tag on the row when it fits; otherwise it moves to the
        // description line rather than eating a fixed column on every row.
        let tag_inline = row.installed && draw::display_width(&line) + draw::display_width(&tag) <= w;
        if tag_inline {
            line.push_str(&tag);
        }
        out.push(draw::truncate(&line, w));
        if !row.desc.is_empty() {
            let desc = if row.installed && !tag_inline {
                format!("   {} {}", tag.trim_start(), row.desc)
            } else {
                format!("   {}", row.desc)
            };
            out.push(draw::truncate(&desc, w));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{matches_terms, search_lines, SearchRow};
    use crate::display::draw::display_width;

    #[test]
    fn all_terms_must_match() {
        let terms = vec!["fire".to_string(), "browser".to_string()];
        assert!(matches_terms("firefox", "fast web browser", &terms));
        assert!(!matches_terms("firefox", "fast web browser", &["fire".into(), "mail".into()]));
    }

    #[test]
    fn terms_can_match_the_description() {
        assert!(matches_terms("firefox", "Fast, private browser", &["private".into()]));
    }

    #[test]
    fn search_rows_fit_every_width() {
        let rows = vec![SearchRow {
            label: "core/firefox".to_string(),
            width: 12,
            version: "136.0-1".to_string(),
            version_width: 7,
            size: "80 MiB".to_string(),
            size_width: 6,
            installed: true,
            desc: "Fast, private browser".to_string(),
        }];
        for w in 12..=120 {
            for line in search_lines(&rows, w) {
                assert!(display_width(&line) <= w, "w={w}: {line:?}");
            }
        }
    }
}
