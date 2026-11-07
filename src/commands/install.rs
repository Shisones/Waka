use crate::alpm::{callbacks, handle::AlpmHandle, transaction};
use crate::commands::common::{commit_epilogue, detail_from_prep_err};
use crate::config::Config;
use crate::display::summary::show_install_summary;
use crate::display::{render, style};
use crate::sys::require_root;
use anyhow::Result;

pub fn run(packages: &[String], assume_yes: bool) -> Result<()> {
    if packages.is_empty() { return Err(anyhow::anyhow!("no packages specified")); }
    require_root()?;
    let config = Config::load()?;
    let assume_yes = assume_yes || config.waka.assume_yes;
    render::require_confirmation_possible(assume_yes)?;
    let mut handle = AlpmHandle::new()?;

    let mut targets: Vec<String> = Vec::new();
    let mut local: Vec<String> = Vec::new();
    let mut missing: Vec<String> = Vec::new();
    for name in packages {
        if std::path::Path::new(name).is_file() && name.contains(".pkg.tar.") {
            local.push(name.clone());
        } else if handle.find_sync_pkg(name).is_some() {
            targets.push(name.clone());
        } else {
            missing.push(name.clone());
        }
    }
    if !missing.is_empty() {
        return Err(anyhow::anyhow!(
            "target not found: {}",
            missing.join(", ")
        ));
    }
    if targets.is_empty() && local.is_empty() {
        return Err(anyhow::anyhow!("no valid packages to install"));
    }

    let local_level = handle.handle.local_file_siglevel();
    // Stage local archives so the loaded bytes cannot change.
    let staging = tempfile::tempdir()?;
    let h = &mut handle.handle;

    transaction::trans_init(h, alpm::TransFlag::NONE)?;
    for name in &targets {
        let mut found = false;
        for db in h.syncdbs() {
            if let Ok(pkg) = db.pkg(name.as_str()) {
                h.trans_add_pkg(pkg).map_err(|e| anyhow::anyhow!("add '{name}': {}", e.error))?;
                found = true;
                break;
            }
        }
        if !found { return Err(anyhow::anyhow!("package '{name}' disappeared")); }
    }
    let mut explicit = targets.clone();
    for (index, path) in local.iter().enumerate() {
        let source = std::path::Path::new(path);
        let meta = std::fs::metadata(source)
            .map_err(|e| anyhow::anyhow!("cannot read '{path}': {e}"))?;
        if !meta.file_type().is_file() {
            return Err(anyhow::anyhow!("'{path}' is not a regular file"));
        }
        let name = source
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| "package".to_string());
        let staged = staging.path().join(format!("{index}-{name}"));
        std::fs::copy(source, &staged)
            .map_err(|e| anyhow::anyhow!("cannot stage '{path}': {e}"))?;
        let staged = staged
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("staged path is not UTF-8"))?;
        let loaded = h.pkg_load(staged, true, local_level)
            .map_err(|e| anyhow::anyhow!("load '{path}': {e}"))?;
        explicit.push(loaded.pkg().name().to_string());
        h.trans_add_pkg(loaded)
            .map_err(|e| anyhow::anyhow!("add '{path}': {}", e.error))?;
    }
    let shared = callbacks::callbacks(h, "Install", assume_yes);
    let prep_err = match h.trans_prepare() {
        Ok(_) => None,
        Err(e) => Some((e.error().to_string(), detail_from_prep_err(e))),
    };
    if let Some((msg, detail)) = prep_err {
        transaction::end_trans(h);
        shared.lock().unwrap().done();
        if let Some(d) = detail { return Err(anyhow::anyhow!("{}: {}", msg, d)); }
        return Err(anyhow::anyhow!("{}", msg));
    }

    let add = h.trans_add();
    let rm = h.trans_remove();
    show_install_summary(&add, &rm, &explicit);

    if !assume_yes && !render::confirm(&style::bold("\n Do you want to continue? [Y/n] ")) {
        transaction::end_trans(h);
        shared.lock().unwrap().done();
        eprintln!(" {}", style::red("Aborted."));
        return Err(crate::aborted());
    }

    {
        let mut state = shared.lock().unwrap();
        state.total_pkgs = add.len();
        for pkg in add.iter() {
            let size = pkg.download_size();
            state.total_bytes += size;
            state.pkg_sizes.insert(pkg.name().to_string(), size);
        }
        state.render();
    }
    let commit_result = h.trans_commit();
    commit_epilogue(&shared, h, commit_result, "Downloading packages")?;
    Ok(())
}

