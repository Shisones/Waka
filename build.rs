use std::process::Command;

fn main() {
    for var in ["WAKA_GIT_DISTANCE", "WAKA_GIT_HASH", "WAKA_DIRTY"] {
        println!("cargo:rerun-if-env-changed={var}");
    }
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/index");
    println!("cargo:rerun-if-changed=.git/refs/tags");
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=Cargo.toml");

    let overridden = std::env::var("WAKA_GIT_HASH").is_ok();
    let described = Command::new("git")
        .args(["describe", "--tags", "--long", "--always"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty());

    let (mut distance, mut hash) = match described.as_deref() {
        Some(s) => split_describe(s),
        None => (None, None),
    };
    if let Ok(d) = std::env::var("WAKA_GIT_DISTANCE") {
        distance = Some(d);
    }
    if let Ok(h) = std::env::var("WAKA_GIT_HASH") {
        hash = Some(h);
    }

    if let (Some(distance), Some(hash)) = (distance, hash) {
        if distance.chars().all(|c| c.is_ascii_digit()) && hash.chars().all(|c| c.is_ascii_hexdigit()) {
            println!("cargo:rustc-env=WAKA_GIT_DISTANCE={distance}");
            println!("cargo:rustc-env=WAKA_GIT_HASH={hash}");
            if !overridden && dirty() {
                println!("cargo:rustc-env=WAKA_DIRTY=1");
            }
        }
    }
}

/// `v0.1.3-4-gabc1234` -> ("4", "abc1234"). A bare hash has no distance.
fn split_describe(s: &str) -> (Option<String>, Option<String>) {
    let mut parts = s.rsplitn(3, '-');
    let last = parts.next().unwrap_or("");
    match last.strip_prefix('g') {
        Some(hash) => (parts.next().map(|n| n.to_string()), Some(hash.to_string())),
        None => (None, Some(last.to_string())),
    }
}

fn dirty() -> bool {
    Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false)
}
