use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "webdav-cli")]
#[command(version, about = "Safe Obsidian WebDAV CLI for Hermes and automation")]
pub struct Cli {
    #[arg(long, global = true, value_name = "PATH")]
    pub config: Option<String>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Init(InitArgs),
    Doctor(DoctorArgs),
    Ls(LsArgs),
    Cat(CatArgs),
    Search(SearchArgs),
    New(NewArgs),
    Mkdir(MkdirArgs),
}

#[derive(Debug, Args)]
pub struct InitArgs {
    #[arg(long)]
    pub url: Option<String>,
    #[arg(long)]
    pub username: Option<String>,
    #[arg(long)]
    pub password_env: Option<String>,
    #[arg(long)]
    pub write_dir: Option<String>,
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Args)]
pub struct DoctorArgs {
    #[arg(long)]
    pub no_write_test: bool,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct LsArgs {
    pub path: Option<String>,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatMode {
    Full,
    Frontmatter,
    Body,
}

#[derive(Debug, Args)]
#[command(group(
    clap::ArgGroup::new("cat_mode")
        .args(["frontmatter", "body"])
        .multiple(false)
))]
pub struct CatArgs {
    pub path: String,
    #[arg(long)]
    pub frontmatter: bool,
    #[arg(long)]
    pub body: bool,
}

impl CatArgs {
    pub fn mode(&self) -> CatMode {
        if self.frontmatter {
            CatMode::Frontmatter
        } else if self.body {
            CatMode::Body
        } else {
            CatMode::Full
        }
    }
}

impl Cli {
    pub fn wants_json(&self) -> bool {
        match &self.command {
            Commands::Doctor(args) => args.json,
            Commands::Ls(args) => args.json,
            Commands::Search(args) => args.json,
            Commands::New(args) => args.json,
            Commands::Mkdir(args) => args.json,
            Commands::Init(_) | Commands::Cat(_) => false,
        }
    }
}

#[derive(Debug, Args)]
pub struct SearchArgs {
    pub query: String,
    #[arg(long)]
    pub dir: Option<String>,
    #[arg(long)]
    pub case_sensitive: bool,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct NewArgs {
    #[arg(long)]
    pub title: String,
    #[arg(long)]
    pub body: Option<String>,
    #[arg(long = "tag")]
    pub tags: Vec<String>,
    #[arg(long)]
    pub source: Option<String>,
    #[arg(long)]
    pub template: Option<String>,
    #[arg(long = "dir")]
    pub dir: Option<String>,
    #[arg(long)]
    pub unique: bool,
    #[arg(long)]
    pub overwrite: bool,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct MkdirArgs {
    pub path: String,
    #[arg(short = 'p', long)]
    pub parents: bool,
    #[arg(long)]
    pub json: bool,
}
