use crate::cli::{CatMode, Cli, Commands};
use crate::config::{
    AppConfig, ConfigError, InitConfig, config_path_for_init, config_path_from_cli, load_config,
    write_initial_config,
};
use crate::markdown::{MarkdownError, NoteDraft, filename_timestamp, render_note, slugify_title};
use crate::pathguard::{PathError, normalize_vault_path};
use crate::webdav::{DavEntry, WebdavClient, WebdavError, WebdavSettings};
use chrono::Utc;
use chrono_tz::Tz;
use reqwest::StatusCode;
use serde::Serialize;
use serde_json::{Value, json};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    pub message: String,
}

#[derive(Debug, Error)]
pub enum CommandError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Path(#[from] PathError),
    #[error(transparent)]
    Webdav(#[from] WebdavError),
    #[error(transparent)]
    Markdown(#[from] MarkdownError),
    #[error("{0}")]
    InvalidArgs(String),
    #[error("file already exists: {0}")]
    FileAlreadyExists(String),
    #[error("command is not implemented yet: {0}")]
    NotImplemented(&'static str),
}

pub async fn run_command(cli: Cli, stdin: &str) -> Result<CommandOutput, CommandError> {
    match cli.command {
        Commands::Init(args) => {
            let path = config_path_for_init(cli.config.as_deref(), None)?;
            let mut stdin_lines = stdin.lines();
            let url = init_value(args.url, &mut stdin_lines, None, "init requires WebDAV URL")?;
            let username = init_value(
                args.username,
                &mut stdin_lines,
                None,
                "init requires username",
            )?;
            let password_env = init_value(
                args.password_env,
                &mut stdin_lines,
                Some("OBSIDIAN_WEBDAV_PASSWORD"),
                "init requires password env name",
            )?;
            let write_dir = init_value(
                args.write_dir,
                &mut stdin_lines,
                Some("Inbox/Hermes"),
                "init requires default write directory",
            )?;
            write_initial_config(
                &path,
                InitConfig {
                    url,
                    username,
                    password_env,
                    write_dir,
                },
                args.force,
            )?;

            Ok(CommandOutput {
                message: format!("Config written: {}", path.display()),
            })
        }
        Commands::Doctor(args) => {
            run_doctor(cli.config.as_deref(), args.no_write_test, args.json).await
        }
        Commands::Ls(args) => {
            let client = client_from_config_path(cli.config.as_deref())?;
            let path = args.path.unwrap_or_default();
            let entries = client.list_dir(&path).await?;
            Ok(CommandOutput {
                message: if args.json {
                    json!({
                        "ok": true,
                        "action": "listed",
                        "entries": entries,
                    })
                    .to_string()
                } else {
                    format_ls(&path, entries)
                },
            })
        }
        Commands::Cat(args) => {
            let client = client_from_config_path(cli.config.as_deref())?;
            let content = client.get_text(&args.path).await?;
            Ok(CommandOutput {
                message: render_cat(&content, args.mode()),
            })
        }
        Commands::Search(args) => {
            let client = client_from_config_path(cli.config.as_deref())?;
            let root = args.dir.unwrap_or_default();
            let matches = search_notes(&client, &root, &args.query, args.case_sensitive).await?;
            Ok(CommandOutput {
                message: if args.json {
                    json!({
                        "ok": true,
                        "action": "searched",
                        "matches": matches,
                    })
                    .to_string()
                } else {
                    matches
                        .into_iter()
                        .map(|m| format!("{}:{}: {}", m.path, m.line, m.snippet))
                        .collect::<Vec<_>>()
                        .join("\n")
                },
            })
        }
        Commands::New(args) => {
            let path = config_path_from_cli(cli.config.as_deref(), None)?;
            let config = load_config(&path)?;
            let client = client_from_config(config.clone())?;
            let write_dir = args
                .dir
                .clone()
                .unwrap_or_else(|| config.vault.default_write_dir.clone());
            let body = args.body.clone().unwrap_or_else(|| stdin.to_string());
            let source = args
                .source
                .clone()
                .unwrap_or_else(|| config.markdown.default_source.clone());
            let template = args.template.clone().unwrap_or_else(|| "basic".to_string());
            let created = current_time(&config);
            let note = render_note(NoteDraft {
                title: args.title.clone(),
                body,
                source,
                default_tags: config.vault.default_tags.clone(),
                tags: args.tags.clone(),
                template,
                created,
                frontmatter: config.markdown.frontmatter,
                heading_title: config.markdown.heading_title,
                add_created_time: config.markdown.add_created_time,
                add_source: config.markdown.add_source,
            })?;
            let target_path =
                next_note_path(&client, &config, &write_dir, &args.title, args.unique).await?;
            let allowed_path = crate::pathguard::assert_write_allowed(
                &target_path,
                &config.behavior.allow_write_dirs,
            )?
            .into_string();

            if client.exists(&allowed_path).await?
                && !(args.overwrite && config.behavior.allow_overwrite)
            {
                return Err(CommandError::FileAlreadyExists(allowed_path));
            }

            client.put_text(&allowed_path, &note).await?;
            let url = client.url_for(&allowed_path, false)?.to_string();
            Ok(CommandOutput {
                message: if args.json {
                    json!({
                        "ok": true,
                        "action": "created",
                        "path": allowed_path,
                        "url": url,
                    })
                    .to_string()
                } else {
                    format!("Created: {allowed_path}")
                },
            })
        }
        Commands::Mkdir(args) => {
            let path = config_path_from_cli(cli.config.as_deref(), None)?;
            let config = load_config(&path)?;
            let client = client_from_config(config.clone())?;
            let target = crate::pathguard::assert_write_allowed(
                &args.path,
                &config.behavior.allow_write_dirs,
            )?
            .into_string();
            if args.parents {
                create_parent_dirs(&client, &config, &target).await?;
            } else {
                client.mkcol(&target).await?;
            }

            Ok(CommandOutput {
                message: if args.json {
                    json!({
                        "ok": true,
                        "action": "created_directory",
                        "path": target,
                    })
                    .to_string()
                } else {
                    format!("Created directory: {target}")
                },
            })
        }
    }
}

fn init_value<'a, I>(
    arg: Option<String>,
    stdin_lines: &mut I,
    default: Option<&str>,
    missing_message: &str,
) -> Result<String, CommandError>
where
    I: Iterator<Item = &'a str>,
{
    if let Some(value) = arg {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }

    if let Some(line) = stdin_lines.next() {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }

    default
        .map(ToString::to_string)
        .ok_or_else(|| CommandError::InvalidArgs(missing_message.to_string()))
}

fn client_from_config_path(cli_config: Option<&str>) -> Result<WebdavClient, CommandError> {
    let path = config_path_from_cli(cli_config, None)?;
    let config = load_config(&path)?;
    client_from_config(config)
}

fn client_from_config(config: AppConfig) -> Result<WebdavClient, CommandError> {
    let password = config.resolve_password()?;
    Ok(WebdavClient::new(WebdavSettings {
        base_url: config.webdav.base_url,
        username: config.webdav.username,
        password,
        timeout_secs: config.webdav.timeout,
    })?)
}

fn format_ls(requested: &str, entries: Vec<DavEntry>) -> String {
    let requested = normalize_vault_path(requested)
        .map(|path| path.into_string())
        .unwrap_or_default();
    entries
        .into_iter()
        .map(|entry| {
            let name = child_display_name(&requested, &entry.path);
            if entry.is_dir {
                format!("{name}/")
            } else {
                name
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn child_display_name(requested: &str, entry_path: &str) -> String {
    if requested.is_empty() {
        return entry_path.to_string();
    }
    entry_path
        .strip_prefix(&format!("{requested}/"))
        .unwrap_or(entry_path)
        .to_string()
}

fn render_cat(content: &str, mode: CatMode) -> String {
    let (frontmatter, body) = split_frontmatter(content);
    match mode {
        CatMode::Full => content.to_string(),
        CatMode::Frontmatter => frontmatter.unwrap_or_default().to_string(),
        CatMode::Body => body.to_string(),
    }
}

fn split_frontmatter(content: &str) -> (Option<&str>, &str) {
    let Some(rest) = content.strip_prefix("---\n") else {
        return (None, content);
    };
    let Some(end) = rest.find("\n---") else {
        return (None, content);
    };
    let frontmatter = &rest[..end];
    let body_start = end + "\n---".len();
    let body = rest[body_start..]
        .strip_prefix("\n")
        .unwrap_or(&rest[body_start..]);
    let body = body.strip_prefix("\n").unwrap_or(body);
    (Some(frontmatter), body)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct SearchMatch {
    path: String,
    line: usize,
    snippet: String,
}

async fn search_notes(
    client: &WebdavClient,
    root: &str,
    query: &str,
    case_sensitive: bool,
) -> Result<Vec<SearchMatch>, CommandError> {
    let root = normalize_vault_path(root)?.into_string();
    let mut stack = vec![root];
    let mut matches = Vec::new();

    while let Some(dir) = stack.pop() {
        for entry in client.list_dir(&dir).await? {
            if should_skip_search_path(&entry.path) {
                continue;
            }
            if entry.is_dir {
                stack.push(entry.path);
                continue;
            }
            if !entry.path.ends_with(".md") {
                continue;
            }
            let content = client.get_text(&entry.path).await?;
            matches.extend(find_in_note(&entry.path, &content, query, case_sensitive));
        }
    }

    Ok(matches)
}

fn should_skip_search_path(path: &str) -> bool {
    path.split('/').any(|part| part == ".obsidian") || path.starts_with("Attachments/")
}

fn find_in_note(path: &str, content: &str, query: &str, case_sensitive: bool) -> Vec<SearchMatch> {
    let needle = if case_sensitive {
        query.to_string()
    } else {
        query.to_lowercase()
    };
    content
        .lines()
        .enumerate()
        .filter_map(|(idx, line)| {
            let haystack = if case_sensitive {
                line.to_string()
            } else {
                line.to_lowercase()
            };
            haystack.contains(&needle).then(|| SearchMatch {
                path: path.to_string(),
                line: idx + 1,
                snippet: line.trim().to_string(),
            })
        })
        .collect()
}

async fn next_note_path(
    client: &WebdavClient,
    config: &AppConfig,
    write_dir: &str,
    title: &str,
    unique: bool,
) -> Result<String, CommandError> {
    let created = current_time(config);
    let timestamp = filename_timestamp(created, &config.vault.filename_time_format);
    let slug = slugify_title(title);
    let base = format!("{}/{}-{}.md", trim_slash(write_dir), timestamp, slug);
    if !unique || !client.exists(&base).await? {
        return Ok(base);
    }

    for idx in 1..1000 {
        let candidate = format!(
            "{}/{}-{}-{}.md",
            trim_slash(write_dir),
            timestamp,
            slug,
            idx
        );
        if !client.exists(&candidate).await? {
            return Ok(candidate);
        }
    }

    Err(CommandError::InvalidArgs(
        "could not generate a unique note path".to_string(),
    ))
}

fn current_time(config: &AppConfig) -> chrono::DateTime<Tz> {
    let timezone = config
        .vault
        .timezone
        .parse::<Tz>()
        .unwrap_or(chrono_tz::Asia::Shanghai);
    Utc::now().with_timezone(&timezone)
}

fn trim_slash(path: &str) -> &str {
    path.trim().trim_matches('/')
}

async fn create_parent_dirs(
    client: &WebdavClient,
    config: &AppConfig,
    target: &str,
) -> Result<(), CommandError> {
    let normalized_target = normalize_vault_path(target)?.into_string();
    let mut matching_base = None;
    for allowed in &config.behavior.allow_write_dirs {
        let allowed = normalize_vault_path(allowed)?.into_string();
        if normalized_target == allowed || normalized_target.starts_with(&format!("{allowed}/")) {
            matching_base = Some(allowed);
            break;
        }
    }
    let base = matching_base.ok_or_else(|| PathError::WriteNotAllowed {
        path: normalized_target.clone(),
        allowed: config.behavior.allow_write_dirs.join(", "),
    })?;

    if normalized_target == base {
        client.mkcol(&base).await?;
        return Ok(());
    }

    let remainder = normalized_target
        .strip_prefix(&format!("{base}/"))
        .unwrap_or_default();
    let mut current = base;
    for part in remainder.split('/').filter(|part| !part.is_empty()) {
        current.push('/');
        current.push_str(part);
        client.mkcol(&current).await?;
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
struct DoctorCheck {
    name: String,
    ok: bool,
}

async fn run_doctor(
    cli_config: Option<&str>,
    no_write_test: bool,
    json_output: bool,
) -> Result<CommandOutput, CommandError> {
    let path = config_path_from_cli(cli_config, None)?;
    let config = load_config(&path)?;
    let client = client_from_config(config.clone())?;
    let mut checks = vec![DoctorCheck {
        name: "Config loaded".to_string(),
        ok: true,
    }];

    client.options_allow("", true).await?;
    checks.push(DoctorCheck {
        name: "WebDAV connected".to_string(),
        ok: true,
    });

    client.list_dir("").await?;
    checks.push(DoctorCheck {
        name: "Root vault readable".to_string(),
        ok: true,
    });

    client.list_dir(&config.vault.default_write_dir).await?;
    let write_dir_exists_label = format!("{} exists", config.vault.default_write_dir);
    checks.push(DoctorCheck {
        name: write_dir_exists_label,
        ok: true,
    });

    if no_write_test {
        checks.push(DoctorCheck {
            name: "Write test skipped".to_string(),
            ok: true,
        });
    } else {
        let marker = format!(
            "{}/.webdav-cli-doctor-write-{}.md",
            trim_slash(&config.vault.default_write_dir),
            Utc::now().timestamp()
        );
        crate::pathguard::assert_write_allowed(&marker, &config.behavior.allow_write_dirs)?;
        client.put_text(&marker, "doctor write probe\n").await?;
        let writable_label = format!("{} writable", config.vault.default_write_dir);
        checks.push(DoctorCheck {
            name: writable_label,
            ok: true,
        });
    }

    if let Some(readonly_dir) = config.behavior.readonly_dirs.first() {
        let allow = client.options_allow(readonly_dir, true).await?;
        if contains_write_or_destructive_method(&allow) {
            return Err(CommandError::InvalidArgs(format!(
                "{readonly_dir} is not readonly; server allows: {}",
                allow.join(", ")
            )));
        }
        let readonly_label = format!("{readonly_dir} directory readonly");
        checks.push(DoctorCheck {
            name: readonly_label,
            ok: true,
        });
    }

    let delete_probe = format!(
        "{}/.webdav-cli-doctor-delete-probe.md",
        trim_slash(&config.vault.default_write_dir)
    );
    let delete_status = client.delete_status(&delete_probe).await?;
    if !is_delete_forbidden(delete_status) {
        return Err(CommandError::InvalidArgs(format!(
            "DELETE is not forbidden; server returned {delete_status}"
        )));
    }
    checks.push(DoctorCheck {
        name: "DELETE forbidden".to_string(),
        ok: true,
    });

    if json_output {
        Ok(CommandOutput {
            message: json!({
                "ok": true,
                "action": "doctor",
                "checks": checks,
            })
            .to_string(),
        })
    } else {
        Ok(CommandOutput {
            message: checks
                .into_iter()
                .map(|check| format!("[OK] {}", check.name))
                .collect::<Vec<_>>()
                .join("\n"),
        })
    }
}

fn contains_write_or_destructive_method(allow: &[String]) -> bool {
    allow.iter().any(|method| {
        matches!(
            method.as_str(),
            "PUT" | "MKCOL" | "DELETE" | "MOVE" | "COPY" | "PROPPATCH" | "LOCK" | "UNLOCK"
        )
    })
}

fn is_delete_forbidden(status: StatusCode) -> bool {
    matches!(
        status,
        StatusCode::FORBIDDEN | StatusCode::METHOD_NOT_ALLOWED | StatusCode::NOT_IMPLEMENTED
    )
}

impl CommandError {
    pub fn code(&self) -> &'static str {
        match self {
            CommandError::Config(ConfigError::MissingPasswordEnv(_)) => "missing_password_env",
            CommandError::Config(ConfigError::AlreadyExists(_)) => "config_exists",
            CommandError::Config(_) => "config_error",
            CommandError::Path(PathError::WriteNotAllowed { .. }) => "write_not_allowed",
            CommandError::Path(_) => "invalid_path",
            CommandError::Webdav(_) => "webdav_error",
            CommandError::Markdown(_) => "markdown_error",
            CommandError::InvalidArgs(_) => "invalid_args",
            CommandError::FileAlreadyExists(_) => "file_already_exists",
            CommandError::NotImplemented(_) => "not_implemented",
        }
    }
}

pub fn error_json(error: &CommandError) -> Value {
    json!({
        "ok": false,
        "error": error.code(),
        "message": error.to_string(),
    })
}
