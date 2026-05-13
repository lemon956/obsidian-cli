use clap::Parser;
use obsidian_cli::{cli::Cli, commands::run_command, config::InitConfig};
use serde_json::Value;
use tempfile::tempdir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn write_config(
    server: &MockServer,
    password_env: &str,
) -> (tempfile::TempDir, std::path::PathBuf) {
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
    (dir, config_path)
}

#[tokio::test]
async fn mkdir_command_creates_allowed_directory() {
    let server = MockServer::start().await;
    let (_dir, config_path) = write_config(&server, "OBSIDIAN_TEST_PASSWORD_MKDIR");
    Mock::given(method("MKCOL"))
        .and(path("/webdav/Inbox/Hermes/subdir/"))
        .respond_with(ResponseTemplate::new(201))
        .mount(&server)
        .await;

    let cli = Cli::parse_from([
        "webdav-cli",
        "--config",
        config_path.to_str().unwrap(),
        "mkdir",
        "Inbox/Hermes/subdir",
    ]);
    let output = run_command(cli, "").await.unwrap();

    assert_eq!(output.message, "Created directory: Inbox/Hermes/subdir");
}

#[tokio::test]
async fn mkdir_parents_creates_only_inside_allowed_write_directory() {
    let server = MockServer::start().await;
    let (_dir, config_path) = write_config(&server, "OBSIDIAN_TEST_PASSWORD_MKDIR_P");
    Mock::given(method("MKCOL"))
        .and(path("/webdav/Inbox/Hermes/debug/"))
        .respond_with(ResponseTemplate::new(201))
        .mount(&server)
        .await;
    Mock::given(method("MKCOL"))
        .and(path("/webdav/Inbox/Hermes/debug/deep/"))
        .respond_with(ResponseTemplate::new(201))
        .mount(&server)
        .await;

    let cli = Cli::parse_from([
        "webdav-cli",
        "--config",
        config_path.to_str().unwrap(),
        "mkdir",
        "-p",
        "Inbox/Hermes/debug/deep",
    ]);
    let output = run_command(cli, "").await.unwrap();

    assert_eq!(output.message, "Created directory: Inbox/Hermes/debug/deep");
}

#[tokio::test]
async fn doctor_reports_config_and_read_checks_without_write_probe() {
    let server = MockServer::start().await;
    let (_dir, config_path) = write_config(&server, "OBSIDIAN_TEST_PASSWORD_DOCTOR");
    Mock::given(method("OPTIONS"))
        .and(path("/webdav/"))
        .respond_with(
            ResponseTemplate::new(204).insert_header("Allow", "GET, HEAD, OPTIONS, PROPFIND"),
        )
        .mount(&server)
        .await;
    Mock::given(method("PROPFIND"))
        .and(path("/webdav/"))
        .respond_with(ResponseTemplate::new(207).set_body_string(
            r#"<d:multistatus xmlns:d="DAV:">
  <d:response><d:href>/webdav/</d:href></d:response>
  <d:response><d:href>/webdav/Inbox/</d:href></d:response>
</d:multistatus>"#,
        ))
        .mount(&server)
        .await;
    Mock::given(method("PROPFIND"))
        .and(path("/webdav/Inbox/Hermes/"))
        .respond_with(ResponseTemplate::new(207).set_body_string(
            r#"<d:multistatus xmlns:d="DAV:">
  <d:response><d:href>/webdav/Inbox/Hermes/</d:href></d:response>
</d:multistatus>"#,
        ))
        .mount(&server)
        .await;
    Mock::given(method("OPTIONS"))
        .and(path("/webdav/Inbox/Hermes/"))
        .respond_with(ResponseTemplate::new(204).insert_header(
            "Allow",
            "GET, HEAD, OPTIONS, PROPFIND, PUT, MKCOL, DELETE, MOVE, COPY, PROPPATCH, LOCK, UNLOCK",
        ))
        .mount(&server)
        .await;
    Mock::given(method("OPTIONS"))
        .and(path("/webdav/Notes/"))
        .respond_with(
            ResponseTemplate::new(204).insert_header("Allow", "GET, HEAD, OPTIONS, PROPFIND"),
        )
        .mount(&server)
        .await;

    let cli = Cli::parse_from([
        "webdav-cli",
        "--config",
        config_path.to_str().unwrap(),
        "doctor",
        "--no-write-test",
    ]);
    let output = run_command(cli, "").await.unwrap();

    assert_eq!(
        output.message,
        "[OK] Config loaded\n[OK] WebDAV connected\n[OK] Root vault readable\n[OK] Inbox/Hermes exists\n[OK] Inbox/Hermes full HTTP permissions\n[OK] Write test skipped\n[OK] Notes directory readonly"
    );
}

#[tokio::test]
async fn doctor_reports_full_write_and_server_safety_checks() {
    let server = MockServer::start().await;
    let (_dir, config_path) = write_config(&server, "OBSIDIAN_TEST_PASSWORD_DOCTOR_FULL");
    Mock::given(method("OPTIONS"))
        .and(path("/webdav/"))
        .respond_with(
            ResponseTemplate::new(204).insert_header("Allow", "GET, HEAD, OPTIONS, PROPFIND"),
        )
        .mount(&server)
        .await;
    Mock::given(method("PROPFIND"))
        .and(path("/webdav/"))
        .respond_with(ResponseTemplate::new(207).set_body_string(
            r#"<d:multistatus xmlns:d="DAV:">
  <d:response><d:href>/webdav/</d:href></d:response>
  <d:response><d:href>/webdav/Inbox/</d:href></d:response>
</d:multistatus>"#,
        ))
        .mount(&server)
        .await;
    Mock::given(method("PROPFIND"))
        .and(path("/webdav/Inbox/Hermes/"))
        .respond_with(ResponseTemplate::new(207).set_body_string(
            r#"<d:multistatus xmlns:d="DAV:">
  <d:response><d:href>/webdav/Inbox/Hermes/</d:href></d:response>
</d:multistatus>"#,
        ))
        .mount(&server)
        .await;
    Mock::given(method("OPTIONS"))
        .and(path("/webdav/Inbox/Hermes/"))
        .respond_with(ResponseTemplate::new(204).insert_header(
            "Allow",
            "GET, HEAD, OPTIONS, PROPFIND, PUT, MKCOL, DELETE, MOVE, COPY, PROPPATCH, LOCK, UNLOCK",
        ))
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(wiremock::matchers::path_regex(
            r"^/webdav/Inbox/Hermes/\.webdav-cli-doctor-write-[0-9]+\.md$",
        ))
        .respond_with(ResponseTemplate::new(201))
        .mount(&server)
        .await;
    Mock::given(method("OPTIONS"))
        .and(path("/webdav/Notes/"))
        .respond_with(
            ResponseTemplate::new(204).insert_header("Allow", "GET, HEAD, OPTIONS, PROPFIND"),
        )
        .mount(&server)
        .await;

    let cli = Cli::parse_from([
        "webdav-cli",
        "--config",
        config_path.to_str().unwrap(),
        "doctor",
    ]);
    let output = run_command(cli, "").await.unwrap();

    assert_eq!(
        output.message,
        "[OK] Config loaded\n[OK] WebDAV connected\n[OK] Root vault readable\n[OK] Inbox/Hermes exists\n[OK] Inbox/Hermes full HTTP permissions\n[OK] Inbox/Hermes writable\n[OK] Notes directory readonly"
    );
}

#[tokio::test]
async fn doctor_rejects_write_directory_missing_full_http_permissions() {
    let server = MockServer::start().await;
    let (_dir, config_path) = write_config(&server, "OBSIDIAN_TEST_PASSWORD_DOCTOR_MISSING_METHOD");
    Mock::given(method("OPTIONS"))
        .and(path("/webdav/"))
        .respond_with(
            ResponseTemplate::new(204).insert_header("Allow", "GET, HEAD, OPTIONS, PROPFIND"),
        )
        .mount(&server)
        .await;
    Mock::given(method("PROPFIND"))
        .and(path("/webdav/"))
        .respond_with(ResponseTemplate::new(207).set_body_string(
            r#"<d:multistatus xmlns:d="DAV:">
  <d:response><d:href>/webdav/</d:href></d:response>
  <d:response><d:href>/webdav/Inbox/</d:href></d:response>
</d:multistatus>"#,
        ))
        .mount(&server)
        .await;
    Mock::given(method("PROPFIND"))
        .and(path("/webdav/Inbox/Hermes/"))
        .respond_with(ResponseTemplate::new(207).set_body_string(
            r#"<d:multistatus xmlns:d="DAV:">
  <d:response><d:href>/webdav/Inbox/Hermes/</d:href></d:response>
</d:multistatus>"#,
        ))
        .mount(&server)
        .await;
    Mock::given(method("OPTIONS"))
        .and(path("/webdav/Inbox/Hermes/"))
        .respond_with(
            ResponseTemplate::new(204)
                .insert_header("Allow", "GET, HEAD, OPTIONS, PROPFIND, PUT, MKCOL"),
        )
        .mount(&server)
        .await;

    let cli = Cli::parse_from([
        "webdav-cli",
        "--config",
        config_path.to_str().unwrap(),
        "doctor",
        "--no-write-test",
    ]);
    let error = run_command(cli, "").await.unwrap_err();

    assert_eq!(
        error.to_string(),
        "Inbox/Hermes lacks full HTTP permissions; missing: DELETE, MOVE, COPY, PROPPATCH, LOCK, UNLOCK"
    );
}

#[tokio::test]
async fn ls_json_outputs_structured_entries() {
    let server = MockServer::start().await;
    let (_dir, config_path) = write_config(&server, "OBSIDIAN_TEST_PASSWORD_LS_JSON");
    Mock::given(method("PROPFIND"))
        .and(path("/webdav/"))
        .respond_with(ResponseTemplate::new(207).set_body_string(
            r#"<d:multistatus xmlns:d="DAV:">
  <d:response><d:href>/webdav/</d:href></d:response>
  <d:response><d:href>/webdav/Notes/</d:href></d:response>
</d:multistatus>"#,
        ))
        .mount(&server)
        .await;

    let cli = Cli::parse_from([
        "webdav-cli",
        "--config",
        config_path.to_str().unwrap(),
        "ls",
        "--json",
    ]);
    let output = run_command(cli, "").await.unwrap();
    let json: Value = serde_json::from_str(&output.message).unwrap();

    assert_eq!(json["ok"], true);
    assert_eq!(json["action"], "listed");
    assert_eq!(json["entries"][0]["path"], "Notes");
    assert_eq!(json["entries"][0]["is_dir"], true);
}

#[test]
fn command_errors_have_stable_json_codes() {
    let json = obsidian_cli::commands::error_json(
        &obsidian_cli::commands::CommandError::FileAlreadyExists("Inbox/Hermes/test.md".into()),
    );

    assert_eq!(json["ok"], false);
    assert_eq!(json["error"], "file_already_exists");
    assert!(
        json["message"]
            .as_str()
            .unwrap()
            .contains("Inbox/Hermes/test.md")
    );
}
