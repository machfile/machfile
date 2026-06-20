use std::io::{Stdout, Write, stdout};

use crossterm::{
    queue,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor},
};

use machfile::{Config, config::TaskOptions};

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
            Print('\n'),
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
            SetAttribute(Attribute::Reset),
            Print('\n'),
        );
        for line in script {
            let _ = queue!(stdout, Print('\t'), Print(line), Print('\n'),);
        }

        let _ = queue!(stdout, Print('\n'));
    }

    if !task.deps.is_empty() {
        print_dependencies(&mut stdout, config, &task.deps);
    }

    print_options(&mut stdout, &task.options);

    let _ = stdout.flush();
}

fn print_dependencies(stdout: &mut Stdout, config: &Config, deps: &Vec<String>) {
    let _ = queue!(
        stdout,
        Print('\n'),
        SetAttribute(Attribute::Bold),
        Print("Depends on:"),
        SetAttribute(Attribute::Reset),
        Print('\n'),
    );

    for line in deps {
        let dep = config.tasks.get(line).unwrap();

        for sub_line in &dep.deps {
            let sub_dep = config.tasks.get(sub_line).unwrap();
            let _ = queue!(
                stdout,
                SetForegroundColor(Color::Grey),
                Print("  - "),
                Print(sub_line),
            );
            if let Some(desc) = &sub_dep.desc {
                let _ = queue!(
                    stdout,
                    Print('\t'),
                    SetAttribute(Attribute::Italic),
                    Print(desc),
                    SetAttribute(Attribute::NoItalic),
                );
            }

            let _ = queue!(stdout, SetAttribute(Attribute::Reset), Print('\n'));
        }

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

    let _ = queue!(stdout, Print('\n'));
}

pub fn print_options(stdout: &mut Stdout, opts: &TaskOptions) {
    let _ = queue!(
        stdout,
        Print("Working directory\n\t"),
        SetAttribute(Attribute::Bold),
        Print(opts.working_directory.display()),
        SetAttribute(Attribute::Reset),
        Print('\n'),
    );

    if !opts.environment.is_empty() {
        let _ = queue!(
            stdout,
            Print('\n'),
            SetAttribute(Attribute::Bold),
            Print("Environment variables"),
            SetAttribute(Attribute::Reset),
            Print('\n'),
        );

        for (key, val) in opts.environment.clone().into_iter() {
            let _ = queue!(
                stdout,
                Print('\t'),
                SetAttribute(Attribute::Bold),
                Print(key),
                SetAttribute(Attribute::Reset),
                Print('\t'),
                Print(val),
                Print('\n'),
            );
        }
    }

    let _ = queue!(stdout, Print('\n'));
}
