use std::collections::HashSet;
use waka::commands::clean::{should_remove, unused_sync_db};

fn installed(names: &[&str]) -> HashSet<String> {
    names.iter().map(|n| n.to_string()).collect()
}

#[test]
fn keeps_the_installed_version() {
    let set = installed(&["bash-5.3-1"]);
    assert!(!should_remove("bash-5.3-1-x86_64.pkg.tar.zst", false, &set));
}

#[test]
fn removes_superseded_versions() {
    let set = installed(&["bash-5.3-1"]);
    assert!(should_remove("bash-5.2-1-x86_64.pkg.tar.zst", false, &set));
}

#[test]
fn database_files_follow_the_all_flag() {
    let set = installed(&[]);
    assert!(!should_remove("core.db", false, &set));
    assert!(should_remove("core.db", true, &set));
    assert!(!should_remove("core.db.sig", false, &set));
    assert!(should_remove("core.db.sig", true, &set));
}

#[test]
fn package_signatures_follow_their_archive() {
    let set = installed(&["bash-5.3-1"]);
    assert!(!should_remove("bash-5.3-1-x86_64.pkg.tar.zst.sig", true, &set));
    assert!(should_remove("bash-5.2-1-x86_64.pkg.tar.zst.sig", true, &set));
}

#[test]
fn partial_downloads_are_always_removed() {
    let set = installed(&["bash-5.3-1"]);
    assert!(should_remove("bash-5.3-1-x86_64.pkg.tar.zst.part", false, &set));
}

#[test]
fn only_unused_sync_databases_are_removed() {
    let configured: HashSet<String> = ["core", "extra", "multilib"]
        .iter()
        .map(|name| name.to_string())
        .collect();
    assert!(!unused_sync_db("core.db", &configured));
    assert!(!unused_sync_db("extra.files", &configured));
    assert!(!unused_sync_db("multilib.db.sig", &configured));
    assert!(unused_sync_db("old.db", &configured));
    assert!(unused_sync_db("old.db.sig", &configured));
    assert!(unused_sync_db("old.files", &configured));
    assert!(unused_sync_db("old.files.sig", &configured));
    assert!(unused_sync_db("old.db.part", &configured));
    assert!(!unused_sync_db("unrelated.txt", &configured));
}
