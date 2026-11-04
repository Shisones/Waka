use std::collections::HashSet;
use waka::alpm::operations::should_remove;

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
fn package_signatures_are_never_removed_here() {
    let set = installed(&[]);
    assert!(!should_remove("bash-5.3-1-x86_64.pkg.tar.zst.sig", true, &set));
}
