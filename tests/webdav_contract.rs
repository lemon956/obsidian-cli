use obsidian_cli::webdav::{DavEntry, WebdavClient, WebdavSettings};
use wiremock::matchers::{body_string, header_exists, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn settings(server: &MockServer) -> WebdavSettings {
    WebdavSettings {
        base_url: format!("{}/webdav/", server.uri()),
        username: "hermes".to_string(),
        password: "secret".to_string(),
        timeout_secs: 10,
    }
}

#[tokio::test]
async fn propfind_lists_directory_entries() {
    let server = MockServer::start().await;
    let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<d:multistatus xmlns:d="DAV:">
  <d:response><d:href>/webdav/Notes/</d:href></d:response>
  <d:response><d:href>/webdav/Notes/Hermes.md</d:href></d:response>
  <d:response><d:href>/webdav/Notes/Subdir/</d:href></d:response>
</d:multistatus>"#;

    Mock::given(method("PROPFIND"))
        .and(path("/webdav/Notes/"))
        .and(header_exists("authorization"))
        .respond_with(ResponseTemplate::new(207).set_body_string(xml))
        .mount(&server)
        .await;

    let client = WebdavClient::new(settings(&server)).unwrap();
    let entries = client.list_dir("Notes").await.unwrap();

    assert_eq!(
        entries,
        vec![
            DavEntry {
                path: "Notes/Hermes.md".to_string(),
                is_dir: false,
            },
            DavEntry {
                path: "Notes/Subdir".to_string(),
                is_dir: true,
            },
        ]
    );
}

#[tokio::test]
async fn get_put_mkcol_and_head_use_expected_http_methods() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/webdav/Notes/Hermes.md"))
        .and(header_exists("authorization"))
        .respond_with(ResponseTemplate::new(200).set_body_string("# Hermes\n"))
        .mount(&server)
        .await;
    Mock::given(method("HEAD"))
        .and(path("/webdav/Inbox/Hermes/existing.md"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/webdav/Inbox/Hermes/new.md"))
        .and(body_string("hello"))
        .respond_with(ResponseTemplate::new(201))
        .mount(&server)
        .await;
    Mock::given(method("MKCOL"))
        .and(path("/webdav/Inbox/Hermes/subdir/"))
        .respond_with(ResponseTemplate::new(201))
        .mount(&server)
        .await;

    let client = WebdavClient::new(settings(&server)).unwrap();

    assert_eq!(
        client.get_text("Notes/Hermes.md").await.unwrap(),
        "# Hermes\n"
    );
    assert!(client.exists("Inbox/Hermes/existing.md").await.unwrap());
    client
        .put_text("Inbox/Hermes/new.md", "hello")
        .await
        .unwrap();
    client.mkcol("Inbox/Hermes/subdir").await.unwrap();
}

#[tokio::test]
async fn options_and_delete_probe_return_diagnostic_status() {
    let server = MockServer::start().await;

    Mock::given(method("OPTIONS"))
        .and(path("/webdav/Notes/"))
        .respond_with(ResponseTemplate::new(204).insert_header("Allow", "GET, HEAD, PROPFIND"))
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/webdav/Inbox/Hermes/.webdav-cli-delete-probe.md"))
        .respond_with(ResponseTemplate::new(405))
        .mount(&server)
        .await;

    let client = WebdavClient::new(settings(&server)).unwrap();

    assert_eq!(
        client.options_allow("Notes", true).await.unwrap(),
        vec!["GET", "HEAD", "PROPFIND"]
    );
    assert_eq!(
        client
            .delete_status("Inbox/Hermes/.webdav-cli-delete-probe.md")
            .await
            .unwrap()
            .as_u16(),
        405
    );
}
