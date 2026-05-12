use clap::Parser;
use obsidian_cli::{cli::Cli, commands::run_command, config::InitConfig};
use tempfile::tempdir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn write_config(server: &MockServer, password_env: &str) -> tempfile::TempDir {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.yaml");
    let config = obsidian_cli::config::AppConfig::from_init(InitConfig {
        url: format!("{}/webdav/", server.uri()),
        username: "hermes".to_string(),
        password_env: password_env.to_string(),
        write_dir: "Inbox/Hermes".to_string(),
    });
    std::fs::write(&config_path, serde_yaml::to_string(&config).unwrap()).unwrap();
    unsafe {
        std::env::set_var(password_env, "secret");
    }
    dir
}

#[tokio::test]
async fn ls_command_lists_child_names() {
    let server = MockServer::start().await;
    let config_dir = write_config(&server, "OBSIDIAN_TEST_PASSWORD_LS").await;
    let config_path = config_dir.path().join("config.yaml");
    let xml = r#"<d:multistatus xmlns:d="DAV:">
  <d:response><d:href>/webdav/Notes/</d:href></d:response>
  <d:response><d:href>/webdav/Notes/Hermes.md</d:href></d:response>
  <d:response><d:href>/webdav/Notes/Subdir/</d:href></d:response>
</d:multistatus>"#;
    Mock::given(method("PROPFIND"))
        .and(path("/webdav/Notes/"))
        .respond_with(ResponseTemplate::new(207).set_body_string(xml))
        .mount(&server)
        .await;

    let cli = Cli::parse_from([
        "webdav-cli",
        "--config",
        config_path.to_str().unwrap(),
        "ls",
        "Notes",
    ]);
    let output = run_command(cli, "").await.unwrap();

    assert_eq!(output.message, "Hermes.md\nSubdir/");
}

#[tokio::test]
async fn cat_command_can_print_only_body() {
    let server = MockServer::start().await;
    let config_dir = write_config(&server, "OBSIDIAN_TEST_PASSWORD_CAT").await;
    let config_path = config_dir.path().join("config.yaml");
    Mock::given(method("GET"))
        .and(path("/webdav/Notes/Hermes.md"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("---\ntitle: Hermes\n---\n\n# Hermes\n\nBody text\n"),
        )
        .mount(&server)
        .await;

    let cli = Cli::parse_from([
        "webdav-cli",
        "--config",
        config_path.to_str().unwrap(),
        "cat",
        "Notes/Hermes.md",
        "--body",
    ]);
    let output = run_command(cli, "").await.unwrap();

    assert_eq!(output.message, "# Hermes\n\nBody text\n");
}

#[tokio::test]
async fn search_command_recursively_searches_markdown_notes() {
    let server = MockServer::start().await;
    let config_dir = write_config(&server, "OBSIDIAN_TEST_PASSWORD_SEARCH").await;
    let config_path = config_dir.path().join("config.yaml");
    Mock::given(method("PROPFIND"))
        .and(path("/webdav/Notes/"))
        .respond_with(ResponseTemplate::new(207).set_body_string(
            r#"<d:multistatus xmlns:d="DAV:">
  <d:response><d:href>/webdav/Notes/</d:href></d:response>
  <d:response><d:href>/webdav/Notes/Hermes.md</d:href></d:response>
  <d:response><d:href>/webdav/Notes/image.png</d:href></d:response>
</d:multistatus>"#,
        ))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/webdav/Notes/Hermes.md"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("# Hermes\nHermes gateway connects Telegram.\n"),
        )
        .mount(&server)
        .await;

    let cli = Cli::parse_from([
        "webdav-cli",
        "--config",
        config_path.to_str().unwrap(),
        "search",
        "gateway",
        "--dir",
        "Notes",
    ]);
    let output = run_command(cli, "").await.unwrap();

    assert_eq!(
        output.message,
        "Notes/Hermes.md:2: Hermes gateway connects Telegram."
    );
}
