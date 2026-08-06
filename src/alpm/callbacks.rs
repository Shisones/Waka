use crate::display::{format as fmt, render, style};
use alpm::{DownloadEvent, Event, PackageOperation};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub fn callbacks(handle: &alpm::Alpm, title: &str) -> Arc<Mutex<render::DownloadState>> {
    let shared = Arc::new(Mutex::new(render::DownloadState {
        rendered: 0, title: style::bold(title), phase: false,
        total_bytes: 0, completed_bytes: 0, total_files: 0, completed_files: 0,
        currently_downloading: None,
        completed_downloads: Vec::new(), file_to_pkg: HashMap::new(),
        start: Instant::now(), last_render: Instant::now(),
        per_file: HashMap::new(), log: Vec::new(),
    }));

    let s = Arc::clone(&shared);
    handle.set_dl_cb(s, |filename, event, arc| {
        let fname = filename.rsplit('/').next().unwrap_or(filename);
        let mut state = arc.lock().unwrap();
        match event.event() {
            DownloadEvent::Init(_) => {
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
                let delta = {
                    let entry = state.per_file.entry(fname.to_string()).or_insert(render::FileProgress { downloaded: 0, total: 0 });
                    if entry.total == 0 && prog.total > 0 { entry.total = prog.total; }
                    let d = prog.downloaded - entry.downloaded;
                    if d > 0 { entry.downloaded = prog.downloaded; }
                    d
                };
                if delta > 0 { state.completed_bytes += delta; }
                state.currently_downloading = state.best_progress().or(state.currently_downloading.clone());
                state.render();
            }
            DownloadEvent::Completed(comp) => {
                if comp.total > 0 {
                    let remaining = {
                        let entry = state.per_file.entry(fname.to_string()).or_insert(render::FileProgress { downloaded: 0, total: 0 });
                        if entry.total == 0 { entry.total = comp.total; }
                        let r = comp.total - entry.downloaded;
                        if r > 0 { entry.downloaded = comp.total; }
                        r
                    };
                    if remaining > 0 {
                        state.completed_bytes += remaining;
                    }
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
                let l = ev.line().trim().to_string();
                if !l.is_empty() { state.log.push(format!("    {}", style::dim(&l))); state.render(); }
            }
            Event::HookRunStart(ev) => {
                if !state.phase { state.next_phase(); }
                state.phase = true;
                let desc = ev.desc().unwrap_or(ev.name());
                state.log.push(format!("{} {}", style::cyan("Processing:"), desc));
                state.render();
            }
            Event::HookRunDone(_) => {
                state.render();
            }
            _ => {}
        }
    });

    handle.set_question_cb((), |q, _| {
        use alpm::Question;
        match q.question() {
            Question::InstallIgnorepkg(mut q) => q.set_install(true),
            Question::Replace(q) => q.set_replace(true),
            Question::Conflict(mut q) => q.set_remove(true),
            Question::SelectProvider(mut q) => q.set_index(0),
            Question::ImportKey(mut q) => q.set_import(true),
            Question::Corrupted(mut q) => q.set_remove(true),
            Question::RemovePkgs(mut q) => q.set_skip(false),
        }
    });

    shared
}
