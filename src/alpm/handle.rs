use crate::constants;
use alpm::SigLevel;
use anyhow::{Context, Result};
use std::collections::HashMap;

#[derive(Debug)]
pub struct AlpmHandle {
    pub handle: alpm::Alpm,
}

impl AlpmHandle {
    pub fn new() -> Result<Self> {
        let db = constants::db_path();
        let db_str = db.to_str().context("db path contains non-UTF8 characters")?;
        let mut handle = alpm::Alpm::new("/", db_str)
            .map_err(|e| anyhow::anyhow!("failed to init alpm: {e}"))?;

        handle.set_dbext(".db");

        if let Some(gpg) = constants::gpg_dir().to_str() {
            if let Err(e) = handle.set_gpgdir(gpg) {
                eprintln!("  {} set_gpgdir failed: {e}", crate::display::style::yellow("warn"));
            }
        }
        if let Some(log) = constants::pacman_log().to_str() {
            if let Err(e) = handle.set_logfile(log) {
                eprintln!("  {} set_logfile failed: {e}", crate::display::style::yellow("warn"));
            }
        }
        if let Some(cache) = constants::cache_dir().to_str() {
            if let Err(e) = handle.set_cachedirs(std::iter::once(cache)) {
                eprintln!("  {} set_cachedirs failed: {e}", crate::display::style::yellow("warn"));
            }
        }
        handle.set_parallel_downloads(5);

        Self::configure_from_file(&mut handle)?;

        Ok(AlpmHandle { handle })
    }

    fn configure_from_file(handle: &mut alpm::Alpm) -> Result<()> {
        let content = Self::read_conf_file(&constants::pacman_conf())?;
        let mut current: Option<String> = None;
        let mut servers: Vec<String> = Vec::new();
        let mut default_sig: SigLevel = SigLevel::NONE;
        let mut local_sig: SigLevel = SigLevel::NONE;
        let mut remote_sig: SigLevel = SigLevel::NONE;
        let mut db_sig: HashMap<String, SigLevel> = HashMap::new();
        let mut architectures: Vec<String> = Vec::new();
        let mut cache_dirs: Vec<String> = Vec::new();
        let mut include_paths: Vec<String> = Vec::new();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') { continue; }

            if line.starts_with('[') && line.ends_with(']') {
                Self::finalize_prev_db(handle, &current, &servers, &db_sig, &default_sig, &include_paths)?;
                current = Some(line[1..line.len() - 1].to_string());
                servers.clear();
                include_paths.clear();
                continue;
            }

            let Some(section) = current.as_ref() else { continue; };
            if section == "options" {
                if let Some(v) = line.strip_prefix("SigLevel = ") {
                    default_sig = Self::parse_siglevel(v);
                } else if let Some(v) = line.strip_prefix("LocalFileSigLevel = ") {
                    local_sig = Self::parse_siglevel(v);
                } else if let Some(v) = line.strip_prefix("RemoteFileSigLevel = ") {
                    remote_sig = Self::parse_siglevel(v);
                } else if let Some(v) = line.strip_prefix("CacheDir = ") {
                    cache_dirs.push(v.trim().to_string());
                } else if let Some(v) = line.strip_prefix("Architecture = ") {
                    let arch = v.trim();
                    let actual = if arch == "auto" { std::env::consts::ARCH } else { arch };
                    architectures.push(actual.to_string());
                } else if let Some(v) = line.strip_prefix("Include = ") {
                    if let Ok(lines) = Self::resolve_include(v.trim()) {
                        for l in lines {
                            if let Some(iv) = l.strip_prefix("SigLevel = ") {
                                default_sig = Self::parse_siglevel(iv);
                            } else if let Some(iv) = l.strip_prefix("LocalFileSigLevel = ") {
                                local_sig = Self::parse_siglevel(iv);
                            } else if let Some(iv) = l.strip_prefix("RemoteFileSigLevel = ") {
                                remote_sig = Self::parse_siglevel(iv);
                            }
                        }
                    }
                } else {
                    Self::handle_option(handle, line);
                }
            } else if let Some(v) = line.strip_prefix("SigLevel = ") {
                db_sig.insert(section.clone(), Self::parse_siglevel(v));
            } else if let Some(v) = line.strip_prefix("Include = ") {
                include_paths.push(v.trim().to_string());
            } else {
                Self::handle_repo_line(line, &mut servers);
            }
        }

        Self::finalize_prev_db(handle, &current, &servers, &db_sig, &default_sig, &include_paths)?;

        if let Err(e) = handle.set_local_file_siglevel(local_sig) {
            eprintln!("  {} set_local_file_siglevel failed: {e}", crate::display::style::yellow("warn"));
        }
        if let Err(e) = handle.set_remote_file_siglevel(remote_sig) {
            eprintln!("  {} set_remote_file_siglevel failed: {e}", crate::display::style::yellow("warn"));
        }

        if !cache_dirs.is_empty() {
            if let Err(e) = handle.set_cachedirs(cache_dirs.iter().map(|s| s.as_str())) {
                eprintln!("  {} set_cachedirs from config failed: {e}", crate::display::style::yellow("warn"));
            }
        }

        if architectures.is_empty() {
            if let Err(e) = handle.set_architectures(std::iter::once(std::env::consts::ARCH)) {
                eprintln!("  {} set_architectures failed: {e}", crate::display::style::yellow("warn"));
            }
            if let Err(e) = handle.add_architecture("any") {
                eprintln!("  {} add_architecture('any') failed: {e}", crate::display::style::yellow("warn"));
            }
        } else {
            let mut first = true;
            for arch in &architectures {
                if first {
                    if let Err(e) = handle.set_architectures(std::iter::once(arch.as_str())) {
                        eprintln!("  {} set_architectures failed: {e}", crate::display::style::yellow("warn"));
                    }
                    first = false;
                } else if let Err(e) = handle.add_architecture(arch.as_str()) {
                    eprintln!("  {} add_architecture('{arch}') failed: {e}", crate::display::style::yellow("warn"));
                }
            }
            if let Err(e) = handle.add_architecture("any") {
                eprintln!("  {} add_architecture('any') failed: {e}", crate::display::style::yellow("warn"));
            }
        }

        Ok(())
    }

    fn read_conf_file(path: &std::path::Path) -> Result<String> {
        if !path.exists() { return Ok(String::new()); }
        std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))
    }

    fn parse_siglevel(val: &str) -> SigLevel {
        let mut sig = SigLevel::NONE;
        for part in val.split_whitespace() {
            match part {
                "Never" => return SigLevel::NONE,
                "Optional" => {
                    sig |= SigLevel::PACKAGE | SigLevel::PACKAGE_OPTIONAL
                        | SigLevel::PACKAGE_MARGINAL_OK | SigLevel::PACKAGE_UNKNOWN_OK
                        | SigLevel::DATABASE | SigLevel::DATABASE_OPTIONAL
                        | SigLevel::DATABASE_MARGINAL_OK | SigLevel::DATABASE_UNKNOWN_OK;
                }
                "Required" => {
                    sig |= SigLevel::PACKAGE | SigLevel::PACKAGE_MARGINAL_OK
                        | SigLevel::DATABASE | SigLevel::DATABASE_MARGINAL_OK;
                }
                "Package" => sig |= SigLevel::PACKAGE,
                "PackageOptional" => {
                    sig |= SigLevel::PACKAGE | SigLevel::PACKAGE_OPTIONAL
                        | SigLevel::PACKAGE_MARGINAL_OK | SigLevel::PACKAGE_UNKNOWN_OK;
                }
                "PackageRequired" => {
                    sig |= SigLevel::PACKAGE | SigLevel::PACKAGE_MARGINAL_OK;
                }
                "PackageTrustedOnly" => sig |= SigLevel::PACKAGE,
                "PackageTrustedAll" => sig |= SigLevel::PACKAGE | SigLevel::PACKAGE_UNKNOWN_OK,
                "Database" => sig |= SigLevel::DATABASE,
                "DatabaseOptional" => {
                    sig |= SigLevel::DATABASE | SigLevel::DATABASE_OPTIONAL
                        | SigLevel::DATABASE_MARGINAL_OK | SigLevel::DATABASE_UNKNOWN_OK;
                }
                "DatabaseRequired" => {
                    sig |= SigLevel::DATABASE | SigLevel::DATABASE_MARGINAL_OK;
                }
                "DatabaseTrustedOnly" => sig |= SigLevel::DATABASE,
                "DatabaseTrustedAll" => sig |= SigLevel::DATABASE | SigLevel::DATABASE_UNKNOWN_OK,
                "TrustedOnly" => {}
                "TrustedAll" => {
                    sig |= SigLevel::PACKAGE_UNKNOWN_OK | SigLevel::DATABASE_UNKNOWN_OK;
                }
                _ => {}
            }
        }
        sig
    }

    fn resolve_include(path: &str) -> Result<Vec<String>> {
        let p = if path.starts_with('/') { path.to_string() } else { format!("/etc/pacman.d/{path}") };

        if p.contains('*') {
            if let Some(parent) = std::path::Path::new(&p).parent() {
                if parent.exists() {
                    let pattern = std::path::Path::new(&p)
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let mut results = Vec::new();
                    if let Ok(entries) = std::fs::read_dir(parent) {
                        for entry in entries.flatten() {
                            if let Some(name) = entry.file_name().to_str() {
                                if glob_match(&pattern, name) {
                                    if let Ok(c) = Self::read_conf_file(&entry.path()) {
                                        results.extend(c.lines().map(|l| l.to_string()));
                                    }
                                }
                            }
                        }
                    }
                    return Ok(results);
                }
            }
            return Ok(Vec::new());
        }

        Self::read_conf_file(std::path::Path::new(&p))
            .map(|c| c.lines().map(|l| l.to_string()).collect())
            .or_else(|_| Ok(Vec::new()))
    }

    fn resolve_include_servers(path: &str, name: &str, servers: &mut Vec<String>) {
        let p = if path.starts_with('/') { path.to_string() } else { format!("/etc/pacman.d/{path}") };

        if p.contains('*') {
            if let Some(parent) = std::path::Path::new(&p).parent() {
                if parent.exists() {
                    let pattern = std::path::Path::new(&p)
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    if let Ok(entries) = std::fs::read_dir(parent) {
                        for entry in entries.flatten() {
                            if let Some(name_str) = entry.file_name().to_str() {
                                if glob_match(&pattern, name_str) {
                                    if let Ok(c) = Self::read_conf_file(&entry.path()) {
                                        for line in c.lines() {
                                            let line = line.trim();
                                            if let Some(v) = line.strip_prefix("Server = ") {
                                                let expanded = v.replace("$repo", name).replace("$arch", std::env::consts::ARCH);
                                                servers.push(expanded);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            return;
        }

        if let Ok(c) = Self::read_conf_file(std::path::Path::new(&p)) {
            for line in c.lines() {
                let line = line.trim();
                if let Some(v) = line.strip_prefix("Server = ") {
                    let expanded = v.replace("$repo", name).replace("$arch", std::env::consts::ARCH);
                    servers.push(expanded);
                } else if let Some(v) = line.strip_prefix("Include = ") {
                    Self::resolve_include_servers(v.trim(), name, servers);
                }
            }
        }
    }

    fn handle_option(handle: &mut alpm::Alpm, line: &str) {
        if let Some(v) = line.strip_prefix("DownloadUser = ") {
            if let Err(e) = handle.set_sandbox_user(Some(v.trim())) {
                eprintln!("  {} set_sandbox_user failed: {e}", crate::display::style::yellow("warn"));
            }
        }
        if let Some(rest) = line.strip_prefix("DisableSandbox") {
            if rest.is_empty() || rest.starts_with(' ') || rest.starts_with('=') {
                handle.set_disable_sandbox_filesystem(true);
                handle.set_disable_sandbox_syscalls(true);
            }
        }
        if let Some(v) = line.strip_prefix("ParallelDownloads = ") {
            if let Ok(n) = v.trim().parse::<u32>() {
                handle.set_parallel_downloads(n);
            }
        }
    }

    fn handle_repo_line(line: &str, servers: &mut Vec<String>) {
        if let Some(v) = line.strip_prefix("Server = ") {
            servers.push(v.trim().to_string());
        }
    }

    fn finalize_prev_db(handle: &mut alpm::Alpm, current: &Option<String>, servers: &[String],
                        db_sig: &HashMap<String, SigLevel>, default_sig: &SigLevel,
                        include_paths: &[String]) -> Result<()> {
        if let Some(name) = current {
            if name != "options" {
                let mut final_servers = servers.to_vec();
                for inc in include_paths {
                    Self::resolve_include_servers(inc, name, &mut final_servers);
                }
                let sig = db_sig.get(name.as_str()).copied().unwrap_or(*default_sig);
                Self::finalize_db(handle, name, &final_servers, sig)?;
            }
        }
        Ok(())
    }

    fn finalize_db(handle: &mut alpm::Alpm, name: &str, servers: &[String], sig: SigLevel) -> Result<()> {
        let db = handle.register_syncdb_mut(name, sig)
            .with_context(|| format!("Failed to register db '{name}'"))?;
        let arch = std::env::consts::ARCH;
        for s in servers {
            let expanded = s.replace("$repo", name).replace("$arch", arch);
            db.add_server(expanded.as_str())
                .with_context(|| format!("Failed to add server '{s}' to '{name}'"))?;
        }
        Ok(())
    }

    pub fn find_local_pkg(&self, name: &str) -> Option<&alpm::Package> {
        self.handle.localdb().pkg(name).ok()
    }

    pub fn find_sync_pkg(&self, name: &str) -> Option<&alpm::Package> {
        for db in self.handle.syncdbs() {
            if let Ok(pkg) = db.pkg(name) { return Some(pkg); }
        }
        None
    }
}

fn glob_match(pattern: &str, filename: &str) -> bool {
    if pattern == "*" { return true; }
    if let Some(prefix) = pattern.strip_suffix('*') {
        return filename.starts_with(prefix);
    }
    if let Some(suffix) = pattern.strip_prefix('*') {
        return filename.ends_with(suffix);
    }
    pattern == filename
}
