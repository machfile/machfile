use std::env;

use clap::{Command, builder::styling, crate_authors, crate_version};
use log::warn;

#[cfg(feature = "complete")]
use crate::complete::{handle_auto_complete, handle_setup_complete};

use crate::{
    executer::{CommandError, execute},
    parser::{Config, get_config, load_config},
};

/// Configure styles for clap
fn build_clap_styles() -> styling::Styles {
    styling::Styles::styled()
        .header(styling::AnsiColor::Green.on_default() | styling::Effects::BOLD)
        .usage(styling::AnsiColor::Green.on_default() | styling::Effects::BOLD)
        .literal(styling::AnsiColor::Blue.on_default() | styling::Effects::BOLD)
        .placeholder(styling::AnsiColor::Cyan.on_default())
}

/// Construct the clap command
///
/// This constructs dynamic commands based on the found configuration. See [`parse_config`] for how
/// configuration is loaded.
pub fn build_cli_commands(config: Option<&Config>) -> Command {
    let mut app = Command::new("mach")
        .author(crate_authors!("\n"))
        .version(crate_version!())
        .about("Run stuff, get shit done")
        .arg_required_else_help(true)
        .styles(build_clap_styles());

    #[cfg(feature = "complete")]
    {
        use crate::complete::add_complete_commands;

        app = add_complete_commands(app);
    }

    if let Some(conf) = config {
        for (name, command) in &conf.tasks {
            let mut sub = Command::new(name);
            if let Some(desc) = &command.desc {
                sub = sub.about(desc);
            }

            app = app.subcommand(sub);
        }
    }

    app
}

/// Execute the [`clap`] CLI
///
/// The available commands are populated dynamically by searching for a mach configuration file in
/// the current working directory (and upwards if inside a git repository).
///
/// For more information, see [`crate::parser::load_config`]
///
/// # Errors
///
/// Throws errors upwards, so the CLI can exit accordingly
pub fn cli() -> Result<(), CommandError> {
    let config_override = env::var_os("MACH_CONFIG_PATH");

    if load_config(config_override).is_err() {
        warn!("Failed to parse configuration");
    }

    let config = get_config();
    let matches = build_cli_commands(config).get_matches();

    match matches.subcommand() {
        None => unreachable!(),
        #[cfg(feature = "complete")]
        Some(("setup_complete", args)) => handle_setup_complete(args),
        Some((cmd, args)) => {
            if config.is_none() {
                println!("Failed to parse configuration");
                return Err(CommandError {
                    message: "Failed to parse configuration".to_owned(),
                });
            }
            match (cmd, args) {
                #[cfg(feature = "complete")]
                ("auto_complete", args) => handle_auto_complete(args),
                (name, _) => execute(name),
            }
        }
    }
}
