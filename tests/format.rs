use waka::display::format::{format_size, format_speed, pick_unit};

#[test]
fn format_size_zero() {
    assert_eq!(format_size(0), "0 B");
}

#[test]
fn format_size_small() {
    assert_eq!(format_size(512), "512 B");
    assert_eq!(format_size(1023), "1023 B");
}

#[test]
fn format_size_boundaries() {
    assert_eq!(format_size(1024), "1.0 KiB");
    assert_eq!(format_size(1024 * 1024), "1.0 MiB");
    assert_eq!(format_size(1024 * 1024 * 1024), "1.0 GiB");
}

#[test]
fn format_size_precision_switch() {
    assert_eq!(format_size(1536), "1.5 KiB");
    assert_eq!(format_size(10240), "10 KiB");
}

#[test]
fn format_size_negative() {
    assert_eq!(format_size(-512), "-512 B");
    assert_eq!(format_size(-1536), "-1.5 KiB");
}

#[test]
fn format_speed_units() {
    assert_eq!(format_speed(0.0), "0 B/s");
    assert_eq!(format_speed(500.0), "500 B/s");
    assert_eq!(format_speed(1024.0), "1.0 KiB/s");
    assert_eq!(format_speed(1024.0 * 1024.0), "1.0 MiB/s");
    assert_eq!(format_speed(1024.0 * 1024.0 * 1024.0), "1.0 GiB/s");
}

#[test]
fn pick_unit_matches_size() {
    assert_eq!(pick_unit(1), (1, "B"));
    assert_eq!(pick_unit(1024), (1024, "KiB"));
    assert_eq!(pick_unit(1024 * 1024), (1024 * 1024, "MiB"));
}