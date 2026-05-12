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
