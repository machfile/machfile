use std::{collections::VecDeque, io, process::Command};

use crossterm::{
    execute,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor},
};

use crate::parser::CommandConfig;

/// Execute defined commands
pub fn execute(command_name: &str, config: &CommandConfig) {
    let Some(command_string) = &config.command else {
        let _ = execute!(
            io::stdout(),
            SetForegroundColor(Color::Red),
            SetAttribute(Attribute::Bold),
            Print("[Error]"),
            SetAttribute(Attribute::Reset),
            SetForegroundColor(Color::Red),
            Print(" Task without command are not supported yet\n"),
            ResetColor,
        );
        unimplemented!();
    };

    if let Some(deps) = &config.deps {
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

    let mut command_parts: VecDeque<&str> =
        VecDeque::from(command_string.split_whitespace().collect::<Vec<&str>>());
    let output = Command::new(command_parts.pop_front().unwrap())
        .args(command_parts)
        .stdout(io::stdout())
        .output();

    println!("\n===============\nCommand output:\n{output:#?}");
}
