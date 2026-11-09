use waka::display::draw::strip_ansi;
use waka::pkg::list::upgrade_line;

#[test]
fn upgrade_line_shows_installed_and_available_versions() {
    let line = upgrade_line("core", "firefox", "136.0-1", "136.0-2");
    assert_eq!(strip_ansi(&line), " core/firefox 136.0-1 \u{2192} 136.0-2");
}

#[test]
fn unknown_repo_label_is_aur() {
    assert_eq!(waka::pkg::list::UNKNOWN_REPO, "aur");
}
