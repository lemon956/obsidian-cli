use chrono::TimeZone;
use chrono_tz::Asia::Shanghai;
use obsidian_cli::markdown::{NoteDraft, render_note, slugify_title};

#[test]
fn slugifies_title_without_dropping_cjk_text() {
    assert_eq!(
        slugify_title("Hermes Telegram 流式卡顿排查"),
        "hermes-telegram-流式卡顿排查"
    );
    assert_eq!(slugify_title("  SSH: dead?  "), "ssh-dead");
}

#[test]
fn renders_troubleshooting_note_with_frontmatter_and_deduped_tags() {
    let created = Shanghai.with_ymd_and_hms(2026, 4, 24, 15, 30, 12).unwrap();
    let note = render_note(NoteDraft {
        title: "Hermes Gateway Debug".to_string(),
        body: "原始日志内容".to_string(),
        source: "telegram".to_string(),
        default_tags: vec!["hermes".to_string(), "inbox".to_string()],
        tags: vec!["hermes".to_string(), "debug".to_string()],
        template: "troubleshooting".to_string(),
        created,
        frontmatter: true,
        heading_title: true,
        add_created_time: true,
        add_source: true,
    })
    .unwrap();

    assert!(note.contains("title: \"Hermes Gateway Debug\""));
    assert!(note.contains("created: \"2026-04-24 15:30:12\""));
    assert!(note.contains("source: \"telegram\""));
    assert!(note.contains("  - hermes\n  - inbox\n  - debug\n  - troubleshooting"));
    assert!(note.contains("## 问题\n\n原始日志内容\n"));
    assert!(note.contains("## 解决方案"));
}
