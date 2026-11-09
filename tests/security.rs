use waka::display::table::package_cell;
use waka::pkg::info::format_maintainer;
use waka::pkg::search::{search_lines, SearchRow};

#[test]
fn metadata_is_sanitized_before_display() {
    let maintainer = format_maintainer("Name \x1b]0;pwned\x07 <mail@example.com>");
    assert!(!maintainer.contains('\x1b'));
    assert!(!maintainer.contains('\u{07}'));

    let cell = package_cell("foo\x1b]0;pwned\x07", false);
    assert!(!cell.contains('\x1b'));
    assert!(!cell.contains('\u{07}'));

    let rows = vec![SearchRow {
        label: "core/foo".to_string(),
        width: 8,
        version: "1.0".to_string(),
        version_width: 3,
        size: "1 B".to_string(),
        size_width: 3,
        installed: false,
        desc: "bad \x1b]0;pwned\x07 description".to_string(),
    }];
    for line in search_lines(&rows, 80) {
        assert!(!line.contains('\x1b'), "{line:?}");
        assert!(!line.contains('\u{07}'), "{line:?}");
    }
}
