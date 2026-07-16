use std::{
    env,
    io::{self, Write},
    path::PathBuf,
};

use clap::{Arg, ArgAction, ArgMatches, Command, builder::styling, crate_authors, crate_version};
use crossterm::{
    execute, queue,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor},
};
use log::{debug, warn};

use machfile::{Builder, Config, MachConfig, config::ScriptCommand, utils::CommandError};

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

    // Global arguments and flags
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
        )
        .arg(
            Arg::new("verbose")
                .action(ArgAction::SetTrue)
                .global(true)
                .long("verbose")
                .short('v')
                .help("Show additional information"),
        );

    if let Some(conf) = config {
        for (name, command) in &conf.tasks {
            let mut sub = Command::new(name);
            if let Some(desc) = &command.desc {
                sub = sub.about(desc);
            }

            // Add dry-run only to commands
            sub = sub.arg(
                Arg::new("dry_run")
                    .action(ArgAction::SetTrue)
                    .long("dry-run")
                    .help("Displays each script that would be executed without executing them"),
            );

            if command.options.allow_args {
                sub = sub.arg(
                    Arg::new("args")
                        .num_args(0..)
                        .help("Arguments to pass to the first command of the script")
                        .trailing_var_arg(true),
                );
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
        debug!("Disable env file: {}", matches.get_flag("disable_env_file"));
        builder = builder.disable_env_file();
    }
    if let Some(env_file) = matches.get_one::<String>("env_file") {
        builder = builder.with_env_file(PathBuf::from(env_file));
    }

    if matches.get_flag("verbose") {
        builder = builder.enable_verbose();
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
                (name, args) => {
                    if matches.get_flag("show_config") {
                        print_task_config(&conf.config, cmd);
                        Ok(())
                    } else {
                        run_task(&conf, name, args, args.get_flag("dry_run"))
                    }
                }
            }
        }
    }
}

fn run_task(
    config: &MachConfig,
    task_name: &str,
    args: &ArgMatches,
    dry_run: bool,
) -> Result<(), CommandError> {
    let task = config.config.tasks.get(task_name).unwrap();

    let mut stdout = io::stdout();

    if task.script.is_none() && task.deps.is_empty() {
        let _ = execute!(
            stdout,
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

    let mut script_args: Option<Vec<String>> = if task.options.allow_args {
        args.get_many::<String>("args")
            .map(|a| a.map(String::from).collect())
    } else {
        None
    };

    let chain = config.get_task_execution_chain(task_name);
    let last_task_idx = chain.len() - 1;

    for (ti, task) in chain.iter().enumerate() {
        let _ = queue!(
            stdout,
            SetForegroundColor(Color::Blue),
            Print(if dry_run {
                "Would run task \""
            } else {
                "Running task \""
            }),
            SetAttribute(Attribute::Bold),
            Print(&task.name),
            SetAttribute(Attribute::Reset),
            SetForegroundColor(Color::Blue),
            Print("\"\n"),
            ResetColor,
        );

        if config.is_verbose {
            print_options(&mut stdout, &task.options);
        }

        if dry_run {
            if let Some(script) = &task.script {
                for (ci, cmd) in script.iter().enumerate() {
                    if ti == last_task_idx
                        && ci == 0
                        && let Some(ref mut args) = script_args
                    {
                        let mut expanded_comand = cmd.clone();
                        expanded_comand.args.append(args);
                        queue_cmd_print(&mut stdout, &expanded_comand);
                    } else {
                        queue_cmd_print(&mut stdout, cmd);
                    }
                }
            }

            let _ = queue!(stdout, Print("\n"), ResetColor,);
        } else {
            // Only flushing stdout here improves rendering on dry-runs
            stdout.flush().unwrap();
            if ti == last_task_idx {
                task.execute_with_args(script_args.clone())?;
            } else {
                task.execute()?;
            }
        }
    }

    stdout.flush().unwrap();

    Ok(())
}

fn queue_cmd_print(stdout: &mut io::Stdout, cmd: &ScriptCommand) {
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
