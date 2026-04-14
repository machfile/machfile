//! Runs [tasks](`TaskConfig`)

use std::{error::Error, ffi::OsStr, fmt, io, process::Command};

use crossterm::{
    execute,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor},
};

use crate::parser::{TaskConfig, get_config};

#[derive(Debug)]
pub struct CommandError {
    pub message: String,
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Command encountered an unexpected error: {}",
            self.message
        )
    }
}

impl Error for CommandError {}

/// Execute defined commands
///
/// # Errors
///
/// Returns a [`CommandError`] when the given task was not found or a executed command fails to
/// spawn or returns an error.
///
/// # Panics
///
/// Panics if no configuration has been loaded
pub fn execute(command_name: &str) -> Result<(), CommandError> {
    let config = get_config().expect("No configuration was loaded");
    let Some(task): Option<&TaskConfig> = config.tasks.get(command_name) else {
        return Err(CommandError {
            message: "Task {command_name} not found".to_owned(),
        });
    };

    if task.script.is_none() && task.deps.is_none() {
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
        unimplemented!();
    }

    if let Some(deps) = &task.deps {
        for dep in deps {
            if execute(dep).is_err() {
                return Err(CommandError {
                    message: "Dependency failed".to_owned(),
                });
            }
        }
    }

    let _ = execute!(
        io::stdout(),
        SetForegroundColor(Color::Blue),
        Print("Running task \""),
        SetAttribute(Attribute::Bold),
        Print(command_name),
        SetAttribute(Attribute::Reset),
        SetForegroundColor(Color::Blue),
        Print("\"\n"),
        ResetColor,
    );

    if let Some(script) = &task.script {
        for command in script {
            let mut sys_command = command.create_sys_command();

            let _ = execute!(
                io::stdout(),
                SetForegroundColor(Color::Grey),
                SetAttribute(Attribute::Dim),
                Print("  "),
                Print(format_command(&sys_command)),
                SetAttribute(Attribute::Reset),
                SetForegroundColor(Color::Blue),
                Print("\n"),
                ResetColor,
            );

            for (key, value) in &task.options.environment {
                sys_command.env(key, value);
            }
            sys_command.current_dir(&task.options.working_directory);

            let proc = sys_command.spawn();

            if let Ok(mut proc) = proc
                && let Ok(code) = proc.wait()
            {
                if code.success() {
                    continue;
                }

                return Err(CommandError {
                    message: format!("Command failed with status {code}"),
                });
            }

            return Err(CommandError {
                message: "Command failed to spawn".to_owned(),
            });
        }
    }

    Ok(())
}

fn format_command(cmd: &Command) -> String {
    format!(
        "{} {}",
        cmd.get_program().display(),
        cmd.get_args()
            .collect::<Vec<&OsStr>>()
            .join(OsStr::new(" "))
            .into_string()
            .unwrap()
    )
    .trim()
    .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_command_formats_cmd_without_args_correctly() {
        let cmd = Command::new("asdf");

        assert_eq!(format_command(&cmd), String::from("asdf"));
    }

    #[test]
    fn format_command_formats_cmd_with_string_arg_correctly() {
        let mut cmd = Command::new("asdf");
        cmd.arg("exe");

        assert_eq!(format_command(&cmd), String::from("asdf exe"));
    }

    #[test]
    fn format_command_formats_cmd_with_number_arg_correctly() {
        let mut cmd = Command::new("asdf");
        cmd.arg("2");

        assert_eq!(format_command(&cmd), String::from("asdf 2"));
    }

    #[test]
    fn format_command_formats_cmd_with_args_correctly() {
        let mut cmd = Command::new("asdf");
        cmd.arg("2");
        cmd.arg("exe");

        assert_eq!(format_command(&cmd), String::from("asdf 2 exe"));
    }
}
