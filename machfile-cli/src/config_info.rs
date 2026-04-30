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

    let _ = queue!(
        stdout,
        SetAttribute(Attribute::Bold),
        Print("Available tasks:\n\n"),
    );

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

        if tasks.peek().is_some() {
            let _ = queue!(stdout, Print("\n"),);
        }
    }

    let _ = stdout.flush();
}

pub fn print_task_config(config: &Config, task_name: &str) {
    let mut stdout = stdout();

    let Some(task) = config.tasks.get(task_name) else {
        let _ = queue!(
            stdout,
            SetForegroundColor(Color::Red),
            Print("Somehow managed to print the config of a non-existent task. How???"),
            ResetColor,
        );
        let _ = stdout.flush();
        return;
    };

    let _ = queue!(
        stdout,
        Print('\n'),
        SetAttribute(Attribute::Bold),
        SetForegroundColor(Color::Blue),
        Print(task_name),
        SetAttribute(Attribute::NoBold),
        ResetColor,
        Print('\n'),
    );

    if let Some(desc) = &task.desc {
        let _ = queue!(
            stdout,
            Print("\n\t"),
            SetAttribute(Attribute::Italic),
            Print(desc),
            SetAttribute(Attribute::NoItalic),
            Print('\n'),
        );
    }

    if let Some(script) = &task.script {
        let _ = queue!(
            stdout,
            Print('\n'),
            SetAttribute(Attribute::Bold),
            Print("Script"),
            SetAttribute(Attribute::NoBold),
            Print('\n'),
        );
        for line in script {
            let _ = queue!(stdout, Print('\t'), Print(line), Print('\n'),);
        }
    }

    if !task.deps.is_empty() {
        let _ = queue!(
            stdout,
            Print('\n'),
            SetAttribute(Attribute::Bold),
            Print("Depends on:"),
            SetAttribute(Attribute::NoBold),
            Print('\n'),
        );
        for line in &task.deps {
            let dep = config.tasks.get(line).unwrap();
            let _ = queue!(stdout, Print("  - "), Print(line));

            if let Some(desc) = &dep.desc {
                let _ = queue!(
                    stdout,
                    Print('\t'),
                    SetAttribute(Attribute::Italic),
                    Print(desc),
                    SetAttribute(Attribute::NoItalic),
                );
            }

            let _ = queue!(stdout, Print('\n'));
        }
    }

    let _ = stdout.flush();
}
