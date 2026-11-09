mod common;

use common::download_state;
use std::collections::HashMap;
use std::process::Command;
use waka::display::db::{bar_line, db_sync_lines};
use waka::display::draw::{
    box_bottom_w, box_row_w, box_top_w, clear_block, display_width, progress_bar, sanitize,
    strip_ansi, truncate,
};
use waka::display::format::{format_size, format_speed, pick_unit};
use waka::display::style::{colors_enabled, error, Color};
use waka::display::table::{pad, package_cell, plural, summary_row_line};
use waka::pkg::info::format_maintainer;
use waka::pkg::search::{matches_terms, search_lines, SearchRow};
use waka::version::{compose, PKG_VERSION};

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_waka"))
        .args(args)
        .output()
        .unwrap()
}

fn file(path: &str) -> String {
    std::fs::read_to_string(format!("{}/{}", env!("CARGO_MANIFEST_DIR"), path)).unwrap()
}

#[test]
fn box_top_with_empty_title_fills_width() {
    let line = box_top_w("", 10);
    assert_eq!(line, "┌────────┐");
    assert_eq!(display_width(&line), 10);
}

#[test]
fn box_top_truncates_a_long_title() {
    let line = box_top_w(&"x".repeat(100), 20);
    assert_eq!(display_width(&line), 20);
}

#[test]
fn box_row_of_empty_content_fills_width() {
    assert_eq!(display_width(&box_row_w("", 5)), 5);
}

#[test]
fn box_bottom_rejects_widths_below_two() {
    assert!(box_bottom_w(1).is_empty());
}

#[test]
fn truncate_keeps_text_that_fits() {
    assert_eq!(truncate("abc", 3), "abc");
}

#[test]
fn truncate_adds_ellipsis() {
    assert_eq!(truncate("abcdef", 3), "ab…");
}

#[test]
fn truncate_handles_wide_characters() {
    assert!(display_width(&truncate("你好世界", 5)) <= 5);
}

#[test]
fn sanitize_replaces_newlines() {
    assert_eq!(sanitize("a\nb"), "a b");
}

#[test]
fn display_width_of_empty_text_is_zero() {
    assert_eq!(display_width(""), 0);
}

#[test]
fn progress_bar_clamps_negative_percent() {
    assert_eq!(progress_bar(-10.0, 10), "─".repeat(10));
}

#[test]
fn progress_bar_handles_one_column() {
    assert_eq!(display_width(&progress_bar(50.0, 1)), 1);
}

#[test]
fn clear_block_clears_each_row() {
    let seq = clear_block(1);
    assert_eq!(seq.matches("\x1b[1A").count(), 1);
    assert_eq!(seq.matches("\x1b[2K").count(), 1);
}

#[test]
fn strip_ansi_removes_multiple_codes() {
    assert_eq!(strip_ansi("\x1b[1m\x1b[31mx\x1b[0m"), "x");
}

#[test]
fn box_row_ignores_ansi_for_width() {
    assert_eq!(display_width(&box_row_w("\x1b[31mx\x1b[0m", 5)), 5);
}

#[test]
fn summary_row_at_wide_width_shows_all_columns() {
    let line = summary_row_line("firefox", "135.0-1", "136.0-1", "12 MiB", "80 MiB", 120);
    assert!(line.contains("135.0-1"));
    assert!(line.contains("136.0-1"));
    assert!(line.contains("12 MiB"));
    assert!(line.contains("80 MiB"));
}

#[test]
fn summary_row_at_39_drops_old_version() {
    let line = summary_row_line("firefox", "135.0-1", "136.0-1", "12 MiB", "80 MiB", 39);
    assert!(!line.contains("135.0-1"));
    assert!(line.contains("136.0"));
}

#[test]
fn summary_row_at_30_drops_net_change() {
    let line = summary_row_line("firefox", "135.0-1", "136.0-1", "12 MiB", "80 MiB", 30);
    assert!(!line.contains("12 MiB"));
    assert!(line.contains("80 MiB"));
}

#[test]
fn summary_row_at_20_drops_size() {
    let line = summary_row_line("firefox", "135.0-1", "136.0-1", "12 MiB", "80 MiB", 20);
    assert!(!line.contains("80 MiB"));
    assert!(line.contains("firefox"));
}

#[test]
fn summary_row_at_10_keeps_name() {
    let line = summary_row_line("firefox", "135.0-1", "136.0-1", "12 MiB", "80 MiB", 10);
    assert!(display_width(&line) <= 10);
    assert!(line.contains("f"));
}

#[test]
fn summary_row_at_zero_width_is_empty() {
    assert!(summary_row_line("firefox", "", "", "", "", 0).is_empty());
}

#[test]
fn pad_right_aligns() {
    assert_eq!(pad("x", 3, true), "  x");
}

#[test]
fn pad_left_aligns() {
    assert_eq!(pad("x", 3, false), "x  ");
}

#[test]
fn plural_handles_zero_and_one() {
    assert_eq!(plural("file", 0), "files");
    assert_eq!(plural("file", 1), "file");
}

#[test]
fn package_cell_without_dependency_has_no_indent() {
    assert_eq!(package_cell("firefox", false), "firefox");
}

#[test]
fn format_size_one_byte() {
    assert_eq!(format_size(1), "1.0 B");
}

#[test]
fn format_size_1023_bytes() {
    assert_eq!(format_size(1023), "1023 B");
}

#[test]
fn format_size_negative_mib() {
    assert_eq!(format_size(-1536), "-1.5 KiB");
}

#[test]
fn format_speed_zero() {
    assert_eq!(format_speed(0.0), "0 B/s");
}

#[test]
fn format_speed_gib() {
    assert_eq!(format_speed(1024.0 * 1024.0 * 1024.0), "1.0 GiB/s");
}

#[test]
fn pick_unit_zero_is_bytes() {
    assert_eq!(pick_unit(0), (1, "B"));
}

#[test]
fn bar_line_zero_progress_has_no_filled_cell() {
    let line = bar_line("core", 0, 100, 4, 5, 80, 0.0);
    assert!(!line.contains('━'));
    assert!(line.contains('─'));
}

#[test]
fn bar_line_half_progress_has_equal_cells() {
    let line = bar_line("core", 50, 100, 4, 5, 80, 0.0);
    assert_eq!(
        line.chars().filter(|c| *c == '━').count(),
        line.chars().filter(|c| *c == '─').count()
    );
}

#[test]
fn bar_line_full_progress_has_no_empty_cell() {
    let line = bar_line("core", 100, 100, 4, 5, 80, 0.0);
    assert!(line.contains('━'));
    assert!(!line.contains('─'));
}

#[test]
fn bar_line_long_name_fits() {
    let line = bar_line(&"x".repeat(40), 0, 0, 40, 3, 40, 100.0);
    assert!(display_width(&line) <= 40);
}

#[test]
fn bar_line_negative_bytes_do_not_panic() {
    let line = bar_line("core", -5, 100, 4, 5, 80, 0.0);
    assert!(display_width(&line) <= 80);
}

#[test]
fn db_sync_lines_partial_repo_has_both_bar_cells() {
    let mut bytes = HashMap::new();
    bytes.insert("core".to_string(), (50, 100));
    let lines = db_sync_lines(&bytes, &["core".to_string()]);
    assert!(lines[0].contains('━'));
    assert!(lines[0].contains('─'));
}

#[test]
fn db_sync_lines_empty_list_is_empty() {
    assert!(db_sync_lines(&HashMap::new(), &[]).is_empty());
}

#[test]
fn prepare_download_with_no_packages() {
    let mut state = download_state(0, 0);
    state.prepare_download(0, &[]);
    assert_eq!(state.rendered, 0);
    assert_eq!(state.total_pkgs, 0);
    assert_eq!(state.total_bytes, 0);
}

#[test]
fn complete_download_with_zero_total() {
    let mut state = download_state(0, 0);
    state.complete_download();
    assert_eq!(state.completed_bytes, 0);
    assert_eq!(state.completed_files, 0);
}

#[test]
fn log_box_shows_preparing_when_empty() {
    let state = download_state(0, 0);
    let lines = state.build_at(80, 24);
    assert!(lines.iter().any(|line| line.contains("Preparing")));
}

#[test]
fn log_box_shows_only_the_tail() {
    let mut state = download_state(0, 0);
    state.log = (0..10).map(|index| format!("line{index}")).collect();
    let lines = state.build_at(80, 24);
    assert!(lines.iter().any(|line| line.contains("line9")));
    assert!(!lines.iter().any(|line| line.contains("line0")));
}

#[test]
fn log_rows_field_limits_the_tail() {
    let mut state = download_state(0, 0);
    state.log_rows = 2;
    state.log = vec!["one".to_string(), "two".to_string(), "three".to_string()];
    let lines = state.build_at(80, 24);
    assert!(lines.iter().any(|line| line.contains("three")));
    assert!(!lines.iter().any(|line| line.contains("one")));
}

#[test]
fn eta_with_no_total_is_unknown() {
    let state = download_state(0, 0);
    assert_eq!(state.eta(1.0), "-:--:--");
}

#[test]
fn eta_when_complete_is_zero() {
    let state = download_state(100, 100);
    assert_eq!(state.eta(1.0), "0:00:00");
}

#[test]
fn speed_with_no_bytes_is_zero() {
    let state = download_state(0, 100);
    assert_eq!(state.speed(1.0), "0 B/s");
}

#[test]
fn download_status_with_zero_total() {
    let state = download_state(0, 0);
    assert_eq!(state.dl_status(), "0/1 B");
}

#[test]
fn capped_bytes_never_negative() {
    assert_eq!(waka::display::render::capped_bytes(-1, Some(10)), 0);
}

#[test]
fn archive_name_with_zst_extension() {
    assert_eq!(
        waka::display::render::pkg_name_from_filename("foo-1.0-1-any.pkg.tar.zst"),
        "foo"
    );
}

#[test]
fn color_codes_are_stable() {
    assert_eq!(Color::Red.code(), "1;31");
    assert_eq!(Color::Yellow.code(), "1;33");
    assert_eq!(Color::Green.code(), "1;32");
}

#[test]
fn error_prefix_is_plain_when_color_is_off() {
    assert!(error("bad").starts_with("error:"));
}

#[test]
fn color_is_off_when_no_color_is_set() {
    assert!(!colors_enabled(true, true));
    assert!(!colors_enabled(false, false));
}

#[test]
fn terms_match_across_case() {
    assert!(matches_terms("Firefox", "Browser", &["fire".into()]));
}

#[test]
fn empty_terms_match_everything() {
    assert!(matches_terms("firefox", "browser", &[]));
}

#[test]
fn search_lines_empty_input_is_empty() {
    assert!(search_lines(&[], 80).is_empty());
}

#[test]
fn search_lines_shows_installed_tag_when_wide() {
    let rows = vec![SearchRow {
        label: "core/firefox".to_string(),
        width: 12,
        version: "136.0-1".to_string(),
        version_width: 7,
        size: "80 MiB".to_string(),
        size_width: 6,
        installed: true,
        desc: "Fast".to_string(),
    }];
    let lines = search_lines(&rows, 120);
    assert!(lines[0].contains("[installed]"));
}

#[test]
fn search_lines_moves_installed_tag_when_narrow() {
    let rows = vec![SearchRow {
        label: "core/firefox".to_string(),
        width: 12,
        version: "136.0-1".to_string(),
        version_width: 7,
        size: "80 MiB".to_string(),
        size_width: 6,
        installed: true,
        desc: "Fast".to_string(),
    }];
    let lines = search_lines(&rows, 30);
    assert!(!lines[0].contains("[installed]"));
    assert!(lines[1].contains("[installed]"));
}

#[test]
fn search_lines_fit_narrow_widths() {
    let rows = vec![SearchRow {
        label: "core/firefox".to_string(),
        width: 12,
        version: "136.0-1".to_string(),
        version_width: 7,
        size: "80 MiB".to_string(),
        size_width: 6,
        installed: false,
        desc: "A very long description that must be truncated".to_string(),
    }];
    for width in 12..=120 {
        for line in search_lines(&rows, width) {
            assert!(display_width(&line) <= width);
        }
    }
}

#[test]
fn search_lines_emit_description_rows() {
    let rows = vec![
        SearchRow {
            label: "core/a".to_string(),
            width: 6,
            version: "1".to_string(),
            version_width: 1,
            size: "1 B".to_string(),
            size_width: 3,
            installed: false,
            desc: "first".to_string(),
        },
        SearchRow {
            label: "core/b".to_string(),
            width: 6,
            version: "1".to_string(),
            version_width: 1,
            size: "1 B".to_string(),
            size_width: 3,
            installed: false,
            desc: "second".to_string(),
        },
    ];
    assert_eq!(search_lines(&rows, 80).len(), 4);
}

#[test]
fn maintainer_without_email_is_unchanged() {
    assert_eq!(format_maintainer("Someone"), "Someone");
}

#[test]
fn maintainer_empty_is_empty() {
    assert_eq!(format_maintainer(""), "");
}

#[test]
fn maintainer_with_broken_angles_does_not_panic() {
    assert!(!format_maintainer("x> y <z").is_empty());
}

#[test]
fn compose_zero_distance() {
    assert_eq!(
        compose("0.1.6", Some("0"), Some("abcdef1"), false),
        "0.1.6+0.abcdef1"
    );
}

#[test]
fn compose_invalid_distance_falls_back() {
    assert_eq!(compose("0.1.6", Some("x"), Some("abcdef1"), false), "0.1.6");
}

#[test]
fn compose_invalid_hash_falls_back() {
    assert_eq!(compose("0.1.6", Some("2"), Some("not hex"), false), "0.1.6");
}

#[test]
fn compose_missing_hash_falls_back() {
    assert_eq!(compose("0.1.6", None, Some("abc"), false), "0.1.6");
}

#[test]
fn help_lists_commands() {
    let output = run(&["--help"]);
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("update"));
    assert!(text.contains("install"));
    assert!(text.contains("remove"));
    assert!(text.contains("meow"));
    assert!(!text.contains("--aur"));
    assert!(!text.contains("--paranoid"));
}

#[test]
fn version_output_starts_with_package_version() {
    let output = run(&["--version"]);
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.starts_with(&format!("waka {PKG_VERSION}")));
}

#[test]
fn install_help_mentions_local_archives() {
    let output = run(&["install", "--help"]);
    assert!(String::from_utf8_lossy(&output.stdout).contains(".pkg.tar."));
}

#[test]
fn clean_help_mentions_all() {
    let output = run(&["clean", "--help"]);
    assert!(String::from_utf8_lossy(&output.stdout).contains("--all"));
}

#[test]
fn pkgbuild_depends_on_pacman() {
    assert!(file("PKGBUILD").contains("depends=('glibc' 'gcc-libs' 'pacman')"));
}

#[test]
fn binary_pkgbuild_provides_waka() {
    assert!(file("dist/PKGBUILD").contains("provides=('waka')"));
}

#[test]
fn readme_lists_meow() {
    assert!(file("README.md").contains("| meow"));
}

#[test]
fn readme_has_no_aur_section() {
    assert!(!file("README.md").contains("AUR"));
}

#[test]
fn cargo_uses_edition_2024() {
    assert!(file("Cargo.toml").contains("edition = \"2024\""));
}
