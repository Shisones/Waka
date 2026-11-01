use waka::display::draw::{box_bottom_w, box_row_w, box_top_w, display_width, progress_bar, strip_ansi, truncate};

#[test]
fn box_lines_never_exceed_width() {
    let long = "\x1b[1;32mfirefox-136.0-1-x86_64.pkg.tar.zst\x1b[0m";
    for w in 1..=120 {
        assert!(display_width(&box_top_w("Download", w)) <= w, "top at {w}");
        assert!(display_width(&box_bottom_w(w)) <= w, "bottom at {w}");
        assert!(display_width(&box_row_w(long, w)) <= w, "row at {w}");
    }
}

#[test]
fn truncate_keeps_escape_sequences_whole() {
    let out = truncate("\x1b[1;32mabcdefghij\x1b[0m", 5);
    assert!(display_width(&out) <= 5);
    assert!(out.starts_with("\x1b[1;32m"));
    assert!(out.contains("\x1b[0m"));
    assert!(out.ends_with('\u{2026}'));
    assert_eq!(strip_ansi(&out), "abcd\u{2026}");
}

#[test]
fn truncate_zero_columns_is_empty() {
    assert_eq!(truncate("anything", 0), "");
}

#[test]
fn progress_bar_handles_zero_width() {
    assert_eq!(progress_bar(50.0, 0), "");
}

#[test]
fn progress_bar_full_and_empty() {
    assert_eq!(progress_bar(100.0, 10), "━".repeat(10));
    assert_eq!(progress_bar(0.0, 10), "─".repeat(10));
}

#[test]
fn progress_bar_half() {
    assert_eq!(progress_bar(50.0, 10), format!("{}{}", "━".repeat(5), "─".repeat(5)));
}

#[test]
fn progress_bar_rounds_down() {
    assert_eq!(progress_bar(25.0, 10), format!("{}{}", "━".repeat(2), "─".repeat(8)));
}

#[test]
fn progress_bar_clamps_overflow_and_rejects_nan() {
    assert_eq!(progress_bar(150.0, 10), "━".repeat(10));
    assert_eq!(progress_bar(f64::NAN, 10), "─".repeat(10));
}

#[test]
fn strip_ansi_removes_codes() {
    assert_eq!(strip_ansi("\x1b[31mred\x1b[0m"), "red");
    assert_eq!(strip_ansi("\x1b[1;32mbold green\x1b[0m"), "bold green");
    assert_eq!(strip_ansi("plain"), "plain");
}

#[test]
fn display_width_ignores_ansi() {
    assert_eq!(display_width("\x1b[1mhi\x1b[0m"), 2);
}

#[test]
fn display_width_counts_wide_chars() {
    assert_eq!(display_width("héllo"), 5);
    assert_eq!(display_width("你好"), 4);
}