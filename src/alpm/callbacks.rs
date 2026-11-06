use crate::display::{draw, format as fmt, render, style};
use alpm::{DownloadEvent, Event, PackageOperation};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub fn callbacks(handle: &alpm::Alpm, title: &str, assume_yes: bool) -> Arc<Mutex<render::DownloadState>> {
    let shared = Arc::new(Mutex::new(render::DownloadState {
        rendered: 0, last_size: None, resize: Default::default(), plain_log: 0,
        title: style::bold(title), phase: false,
        total_bytes: 0, completed_bytes: 0, total_files: 0, total_pkgs: 0, completed_files: 0,
        currently_downloading: None,
        completed_downloads: Vec::new(), file_to_pkg: HashMap::new(),
        start: Instant::now(), last_render: Instant::now(),
        per_file: HashMap::new(), pkg_sizes: HashMap::new(), log: Vec::new(), log_rows: 5,
    }));

    let s = Arc::clone(&shared);
    handle.set_dl_cb(s, |filename, event, arc| {
        let fname = filename.rsplit('/').next().unwrap_or(filename);
        let mut state = arc.lock().unwrap();
        match event.event() {
            DownloadEvent::Init(_) => {
                if fname.ends_with(".sig") {
                    return;
                }
                state.per_file.entry(fname.to_string()).or_insert(render::FileProgress { downloaded: 0, total: 0 });
                state.total_files += 1;
                let name = render::pkg_name_from_filename(fname);
                state.file_to_pkg.insert(fname.to_string(), name);
                if state.currently_downloading.is_none() {
                    state.currently_downloading = Some(fname.to_string());
                }
                state.render();
            }
            DownloadEvent::Progress(prog) => {
                if fname.ends_with(".sig") {
                    return;
                }
                let cap = state.file_to_pkg.get(fname).and_then(|pkg| state.pkg_sizes.get(pkg)).copied();
                let target = render::capped_bytes(prog.downloaded, cap);
                let delta = {
                    let entry = state.per_file.entry(fname.to_string()).or_insert(render::FileProgress { downloaded: 0, total: 0 });
                    if entry.total == 0 && prog.total > 0 { entry.total = prog.total; }
                    let d = target - entry.downloaded;
                    if d > 0 { entry.downloaded = target; }
                    d
                };
                if delta > 0 { state.completed_bytes += delta; }
                state.currently_downloading = state.best_progress().or(state.currently_downloading.clone());
                state.render();
            }
            DownloadEvent::Completed(comp) => {
                if fname.ends_with(".sig") {
                    return;
                }
                let cap = state.file_to_pkg.get(fname).and_then(|pkg| state.pkg_sizes.get(pkg)).copied();
                let target = render::capped_bytes(comp.total, cap);
                let remaining = {
                    let entry = state.per_file.entry(fname.to_string()).or_insert(render::FileProgress { downloaded: 0, total: 0 });
                    if entry.total == 0 { entry.total = cap.unwrap_or(comp.total); }
                    let r = target - entry.downloaded;
                    if r > 0 { entry.downloaded = target; }
                    r
                };
                if remaining > 0 {
                    state.completed_bytes += remaining;
                }
                state.completed_files += 1;
                if let Some(name) = state.file_to_pkg.remove(fname) {
                    state.currently_downloading = Some(fname.to_string());
                    state.last_render = Instant::now() - Duration::from_millis(100);
                    state.render();
                    state.completed_downloads.push(name);
                }
                state.currently_downloading = state.best_progress();
            }
            _ => {}
        }
    });

    let s = Arc::clone(&shared);
    handle.set_progress_cb(s, |_, _, _, _, _, arc| {
        arc.lock().unwrap().render();
    });

    let s = Arc::clone(&shared);
    handle.set_event_cb(s, |event, arc| {
        let mut state = arc.lock().unwrap();
        match event.event() {
            Event::ResolveDepsStart => {
                state.log.push(style::cyan("Resolving dependencies").to_string());
                state.render();
            }
            Event::ResolveDepsDone => {}
            Event::InterConflictsStart => {
                state.log.push(style::cyan("Checking inter-conflicts").to_string());
                state.render();
            }
            Event::InterConflictsDone => {}
            Event::PackageOperationDone(op) => {
                if !state.phase { state.next_phase(); }
                state.phase = true;
                let (label, msg) = match op.operation() {
                    PackageOperation::Install(p) => (style::green("Installing:"), format!("{} {}", p.name(), fmt::formatted_version(p.version()))),
                    PackageOperation::Upgrade(n, o) => (style::green("Upgrading:"), format!("{} {} -> {}", n.name(), o.version(), n.version())),
                    PackageOperation::Reinstall(n, _) => (style::green("Reinstalling:"), format!("{} {}", n.name(), fmt::formatted_version(n.version()))),
                    PackageOperation::Downgrade(n, o) => (style::yellow("Downgrading:"), format!("{} {} -> {}", n.name(), o.version(), n.version())),
                    PackageOperation::Remove(p) => (style::red("Removing:"), format!("{} {}", p.name(), fmt::formatted_version(p.version()))),
                };
                state.log.push(format!("{} {}", label, msg));
                state.render();
            }
            Event::ScriptletInfo(ev) => {
                let l = draw::sanitize(ev.line().trim());
                if !l.is_empty() { state.log.push(format!("    {}", style::dim(&l))); state.render(); }
            }
            Event::HookRunStart(ev) => {
                if !state.phase { state.next_phase(); }
                state.phase = true;
                let desc = draw::sanitize(ev.desc().unwrap_or(ev.name()));
                state.log.push(format!("{} {}", style::cyan("Processing:"), desc));
                state.render();
            }
            Event::HookRunDone(_) => {
                state.render();
            }
            Event::PacnewCreated(ev) => {
                let file = draw::sanitize(ev.file());
                state.log.push(format!("{} {} installed as .pacnew", style::yellow("warning:"), file));
                state.render();
            }
            Event::PacsaveCreated(ev) => {
                let file = draw::sanitize(ev.file());
                state.log.push(format!("{} {} saved as .pacsave", style::yellow("warning:"), file));
                state.render();
            }
            _ => {}
        }
    });

    let s = Arc::clone(&shared);
    handle.set_log_cb(s, |level, msg, arc| {
        if !level.intersects(alpm::LogLevel::WARNING | alpm::LogLevel::ERROR) {
            return;
        }
        let msg = draw::sanitize(msg.trim());
        if msg.is_empty() {
            return;
        }
        let mut state = arc.lock().unwrap();
        state.log.push(format!("{} {msg}", style::yellow("warning:")));
        state.render();
    });

    let s = Arc::clone(&shared);
    handle.set_question_cb(s, move |q, arc| {
        use alpm::Question;
        let pause = |arc: &Arc<Mutex<render::DownloadState>>| {
            arc.lock().unwrap().done();
        };
        let note = match q.question() {
            Question::InstallIgnorepkg(mut q) => {
                let name = draw::sanitize(q.pkg().name());
                let install = if assume_yes {
                    false
                } else {
                    pause(arc);
                    render::confirm_no(&style::bold(&format!(
                        "\n {name} is in IgnorePkg. Install it anyway? [y/N] "
                    )))
                };
                q.set_install(install);
                if install { "installing ignored package" } else { "keeping ignored package" }
            }
            Question::Replace(q) => {
                let old = draw::sanitize(q.oldpkg().name());
                let new = draw::sanitize(q.newpkg().name());
                let replace = if assume_yes {
                    true
                } else {
                    pause(arc);
                    render::confirm(&style::bold(&format!(
                        "\n Replace {old} with {new}? [Y/n] "
                    )))
                };
                q.set_replace(replace);
                if replace { "replacing package" } else { "keeping package" }
            }
            Question::Conflict(mut q) => {
                let (first, second) = {
                    let conflict = q.conflict();
                    (
                        draw::sanitize(conflict.package1().name()),
                        draw::sanitize(conflict.package2().name()),
                    )
                };
                let remove = if assume_yes {
                    false
                } else {
                    pause(arc);
                    render::confirm_no(&style::bold(&format!(
                        "\n {first} and {second} conflict. Remove the installed one? [y/N] "
                    )))
                };
                q.set_remove(remove);
                if remove { "removing conflicting package" } else { "keeping conflicting package" }
            }
            Question::SelectProvider(mut q) => {
                let depend = draw::sanitize(q.depend().name());
                let options: Vec<String> = q
                    .providers()
                    .iter()
                    .map(|pkg| format!("{} {}", draw::sanitize(pkg.name()), pkg.version()))
                    .collect();
                let index = if assume_yes || options.len() <= 1 {
                    0
                } else {
                    pause(arc);
                    render::select(&format!("\n Providers for {depend}:"), &options, 0)
                };
                q.set_index(index as i32);
                "selected provider"
            }
            Question::ImportKey(mut q) => {
                let import = if assume_yes {
                    true
                } else {
                    pause(arc);
                    render::confirm(&style::bold("\n Import the required PGP key? [Y/n] "))
                };
                q.set_import(import);
                if import { "imported PGP key" } else { "declined PGP key import" }
            }
            Question::Corrupted(mut q) => {
                let file = draw::sanitize(q.filepath());
                let remove = if assume_yes {
                    true
                } else {
                    pause(arc);
                    render::confirm(&style::bold(&format!(
                        "\n {file} is corrupted. Delete it? [Y/n] "
                    )))
                };
                q.set_remove(remove);
                if remove { "deleting corrupted package" } else { "keeping corrupted package" }
            }
            Question::RemovePkgs(mut q) => {
                let names: Vec<String> = q
                    .packages()
                    .iter()
                    .map(|pkg| draw::sanitize(pkg.name()))
                    .collect();
                let skip = if assume_yes {
                    false
                } else {
                    pause(arc);
                    !render::confirm(&style::bold(&format!(
                        "\n Remove {}? [Y/n] ",
                        names.join(", ")
                    )))
                };
                q.set_skip(skip);
                if skip { "skipped removal" } else { "proceeding with removal" }
            }
        };
        let mut state = arc.lock().unwrap();
        state.log.push(format!(" {}", style::dim(note)));
        state.render();
    });

    shared
}
