use std::env;

use clap::{Command, builder::styling, crate_authors, crate_version};

use crate::{
    CommandError,
    executer::execute,
    parser::{Config, load_config},
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
fn build_cli_commands(config: &Config) -> Command {
    let mut app = Command::new("mach")
        .author(crate_authors!("\n"))
        .version(crate_version!())
        .about("Run stuff, get shit done")
        .arg_required_else_help(true)
        .styles(build_clap_styles());

    for (name, command) in &config.tasks {
        let mut sub = Command::new(name);
        if let Some(desc) = &command.desc {
            sub = sub.about(desc);
        }

        app = app.subcommand(sub);
    }

    app
}

/// Execute the CLI command
///
/// # Errors
/// Throws errors upwards, so the CLI can exit accordingly
pub fn cli() -> Result<(), CommandError> {
    let config_override = env::var_os("MACH_CONFIG_PATH");

    let Ok(config) = load_config(config_override) else {
        println!("Failed to parse configuration");
        return Err(CommandError);
    };
    let matches = build_cli_commands(&config).get_matches();

    let result: Result<(), String> = match matches.subcommand() {
        None => unreachable!(),
        Some((name, _)) => {
            #[expect(
                clippy::missing_panics_doc,
                reason = "clap makes this impossible to panic"
            )]
            let command_config = config.tasks.get(name).unwrap();
            execute(name, command_config);
            Ok(())
        }
    };

    result.map_err(|msg| {
        println!("Failed to run command: {msg}");
        CommandError
    })
}
