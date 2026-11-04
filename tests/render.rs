mod common;

use common::download_state;
use waka::display::draw::display_width;
use waka::display::render::pkg_name_from_filename;

fn assert_fits(state: &waka::display::render::DownloadState, w: usize, rows: usize) {
    let lines = state.build_at(w, rows);
    assert!(lines.len() <= rows.max(3), "{} lines at w={w} rows={rows}", lines.len());
    for line in &lines {
        assert!(display_width(line) <= w, "w={w} rows={rows} line={line:?}");
    }
}

#[test]
fn download_box_fits_small_terminals() {
    let mut s = download_state(512 * 1024, 4 * 1024 * 1024);
    s.total_files = 4;
    s.completed_downloads = vec!["a-very-long-package-name-1.0-1-x86_64".to_string()];
    s.currently_downloading = Some("another-long-package-name-2.0-1-x86_64.pkg.tar.zst".to_string());
    for w in 1..=120 {
        for rows in [4, 6, 24, 60] {
            assert_fits(&s, w, rows);
        }
    }
}

#[test]
fn log_box_fits_small_terminals() {
    let mut s = download_state(0, 0);
    s.phase = true;
    s.log = vec!["\x1b[1;32mInstalling:\x1b[0m firefox 136.0-1".to_string(); 20];
    for w in 1..=120 {
        for rows in [4, 8, 24] {
            assert_fits(&s, w, rows);
        }
    }
}


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
#[test]
fn pending_log_reports_unprinted_entries() {
    let mut s = download_state(0, 0);
    s.log.push("Resolving dependencies".to_string());
    assert_eq!(s.pending_log(), ["Resolving dependencies"]);
    s.plain_log = 1;
    assert!(s.pending_log().is_empty());
}

#[test]
fn current_download_uses_the_package_name() {
    let mut s = download_state(0, 1024);
    s.total_files = 1;
    s.currently_downloading = Some("firefox-136.0-1-x86_64.pkg.tar.zst".to_string());
    s.file_to_pkg.insert(
        "firefox-136.0-1-x86_64.pkg.tar.zst".to_string(),
        "firefox".to_string(),
    );
    let lines = s.build_at(80, 24);
    let line = lines.iter().find(|l| l.contains("Downloading:")).expect("download row");
    assert!(line.contains("firefox"), "{line:?}");
    assert!(!line.contains(".pkg.tar"), "{line:?}");
}

#[test]
fn progress_bar_ends_with_percentage_and_details_split_left_right() {
    let mut s = download_state(512, 2048);
    s.total_files = 1;
    let wide = s.build_at(120, 24);

    let bar = wide.iter().find(|l| l.contains('━')).expect("bar row");
    assert_eq!(display_width(bar), 120, "{bar:?}");
    let bar_text = bar.trim_end_matches('\u{2502}').trim_end();
    assert!(bar_text.ends_with("25.0%"), "{bar:?}");

    let details = wide.iter().find(|l| l.contains("ETA")).expect("detail row");
    let rest = details.strip_prefix("\u{2502} ").expect("box prefix");
    assert!(rest.starts_with("ETA "), "{details:?}");
    assert!(rest.contains("KiB"), "{details:?}");
    let before_border = details.trim_end_matches('\u{2502}').trim_end();
    assert!(before_border.ends_with("/s"), "speed should sit on the right: {details:?}");

    let narrow = s.build_at(30, 24);
    let details = narrow.iter().find(|l| l.contains("ETA")).expect("narrow detail");
    assert!(!details.contains("/s"), "speed should be dropped: {details:?}");
}
