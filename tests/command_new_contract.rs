use clap::Parser;
use obsidian_cli::{cli::Cli, commands::run_command, config::InitConfig};
use predicates::str::contains;
use tempfile::tempdir;
use wiremock::matchers::{body_string_contains, method, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn new_command_writes_generated_markdown_to_default_inbox() {
    let server = MockServer::start().await;
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.yaml");
    let config = obsidian_cli::config::AppConfig::from_init(InitConfig {
        url: format!("{}/webdav/", server.uri()),
        username: "hermes".to_string(),
        password_env: "OBSIDIAN_TEST_PASSWORD_NEW".to_string(),
        write_dir: "Inbox/Hermes".to_string(),
    });
    std::fs::write(&config_path, serde_yaml::to_string(&config).unwrap()).unwrap();
    unsafe {
        std::env::set_var("OBSIDIAN_TEST_PASSWORD_NEW", "secret");
    }

    let note_path =
        r"^/webdav/Inbox/Hermes/[0-9]{4}-[0-9]{2}-[0-9]{2}-[0-9]{6}-hermes-gateway-debug\.md$";
    Mock::given(method("HEAD"))
        .and(path_regex(note_path))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path_regex(note_path))
        .and(body_string_contains("title: \"Hermes Gateway Debug\""))
        .and(body_string_contains("source: \"telegram\""))
        .and(body_string_contains("## 问题\n\nBody text"))
        .respond_with(ResponseTemplate::new(201))
        .mount(&server)
        .await;

    let cli = Cli::parse_from([
        "webdav-cli",
        "--config",
        config_path.to_str().unwrap(),
        "new",
        "--title",
        "Hermes Gateway Debug",
        "--body",
        "Body text",
        "--source",
        "telegram",
        "--tag",
        "debug",
        "--template",
        "troubleshooting",
    ]);
    let output = run_command(cli, "").await.unwrap();

    assert!(output.message.starts_with("Created: Inbox/Hermes/"));
    assert!(output.message.ends_with("-hermes-gateway-debug.md"));
}

#[tokio::test]
async fn new_command_refuses_existing_file_without_overwrite() {
    let server = MockServer::start().await;
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.yaml");
    let mut config = obsidian_cli::config::AppConfig::from_init(InitConfig {
        url: format!("{}/webdav/", server.uri()),
        username: "hermes".to_string(),
        password_env: "OBSIDIAN_TEST_PASSWORD_NEW_EXISTS".to_string(),
        write_dir: "Inbox/Hermes".to_string(),
    });
    config.vault.filename_time_format = "fixed".to_string();
    std::fs::write(&config_path, serde_yaml::to_string(&config).unwrap()).unwrap();
    unsafe {
        std::env::set_var("OBSIDIAN_TEST_PASSWORD_NEW_EXISTS", "secret");
    }

    Mock::given(method("HEAD"))
        .and(path_regex(r"^/webdav/Inbox/Hermes/fixed-test\.md$"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let cli = Cli::parse_from([
        "webdav-cli",
        "--config",
        config_path.to_str().unwrap(),
        "new",
        "--title",
        "Test",
        "--body",
        "hello",
    ]);
    let err = run_command(cli, "").await.unwrap_err();

    assert!(err.to_string().contains("file already exists"));
}

#[tokio::test]
async fn new_unique_writes_numbered_path_when_base_file_exists() {
    let server = MockServer::start().await;
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.yaml");
    let mut config = obsidian_cli::config::AppConfig::from_init(InitConfig {
        url: format!("{}/webdav/", server.uri()),
        username: "hermes".to_string(),
        password_env: "OBSIDIAN_TEST_PASSWORD_UNIQUE".to_string(),
        write_dir: "Inbox/Hermes".to_string(),
    });
    config.vault.filename_time_format = "fixed".to_string();
    std::fs::write(&config_path, serde_yaml::to_string(&config).unwrap()).unwrap();
    unsafe {
        std::env::set_var("OBSIDIAN_TEST_PASSWORD_UNIQUE", "secret");
    }

    Mock::given(method("HEAD"))
        .and(path_regex(r"^/webdav/Inbox/Hermes/fixed-test\.md$"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;
    Mock::given(method("HEAD"))
        .and(path_regex(r"^/webdav/Inbox/Hermes/fixed-test-1\.md$"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path_regex(r"^/webdav/Inbox/Hermes/fixed-test-1\.md$"))
        .respond_with(ResponseTemplate::new(201))
        .mount(&server)
        .await;

    let cli = Cli::parse_from([
        "webdav-cli",
        "--config",
        config_path.to_str().unwrap(),
        "new",
        "--title",
        "Test",
        "--body",
        "hello",
        "--unique",
    ]);
    let output = run_command(cli, "").await.unwrap();

    assert_eq!(output.message, "Created: Inbox/Hermes/fixed-test-1.md");
}

#[tokio::test]
async fn new_overwrite_succeeds_only_when_config_allows_it() {
    let server = MockServer::start().await;
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.yaml");
    let mut config = obsidian_cli::config::AppConfig::from_init(InitConfig {
        url: format!("{}/webdav/", server.uri()),
        username: "hermes".to_string(),
        password_env: "OBSIDIAN_TEST_PASSWORD_OVERWRITE".to_string(),
        write_dir: "Inbox/Hermes".to_string(),
    });
    config.vault.filename_time_format = "fixed".to_string();
    config.behavior.allow_overwrite = true;
    std::fs::write(&config_path, serde_yaml::to_string(&config).unwrap()).unwrap();
    unsafe {
        std::env::set_var("OBSIDIAN_TEST_PASSWORD_OVERWRITE", "secret");
    }

    Mock::given(method("HEAD"))
        .and(path_regex(r"^/webdav/Inbox/Hermes/fixed-test\.md$"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path_regex(r"^/webdav/Inbox/Hermes/fixed-test\.md$"))
        .and(body_string_contains("replacement body"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    let cli = Cli::parse_from([
        "webdav-cli",
        "--config",
        config_path.to_str().unwrap(),
        "new",
        "--title",
        "Test",
        "--body",
        "replacement body",
        "--overwrite",
    ]);
    let output = run_command(cli, "").await.unwrap();

    assert_eq!(output.message, "Created: Inbox/Hermes/fixed-test.md");
}

#[tokio::test]
async fn binary_new_command_reads_body_from_stdin_pipe() {
    let server = MockServer::start().await;
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.yaml");
    let config = obsidian_cli::config::AppConfig::from_init(InitConfig {
        url: format!("{}/webdav/", server.uri()),
        username: "hermes".to_string(),
        password_env: "OBSIDIAN_TEST_PASSWORD_BINARY_STDIN".to_string(),
        write_dir: "Inbox/Hermes".to_string(),
    });
    std::fs::write(&config_path, serde_yaml::to_string(&config).unwrap()).unwrap();

    let note_path = r"^/webdav/Inbox/Hermes/[0-9]{4}-[0-9]{2}-[0-9]{2}-[0-9]{6}-piped-note\.md$";
    Mock::given(method("HEAD"))
        .and(path_regex(note_path))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path_regex(note_path))
        .and(body_string_contains("Piped body from shell"))
        .respond_with(ResponseTemplate::new(201))
        .mount(&server)
        .await;

    let mut cmd = assert_cmd::Command::cargo_bin("webdav-cli").unwrap();
    cmd.arg("--config")
        .arg(&config_path)
        .arg("new")
        .arg("--title")
        .arg("Piped Note")
        .env("OBSIDIAN_TEST_PASSWORD_BINARY_STDIN", "secret")
        .write_stdin("Piped body from shell\n")
        .assert()
        .success()
        .stdout(contains("Created: Inbox/Hermes/"));
}
