use std::collections::HashMap;
use waka::display::db::{bar_line, db_sync_lines, DbSyncState};

fn state_with_repos(repos: &[&str]) -> DbSyncState {
    let mut s = DbSyncState::default();
    for r in repos {
        s.bytes.insert(r.to_string(), (0, 0));
    }
    s
}

#[test]
fn update_tracks_downloaded_bytes() {
    let mut s = state_with_repos(&["core", "extra"]);
    s.update("core.db.part", 512, 1024);
    s.update("extra.db", 2048, 4096);
    assert_eq!(s.bytes.get("core"), Some(&(512, 1024)));
    assert_eq!(s.bytes.get("extra"), Some(&(2048, 4096)));
}

#[test]
fn update_completed_sets_full_total() {
    let mut s = state_with_repos(&["core"]);
    s.update("core.db.part", 999, 1024);
    s.update("core.db", 1024, 1024);
    assert_eq!(s.bytes.get("core"), Some(&(1024, 1024)));
}

#[test]
fn update_accepts_plain_names_and_paths() {
    let mut s = state_with_repos(&["core", "extra"]);
    s.update("core.db", 100, 200);
    s.update("/var/lib/pacman/sync/extra.db.part", 300, 400);
    assert_eq!(s.bytes.get("core"), Some(&(100, 200)));
    assert_eq!(s.bytes.get("extra"), Some(&(300, 400)));
}

#[test]
fn update_marks_repo_active() {
    let mut s = state_with_repos(&["core", "extra"]);
    assert!(s.active_repos().is_empty());
    s.update("core.db.part", 1, 2);
    assert_eq!(s.active_repos(), vec!["core".to_string()]);
    s.update("extra.db", 1, 2);
    assert_eq!(s.active_repos(), vec!["core".to_string(), "extra".to_string()]);
}

#[test]
fn update_ignores_signatures_and_unknown_files() {
    let mut s = state_with_repos(&["core"]);
    s.update("core.db.sig", 100, 100);
    s.update("unknown.db.part", 100, 100);
    s.update("unrelated.txt", 100, 100);
    assert!(s.active_repos().is_empty());
    assert_eq!(s.bytes.get("core"), Some(&(0, 0)));
}

#[test]
fn bar_line_fills_according_to_percent() {
    let line = bar_line("core", 1024, 2048, 4, 7, 80, 0.0);
    assert!(line.starts_with(" core"));
    assert!(line.contains("1.0 KiB"));
    assert_eq!(line.chars().filter(|&c| c == '━').count(), 10);
    assert_eq!(line.chars().filter(|&c| c == '─').count(), 10);
}

#[test]
fn bar_line_unknown_total_uses_unknown_pct() {
    let empty = bar_line("core", 0, 0, 4, 7, 80, 0.0);
    assert_eq!(empty.chars().filter(|&c| c == '━').count(), 0);

    let done = bar_line("core", 0, 0, 4, 7, 80, 100.0);
    assert!(done.chars().filter(|&c| c == '━').count() > 0);
}

#[test]
fn db_sync_lines_prints_all_repos_in_given_order() {
    let mut b: HashMap<String, (i64, i64)> = HashMap::new();
    b.insert("core".into(), (1024, 1024));
    b.insert("extra".into(), (512, 2048));
    let names = vec!["core".to_string(), "extra".to_string()];
    let lines = db_sync_lines(&b, &names);
    assert_eq!(lines.len(), 2);
    assert!(lines[0].starts_with(" core"));
    assert!(lines[1].starts_with(" extra"));
}

#[test]
fn db_sync_lines_marks_untouched_repo_as_done() {
    let mut b: HashMap<String, (i64, i64)> = HashMap::new();
    b.insert("core".into(), (0, 0));
    let lines = db_sync_lines(&b, &["core".to_string()]);
    assert!(lines[0].contains('━'));
    assert!(lines[0].contains("0 B"));
}