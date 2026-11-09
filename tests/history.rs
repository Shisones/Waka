use waka::commands::history::transaction_lines;

#[test]
fn same_second_operations_are_included() {
    let entries = vec![
        "[2026-11-09T10:37:09+0700] [ALPM] transaction started".to_string(),
        "[2026-11-09T10:37:09+0700] [ALPM] installed foo (1.0-1)".to_string(),
        "[2026-11-09T10:37:09+0700] [ALPM] transaction completed".to_string(),
        "[2026-11-09T10:38:09+0700] [ALPM] transaction started".to_string(),
        "[2026-11-09T10:38:09+0700] [ALPM] installed bar (2.0-1)".to_string(),
    ];

    let lines = transaction_lines(&entries, &entries[0]);
    assert_eq!(lines.len(), 3);
    assert!(lines[0].contains("transaction started"));
    assert!(lines[1].contains("installed foo"));
    assert!(lines[2].contains("transaction completed"));
}
