pub const PKG_VERSION: &str = env!("CARGO_PKG_VERSION");

/// `0.1.6+5.abc1234`, or plain `0.1.6` when the build has no git metadata.
pub fn full() -> String {
    compose(
        PKG_VERSION,
        option_env!("WAKA_GIT_DISTANCE"),
        option_env!("WAKA_GIT_HASH"),
        option_env!("WAKA_DIRTY") == Some("1"),
    )
}

pub fn compose(pkg: &str, distance: Option<&str>, hash: Option<&str>, dirty: bool) -> String {
    match (distance, hash) {
        (Some(distance), Some(hash))
            if !distance.is_empty()
                && distance.chars().all(|c| c.is_ascii_digit())
                && !hash.is_empty()
                && hash.chars().all(|c| c.is_ascii_hexdigit()) =>
        {
            let dirty = if dirty { ".dirty" } else { "" };
            format!("{pkg}+{distance}.{hash}{dirty}")
        }
        _ => pkg.to_string(),
    }
}
