use std::{
    io,
    process::{Command, Stdio},
};

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
            let mut proc = Command::new(command.command.clone())
                .args(command.args.clone())
                .stdout(io::stdout())
                .stderr(Stdio::inherit())
                .spawn()
                .unwrap();

            if let Ok(code) = proc.wait() {
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
