use waka::display::draw::display_width;
use waka::display::table::{pad, summary_row_line};

#[test]
fn summary_row_fits_every_width() {
    for w in 1..=120 {
        let line = summary_row_line("firefox", "135.0-1", "136.0-1", "12.0 MiB", "80.0 MiB", w);
        assert!(display_width(&line) <= w, "w={w}: {line:?}");
    }
}

#[test]
fn summary_row_drops_columns_as_it_narrows() {
    let wide = summary_row_line("firefox", "135.0-1", "136.0-1", "12.0 MiB", "80.0 MiB", 80);
    assert!(wide.contains("135.0-1"));
    assert!(wide.contains("80.0 MiB"));
    let narrow = summary_row_line("firefox", "135.0-1", "136.0-1", "12.0 MiB", "80.0 MiB", 20);
    assert!(!narrow.contains("135.0-1"));
    assert!(narrow.contains("firefox"));
    assert!(narrow.contains("136.0-1"));
}

#[test]
fn pad_does_not_widen_content() {
    assert_eq!(display_width(&pad("abcdef", 3, false)), 3);
    assert_eq!(display_width(&pad("ab", 5, false)), 5);
}
