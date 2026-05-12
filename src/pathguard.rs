use percent_encoding::percent_decode_str;
use std::fmt;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaultPath(String);

impl VaultPath {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Display for VaultPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PathError {
    #[error("absolute paths are not allowed: {0}")]
    Absolute(String),
    #[error("URL-like paths are not allowed: {0}")]
    UrlLike(String),
    #[error("path contains invalid UTF-8 after percent decoding")]
    InvalidEncoding,
    #[error("path traversal is not allowed: {0}")]
    Traversal(String),
    #[error("protected path is not writable: {0}")]
    Protected(String),
    #[error("writing to {path} is not allowed; allowed write directories: {allowed}")]
    WriteNotAllowed { path: String, allowed: String },
}

pub fn normalize_vault_path(raw: &str) -> Result<VaultPath, PathError> {
    let trimmed = raw.trim();
    if trimmed.contains("://") {
        return Err(PathError::UrlLike(trimmed.to_string()));
    }
    if trimmed.starts_with('/') || trimmed.starts_with('\\') {
        return Err(PathError::Absolute(trimmed.to_string()));
    }
    if trimmed.contains('\\') {
        return Err(PathError::Traversal(trimmed.to_string()));
    }

    let decoded = percent_decode_str(trimmed)
        .decode_utf8()
        .map_err(|_| PathError::InvalidEncoding)?;

    let mut parts = Vec::new();
    for part in decoded.split('/') {
        let part = part.trim();
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." {
            return Err(PathError::Traversal(trimmed.to_string()));
        }
        parts.push(part);
    }

    Ok(VaultPath(parts.join("/")))
}

pub fn assert_write_allowed(
    raw: &str,
    allow_write_dirs: &[String],
) -> Result<VaultPath, PathError> {
    let path = normalize_vault_path(raw)?;
    let normalized = path.as_str();

    if is_protected_write_path(normalized) {
        return Err(PathError::Protected(normalized.to_string()));
    }

    let mut normalized_allowed = Vec::with_capacity(allow_write_dirs.len());
    for allowed in allow_write_dirs {
        normalized_allowed.push(normalize_vault_path(allowed)?.into_string());
    }

    let allowed = normalized_allowed
        .iter()
        .any(|dir| normalized == dir || normalized.starts_with(&format!("{dir}/")));

    if !allowed {
        return Err(PathError::WriteNotAllowed {
            path: normalized.to_string(),
            allowed: normalized_allowed.join(", "),
        });
    }

    Ok(path)
}

fn is_protected_write_path(path: &str) -> bool {
    let Some(first) = path.split('/').next() else {
        return false;
    };
    matches!(
        first,
        ".obsidian"
            | "Attachments"
            | "Notes"
            | "Projects"
            | "Troubleshooting"
            | "Index"
            | "Daily"
            | "Sources"
    )
}
