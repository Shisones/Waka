use std::collections::HashSet;
use waka::commands::clean::{should_remove, CLEAN_PROMPT};

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
fn ignores_files_that_are_not_package_archives() {
    let set = installed(&[]);
    assert!(!should_remove("notes.txt", false, &set));
    assert!(!should_remove("notes.txt", true, &set));
    assert!(!should_remove("source-1.0-1.src.tar.gz", false, &set));
    assert!(!should_remove("core.files", false, &set));
}

#[test]
fn clean_prompt_has_one_blank_line() {
    assert_eq!(CLEAN_PROMPT.matches('\n').count(), 1);
}

#[test]
fn clean_does_not_add_a_second_blank_line() {
    let source = include_str!("../src/commands/clean.rs");
    let info = source.find("will be freed").expect("info line");
    let prompt = info + source[info..].find("CLEAN_PROMPT").expect("prompt");
    assert!(!source[info..prompt].contains("println!();"));
}
