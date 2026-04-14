use std::io;

use crossterm::{
    execute,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor},
};
use log::error;

use crate::parser::TaskConfig;

/// Execute defined commands
pub fn execute(command_name: &str, task: &TaskConfig) {
    if task.script.is_none() {
        let _ = execute!(
            io::stdout(),
            SetForegroundColor(Color::Red),
            SetAttribute(Attribute::Bold),
            Print("[Error]"),
            SetAttribute(Attribute::Reset),
            SetForegroundColor(Color::Red),
            Print(" Tasks without script are not supported yet\n"),
            ResetColor,
        );
        unimplemented!();
    }

    if let Some(deps) = &task.deps {
        let _ = execute!(
            io::stdout(),
            SetForegroundColor(Color::Yellow),
            SetAttribute(Attribute::Bold),
            Print("[Warning]"),
            SetAttribute(Attribute::Reset),
            SetForegroundColor(Color::Yellow),
            Print(" Task dependencies are not supported yet\nDetected deps: "),
            Print(format!("{deps:#?}\n\n")),
            ResetColor,
        );
    }

    let _ = execute!(
        io::stdout(),
        SetForegroundColor(Color::Blue),
        Print("Running task \""),
        SetAttribute(Attribute::Bold),
        Print(command_name),
        SetAttribute(Attribute::Reset),
        SetForegroundColor(Color::Blue),
        Print("\"\n\n"),
        ResetColor,
    );

    if let Some(script) = &task.script {
        for command in script {
            let mut sys_command = command.create_sys_command();

            // If a working directory is set for the task, set it
            if let Some(opts) = &task.options
                && let Some(work_dir) = &opts.working_directory
            {
                sys_command.current_dir(work_dir);
            }

            let proc = sys_command.spawn();

            if let Ok(mut proc) = proc
                && let Ok(code) = proc.wait()
            {
                if code.success() {
                    continue;
                }

                error!("Command failed with status {code}");
                return;
            }

            error!("Command failed to spawn");
            return;
        }
    }
}
