use std::path::PathBuf;

use crate::prelude::StrictPath;

use clap::ValueEnum;

fn parse_strict_path(path: &str) -> Result<StrictPath, std::io::Error> {
    let cwd = StrictPath::cwd();
    Ok(StrictPath::relative(path.to_owned(), Some(cwd.raw())))
}

fn styles() -> clap::builder::styling::Styles {
    use clap::builder::styling::{AnsiColor, Effects, Styles};

    Styles::styled()
        .header(AnsiColor::Yellow.on_default() | Effects::BOLD)
        .usage(AnsiColor::Yellow.on_default() | Effects::BOLD)
        .literal(AnsiColor::Green.on_default() | Effects::BOLD)
        .placeholder(AnsiColor::Green.on_default())
}

#[derive(clap::Subcommand, Clone, Debug, PartialEq, Eq)]
pub enum CompletionShell {
    #[clap(about = "Completions for Bash")]
    Bash,
    #[clap(about = "Completions for Fish")]
    Fish,
    #[clap(about = "Completions for Zsh")]
    Zsh,
    #[clap(name = "powershell", about = "Completions for PowerShell")]
    PowerShell,
    #[clap(about = "Completions for Elvish")]
    Elvish,
}

/// Serialization format
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum SerializationFormat {
    #[default]
    Json,
    Yaml,
}

#[derive(clap::Subcommand, Clone, Debug, PartialEq, Eq)]
pub enum Subcommand {
    /// Generate shell completion scripts
    Complete {
        #[clap(subcommand)]
        shell: CompletionShell,
    },
    /// Display schemas that the application uses
    Schema {
        #[clap(long, value_enum, value_name = "FORMAT")]
        format: Option<SerializationFormat>,

        #[clap(subcommand)]
        kind: SchemaSubcommand,
    },
}

#[derive(clap::Subcommand, Clone, Debug, PartialEq, Eq)]
pub enum SchemaSubcommand {
    #[clap(about = "Schema for config.yaml")]
    Config,
    #[clap(about = "Schema for playlist.madamiru")]
    Playlist,
}

/// Play multiple videos at once
#[derive(clap::Parser, Clone, Debug, PartialEq, Eq)]
#[clap(name = "madamiru", version, max_term_width = 100, next_line_help = true, styles = styles())]
pub struct Cli {
    /// Use configuration found in DIRECTORY
    #[clap(long, value_name = "DIRECTORY")]
    pub config: Option<PathBuf>,

    /// Files and folders to load.
    /// Alternatively supports stdin (one value per line).
    #[clap(value_parser = parse_strict_path)]
    pub sources: Vec<StrictPath>,

    /// Glob patterns to load.
    #[clap(long)]
    pub glob: Vec<String>,

    #[clap(subcommand)]
    pub sub: Option<Subcommand>,
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::*;

    fn check_args(args: &[&str], expected: Cli) {
        assert_eq!(expected, Cli::parse_from(args));
    }

    #[test]
    fn accepts_cli_without_arguments() {
        check_args(
            &["madamiru"],
            Cli {
                config: None,
                sources: vec![],
                glob: vec![],
                sub: None,
            },
        );
    }
}
