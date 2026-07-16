use std::{env, io::{self, Write}, path::PathBuf};

use clap::{Arg, ArgAction, Command, builder::styling, crate_authors, crate_version};
use crossterm::{
    execute, queue,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor},
};
use log::warn;

use machfile::{Builder, Config, MachConfig, utils::CommandError};

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
pub fn build_cli_commands(config: &Option<&Config>) -> Command {
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

    // Environment arguments
    app = app
        .arg(
            Arg::new("disable_env_file")
                .action(ArgAction::SetTrue)
                .long("no-env-file")
                .help("Disable env file parsing"),
        )
        .arg(
            Arg::new("env_file")
                .action(ArgAction::Set)
                .conflicts_with("disable_env_file")
                .long("env-file")
                .help("Override env file path"),
        );

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
    let mut builder = if let Some(c) = config_override {
        Builder::new(PathBuf::from(c))
    } else {
        Builder::from_current_dir()
    };

    let config = match builder.get_config() {
        Err(e) => {
            match e {
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
                    warn!("Failed to load/parse configuration: {e}");
                }
            }
            None
        }
        Ok(conf) => Some(conf),
    };

    let matches = build_cli_commands(&config).get_matches();

    if matches.get_flag("disable_env_file") {
        println!("Disable env file: {}", matches.get_flag("disable_env_file"));
        builder = builder.disable_env_file();
    }
    if let Some(env_file) = matches.get_one::<String>("env_file") {
        builder = builder.with_env_file(PathBuf::from(env_file));
    }

    let mach_config = builder.build();

    match matches.subcommand() {
        None => {
            if matches.get_flag("show_config") {
                let conf = mach_config.unwrap();
                print_config(&conf.config);
            }
            Ok(())
        }
        #[cfg(feature = "complete")]
        Some(("setup_complete", args)) => handle_setup_complete(args),
        Some((cmd, args)) => {
            if mach_config.is_err() {
                println!("Failed to parse configuration");
                return Err(CommandError {
                    message: "failed to parse configuration".to_owned(),
                });
            }

            let conf = mach_config.unwrap();

            match (cmd, args) {
                #[cfg(feature = "complete")]
                ("auto_complete", args) => handle_auto_complete(args),
                (name, _) => {
                    if matches.get_flag("dry_run") {
                        display_task(&conf.config, name)
                    } else if matches.get_flag("show_config") {
                        print_task_config(&conf.config, cmd);
                        Ok(())
                    } else {
                        run_task(&conf, name)
                    }
                }
            }
        }
    }
}

fn run_task(config: &MachConfig, task_name: &str) -> Result<(), CommandError> {
    let task = config.config.tasks.get(task_name).unwrap();
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
    let chain = config.get_task_execution_chain(task_name);

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
