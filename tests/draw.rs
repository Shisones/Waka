use waka::display::draw::{display_width, progress_bar, strip_ansi};

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