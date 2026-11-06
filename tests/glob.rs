use waka::sys::glob_match;

#[test]
fn holdpkg_entries_match_like_pacman() {
    assert!(glob_match("pacman", "pacman"));
    assert!(!glob_match("pacman", "pacman-git"));
    assert!(glob_match("lib*", "libarchive"));
    assert!(glob_match("*-git", "foo-git"));
    assert!(!glob_match("*kit", "foo-git"));
}
