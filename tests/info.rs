use waka::pkg::info::format_maintainer;

#[test]
fn malformed_packager_strings_do_not_panic() {
    for raw in ["x> y <z", "<broken", "plain", "Name <mail@example.com>"] {
        let _ = format_maintainer(raw);
    }
}

#[test]
fn keeps_the_email_colored_outside_the_name() {
    let formatted = format_maintainer("Name <mail@example.com>");
    assert!(formatted.contains("Name"));
    assert!(formatted.contains("mail@example.com"));
}
