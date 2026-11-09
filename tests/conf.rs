use alpm::SigLevel;
use waka::alpm::conf::{apply_siglevel, default_siglevel, option_value, parse_siglevel};

#[test]
fn reads_directives_with_any_spacing() {
    assert_eq!(
        option_value("SigLevel    = Required DatabaseOptional", "SigLevel"),
        Some("Required DatabaseOptional")
    );
    assert_eq!(option_value("SigLevel\t=\tNever", "SigLevel"), Some("Never"));
    assert_eq!(option_value("SigLevel=Optional", "SigLevel"), Some("Optional"));
    assert_eq!(option_value("LocalFileSigLevel = Optional", "SigLevel"), None);
}

#[test]
fn required_keeps_package_and_database_checks() {
    let level = parse_siglevel("Required DatabaseOptional");
    assert!(level.contains(SigLevel::PACKAGE));
    assert!(level.contains(SigLevel::DATABASE));
    assert!(level.contains(SigLevel::DATABASE_OPTIONAL));
    assert_eq!(parse_siglevel("Never"), SigLevel::NONE);
}

#[test]
fn trusted_only_is_not_a_disabled_level() {
    let level = parse_siglevel("TrustedOnly");
    assert!(level.contains(SigLevel::PACKAGE));
    assert!(level.contains(SigLevel::DATABASE));
    assert!(!level.contains(SigLevel::PACKAGE_UNKNOWN_OK));
    assert!(!level.contains(SigLevel::DATABASE_UNKNOWN_OK));
}

#[test]
fn pacman_token_names_are_understood() {
    let trusted = parse_siglevel("TrustAll");
    assert!(trusted.contains(SigLevel::PACKAGE_UNKNOWN_OK));
    assert!(trusted.contains(SigLevel::DATABASE_UNKNOWN_OK));

    let package = parse_siglevel("PackageTrustAll");
    assert!(package.contains(SigLevel::PACKAGE_UNKNOWN_OK));
    assert!(!package.contains(SigLevel::DATABASE_UNKNOWN_OK));

    let database = parse_siglevel("DatabaseTrustAll");
    assert!(database.contains(SigLevel::DATABASE_UNKNOWN_OK));
    assert!(!database.contains(SigLevel::PACKAGE_UNKNOWN_OK));
}

#[test]
fn never_does_not_discard_later_tokens() {
    let level = parse_siglevel("Never DatabaseRequired");
    assert!(!level.contains(SigLevel::PACKAGE));
    assert!(level.contains(SigLevel::DATABASE));
}

#[test]
fn package_never_only_clears_packages() {
    let level = parse_siglevel("PackageNever");
    assert!(!level.contains(SigLevel::PACKAGE));
    assert!(level.contains(SigLevel::DATABASE));
}

#[test]
fn database_never_only_clears_databases() {
    let level = parse_siglevel("DatabaseNever");
    assert!(level.contains(SigLevel::PACKAGE));
    assert!(!level.contains(SigLevel::DATABASE));
}

#[test]
fn repo_level_applies_on_top_of_global() {
    let global = apply_siglevel(default_siglevel(), "Required DatabaseOptional");
    let repo = apply_siglevel(global, "PackageRequired");
    assert!(repo.contains(SigLevel::PACKAGE));
    assert!(repo.contains(SigLevel::DATABASE_OPTIONAL));
    let never = apply_siglevel(global, "Never");
    assert!(!never.contains(SigLevel::PACKAGE));
    assert!(!never.contains(SigLevel::DATABASE));
}
