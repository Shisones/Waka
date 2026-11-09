use std::fs;

fn read(path: &str) -> String {
    fs::read_to_string(format!("{}/{}", env!("CARGO_MANIFEST_DIR"), path)).unwrap()
}

#[test]
fn pkgbuild_disables_the_debug_package() {
    assert!(read("PKGBUILD").contains("options=('!debug')"));
}

#[test]
fn all_versions_match() {
    let expected = "0.1.7";
    assert!(read("Cargo.toml").contains(&format!("version = \"{expected}\"")));
    assert!(read("PKGBUILD").contains(&format!("pkgver={expected}")));
    assert!(read("dist/PKGBUILD").contains(&format!("pkgver={expected}")));
    assert!(read(".SRCINFO").contains(&format!("pkgver = {expected}")));
    assert!(read("dist/.SRCINFO").contains(&format!("pkgver = {expected}")));
}
