use std::{error::Error, fmt, io};

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
pub fn execute(command_name: &str) -> Result<(), CommandError> {
    let config = get_config();
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
                Print(format!("  {sys_command:?}")),
                SetAttribute(Attribute::Reset),
                SetForegroundColor(Color::Blue),
                Print("\n"),
                ResetColor,
            );

            let mut command_path = config.path.clone();

            // If a working directory is set for the task, set it
            if let Some(opts) = &task.options {
                if let Some(work_dir) = &opts.working_directory {
                    command_path.push(work_dir);
                }
                for (key, value) in &opts.environment {
                    sys_command.env(key, value);
                }
            }

            sys_command.current_dir(command_path);

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
