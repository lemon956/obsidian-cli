use clap::Parser;
use obsidian_cli::{
    cli::{Cli, Commands},
    commands::run_command,
};
use std::io::{self, IsTerminal, Read};

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let wants_json = cli.wants_json();
    let stdin = match read_command_stdin(&cli) {
        Ok(stdin) => stdin,
        Err(err) => {
            eprintln!("Error: {err}");
            std::process::exit(1);
        }
    };

    match run_command(cli, &stdin).await {
        Ok(output) => {
            if !output.message.is_empty() {
                println!("{}", output.message);
            }
        }
        Err(err) => {
            if wants_json {
                eprintln!("{}", obsidian_cli::commands::error_json(&err));
            } else {
                eprintln!("Error: {err}");
            }
            std::process::exit(1);
        }
    }
}

fn read_command_stdin(cli: &Cli) -> io::Result<String> {
    let needs_stdin = matches!(&cli.command, Commands::New(args) if args.body.is_none());
    if !needs_stdin || io::stdin().is_terminal() {
        return Ok(String::new());
    }

    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;
    Ok(input)
}
