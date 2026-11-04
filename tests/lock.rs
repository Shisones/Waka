use waka::{clear_lockfile, owns_lockfile, set_lockfile};

#[test]
fn lock_ownership_tracks_claim_and_release() {
    clear_lockfile();
    assert!(!owns_lockfile());

    set_lockfile("/tmp/waka-test.lck");
    assert!(owns_lockfile());

    clear_lockfile();
    assert!(!owns_lockfile());
}
