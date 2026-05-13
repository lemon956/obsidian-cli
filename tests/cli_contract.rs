use clap::{CommandFactory, Parser};
use obsidian_cli::cli::{CatMode, Cli, Commands};

#[test]
fn clap_command_name_is_webdav_cli() {
    assert_eq!(Cli::command().get_name(), "webdav-cli");
}

#[test]
fn parses_core_subcommands() {
    let cli = Cli::parse_from(["webdav-cli", "ls", "Notes", "--json"]);
    match cli.command {
        Commands::Ls(args) => {
            assert_eq!(args.path.as_deref(), Some("Notes"));
            assert!(args.json);
        }
        other => panic!("expected ls command, got {other:?}"),
    }

    let cli = Cli::parse_from(["webdav-cli", "cat", "Notes/Hermes.md", "--body"]);
    match cli.command {
        Commands::Cat(args) => {
            assert_eq!(args.path, "Notes/Hermes.md");
            assert_eq!(args.mode(), CatMode::Body);
        }
        other => panic!("expected cat command, got {other:?}"),
    }
}

#[test]
fn parses_new_tags_and_template() {
    let cli = Cli::parse_from([
        "webdav-cli",
        "new",
        "--title",
        "Hermes Gateway Debug",
        "--tag",
        "hermes",
        "--tag",
        "debug",
        "--source",
        "telegram",
        "--template",
        "troubleshooting",
        "--json",
    ]);

    match cli.command {
        Commands::New(args) => {
            assert_eq!(args.title, "Hermes Gateway Debug");
            assert_eq!(args.tags, vec!["hermes", "debug"]);
            assert_eq!(args.source.as_deref(), Some("telegram"));
            assert_eq!(args.template.as_deref(), Some("troubleshooting"));
            assert!(args.json);
        }
        other => panic!("expected new command, got {other:?}"),
    }
}

#[test]
fn parses_webdav_method_subcommands() {
    let cli = Cli::parse_from(["webdav-cli", "delete", "Inbox/Hermes/old.md", "--json"]);
    match cli.command {
        Commands::Delete(args) => {
            assert_eq!(args.path, "Inbox/Hermes/old.md");
            assert!(args.json);
        }
        other => panic!("expected delete command, got {other:?}"),
    }

    let cli = Cli::parse_from([
        "webdav-cli",
        "move",
        "Inbox/Hermes/old.md",
        "Inbox/Hermes/new.md",
        "--overwrite",
    ]);
    match cli.command {
        Commands::Move(args) => {
            assert_eq!(args.source, "Inbox/Hermes/old.md");
            assert_eq!(args.dest, "Inbox/Hermes/new.md");
            assert!(args.overwrite);
        }
        other => panic!("expected move command, got {other:?}"),
    }

    let cli = Cli::parse_from([
        "webdav-cli",
        "copy",
        "Inbox/Hermes/a.md",
        "Inbox/Hermes/b.md",
        "--depth",
        "0",
    ]);
    match cli.command {
        Commands::Copy(args) => {
            assert_eq!(args.source, "Inbox/Hermes/a.md");
            assert_eq!(args.dest, "Inbox/Hermes/b.md");
            assert_eq!(args.depth, "0");
        }
        other => panic!("expected copy command, got {other:?}"),
    }

    let cli = Cli::parse_from([
        "webdav-cli",
        "proppatch",
        "Inbox/Hermes/a.md",
        "--xml",
        "<propertyupdate />",
    ]);
    match cli.command {
        Commands::Proppatch(args) => {
            assert_eq!(args.path, "Inbox/Hermes/a.md");
            assert_eq!(args.xml.as_deref(), Some("<propertyupdate />"));
            assert!(args.xml_file.is_none());
        }
        other => panic!("expected proppatch command, got {other:?}"),
    }

    let cli = Cli::parse_from([
        "webdav-cli",
        "lock",
        "Inbox/Hermes/a.md",
        "--scope",
        "shared",
        "--owner",
        "hermes",
        "--timeout",
        "120",
        "--depth",
        "infinity",
    ]);
    match cli.command {
        Commands::Lock(args) => {
            assert_eq!(args.path, "Inbox/Hermes/a.md");
            assert_eq!(args.scope, "shared");
            assert_eq!(args.owner.as_deref(), Some("hermes"));
            assert_eq!(args.timeout, "120");
            assert_eq!(args.depth, "infinity");
        }
        other => panic!("expected lock command, got {other:?}"),
    }

    let cli = Cli::parse_from([
        "webdav-cli",
        "unlock",
        "Inbox/Hermes/a.md",
        "--token",
        "opaquelocktoken:123",
    ]);
    match cli.command {
        Commands::Unlock(args) => {
            assert_eq!(args.path, "Inbox/Hermes/a.md");
            assert_eq!(args.token, "opaquelocktoken:123");
        }
        other => panic!("expected unlock command, got {other:?}"),
    }
}
