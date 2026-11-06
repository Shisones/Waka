use waka::display::draw::display_width;
use waka::pkg::search::{matches_terms, search_lines, SearchRow};

#[test]
fn all_terms_must_match() {
    let terms = vec!["fire".to_string(), "browser".to_string()];
    assert!(matches_terms("firefox", "fast web browser", &terms));
    assert!(!matches_terms(
        "firefox",
        "fast web browser",
        &["fire".into(), "mail".into()]
    ));
}

#[test]
fn terms_can_match_the_description() {
    assert!(matches_terms("firefox", "Fast, private browser", &["private".into()]));
}

#[test]
fn search_rows_fit_every_width() {
    let rows = vec![SearchRow {
        label: "core/firefox".to_string(),
        width: 12,
        version: "136.0-1".to_string(),
        version_width: 7,
        size: "80 MiB".to_string(),
        size_width: 6,
        installed: true,
        desc: "Fast, private browser".to_string(),
    }];
    for w in 12..=120 {
        for line in search_lines(&rows, w) {
            assert!(display_width(&line) <= w, "w={w}: {line:?}");
        }
    }
}
