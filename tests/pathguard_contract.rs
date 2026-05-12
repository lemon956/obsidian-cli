use obsidian_cli::pathguard::{assert_write_allowed, normalize_vault_path};

#[test]
fn normalizes_safe_relative_paths() {
    let path = normalize_vault_path("Notes//Hermes Gateway.md").unwrap();
    assert_eq!(path.as_str(), "Notes/Hermes Gateway.md");

    let root = normalize_vault_path("").unwrap();
    assert_eq!(root.as_str(), "");
}

#[test]
fn rejects_absolute_traversal_and_encoded_traversal() {
    for raw in [
        "/Notes/Hermes.md",
        "../Notes/Hermes.md",
        "Inbox/%2e%2e/Notes/Hermes.md",
        "Inbox/%2E%2E/Notes/Hermes.md",
        "https://example.com/webdav/Notes/Hermes.md",
    ] {
        assert!(
            normalize_vault_path(raw).is_err(),
            "path should be rejected: {raw}"
        );
    }
}

#[test]
fn rejects_writes_to_protected_or_unlisted_directories() {
    let allowed = ["Inbox/Hermes".to_string()];

    for raw in [
        ".obsidian/app.json",
        "Attachments/image.png",
        "Notes/test.md",
        "Inbox/Hermes2/test.md",
    ] {
        assert!(
            assert_write_allowed(raw, &allowed).is_err(),
            "write should be rejected: {raw}"
        );
    }
}

#[test]
fn allows_writes_inside_configured_write_directories() {
    let allowed = ["Inbox/Hermes".to_string()];

    assert_eq!(
        assert_write_allowed("Inbox/Hermes/test.md", &allowed)
            .unwrap()
            .as_str(),
        "Inbox/Hermes/test.md"
    );
    assert_eq!(
        assert_write_allowed("Inbox/Hermes/subdir", &allowed)
            .unwrap()
            .as_str(),
        "Inbox/Hermes/subdir"
    );
}
