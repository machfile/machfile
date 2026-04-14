use clap::{ArgAction, ArgMatches, Command, arg, value_parser};
use clap_complete::{Generator, Shell, generate};
use log::warn;
use std::io;

use machfile::{load_config, utils::CommandError};

use crate::cli::build_cli_commands;

pub fn add_complete_commands(cmd: Command) -> Command {
    cmd.subcommand(
        Command::new("auto_complete")
            .arg(
                arg!([shell])
                    .action(ArgAction::Set)
                    .required(true)
                    .value_parser(value_parser!(Shell)),
            )
            .hide(true),
    )
    .subcommand(
        Command::new("setup_complete")
            .arg(
                arg!([shell])
                    .action(ArgAction::Set)
                    .required(true)
                    .value_parser(value_parser!(Shell)),
            )
            .hide(true),
    )
}

/// Prints the completion setup script
///
/// This should be called in the shell initialization script (`.zshrc` or `.bashrc`, for example)
pub fn handle_setup_complete(args: &ArgMatches) -> Result<(), CommandError> {
    let shell = args.get_one::<Shell>("shell").unwrap();

    let script = match shell {
        Shell::Zsh => {
            r#"
# Load mach autocompletion
_update_completion() {
    local auto_complete_output
    auto_complete_output=$(mach auto_complete zsh 2>&1)

    if [[ $? -ne 0 ]]; then
        compdef -d mach
        return
    fi

    eval "$auto_complete_output"
}
chpwd() {
    _update_completion
}
_update_completion
"#
        }
        Shell::Bash => {
            r#"
# Load mach autocompletion
_update_completion() {
    local auto_complete_output
    auto_complete_output=$(mach auto_complete bash 2>&1)

    if [[ $? -ne 0 ]]; then
        complete -r mach
        return
    fi

    eval "$auto_complete_output"
}
chpwd() {
    _update_completion
}
"#
        }
        _ => {
            return Err(CommandError {
                message: "Unsupported shell".to_owned(),
            });
        }
    };

    println!("{script}");
    Ok(())
}

/// Prints the current completion script
///
/// This will be run on each working directory change by the script setup in
/// `handle_setup_complete`.
pub fn handle_auto_complete(args: &ArgMatches) -> Result<(), CommandError> {
    let shell = args.get_one::<Shell>("shell").unwrap();

    if !matches!(shell, Shell::Zsh) {
        return Err(CommandError {
            message: "Unsupported shell".to_owned(),
        });
    }

    let config = match load_config(None) {
        Ok(conf) => Some(conf),
        Err(_) => {
            warn!("Failed to parse config during auto complete call");
            None
        }
    };

    let mut cmd = build_cli_commands(&config);
    print_completions(*shell, &mut cmd);
    Ok(())
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
