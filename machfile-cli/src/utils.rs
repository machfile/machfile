use std::io::{Write, stdout};

use crossterm::{
    queue,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor},
};
use machfile::Config;

pub fn print_config(config: &Config) {
    let mut stdout = stdout();

    let _ = queue!(
        stdout,
        SetForegroundColor(Color::Blue),
        Print("Loaded configuration from: "),
        SetAttribute(Attribute::Bold),
        Print(config.path.display()),
        Print("\n\n"),
        ResetColor,
    );

    let _ = queue!(stdout, SetAttribute(Attribute::Bold), Print("Tasks:\n\n"),);

    let tasks = &mut config.tasks.iter().peekable();
    while let Some((name, task)) = tasks.next() {
        let _ = queue!(
            stdout,
            SetAttribute(Attribute::Bold),
            Print(name),
            ResetColor,
            Print("\n"),
        );

        if let Some(desc) = &task.desc {
            let _ = queue!(
                stdout,
                SetAttribute(Attribute::Italic),
                Print("\t"),
                Print(desc),
                ResetColor,
                Print("\n"),
            );
        }

        if !&task.deps.is_empty() {
            let _ = queue!(stdout, Print("\tDepends on: "),);

            let deps = &mut task.deps.iter().peekable();
            while let Some(dep) = deps.next() {
                let _ = queue!(
                    stdout,
                    SetAttribute(Attribute::Bold),
                    Print(dep),
                    SetAttribute(Attribute::NoBold),
                );

                if deps.peek().is_some() {
                    let _ = queue!(stdout, Print(", "),);
                }
            }

            let _ = queue!(stdout, Print("\n"),);
        }

        if tasks.peek().is_some() {
            let _ = queue!(stdout, Print("\n"),);
        }
    }

    let _ = stdout.flush();
}
