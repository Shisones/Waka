mod common;

use common::download_state;
use waka::display::render::pkg_name_from_filename;

#[test]
fn pkg_name_from_arch_specific_package() {
    assert_eq!(pkg_name_from_filename("firefox-136.0-1-x86_64.pkg.tar.zst"), "firefox");
}

#[test]
fn pkg_name_from_any_package() {
    assert_eq!(pkg_name_from_filename("python-3.13.0-1-any.pkg.tar.zst"), "python");
}

#[test]
fn pkg_name_with_multiple_dashes() {
    assert_eq!(pkg_name_from_filename("a-b-c.pkg.tar.xz"), "a");
}

#[test]
fn pkg_name_without_marker() {
    assert_eq!(pkg_name_from_filename("something-1.0.pkg.tar.zst"), "something");
}

#[test]
fn eta_unknown_when_nothing_downloaded() {
    let s = download_state(0, 1024 * 1024);
    assert_eq!(s.eta(1.0), "-:--:--");
}

#[test]
fn eta_estimates_remaining_time() {
    let s = download_state(512 * 1024, 1024 * 1024);
    assert_eq!(s.eta(1.0), "0:00:01");
}

#[test]
fn dl_status_shows_fraction() {
    let s = download_state(512 * 1024, 1024 * 1024);
    assert_eq!(s.dl_status(), "0.5/1.0 MiB");
}

#[test]
fn dl_status_uses_bytes_precision() {
    let s = download_state(500, 1000);
    assert_eq!(s.dl_status(), "500/1000 B");
}