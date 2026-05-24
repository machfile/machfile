use crate::cli::build_cli_commands;
use std::env;

/// Holds flags that are important for the whole execution of the CLI
#[derive(Debug, Clone, Default)]
pub struct CliConfig {
    is_verbose: bool,
}

impl CliConfig {
    pub fn from_matches(matches: &clap::ArgMatches) -> Self {
        Self {
            is_verbose: matches.get_flag("verbose"),
        }
    }

    /// Updates the config from matches, but does not override already set flags.
    /// This allows to first create a config from early matches, and then update it with the final matches after the config is found.
    pub fn update_from_matches(&mut self, matches: &clap::ArgMatches) {
        self.is_verbose = self.is_verbose || matches.get_flag("verbose");
    }

    pub fn is_verbose(&self) -> bool {
        self.is_verbose
    }
}

/// Build the clap configuration for matches before a config is found.
/// This allows to enable flags that could be important for the config finding process
pub fn create_early_cli_config() -> CliConfig {
    let matches = build_cli_commands(&None)
        .disable_help_flag(true)
        .disable_version_flag(true)
        .allow_missing_positional(true)
        .allow_external_subcommands(true)
        .try_get_matches_from(env::args_os());

    if let Ok(matches) = matches {
        CliConfig::from_matches(&matches)
    } else {
        CliConfig::default()
    }
}
