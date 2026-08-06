use std::path::PathBuf;

/// Root filesystem prefix (empty for live system, can be set for chroot).
pub(crate) const ROOT: &str = "";

/// Path to pacman configuration file.
pub fn pacman_conf() -> PathBuf {
    PathBuf::from(format!("{ROOT}/etc/pacman.conf"))
}

/// Path to pacman log file.
pub fn pacman_log() -> PathBuf {
    PathBuf::from(format!("{ROOT}/var/log/pacman.log"))
}

/// Path to the package cache directory.
pub fn cache_dir() -> PathBuf {
    PathBuf::from(format!("{ROOT}/var/cache/pacman/pkg"))
}

/// Path to the pacman database directory.
pub fn db_path() -> PathBuf {
    PathBuf::from(format!("{ROOT}/var/lib/pacman"))
}

/// Path to the pacman database lock file.
pub fn lock_file() -> PathBuf {
    PathBuf::from(format!("{ROOT}/var/lib/pacman/db.lck"))
}

/// Path to the GNUPG directory used for package signature verification.
pub fn gpg_dir() -> PathBuf {
    PathBuf::from(format!("{ROOT}/etc/pacman.d/gnupg"))
}