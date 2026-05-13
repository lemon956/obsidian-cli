use crate::pathguard::normalize_vault_path;
use percent_encoding::{AsciiSet, CONTROLS, percent_decode_str, utf8_percent_encode};
use quick_xml::Reader;
use quick_xml::events::Event;
use reqwest::{Method, StatusCode, Url};
use serde::Serialize;
use std::time::Duration;
use thiserror::Error;

const PATH_ENCODE_SET: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'%')
    .add(b'<')
    .add(b'>')
    .add(b'?')
    .add(b'`')
    .add(b'{')
    .add(b'}');

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebdavSettings {
    pub base_url: String,
    pub username: String,
    pub password: String,
    pub timeout_secs: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DavEntry {
    pub path: String,
    pub is_dir: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockResult {
    pub lock_token: Option<String>,
    pub body: String,
}

#[derive(Debug, Error)]
pub enum WebdavError {
    #[error("invalid WebDAV base URL: {0}")]
    InvalidBaseUrl(#[from] url::ParseError),
    #[error("invalid vault path: {0}")]
    InvalidPath(#[from] crate::pathguard::PathError),
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("WebDAV request failed with status {status}: {context}")]
    Status { status: StatusCode, context: String },
    #[error("failed to parse WebDAV XML: {0}")]
    Xml(String),
}

#[derive(Debug, Clone)]
pub struct WebdavClient {
    client: reqwest::Client,
    base_url: Url,
    username: String,
    password: String,
}

impl WebdavClient {
    pub fn new(settings: WebdavSettings) -> Result<Self, WebdavError> {
        let base_url = Url::parse(&settings.base_url)?;
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(settings.timeout_secs))
            .build()?;

        Ok(Self {
            client,
            base_url,
            username: settings.username,
            password: settings.password,
        })
    }

    pub fn url_for(&self, raw_path: &str, as_dir: bool) -> Result<Url, WebdavError> {
        let normalized = normalize_vault_path(raw_path)?;
        let mut url = self.base_url.clone();
        let base_path = self.base_url.path().trim_end_matches('/');
        let encoded_path = normalized
            .as_str()
            .split('/')
            .filter(|part| !part.is_empty())
            .map(|part| utf8_percent_encode(part, PATH_ENCODE_SET).to_string())
            .collect::<Vec<_>>()
            .join("/");

        let mut full_path = if encoded_path.is_empty() {
            format!("{base_path}/")
        } else {
            format!("{base_path}/{encoded_path}")
        };
        if as_dir && !full_path.ends_with('/') {
            full_path.push('/');
        }
        url.set_path(&full_path);
        Ok(url)
    }

    pub async fn list_dir(&self, raw_path: &str) -> Result<Vec<DavEntry>, WebdavError> {
        let requested = normalize_vault_path(raw_path)?;
        let url = self.url_for(requested.as_str(), true)?;
        let response = self
            .request(custom_method("PROPFIND"), url)
            .header("Depth", "1")
            .send()
            .await?;
        ensure_success(response.status(), "PROPFIND")?;
        let xml = response.text().await?;
        let entries = self.parse_propfind(&xml, requested.as_str())?;
        Ok(entries)
    }

    pub async fn get_text(&self, raw_path: &str) -> Result<String, WebdavError> {
        let url = self.url_for(raw_path, false)?;
        let response = self.request(Method::GET, url).send().await?;
        ensure_success(response.status(), "GET")?;
        Ok(response.text().await?)
    }

    pub async fn put_text(&self, raw_path: &str, body: &str) -> Result<(), WebdavError> {
        let url = self.url_for(raw_path, false)?;
        let response = self
            .request(Method::PUT, url)
            .body(body.to_string())
            .send()
            .await?;
        ensure_success(response.status(), "PUT")?;
        Ok(())
    }

    pub async fn mkcol(&self, raw_path: &str) -> Result<(), WebdavError> {
        let url = self.url_for(raw_path, true)?;
        let response = self.request(custom_method("MKCOL"), url).send().await?;
        let status = response.status();
        if status.is_success() || status == StatusCode::METHOD_NOT_ALLOWED {
            Ok(())
        } else {
            Err(WebdavError::Status {
                status,
                context: "MKCOL".to_string(),
            })
        }
    }

    pub async fn exists(&self, raw_path: &str) -> Result<bool, WebdavError> {
        let url = self.url_for(raw_path, false)?;
        let response = self.request(Method::HEAD, url).send().await?;
        let status = response.status();
        if status == StatusCode::NOT_FOUND {
            return Ok(false);
        }
        ensure_success(status, "HEAD")?;
        Ok(true)
    }

    pub async fn delete(&self, raw_path: &str) -> Result<(), WebdavError> {
        let url = self.url_for(raw_path, false)?;
        let response = self.request(Method::DELETE, url).send().await?;
        ensure_success(response.status(), "DELETE")?;
        Ok(())
    }

    pub async fn move_path(
        &self,
        source_path: &str,
        dest_path: &str,
        overwrite: bool,
    ) -> Result<(), WebdavError> {
        let source_url = self.url_for(source_path, false)?;
        let dest_url = self.url_for(dest_path, false)?;
        let response = self
            .request(custom_method("MOVE"), source_url)
            .header("Destination", dest_url.to_string())
            .header("Overwrite", overwrite_header(overwrite))
            .send()
            .await?;
        ensure_success(response.status(), "MOVE")?;
        Ok(())
    }

    pub async fn copy_path(
        &self,
        source_path: &str,
        dest_path: &str,
        overwrite: bool,
        depth: &str,
    ) -> Result<(), WebdavError> {
        let source_url = self.url_for(source_path, false)?;
        let dest_url = self.url_for(dest_path, false)?;
        let response = self
            .request(custom_method("COPY"), source_url)
            .header("Destination", dest_url.to_string())
            .header("Overwrite", overwrite_header(overwrite))
            .header("Depth", depth)
            .send()
            .await?;
        ensure_success(response.status(), "COPY")?;
        Ok(())
    }

    pub async fn proppatch(&self, raw_path: &str, xml: &str) -> Result<(), WebdavError> {
        let url = self.url_for(raw_path, false)?;
        let response = self
            .request(custom_method("PROPPATCH"), url)
            .header("Content-Type", "application/xml")
            .body(xml.to_string())
            .send()
            .await?;
        ensure_success(response.status(), "PROPPATCH")?;
        Ok(())
    }

    pub async fn lock(
        &self,
        raw_path: &str,
        scope: &str,
        owner: Option<&str>,
        timeout: &str,
        depth: &str,
    ) -> Result<LockResult, WebdavError> {
        let url = self.url_for(raw_path, false)?;
        let body = lock_body(scope, owner);
        let response = self
            .request(custom_method("LOCK"), url)
            .header("Content-Type", "application/xml")
            .header("Depth", depth)
            .header("Timeout", lock_timeout_header(timeout))
            .body(body)
            .send()
            .await?;
        ensure_success(response.status(), "LOCK")?;
        let lock_token = response
            .headers()
            .get("Lock-Token")
            .and_then(|value| value.to_str().ok())
            .map(ToString::to_string);
        let body = response.text().await?;
        Ok(LockResult { lock_token, body })
    }

    pub async fn unlock(&self, raw_path: &str, token: &str) -> Result<(), WebdavError> {
        let url = self.url_for(raw_path, false)?;
        let response = self
            .request(custom_method("UNLOCK"), url)
            .header("Lock-Token", normalize_lock_token(token))
            .send()
            .await?;
        ensure_success(response.status(), "UNLOCK")?;
        Ok(())
    }

    pub async fn options_allow(
        &self,
        raw_path: &str,
        as_dir: bool,
    ) -> Result<Vec<String>, WebdavError> {
        let url = self.url_for(raw_path, as_dir)?;
        let response = self.request(Method::OPTIONS, url).send().await?;
        ensure_success(response.status(), "OPTIONS")?;
        let allow = response
            .headers()
            .get(reqwest::header::ALLOW)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();

        Ok(allow
            .split(',')
            .map(|method| method.trim().to_ascii_uppercase())
            .filter(|method| !method.is_empty())
            .collect())
    }

    fn request(&self, method: Method, url: Url) -> reqwest::RequestBuilder {
        self.client
            .request(method, url)
            .basic_auth(&self.username, Some(&self.password))
    }

    fn parse_propfind(&self, xml: &str, requested: &str) -> Result<Vec<DavEntry>, WebdavError> {
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);
        let mut in_href = false;
        let mut entries = Vec::new();

        loop {
            match reader.read_event() {
                Ok(Event::Start(event)) if is_href_name(event.name().as_ref()) => {
                    in_href = true;
                }
                Ok(Event::Text(text)) if in_href => {
                    let href = text
                        .decode()
                        .map_err(|err| WebdavError::Xml(err.to_string()))?
                        .into_owned();
                    if let Some(entry) = self.entry_from_href(&href, requested)? {
                        entries.push(entry);
                    }
                }
                Ok(Event::End(event)) if is_href_name(event.name().as_ref()) => {
                    in_href = false;
                }
                Ok(Event::Eof) => break,
                Err(err) => return Err(WebdavError::Xml(err.to_string())),
                _ => {}
            }
        }

        Ok(entries)
    }

    fn entry_from_href(
        &self,
        href: &str,
        requested: &str,
    ) -> Result<Option<DavEntry>, WebdavError> {
        let href_path = if href.contains("://") {
            Url::parse(href)?.path().to_string()
        } else {
            href.to_string()
        };
        let is_dir = href_path.ends_with('/');
        let base_path = self.base_url.path();
        let relative = href_path
            .strip_prefix(base_path)
            .or_else(|| href_path.strip_prefix(base_path.trim_end_matches('/')))
            .unwrap_or(href_path.as_str())
            .trim_start_matches('/');
        let relative = relative.trim_end_matches('/');
        let decoded = percent_decode_str(relative)
            .decode_utf8()
            .map_err(|_| WebdavError::Xml("href is not valid UTF-8".to_string()))?;
        let path = normalize_vault_path(&decoded)?;

        if path.as_str().is_empty() || path.as_str() == requested {
            return Ok(None);
        }

        Ok(Some(DavEntry {
            path: path.into_string(),
            is_dir,
        }))
    }
}

fn overwrite_header(overwrite: bool) -> &'static str {
    if overwrite { "T" } else { "F" }
}

fn lock_timeout_header(timeout: &str) -> String {
    if timeout.eq_ignore_ascii_case("infinite") {
        "Infinite".to_string()
    } else if timeout.to_ascii_lowercase().starts_with("second-") {
        timeout.to_string()
    } else {
        format!("Second-{timeout}")
    }
}

fn normalize_lock_token(token: &str) -> String {
    let trimmed = token.trim();
    if trimmed.starts_with('<') && trimmed.ends_with('>') {
        trimmed.to_string()
    } else {
        format!("<{trimmed}>")
    }
}

fn lock_body(scope: &str, owner: Option<&str>) -> String {
    let scope = if scope.eq_ignore_ascii_case("shared") {
        "shared"
    } else {
        "exclusive"
    };
    let owner = owner
        .map(|owner| format!("<d:owner>{}</d:owner>", escape_xml(owner)))
        .unwrap_or_default();
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?><d:lockinfo xmlns:d="DAV:"><d:lockscope><d:{scope}/></d:lockscope><d:locktype><d:write/></d:locktype>{owner}</d:lockinfo>"#
    )
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn custom_method(method: &'static str) -> Method {
    Method::from_bytes(method.as_bytes()).expect("static WebDAV method is valid")
}

fn ensure_success(status: StatusCode, context: &str) -> Result<(), WebdavError> {
    if status.is_success() {
        Ok(())
    } else {
        Err(WebdavError::Status {
            status,
            context: context.to_string(),
        })
    }
}

fn is_href_name(name: &[u8]) -> bool {
    name == b"href" || name.ends_with(b":href")
}
