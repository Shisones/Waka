use waka::display::style::{colors_enabled, Color};

#[test]
fn colors_use_expected_sgr_codes() {
    assert_eq!(Color::Blue.code(), "1;34");
    assert_eq!(Color::Cyan.code(), "1;36");
}

#[test]
fn piped_stdout_disables_color() {
    assert!(!colors_enabled(false, false));
    assert!(colors_enabled(true, false));
    assert!(!colors_enabled(true, true));
}
