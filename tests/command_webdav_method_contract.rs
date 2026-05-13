use clap::Parser;
use obsidian_cli::{cli::Cli, commands::run_command, config::InitConfig};
use serde_json::Value;
use tempfile::tempdir;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn write_config(
    server: &MockServer,
    password_env: &str,
    configure: impl FnOnce(&mut obsidian_cli::config::AppConfig),
) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.yaml");
    let mut config = obsidian_cli::config::AppConfig::from_init(InitConfig {
        url: format!("{}/webdav/", server.uri()),
        username: "hermes".to_string(),
        password_env: password_env.to_string(),
        write_dir: "Inbox/Hermes".to_string(),
    });
    configure(&mut config);
    std::fs::write(&config_path, serde_yaml::to_string(&config).unwrap()).unwrap();
    unsafe {
        std::env::set_var(password_env, "secret");
    }
    (dir, config_path)
}

#[tokio::test]
async fn delete_and_move_require_config_flags() {
    let server = MockServer::start().await;
    let (_dir, config_path) = write_config(&server, "OBSIDIAN_TEST_PASSWORD_METHOD_FLAGS", |_| {});

    let cli = Cli::parse_from([
        "webdav-cli",
        "--config",
        config_path.to_str().unwrap(),
        "delete",
        "Inbox/Hermes/old.md",
    ]);
    let error = run_command(cli, "").await.unwrap_err();
    assert_eq!(
        error.to_string(),
        "delete requires behavior.allow_delete = true"
    );

    let cli = Cli::parse_from([
        "webdav-cli",
        "--config",
        config_path.to_str().unwrap(),
        "move",
        "Inbox/Hermes/old.md",
        "Inbox/Hermes/new.md",
    ]);
    let error = run_command(cli, "").await.unwrap_err();
    assert_eq!(
        error.to_string(),
        "move requires behavior.allow_move = true"
    );
}

#[tokio::test]
async fn delete_move_and_copy_operate_inside_allowed_directory() {
    let server = MockServer::start().await;
    let (_dir, config_path) =
        write_config(&server, "OBSIDIAN_TEST_PASSWORD_METHOD_BASIC", |config| {
            config.behavior.allow_delete = true;
            config.behavior.allow_move = true;
        });
    let destination = format!("{}/webdav/Inbox/Hermes/new.md", server.uri());

    Mock::given(method("DELETE"))
        .and(path("/webdav/Inbox/Hermes/old.md"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;
    Mock::given(method("MOVE"))
        .and(path("/webdav/Inbox/Hermes/old.md"))
        .and(header("Destination", destination.as_str()))
        .and(header("Overwrite", "T"))
        .respond_with(ResponseTemplate::new(201))
        .mount(&server)
        .await;
    Mock::given(method("COPY"))
        .and(path("/webdav/Inbox/Hermes/old.md"))
        .and(header("Destination", destination.as_str()))
        .and(header("Overwrite", "F"))
        .and(header("Depth", "infinity"))
        .respond_with(ResponseTemplate::new(201))
        .mount(&server)
        .await;

    let delete_cli = Cli::parse_from([
        "webdav-cli",
        "--config",
        config_path.to_str().unwrap(),
        "delete",
        "Inbox/Hermes/old.md",
    ]);
    let delete_output = run_command(delete_cli, "").await.unwrap();
    assert_eq!(delete_output.message, "Deleted: Inbox/Hermes/old.md");

    let move_cli = Cli::parse_from([
        "webdav-cli",
        "--config",
        config_path.to_str().unwrap(),
        "move",
        "Inbox/Hermes/old.md",
        "Inbox/Hermes/new.md",
        "--overwrite",
    ]);
    let move_output = run_command(move_cli, "").await.unwrap();
    assert_eq!(
        move_output.message,
        "Moved: Inbox/Hermes/old.md -> Inbox/Hermes/new.md"
    );

    let copy_cli = Cli::parse_from([
        "webdav-cli",
        "--config",
        config_path.to_str().unwrap(),
        "copy",
        "Inbox/Hermes/old.md",
        "Inbox/Hermes/new.md",
    ]);
    let copy_output = run_command(copy_cli, "").await.unwrap();
    assert_eq!(
        copy_output.message,
        "Copied: Inbox/Hermes/old.md -> Inbox/Hermes/new.md"
    );
}

#[tokio::test]
async fn proppatch_lock_and_unlock_return_structured_json() {
    let server = MockServer::start().await;
    let (_dir, config_path) =
        write_config(&server, "OBSIDIAN_TEST_PASSWORD_METHOD_ADVANCED", |_| {});

    Mock::given(method("PROPPATCH"))
        .and(path("/webdav/Inbox/Hermes/meta.md"))
        .respond_with(ResponseTemplate::new(207))
        .mount(&server)
        .await;
    Mock::given(method("LOCK"))
        .and(path("/webdav/Inbox/Hermes/meta.md"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Lock-Token", "<opaquelocktoken:123>")
                .set_body_string("<prop>locked</prop>"),
        )
        .mount(&server)
        .await;
    Mock::given(method("UNLOCK"))
        .and(path("/webdav/Inbox/Hermes/meta.md"))
        .and(header("Lock-Token", "<opaquelocktoken:123>"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    let proppatch_cli = Cli::parse_from([
        "webdav-cli",
        "--config",
        config_path.to_str().unwrap(),
        "proppatch",
        "Inbox/Hermes/meta.md",
        "--xml",
        "<propertyupdate />",
        "--json",
    ]);
    let proppatch_output = run_command(proppatch_cli, "").await.unwrap();
    let proppatch_json: Value = serde_json::from_str(&proppatch_output.message).unwrap();
    assert_eq!(proppatch_json["ok"], true);
    assert_eq!(proppatch_json["action"], "proppatched");
    assert_eq!(proppatch_json["path"], "Inbox/Hermes/meta.md");

    let lock_cli = Cli::parse_from([
        "webdav-cli",
        "--config",
        config_path.to_str().unwrap(),
        "lock",
        "Inbox/Hermes/meta.md",
        "--owner",
        "hermes",
        "--json",
    ]);
    let lock_output = run_command(lock_cli, "").await.unwrap();
    let lock_json: Value = serde_json::from_str(&lock_output.message).unwrap();
    assert_eq!(lock_json["ok"], true);
    assert_eq!(lock_json["action"], "locked");
    assert_eq!(lock_json["path"], "Inbox/Hermes/meta.md");
    assert_eq!(lock_json["lock_token"], "<opaquelocktoken:123>");
    assert_eq!(lock_json["body"], "<prop>locked</prop>");

    let unlock_cli = Cli::parse_from([
        "webdav-cli",
        "--config",
        config_path.to_str().unwrap(),
        "unlock",
        "Inbox/Hermes/meta.md",
        "--token",
        "opaquelocktoken:123",
        "--json",
    ]);
    let unlock_output = run_command(unlock_cli, "").await.unwrap();
    let unlock_json: Value = serde_json::from_str(&unlock_output.message).unwrap();
    assert_eq!(unlock_json["ok"], true);
    assert_eq!(unlock_json["action"], "unlocked");
    assert_eq!(unlock_json["path"], "Inbox/Hermes/meta.md");
}

#[tokio::test]
async fn method_commands_reject_formal_directory_paths() {
    let server = MockServer::start().await;
    let (_dir, config_path) =
        write_config(&server, "OBSIDIAN_TEST_PASSWORD_METHOD_FORMAL", |config| {
            config.behavior.allow_delete = true;
        });

    let cli = Cli::parse_from([
        "webdav-cli",
        "--config",
        config_path.to_str().unwrap(),
        "delete",
        "Notes/formal.md",
    ]);
    let error = run_command(cli, "").await.unwrap_err();

    assert_eq!(
        error.to_string(),
        "protected path is not writable: Notes/formal.md"
    );
}
