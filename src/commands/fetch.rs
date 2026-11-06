use crate::display::style;
use anyhow::Result;

pub fn run() -> Result<()> {
    println!(" {} fetching mirrors...", style::cyan("info"));
    println!();
    println!(" {}", style::bold("To configure mirrors, you can:"));
    println!("  {} install and run 'reflector' to auto-rank mirrors", style::dim("\u{2022}"));
    println!("  {} manually edit /etc/pacman.d/mirrorlist", style::dim("\u{2022}"));
    println!("  {} run 'waka fetch' requires root permissions", style::dim("\u{2022}"));
    println!();
    println!(" {} fetching mirrorlist from archlinux.org...", style::cyan("info"));

    let tmp = tempfile::Builder::new()
        .prefix("waka_mirrorlist.")
        .tempfile()
        .map_err(|e| anyhow::anyhow!("failed to create a temporary file: {e}"))?;
    let tmp_str = tmp.path().to_string_lossy().to_string();

    let status = std::process::Command::new("/usr/bin/curl")
        .args([
            "-sSL",
            "--fail",
            "--proto",
            "=https",
            "--proto-redir",
            "=https",
            "https://archlinux.org/mirrorlist/?country=all&protocol=https&use_mirror_status=on",
            "-o",
        ])
        .arg(&tmp_str)
        .status();

    match status {
        Ok(s) if s.success() => {
            let (_file, path) = tmp.keep().map_err(|e| anyhow::anyhow!("failed to keep the mirrorlist: {e}"))?;
            println!(" {} downloaded to {}", style::green("\u{2713}"), path.display());
            println!(" {} run 'rankmirrors -n 10 {} > /etc/pacman.d/mirrorlist' as root to apply", style::cyan("info"), path.display());
            Ok(())
        }
        Ok(_) => Err(anyhow::anyhow!("failed to download mirrorlist (is curl installed?)")),
        Err(e) => Err(anyhow::anyhow!("failed to run curl: {e}")),
    }
}

