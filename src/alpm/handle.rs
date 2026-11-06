use crate::alpm::conf;
use crate::constants;
use alpm::SigLevel;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::ffi::CString;
use std::path::PathBuf;

struct Options {
    default_sig: SigLevel,
    local_sig: Option<SigLevel>,
    remote_sig: Option<SigLevel>,
    cache_dirs: Vec<String>,
    architectures: Vec<String>,
    hold_pkgs: Vec<String>,
}

#[derive(Debug)]
pub struct AlpmHandle {
    pub handle: alpm::Alpm,
    pub hold_pkgs: Vec<String>,
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

        let hold_pkgs = Self::configure_from_file(&mut handle)?;

        Ok(AlpmHandle { handle, hold_pkgs })
    }

    fn configure_from_file(handle: &mut alpm::Alpm) -> Result<Vec<String>> {
        let content = Self::read_conf_file(&constants::pacman_conf())?;
        let mut current: Option<String> = None;
        let mut servers: Vec<String> = Vec::new();
        let mut include_paths: Vec<String> = Vec::new();
        let mut db_sig: HashMap<String, String> = HashMap::new();
        let mut db_usage: HashMap<String, alpm::Usage> = HashMap::new();
        let mut opts = Options {
            default_sig: conf::default_siglevel(),
            local_sig: None,
            remote_sig: None,
            cache_dirs: Vec::new(),
            architectures: Vec::new(),
            hold_pkgs: Vec::new(),
        };

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') { continue; }
            if line.starts_with('[') && line.ends_with(']') {
                Self::finalize_prev_db(handle, &current, &servers, &db_sig, &db_usage, &opts, &include_paths)?;
                current = Some(line[1..line.len() - 1].to_string());
                servers.clear();
                include_paths.clear();
                continue;
            }
            let Some(section) = current.as_ref() else { continue; };
            if section == "options" {
                Self::apply_options_line(handle, line, &mut opts, 0);
            } else if let Some(v) = conf::option_value(line, "SigLevel") {
                db_sig.insert(section.clone(), v.to_string());
            } else if let Some(v) = conf::option_value(line, "Usage") {
                db_usage.insert(section.clone(), Self::parse_usage(v));
            } else if let Some(v) = conf::option_value(line, "Include") {
                include_paths.push(v.to_string());
            } else {
                Self::handle_repo_line(line, &mut servers);
            }
        }

        Self::finalize_prev_db(handle, &current, &servers, &db_sig, &db_usage, &opts, &include_paths)?;

        if let Err(e) = handle.set_default_siglevel(opts.default_sig) {
            eprintln!("  {} set_default_siglevel failed: {e}", crate::display::style::yellow("warn"));
        }
        let local_sig = opts.local_sig.unwrap_or(opts.default_sig);
        let remote_sig = opts.remote_sig.unwrap_or(opts.default_sig);
        if let Err(e) = handle.set_local_file_siglevel(local_sig) {
            eprintln!("  {} set_local_file_siglevel failed: {e}", crate::display::style::yellow("warn"));
        }
        if let Err(e) = handle.set_remote_file_siglevel(remote_sig) {
            eprintln!("  {} set_remote_file_siglevel failed: {e}", crate::display::style::yellow("warn"));
        }
        if !opts.cache_dirs.is_empty() {
            if let Err(e) = handle.set_cachedirs(opts.cache_dirs.iter().map(|s| s.as_str())) {
                eprintln!("  {} set_cachedirs from config failed: {e}", crate::display::style::yellow("warn"));
            }
        }
        if opts.architectures.is_empty() {
            if let Err(e) = handle.set_architectures(std::iter::once(std::env::consts::ARCH)) {
                eprintln!("  {} set_architectures failed: {e}", crate::display::style::yellow("warn"));
            }
        } else {
            let mut first = true;
            for arch in &opts.architectures {
                if first {
                    if let Err(e) = handle.set_architectures(std::iter::once(arch.as_str())) {
                        eprintln!("  {} set_architectures failed: {e}", crate::display::style::yellow("warn"));
                    }
                    first = false;
                } else if let Err(e) = handle.add_architecture(arch.as_str()) {
                    eprintln!("  {} add_architecture('{arch}') failed: {e}", crate::display::style::yellow("warn"));
                }
            }
        }
        if let Err(e) = handle.add_architecture("any") {
            eprintln!("  {} add_architecture('any') failed: {e}", crate::display::style::yellow("warn"));
        }

        Ok(opts.hold_pkgs)
    }

    fn apply_options_line(
        handle: &mut alpm::Alpm,
        line: &str,
        opts: &mut Options,
        depth: usize,
    ) {
        if let Some(v) = conf::option_value(line, "SigLevel") {
            opts.default_sig = conf::apply_siglevel(opts.default_sig, v);
        } else if let Some(v) = conf::option_value(line, "LocalFileSigLevel") {
            opts.local_sig = Some(conf::apply_siglevel(opts.default_sig, v));
        } else if let Some(v) = conf::option_value(line, "RemoteFileSigLevel") {
            opts.remote_sig = Some(conf::apply_siglevel(opts.default_sig, v));
        } else if let Some(v) = conf::option_value(line, "CacheDir") {
            opts.cache_dirs.push(v.to_string());
        } else if let Some(v) = conf::option_value(line, "Architecture") {
            for arch in v.split_whitespace() {
                opts.architectures.push(if arch == "auto" {
                    std::env::consts::ARCH.to_string()
                } else {
                    arch.to_string()
                });
            }
        } else if let Some(v) = conf::option_value(line, "HoldPkg") {
            opts.hold_pkgs.extend(v.split_whitespace().map(|name| name.to_string()));
        } else if let Some(v) = conf::option_value(line, "IgnorePkg") {
            for name in v.split_whitespace() {
                if let Err(e) = handle.add_ignorepkg(name) {
                    eprintln!("  {} add_ignorepkg('{name}') failed: {e}", crate::display::style::yellow("warn"));
                }
            }
        } else if let Some(v) = conf::option_value(line, "IgnoreGroup") {
            for name in v.split_whitespace() {
                if let Err(e) = handle.add_ignoregroup(name) {
                    eprintln!("  {} add_ignoregroup('{name}') failed: {e}", crate::display::style::yellow("warn"));
                }
            }
        } else if let Some(v) = conf::option_value(line, "NoUpgrade") {
            for path in v.split_whitespace() {
                if let Err(e) = handle.add_noupgrade(path) {
                    eprintln!("  {} add_noupgrade('{path}') failed: {e}", crate::display::style::yellow("warn"));
                }
            }
        } else if let Some(v) = conf::option_value(line, "NoExtract") {
            for path in v.split_whitespace() {
                if let Err(e) = handle.add_noextract(path) {
                    eprintln!("  {} add_noextract('{path}') failed: {e}", crate::display::style::yellow("warn"));
                }
            }
        } else if let Some(v) = conf::option_value(line, "Include") {
            if depth < 8 {
                if let Ok(lines) = Self::resolve_include(v) {
                    for nested in lines {
                        let nested = nested.trim();
                        if nested.is_empty() || nested.starts_with('#') {
                            continue;
                        }
                        Self::apply_options_line(handle, nested, opts, depth + 1);
                    }
                }
            }
        } else {
            Self::handle_option(handle, line);
        }
    }

    fn parse_usage(value: &str) -> alpm::Usage {
        let mut usage = alpm::Usage::empty();
        for part in value.split_whitespace() {
            match part {
                "Sync" => usage |= alpm::Usage::SYNC,
                "Search" => usage |= alpm::Usage::SEARCH,
                "Install" => usage |= alpm::Usage::INSTALL,
                "Upgrade" => usage |= alpm::Usage::UPGRADE,
                "All" => usage |= alpm::Usage::SYNC | alpm::Usage::SEARCH | alpm::Usage::INSTALL | alpm::Usage::UPGRADE,
                other => eprintln!(
                    "  {} unknown Usage token '{other}'",
                    crate::display::style::yellow("warn")
                ),
            }
        }
        usage
    }

    fn read_conf_file(path: &std::path::Path) -> Result<String> {
        if !path.exists() { return Ok(String::new()); }
        std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))
    }

    fn glob_paths(path: &str) -> Vec<PathBuf> {
        let Ok(path) = CString::new(path) else { return Vec::new(); };
        let mut glob: libc::glob_t = unsafe { std::mem::zeroed() };
        let result = unsafe { libc::glob(path.as_ptr(), 0, None, &mut glob) };
        if result != 0 {
            unsafe { libc::globfree(&mut glob) };
            return Vec::new();
        }
        let mut paths = Vec::new();
        for index in 0..glob.gl_pathc {
            let ptr = unsafe { *glob.gl_pathv.add(index) };
            if ptr.is_null() {
                continue;
            }
            if let Ok(value) = unsafe { std::ffi::CStr::from_ptr(ptr) }.to_str() {
                paths.push(PathBuf::from(value));
            }
        }
        unsafe { libc::globfree(&mut glob) };
        paths
    }

    fn resolve_include(path: &str) -> Result<Vec<String>> {
        let path = if path.starts_with('/') {
            path.to_string()
        } else {
            format!("/etc/pacman.d/{path}")
        };
        let mut results = Vec::new();
        for file in Self::glob_paths(&path) {
            if let Ok(content) = Self::read_conf_file(&file) {
                results.extend(content.lines().map(|line| line.to_string()));
            }
        }
        Ok(results)
    }

    fn resolve_include_servers(path: &str, servers: &mut Vec<String>, depth: usize) {
        if depth > 8 {
            return;
        }
        let path = if path.starts_with('/') {
            path.to_string()
        } else {
            format!("/etc/pacman.d/{path}")
        };
        for file in Self::glob_paths(&path) {
            if let Ok(content) = Self::read_conf_file(&file) {
                Self::collect_servers(&content, servers, depth);
            }
        }
    }

    fn collect_servers(content: &str, servers: &mut Vec<String>, depth: usize) {
        for line in content.lines() {
            let line = line.trim();
            if let Some(v) = conf::option_value(line, "Server") {
                servers.push(v.to_string());
            } else if let Some(v) = conf::option_value(line, "Include") {
                Self::resolve_include_servers(v, servers, depth + 1);
            }
        }
    }

    fn handle_option(handle: &mut alpm::Alpm, line: &str) {
        if let Some(v) = conf::option_value(line, "DownloadUser") {
            if let Err(e) = handle.set_sandbox_user(Some(v)) {
                eprintln!("  {} set_sandbox_user failed: {e}", crate::display::style::yellow("warn"));
            }
        }
        if let Some(rest) = line.strip_prefix("DisableSandbox") {
            if rest.is_empty() || rest.starts_with(' ') || rest.starts_with('=') {
                handle.set_disable_sandbox_filesystem(true);
                handle.set_disable_sandbox_syscalls(true);
            }
        }
        if let Some(v) = conf::option_value(line, "ParallelDownloads") {
            if let Ok(n) = v.parse::<u32>() {
                handle.set_parallel_downloads(n);
            }
        }
    }

    fn handle_repo_line(line: &str, servers: &mut Vec<String>) {
        if let Some(v) = conf::option_value(line, "Server") {
            servers.push(v.to_string());
        }
    }

    fn finalize_prev_db(handle: &mut alpm::Alpm, current: &Option<String>, servers: &[String],
                        db_sig: &HashMap<String, String>, db_usage: &HashMap<String, alpm::Usage>,
                        opts: &Options, include_paths: &[String]) -> Result<()> {
        if let Some(name) = current {
            if name != "options" {
                let mut final_servers = servers.to_vec();
                for inc in include_paths {
                    Self::resolve_include_servers(inc, &mut final_servers, 0);
                }
                let sig = match db_sig.get(name.as_str()) {
                    Some(raw) => conf::apply_siglevel(opts.default_sig, raw),
                    None => opts.default_sig,
                };
                let usage = db_usage.get(name.as_str()).copied();
                let arch = opts.architectures.first().map(String::as_str).unwrap_or(std::env::consts::ARCH);
                Self::finalize_db(handle, name, &final_servers, sig, usage, arch)?;
            }
        }
        Ok(())
    }

    fn finalize_db(handle: &mut alpm::Alpm, name: &str, servers: &[String], sig: SigLevel,
                   usage: Option<alpm::Usage>, arch: &str) -> Result<()> {
        let db = handle.register_syncdb_mut(name, sig)
            .with_context(|| format!("Failed to register db '{name}'"))?;
        if let Some(usage) = usage {
            if let Err(e) = db.set_usage(usage) {
                eprintln!("  {} set_usage('{name}') failed: {e}", crate::display::style::yellow("warn"));
            }
        }
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

#[cfg(test)]
mod tests {
    use super::AlpmHandle;

    #[test]
    fn glob_paths_support_question_and_character_classes() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["foo1.conf", "foo2.conf", "bar.conf"] {
            std::fs::write(dir.path().join(name), "").unwrap();
        }
        let question = format!("{}/foo?.conf", dir.path().display());
        let classes = format!("{}/[fb]*.conf", dir.path().display());
        assert_eq!(AlpmHandle::glob_paths(&question).len(), 2);
        assert_eq!(AlpmHandle::glob_paths(&classes).len(), 3);
        assert!(AlpmHandle::glob_paths(&format!("{}/missing?.conf", dir.path().display())).is_empty());
    }
}
