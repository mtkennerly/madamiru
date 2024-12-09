mod parse;

use clap::CommandFactory;

use crate::{
    cli::parse::{Cli, CompletionShell, Subcommand},
    lang, media,
    path::StrictPath,
    prelude::Error,
    resource::{cache::Cache, config::Config, playlist::Playlist, ResourceFile},
};

pub fn parse_sources(sources: Vec<StrictPath>) -> Vec<media::Source> {
    if !sources.is_empty() {
        sources
            .into_iter()
            .filter_map(|path| (!path.is_blank()).then(|| media::Source::new_path(path)))
            .collect()
    } else {
        use std::io::IsTerminal;

        let stdin = std::io::stdin();
        if stdin.is_terminal() {
            vec![]
        } else {
            let sources: Vec<_> = stdin
                .lines()
                .map_while(Result::ok)
                .filter_map(|raw| (!raw.trim().is_empty()).then(|| media::Source::new_path(StrictPath::new(raw))))
                .collect();
            log::debug!("Sources from stdin: {:?}", &sources);
            if sources.is_empty() {
                vec![]
            } else {
                sources
            }
        }
    }
}

pub fn parse() -> Cli {
    use clap::Parser;
    Cli::parse()
}

pub fn run(sub: Subcommand) -> Result<(), Error> {
    let mut config = Config::load()?;
    Cache::load().unwrap_or_default().migrate_config(&mut config);
    lang::set(config.language);

    log::debug!("Config on startup: {config:?}");
    log::debug!("Invocation: {sub:?}");

    match sub {
        Subcommand::Complete { shell } => {
            let clap_shell = match shell {
                CompletionShell::Bash => clap_complete::Shell::Bash,
                CompletionShell::Fish => clap_complete::Shell::Fish,
                CompletionShell::Zsh => clap_complete::Shell::Zsh,
                CompletionShell::PowerShell => clap_complete::Shell::PowerShell,
                CompletionShell::Elvish => clap_complete::Shell::Elvish,
            };
            clap_complete::generate(
                clap_shell,
                &mut Cli::command(),
                env!("CARGO_PKG_NAME"),
                &mut std::io::stdout(),
            )
        }
        Subcommand::Schema { format, kind } => {
            let format = format.unwrap_or_default();
            let schema = match kind {
                parse::SchemaSubcommand::Config => schemars::schema_for!(Config),
                parse::SchemaSubcommand::Playlist => schemars::schema_for!(Playlist),
            };

            let serialized = match format {
                parse::SerializationFormat::Json => serde_json::to_string_pretty(&schema).unwrap(),
                parse::SerializationFormat::Yaml => serde_yaml::to_string(&schema).unwrap(),
            };
            println!("{serialized}");
        }
    }

    Ok(())
}
