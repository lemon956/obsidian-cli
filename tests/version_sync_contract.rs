use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn sync_version_script_updates_project_version_from_tag() {
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let temp = tempdir().unwrap();

    fs::write(
        temp.path().join("Cargo.toml"),
        r#"[package]
name = "obsidian-cli"
version = "0.1.3"
edition = "2024"

[dependencies]
serde = "1.0"
"#,
    )
    .unwrap();
    fs::write(
        temp.path().join("Cargo.lock"),
        r#"version = 4

[[package]]
name = "other"
version = "0.1.3"

[[package]]
name = "obsidian-cli"
version = "0.1.3"
dependencies = []
"#,
    )
    .unwrap();
    fs::write(
        temp.path().join("README.md"),
        "tag example: git tag v0.1.3 && git push origin v0.1.3\n",
    )
    .unwrap();

    let status = Command::new("bash")
        .arg(repo.join("scripts/sync-version-from-tag.sh"))
        .arg("v2.4.6")
        .current_dir(temp.path())
        .status()
        .unwrap();
    assert!(status.success());

    let cargo_toml = fs::read_to_string(temp.path().join("Cargo.toml")).unwrap();
    assert!(cargo_toml.contains(r#"version = "2.4.6""#));
    assert!(cargo_toml.contains(r#"serde = "1.0""#));

    let cargo_lock = fs::read_to_string(temp.path().join("Cargo.lock")).unwrap();
    assert!(cargo_lock.contains(
        r#"name = "other"
version = "0.1.3""#
    ));
    assert!(cargo_lock.contains(
        r#"name = "obsidian-cli"
version = "2.4.6""#
    ));

    let readme = fs::read_to_string(temp.path().join("README.md")).unwrap();
    assert!(readme.contains("git tag v2.4.6 && git push origin v2.4.6"));
}
