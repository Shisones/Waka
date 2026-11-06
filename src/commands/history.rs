use crate::constants;
use crate::display::style;
use anyhow::Result;

pub fn run(id: Option<String>) -> Result<()> {
    let log_path = constants::pacman_log();
    if !log_path.exists() {
        eprintln!(" {} no pacman log found", style::yellow("warn"));
        return Ok(());
    }
    let content = std::fs::read_to_string(&log_path)?;

    let mut entries: Vec<String> = Vec::new();
    for line in content.lines() {
        if line.contains("[ALPM]") {
            entries.push(line.to_string());
        }
    }

    if entries.is_empty() {
        eprintln!(" {} no transaction history found", style::yellow("warn"));
        return Ok(());
    }

    if let Some(tid) = id {
        let txn_entries: Vec<&str> = entries.iter()
            .filter(|l| l.contains("transaction started"))
            .map(|l| l.as_str())
            .collect();
        let idx: usize = match tid.parse() {
            Ok(n) => n,
            Err(_) => {
                return Err(anyhow::anyhow!("'{tid}' is not a valid transaction number"));
            }
        };
        if idx == 0 || idx > txn_entries.len() {
            return Err(anyhow::anyhow!(
                "transaction id out of range (1-{})",
                txn_entries.len()
            ));
        }
        let txn_line = txn_entries[idx - 1];
        let txn_time = txn_line.trim_start_matches('[').split(']').next().unwrap_or("");
        println!(" {} {}", style::bold("Transaction"), idx);
        println!(" {}", style::dim(txn_time));
        for line in &entries {
            if line.contains("[ALPM]") && line.as_str() >= txn_line {
                if line.contains("transaction started") && line != txn_line {
                    break;
                }
                println!(" {}", line.trim_start_matches('[').split_once(']').map(|x| x.1).unwrap_or("").trim());
            }
        }
    } else {
        let mut count = 0;
        let mut prev_date = String::new();
        for line in &entries {
            let date = line.trim_start_matches('[').split('T').next().unwrap_or("");
            if date != prev_date {
                if !prev_date.is_empty() { println!(); }
                prev_date = date.to_string();
            }
            if line.contains("transaction started") {
                count += 1;
                let date_str = line.trim_start_matches('[').split(']').next().unwrap_or("");
                println!(" {}  {} {}", style::bold(&format!("{:>3}", count)), style::dim(date_str), line.split("[ALPM] ").nth(1).unwrap_or(""));
            }
        }
        println!();
        println!(" {} {} in history", style::green("\u{2713}"), count);

        if entries.iter().any(|l| l.contains("transaction started")) {
            println!("  run 'waka history -i <id>' for details");
        }
    }

    Ok(())
}

