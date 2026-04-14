use std::{env, io};

use clap::{ArgAction, Command, arg, builder::styling, crate_authors, crate_version, value_parser};
use clap_complete::{Generator, Shell, generate};

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
fn build_cli_commands(config: &Config) -> Command {
    let mut app = Command::new("mach")
        .author(crate_authors!("\n"))
        .version(crate_version!())
        .about("Run stuff, get shit done")
        .arg_required_else_help(true)
        .styles(build_clap_styles())
        .subcommand(
            Command::new("auto_complete")
                .arg(
                    arg!([shell])
                        .action(ArgAction::Set)
                        .required(true)
                        .value_parser(value_parser!(Shell)),
                )
                .hide(true),
        );

    for (name, command) in &config.tasks {
        let mut sub = Command::new(name);
        if let Some(desc) = &command.desc {
            sub = sub.about(desc);
        }

        app = app.subcommand(sub);
    }

    app
}

/// Generate shell completions
fn print_completions<G: Generator>(generator: G, cmd: &mut Command) {
    generate(
        generator,
        cmd,
        cmd.get_name().to_string(),
        &mut io::stdout(),
    );
}

/// Execute the CLI command
///
/// # Errors
/// Throws errors upwards, so the CLI can exit accordingly
pub fn cli() -> Result<(), CommandError> {
    let config_override = env::var_os("MACH_CONFIG_PATH");

    if load_config(config_override).is_err() {
        println!("Failed to parse configuration");
        return Err(CommandError {
            message: "Failed to parse configuration".to_owned(),
        });
    }

    let config = get_config();
    let matches = build_cli_commands(config).get_matches();

    match matches.subcommand() {
        None => unreachable!(),
        Some(("auto_complete", args)) => {
            #[expect(clippy::missing_panics_doc, reason = "infallible")]
            let shell = args.get_one::<Shell>("shell").unwrap();

            let mut cmd = build_cli_commands(config);
            print_completions(*shell, &mut cmd);
            Ok(())
        }
        Some((name, _)) => execute(name),
    }
}
