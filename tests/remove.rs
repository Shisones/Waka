use alpm::TransFlag;
use waka::commands::remove::remove_flags;

#[test]
fn remove_is_rcns_by_design() {
    let flags = remove_flags();
    assert!(flags.contains(TransFlag::CASCADE));
    assert!(flags.contains(TransFlag::RECURSE));
    assert!(flags.contains(TransFlag::NO_SAVE));
}
