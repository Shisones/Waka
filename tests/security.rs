use waka::display::draw::strip_ansi;
use waka::display::table::package_cell;
use waka::pkg::info::format_maintainer;
use waka::pkg::search::{search_lines, SearchRow};

#[test]
fn metadata_is_sanitized_before_display() {
    let maintainer = format_maintainer("Name \x1b]0;pwned\x07 <mail@example.com>");
    let maintainer_plain = strip_ansi(&maintainer);
    assert!(maintainer_plain.contains("]0;pwned"), "{maintainer:?}");
    assert!(!maintainer.contains('\u{07}'), "{maintainer:?}");
    assert!(!maintainer_plain.contains('\u{07}'), "{maintainer:?}");

    let cell = package_cell("foo\x1b]0;pwned\x07", false);
    let cell_plain = strip_ansi(&cell);
    assert!(cell_plain.contains("]0;pwned"), "{cell:?}");
    assert!(!cell.contains('\u{07}'), "{cell:?}");
    assert!(!cell_plain.contains('\u{07}'), "{cell:?}");

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
    let mut saw_payload = false;
    for line in search_lines(&rows, 80) {
        let plain = strip_ansi(&line);
        if plain.contains("]0;pwned") {
            saw_payload = true;
        }
        assert!(!line.contains('\u{07}'), "{line:?}");
        assert!(!plain.contains('\u{07}'), "{line:?}");
    }
    assert!(saw_payload, "sanitized payload should remain as inert text");
}
