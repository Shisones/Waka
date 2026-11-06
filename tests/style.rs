use waka::display::style::Color;

#[test]
fn colors_use_expected_sgr_codes() {
    assert_eq!(Color::Blue.code(), "1;34");
    assert_eq!(Color::Cyan.code(), "1;36");
}
