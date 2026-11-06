use alpm::SigLevel;

/// Match `Key = Value` (or `Key=Value`, tabs, extra spaces) like pacman.
pub fn option_value<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let (name, value) = line.split_once('=')?;
    if name.trim() == key {
        Some(value.trim())
    } else {
        None
    }
}

/// Pacman's built-in default: `Required TrustedOnly`.
pub fn default_siglevel() -> SigLevel {
    SigLevel::PACKAGE | SigLevel::DATABASE
}

/// Apply one SigLevel value on top of an existing level.
pub fn apply_siglevel(mut level: SigLevel, val: &str) -> SigLevel {
    let package_trust = SigLevel::PACKAGE_MARGINAL_OK | SigLevel::PACKAGE_UNKNOWN_OK;
    let database_trust = SigLevel::DATABASE_MARGINAL_OK | SigLevel::DATABASE_UNKNOWN_OK;
    for part in val.split_whitespace() {
        match part {
            "Never" => return SigLevel::NONE,
            "Optional" => {
                level |= SigLevel::PACKAGE
                    | SigLevel::PACKAGE_OPTIONAL
                    | SigLevel::DATABASE
                    | SigLevel::DATABASE_OPTIONAL;
            }
            "Required" => {
                level |= SigLevel::PACKAGE | SigLevel::DATABASE;
                level.remove(SigLevel::PACKAGE_OPTIONAL | SigLevel::DATABASE_OPTIONAL);
            }
            "Package" => level |= SigLevel::PACKAGE,
            "PackageOptional" => level |= SigLevel::PACKAGE | SigLevel::PACKAGE_OPTIONAL,
            "PackageRequired" => {
                level |= SigLevel::PACKAGE;
                level.remove(SigLevel::PACKAGE_OPTIONAL);
            }
            "PackageTrustedOnly" => level.remove(package_trust),
            "PackageTrustAll" => level |= SigLevel::PACKAGE | package_trust,
            "Database" => level |= SigLevel::DATABASE,
            "DatabaseOptional" => level |= SigLevel::DATABASE | SigLevel::DATABASE_OPTIONAL,
            "DatabaseRequired" => {
                level |= SigLevel::DATABASE;
                level.remove(SigLevel::DATABASE_OPTIONAL);
            }
            "DatabaseTrustedOnly" => level.remove(database_trust),
            "DatabaseTrustAll" => level |= SigLevel::DATABASE | database_trust,
            "TrustedOnly" => level.remove(package_trust | database_trust),
            "TrustAll" => level |= package_trust | database_trust,
            other => eprintln!(
                "  {} unknown SigLevel token '{other}'",
                crate::display::style::yellow("warn")
            ),
        }
    }
    level
}

/// Parse a standalone SigLevel value, starting from pacman's built-in default.
pub fn parse_siglevel(val: &str) -> SigLevel {
    apply_siglevel(default_siglevel(), val)
}
