use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("config file already exists: {0}")]
    AlreadyExists(PathBuf),
    #[error("home directory is unavailable; pass --config explicitly")]
    MissingHome,
    #[error("failed to read config {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to write config {path}: {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse config {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: serde_yaml::Error,
    },
    #[error("failed to serialize config: {0}")]
    Serialize(#[from] serde_yaml::Error),
    #[error("environment variable {0} is not set")]
    MissingPasswordEnv(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitConfig {
    pub url: String,
    pub username: String,
    pub password_env: String,
    pub write_dir: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppConfig {
    pub webdav: WebdavConfig,
    pub vault: VaultConfig,
    pub behavior: BehaviorConfig,
    pub markdown: MarkdownConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebdavConfig {
    pub base_url: String,
    pub username: String,
    pub password_env: String,
    pub timeout: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VaultConfig {
    pub default_write_dir: String,
    pub timezone: String,
    pub filename_time_format: String,
    pub default_tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BehaviorConfig {
    pub allow_overwrite: bool,
    pub allow_delete: bool,
    pub allow_move: bool,
    pub allow_write_dirs: Vec<String>,
    pub readonly_dirs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkdownConfig {
    pub frontmatter: bool,
    pub heading_title: bool,
    pub add_created_time: bool,
    pub add_source: bool,
    pub default_source: String,
}

impl AppConfig {
    pub fn from_init(init: InitConfig) -> Self {
        let write_dir = trim_slashes(&init.write_dir);
        Self {
            webdav: WebdavConfig {
                base_url: normalize_base_url(&init.url),
                username: init.username,
                password_env: init.password_env,
                timeout: 30,
            },
            vault: VaultConfig {
                default_write_dir: write_dir.clone(),
                timezone: "Asia/Shanghai".to_string(),
                filename_time_format: "%Y-%m-%d-%H%M%S".to_string(),
                default_tags: vec!["hermes".to_string(), "inbox".to_string()],
            },
            behavior: BehaviorConfig {
                allow_overwrite: false,
                allow_delete: false,
                allow_move: false,
                allow_write_dirs: vec![write_dir],
                readonly_dirs: vec![
                    "Notes".to_string(),
                    "Projects".to_string(),
                    "Troubleshooting".to_string(),
                    "Index".to_string(),
                    "Daily".to_string(),
                    "Sources".to_string(),
                ],
            },
            markdown: MarkdownConfig {
                frontmatter: true,
                heading_title: true,
                add_created_time: true,
                add_source: true,
                default_source: "hermes".to_string(),
            },
        }
    }

    pub fn resolve_password(&self) -> Result<String, ConfigError> {
        env::var(&self.webdav.password_env)
            .map_err(|_| ConfigError::MissingPasswordEnv(self.webdav.password_env.clone()))
    }
}

pub fn write_initial_config(
    path: &Path,
    init: InitConfig,
    force: bool,
) -> Result<AppConfig, ConfigError> {
    if path.exists() && !force {
        return Err(ConfigError::AlreadyExists(path.to_path_buf()));
    }

    let config = AppConfig::from_init(init);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| ConfigError::Write {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    let yaml = serde_yaml::to_string(&config)?;
    fs::write(path, yaml).map_err(|source| ConfigError::Write {
        path: path.to_path_buf(),
        source,
    })?;

    Ok(config)
}

pub fn load_config(path: &Path) -> Result<AppConfig, ConfigError> {
    let raw = fs::read_to_string(path).map_err(|source| ConfigError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    serde_yaml::from_str(&raw).map_err(|source| ConfigError::Parse {
        path: path.to_path_buf(),
        source,
    })
}

pub fn config_path_from_cli(
    cli_path: Option<&str>,
    home_override: Option<&Path>,
) -> Result<PathBuf, ConfigError> {
    if let Some(cli_path) = cli_path {
        return Ok(PathBuf::from(cli_path));
    }

    let home = match home_override {
        Some(path) => path.to_path_buf(),
        None => env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or(ConfigError::MissingHome)?,
    };

    let new_path = default_config_path(&home);
    let legacy_path = legacy_config_path(&home);
    if !new_path.exists() && legacy_path.exists() {
        Ok(legacy_path)
    } else {
        Ok(new_path)
    }
}

pub fn config_path_for_init(
    cli_path: Option<&str>,
    home_override: Option<&Path>,
) -> Result<PathBuf, ConfigError> {
    if let Some(cli_path) = cli_path {
        return Ok(PathBuf::from(cli_path));
    }

    let home = match home_override {
        Some(path) => path.to_path_buf(),
        None => env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or(ConfigError::MissingHome)?,
    };

    Ok(default_config_path(&home))
}

fn default_config_path(home: &Path) -> PathBuf {
    home.join(".config").join("webdav-cli").join("config.yaml")
}

fn legacy_config_path(home: &Path) -> PathBuf {
    home.join(".config")
        .join("obsidian-cli")
        .join("config.yaml")
}

fn normalize_base_url(url: &str) -> String {
    let trimmed = url.trim();
    if trimmed.ends_with('/') {
        trimmed.to_string()
    } else {
        format!("{trimmed}/")
    }
}

fn trim_slashes(path: &str) -> String {
    path.trim().trim_matches('/').to_string()
}
