use std::process::Command;

fn status(args: &[&str]) -> i32 {
    Command::new(env!("CARGO_BIN_EXE_waka"))
        .args(args)
        .output()
        .unwrap()
        .status
        .code()
        .unwrap_or(-1)
}

#[test]
fn missing_targets_exit_one() {
    assert_eq!(status(&["info", "zzzz-not-a-package"]), 1);
    assert_eq!(status(&["search", "zzzz-not-a-package"]), 1);
    assert_eq!(status(&["list", "zzzz-not-a-package"]), 1);
}

#[test]
fn invalid_history_id_exits_one() {
    let log = std::fs::read_to_string("/var/log/pacman.log").unwrap_or_default();
    if log.contains("transaction started") {
        assert_eq!(status(&["history", "-i", "notanumber"]), 1);
    }
}

#[test]
fn installed_package_does_not_report_zero_download_size() {
    let output = Command::new(env!("CARGO_BIN_EXE_waka"))
        .args(["info", "bash"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("Download Size: 0 B"), "{stdout}");
}

#[test]
fn root_hint_has_no_markdown() {
    let hint = waka::sys::root_hint(&["upgrade".to_string()]);
    assert!(!hint.contains('`'), "{hint}");
    assert!(hint.contains("'sudo waka upgrade'"), "{hint}");
}
