use clap::Parser;
use obsidian_cli::config::{
    AppConfig, InitConfig, config_path_for_init, config_path_from_cli, load_config,
    write_initial_config,
};
use obsidian_cli::{cli::Cli, commands::run_command};
use tempfile::tempdir;

#[test]
fn builds_default_config_from_init_args() {
    let init = InitConfig {
        url: "https://example.com/webdav/".to_string(),
        username: "hermes".to_string(),
        password_env: "OBSIDIAN_WEBDAV_PASSWORD".to_string(),
        write_dir: "Inbox/Hermes".to_string(),
    };

    let config = AppConfig::from_init(init);

    assert_eq!(config.webdav.base_url, "https://example.com/webdav/");
    assert_eq!(config.webdav.username, "hermes");
    assert_eq!(config.webdav.timeout, 30);
    assert_eq!(config.vault.default_write_dir, "Inbox/Hermes");
    assert_eq!(config.vault.default_tags, vec!["hermes", "inbox"]);
    assert_eq!(config.behavior.allow_write_dirs, vec!["Inbox/Hermes"]);
    assert!(!config.behavior.allow_overwrite);
    assert!(!config.behavior.allow_delete);
    assert!(!config.behavior.allow_move);
    assert!(config.markdown.frontmatter);
    assert_eq!(config.markdown.default_source, "hermes");
}

#[test]
fn writes_and_loads_initial_config_without_plaintext_password() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.yaml");
    let init = InitConfig {
        url: "https://example.com/webdav".to_string(),
        username: "hermes".to_string(),
        password_env: "OBSIDIAN_WEBDAV_PASSWORD".to_string(),
        write_dir: "Inbox/Hermes".to_string(),
    };

    write_initial_config(&config_path, init, false).unwrap();
    let raw = std::fs::read_to_string(&config_path).unwrap();

    assert!(raw.contains("base_url: https://example.com/webdav/"));
    assert!(raw.contains("password_env: OBSIDIAN_WEBDAV_PASSWORD"));
    assert!(!raw.contains("your-password"));

    let loaded = load_config(&config_path).unwrap();
    assert_eq!(loaded.webdav.base_url, "https://example.com/webdav/");
    assert_eq!(loaded.behavior.readonly_dirs[0], "Notes");
}

#[test]
fn refuses_to_overwrite_config_without_force() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.yaml");
    std::fs::write(&config_path, "existing: true\n").unwrap();

    let init = InitConfig {
        url: "https://example.com/webdav/".to_string(),
        username: "hermes".to_string(),
        password_env: "OBSIDIAN_WEBDAV_PASSWORD".to_string(),
        write_dir: "Inbox/Hermes".to_string(),
    };

    let err = write_initial_config(&config_path, init, false).unwrap_err();
    assert!(err.to_string().contains("already exists"));
    assert_eq!(
        std::fs::read_to_string(&config_path).unwrap(),
        "existing: true\n"
    );
}

#[test]
fn resolves_default_config_path_when_cli_path_is_absent() {
    let home = tempdir().unwrap();
    let path = config_path_from_cli(None, Some(home.path())).unwrap();

    assert_eq!(
        path,
        home.path()
            .join(".config")
            .join("webdav-cli")
            .join("config.yaml")
    );
}

#[test]
fn falls_back_to_legacy_obsidian_cli_config_path() {
    let home = tempdir().unwrap();
    let legacy_path = home
        .path()
        .join(".config")
        .join("obsidian-cli")
        .join("config.yaml");
    std::fs::create_dir_all(legacy_path.parent().unwrap()).unwrap();
    std::fs::write(&legacy_path, "legacy: true\n").unwrap();

    let path = config_path_from_cli(None, Some(home.path())).unwrap();

    assert_eq!(path, legacy_path);
}

#[test]
fn init_default_path_uses_webdav_cli_even_when_legacy_exists() {
    let home = tempdir().unwrap();
    let legacy_path = home
        .path()
        .join(".config")
        .join("obsidian-cli")
        .join("config.yaml");
    std::fs::create_dir_all(legacy_path.parent().unwrap()).unwrap();
    std::fs::write(&legacy_path, "legacy: true\n").unwrap();

    let path = config_path_for_init(None, Some(home.path())).unwrap();

    assert_eq!(
        path,
        home.path()
            .join(".config")
            .join("webdav-cli")
            .join("config.yaml")
    );
}

#[tokio::test]
async fn init_command_writes_config_to_cli_path() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("custom.yaml");
    let cli = Cli::parse_from([
        "webdav-cli",
        "--config",
        config_path.to_str().unwrap(),
        "init",
        "--url",
        "https://example.com/webdav",
        "--username",
        "hermes",
        "--password-env",
        "OBSIDIAN_WEBDAV_PASSWORD",
        "--write-dir",
        "Inbox/Hermes",
    ]);

    let output = run_command(cli, "").await.unwrap();

    assert_eq!(
        output.message,
        format!("Config written: {}", config_path.display())
    );
    assert_eq!(load_config(&config_path).unwrap().webdav.username, "hermes");
}

#[tokio::test]
async fn init_command_reads_missing_values_from_stdin() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("interactive.yaml");
    let cli = Cli::parse_from([
        "webdav-cli",
        "--config",
        config_path.to_str().unwrap(),
        "init",
    ]);

    let output = run_command(
        cli,
        "https://example.com/webdav/\nhermes\nOBSIDIAN_WEBDAV_PASSWORD\nInbox/Hermes\n",
    )
    .await
    .unwrap();

    assert_eq!(
        output.message,
        format!("Config written: {}", config_path.display())
    );
    let loaded = load_config(&config_path).unwrap();
    assert_eq!(loaded.webdav.base_url, "https://example.com/webdav/");
    assert_eq!(loaded.webdav.username, "hermes");
    assert_eq!(loaded.webdav.password_env, "OBSIDIAN_WEBDAV_PASSWORD");
    assert_eq!(loaded.vault.default_write_dir, "Inbox/Hermes");
}
