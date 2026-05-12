use chrono::{DateTime, Datelike, Timelike};
use chrono_tz::Tz;
use std::collections::HashSet;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct NoteDraft {
    pub title: String,
    pub body: String,
    pub source: String,
    pub default_tags: Vec<String>,
    pub tags: Vec<String>,
    pub template: String,
    pub created: DateTime<Tz>,
    pub frontmatter: bool,
    pub heading_title: bool,
    pub add_created_time: bool,
    pub add_source: bool,
}

#[derive(Debug, Error)]
pub enum MarkdownError {
    #[error("unknown template: {0}")]
    UnknownTemplate(String),
}

pub fn render_note(draft: NoteDraft) -> Result<String, MarkdownError> {
    let template = draft.template.trim().to_lowercase();
    let mut tags = merge_tags(&draft.default_tags, &draft.tags);
    if template != "basic" && !template.is_empty() {
        push_tag(&mut tags, &template);
    }

    let mut out = String::new();
    if draft.frontmatter {
        out.push_str("---\n");
        out.push_str(&format!("title: \"{}\"\n", yaml_quote(&draft.title)));
        if draft.add_created_time {
            out.push_str(&format!(
                "created: \"{}\"\n",
                draft.created.format("%Y-%m-%d %H:%M:%S")
            ));
        }
        if draft.add_source {
            out.push_str(&format!("source: \"{}\"\n", yaml_quote(&draft.source)));
        }
        out.push_str("status: \"inbox\"\n");
        out.push_str("tags:\n");
        for tag in &tags {
            out.push_str(&format!("  - {tag}\n"));
        }
        out.push_str("---\n\n");
    }

    out.push_str(&render_template(
        &template,
        &draft.title,
        &draft.body,
        draft.heading_title,
    )?);
    Ok(out)
}

pub fn slugify_title(title: &str) -> String {
    let mut slug = String::new();
    let mut last_dash = false;

    for ch in title.trim().to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() || is_cjk(ch) {
            slug.push(ch);
            last_dash = false;
        } else if !last_dash {
            slug.push('-');
            last_dash = true;
        }
    }

    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() {
        "note".to_string()
    } else {
        slug
    }
}

pub fn filename_timestamp(created: DateTime<Tz>, configured_format: &str) -> String {
    let fmt = match configured_format {
        "2006-01-02-150405" => "%Y-%m-%d-%H%M%S",
        other => other,
    };

    if fmt == "fixed" {
        return "fixed".to_string();
    }

    let rendered = created.format(fmt).to_string();
    if rendered.trim().is_empty() {
        format!(
            "{:04}-{:02}-{:02}-{:02}{:02}{:02}",
            created.year(),
            created.month(),
            created.day(),
            created.hour(),
            created.minute(),
            created.second()
        )
    } else {
        rendered
    }
}

fn render_template(
    template: &str,
    title: &str,
    body: &str,
    heading_title: bool,
) -> Result<String, MarkdownError> {
    let clean_body = body.trim_end();
    let content = match template {
        "" | "basic" => {
            if heading_title {
                format!("# {title}\n\n{clean_body}\n")
            } else {
                format!("{clean_body}\n")
            }
        }
        "troubleshooting" => format!(
            "# {title}\n\n## 问题\n\n{clean_body}\n\n## 环境\n\n## 现象\n\n## 日志\n\n## 初步判断\n\n## 解决方案\n\n## 验证命令\n\n## 相关笔记\n"
        ),
        "daily" => format!("# {title}\n\n## 记录\n\n{clean_body}\n"),
        "project" => {
            format!("# {title}\n\n## 目标\n\n{clean_body}\n\n## 进展\n\n## 下一步\n")
        }
        "source" => format!("# {title}\n\n## 来源\n\n{clean_body}\n\n## 摘要\n\n## 关键点\n"),
        "meeting" => {
            format!("# {title}\n\n## 议题\n\n{clean_body}\n\n## 结论\n\n## 待办\n")
        }
        other => return Err(MarkdownError::UnknownTemplate(other.to_string())),
    };
    Ok(content)
}

fn merge_tags(default_tags: &[String], extra_tags: &[String]) -> Vec<String> {
    let mut tags = Vec::new();
    for tag in default_tags.iter().chain(extra_tags.iter()) {
        push_tag(&mut tags, tag);
    }
    tags
}

fn push_tag(tags: &mut Vec<String>, tag: &str) {
    let normalized = tag.trim();
    if normalized.is_empty() {
        return;
    }
    let existing: HashSet<&str> = tags.iter().map(String::as_str).collect();
    if !existing.contains(normalized) {
        tags.push(normalized.to_string());
    }
}

fn yaml_quote(input: &str) -> String {
    input.replace('\\', "\\\\").replace('"', "\\\"")
}

fn is_cjk(ch: char) -> bool {
    matches!(
        ch,
        '\u{4E00}'..='\u{9FFF}'
            | '\u{3400}'..='\u{4DBF}'
            | '\u{F900}'..='\u{FAFF}'
            | '\u{3040}'..='\u{30FF}'
            | '\u{AC00}'..='\u{D7AF}'
    )
}
