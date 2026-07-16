use std::io::Write;
use std::{env, io};

use clap::{Arg, ArgAction, Command, builder::styling, crate_authors, crate_version};
use crossterm::{
    execute, queue,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor},
};
use log::warn;

use machfile::{Builder, config::Config, load_config, utils::CommandError};

#[cfg(feature = "complete")]
use crate::complete::{handle_auto_complete, handle_setup_complete};
use crate::config_info::{print_config, print_options, print_task_config};

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
pub fn build_cli_commands(config: &Option<Config>) -> Command {
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

        app = app.arg(
            Arg::new("show_config")
                .action(ArgAction::SetTrue)
                .global(true)
                .long("show-config")
                .help("Show detected configuration"),
        );

        app = app.arg(
            Arg::new("dry_run")
                .action(ArgAction::SetTrue)
                .global(true)
                .long("dry-run")
                .help("Displays each script that would be executed without executing them"),
        );
    }

    app = app.arg(
        Arg::new("verbose")
            .action(ArgAction::SetTrue)
            .global(true)
            .long("verbose")
            .short('v')
            .help("Show additional information"),
    );

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

    let config = match load_config(config_override) {
        Err(error) => {
            match error {
                machfile::utils::ConfigParseError::InvalidTaskDefinition(message) => {
                    warn!("{message}");
                    let _ = execute!(
                        io::stdout(),
                        SetForegroundColor(Color::Red),
                        SetAttribute(Attribute::Bold),
                        Print("[Error]"),
                        SetAttribute(Attribute::Reset),
                        SetForegroundColor(Color::Red),
                        Print(" Failed to parse the configuration:\n"),
                        Print(message),
                        ResetColor,
                    );
                }
                _ => {
                    warn!("Failed to load/parse configuration: {error}");
                }
            }
            None
        }
        Ok(conf) => Some(conf),
    };

    let Ok(builder_config) = Builder::from_current_dir().build() else {
        return Err(CommandError {
            message: String::new(),
        });
    };

    println!("{builder_config:?}");

    let matches = build_cli_commands(&config).get_matches();

    match matches.subcommand() {
        None => {
            if matches.get_flag("show_config") {
                let conf = config.unwrap();
                print_config(&conf);
            }
            Ok(())
        }
        #[cfg(feature = "complete")]
        Some(("setup_complete", args)) => handle_setup_complete(args),
        Some((cmd, args)) => {
            if config.is_none() {
                println!("Failed to parse configuration");
                return Err(CommandError {
                    message: "failed to parse configuration".to_owned(),
                });
            }

            let conf = config.unwrap();

            match (cmd, args) {
                #[cfg(feature = "complete")]
                ("auto_complete", args) => handle_auto_complete(args),
                (name, _args) => {
                    if matches.get_flag("dry_run") {
                        display_task(&conf, name)
                    } else if matches.get_flag("show_config") {
                        print_task_config(&conf, cmd);
                        Ok(())
                    } else {
                        run_task(&conf, name)
                    }
                }
            }
        }
    }
}

fn get_and_validate_task<'a>(
    config: &'a Config,
    task_name: &str,
) -> Result<&'a machfile::config::Task, CommandError> {
    let Some(task) = config.tasks.get(task_name) else {
        return Err(CommandError {
            message: format!("Task \"{task_name}\" not found in configuration."),
        });
    };

    if task.script.is_none() && task.deps.is_empty() {
        let _ = execute!(
            io::stdout(),
            SetForegroundColor(Color::Red),
            SetAttribute(Attribute::Bold),
            Print("[Error]"),
            SetAttribute(Attribute::Reset),
            SetForegroundColor(Color::Red),
            Print(" Tasks require either a script or dependencies\n"),
            ResetColor,
        );
        return Err(CommandError {
            message: format!("Task \"{task_name}\" has no script or dependencies."),
        });
    }

    Ok(task)
}

fn run_task(config: &Config, task_name: &str) -> Result<(), CommandError> {
    let _ = get_and_validate_task(config, task_name)?;

    let chain = config.get_execution_chain(task_name);

    for task in chain {
        let _ = execute!(
            io::stdout(),
            SetForegroundColor(Color::Blue),
            Print("Running task \""),
            SetAttribute(Attribute::Bold),
            Print(&task.name),
            SetAttribute(Attribute::Reset),
            SetForegroundColor(Color::Blue),
            Print("\"\n"),
            ResetColor,
        );
        task.execute()?;
    }
    Ok(())
}

fn display_task(
    config: &Config,
    task_name: &str,
) -> Result<(), CommandError> {
    let _ = get_and_validate_task(config, task_name)?;

    let chain = config.get_execution_chain(task_name);

    let mut stdout = io::stdout();

    for task in chain {
        let _ = execute!(
            stdout,
            SetForegroundColor(Color::Blue),
            Print("Would run task \""),
            SetAttribute(Attribute::Bold),
            Print(&task.name),
            SetAttribute(Attribute::Reset),
            SetForegroundColor(Color::Blue),
            Print("\"\n"),
            ResetColor,
        );

        // if cli_config.is_verbose() {
        //     print_options(&mut stdout, &task.options);
        // }

        if let Some(script) = &task.script {
            for cmd in script {
                let _ = queue!(
                    stdout,
                    SetForegroundColor(Color::Cyan),
                    Print("Would execute \""),
                    SetAttribute(Attribute::Bold),
                    Print(&cmd),
                    SetAttribute(Attribute::Reset),
                    Print("\" \n"),
                    ResetColor
                );
            }
        }

        let _ = queue!(stdout, Print("\n"), ResetColor,);
        stdout.flush().unwrap();
    }

    Ok(())
}
