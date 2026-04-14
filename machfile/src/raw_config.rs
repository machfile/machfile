use std::{
    collections::{HashMap, VecDeque},
    path::Path,
};

use log::error;
use serde::Deserialize;

use super::{
    ConfigParseError,
    config::{ScriptCommand, Task, TaskOptions},
};

#[derive(Debug, Deserialize)]
pub struct RawConfig {
    #[serde(flatten)]
    pub tasks: HashMap<String, RawTaskConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RawTaskConfig {
    script: Option<String>,
    desc: Option<String>,
    deps: Option<Vec<String>>,
    options: Option<RawTaskOptions>,
}

impl RawTaskConfig {
    pub fn parse(self, name: &str, path: &Path) -> Result<Task, ConfigParseError> {
        let options = self.options.unwrap_or_default().parse(path)?;

        let mut task = Task {
            desc: self.desc,
            deps: self.deps.unwrap_or_default(),
            options,
            ..Default::default()
        };

        if let Some(commands) = self.script {
            let mut task_commands: Vec<ScriptCommand> = Vec::new();
            let lines: Vec<&str> = commands
                .lines()
                .filter_map(|s| {
                    let trimmed = s.trim();

                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed)
                    }
                })
                .collect();

            if lines.is_empty() {
                error!("Task {name} has declared an empty script");
                return Err(ConfigParseError::InvalidTaskDefinition(
                    "Missing script".to_owned(),
                ));
            }

            for command in lines {
                let mut parts = command
                    .split_whitespace()
                    .map(str::trim)
                    .collect::<VecDeque<_>>();

                if parts.is_empty() {
                    // empty line gets skipped
                    continue;
                }

                task_commands.push(ScriptCommand {
                    command: parts.pop_front().unwrap().to_string(),
                    args: parts.iter().map(|&s| s.to_string()).collect(),
                });
            }

            task.script = Some(task_commands);
        }

        Ok(task)
    }
}

#[derive(Debug, Deserialize, Default, Clone)]
struct RawTaskOptions {
    working_directory: Option<String>,
    environment: Option<HashMap<String, String>>,
}

impl RawTaskOptions {
    fn parse(self, path: &Path) -> Result<TaskOptions, ConfigParseError> {
        let mut working_directory = path.to_path_buf();
        if let Some(work_dir) = self.working_directory {
            working_directory.push(work_dir);
            if !working_directory.is_dir() {
                return Err(ConfigParseError::InvalidTaskDefinition(
                    "Invalid working_directory".into(),
                ));
            }
        }

        let environment: HashMap<String, String> = self.environment.unwrap_or_default();

        Ok(TaskOptions {
            working_directory,
            environment,
        })
    }
}
