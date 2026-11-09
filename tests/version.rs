use waka::version::{compose, full, PKG_VERSION};

#[test]
fn plain_without_git_metadata() {
    assert_eq!(compose("0.1.6", None, None, false), "0.1.6");
}

#[test]
fn distance_and_hash() {
    assert_eq!(
        compose("0.1.6", Some("26"), Some("7b0a3b0"), false),
        "0.1.6+26.7b0a3b0"
    );
}

#[test]
fn dirty_tree_is_marked() {
    assert_eq!(
        compose("0.1.6", Some("0"), Some("abc1234"), true),
        "0.1.6+0.abc1234.dirty"
    );
}

#[test]
fn rejects_non_semver_metadata() {
    assert_eq!(compose("0.1.6", Some("x"), Some("abc1234"), false), "0.1.6");
    assert_eq!(compose("0.1.6", Some("2"), Some("not hex"), false), "0.1.6");
}

#[test]
fn full_starts_with_package_version() {
    assert!(full().starts_with(PKG_VERSION));
}
