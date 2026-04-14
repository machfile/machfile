use std::{
    collections::{HashMap, VecDeque},
    ffi::OsString,
    fs::read_to_string,
    path::PathBuf,
    str::FromStr,
};

use log::error;
use serde::Deserialize;

#[derive(Debug, PartialEq)]
pub enum ConfigParseError {
    NoConfigFile,
    CorruptConfigFile,
    EmptyConfig,
    InvalidTaskDefinition,
}

#[derive(Debug)]
pub struct Config {
    pub tasks: HashMap<String, TaskConfig>,
}

#[derive(Debug)]
pub struct TaskConfig {
    pub script: Option<Vec<ScriptCommand>>,
    pub desc: Option<String>,
    pub deps: Option<Vec<String>>,
}

#[derive(Debug)]
pub struct ScriptCommand {
    pub command: String,
    pub args: Vec<String>,
}

impl Config {
    fn from_raw(raw: RawConfig) -> Result<Self, ConfigParseError> {
        let mut config = Self {
            tasks: HashMap::new(),
        };

        for (name, raw_task) in raw.tasks {
            let mut task = TaskConfig {
                script: None,
                desc: raw_task.desc,
                deps: raw_task.deps,
            };

            if let Some(commands) = raw_task.script {
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
                    return Err(ConfigParseError::InvalidTaskDefinition);
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

            config.tasks.insert(name, task);
        }

        Ok(config)
    }
}

#[derive(Debug, Deserialize)]
struct RawConfig {
    #[serde(flatten)]
    tasks: HashMap<String, RawTaskConfig>,
}

#[derive(Debug, Deserialize)]
struct RawTaskConfig {
    script: Option<String>,
    desc: Option<String>,
    deps: Option<Vec<String>>,
}

pub fn load_config(config_override: Option<OsString>) -> Result<Config, ConfigParseError> {
    let config_file = if let Some(conf_override) = config_override {
        let path = PathBuf::from(conf_override);
        if path.is_file() {
            println!("Loading override from {}", path.display());
            Some(path)
        } else {
            println!("Invalid config file override: {}", path.display());
            None
        }
    } else {
        get_mach_file_path()
    };

    if config_file.is_none() {
        return Err(ConfigParseError::NoConfigFile);
    }
    // TODO: read file content safely
    let Ok(file_content) = read_to_string(config_file.unwrap()) else {
        return Err(ConfigParseError::CorruptConfigFile);
    };

    parse_config(&file_content)
}

fn get_mach_file_path() -> Option<PathBuf> {
    // TODO: travers up to git boundary if inside git repo
    let path = PathBuf::from_str("./mach.toml").unwrap();

    if path.is_file() { Some(path) } else { None }
}

fn parse_config(config_str: &str) -> Result<Config, ConfigParseError> {
    let Ok(config) = toml::from_str::<RawConfig>(config_str) else {
        return Err(ConfigParseError::InvalidTaskDefinition);
    };

    if config.tasks.is_empty() {
        return Err(ConfigParseError::EmptyConfig);
    }

    Config::from_raw(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_config_parses() {
        let conf = "[task]\ncommand = \"hello world\"";

        assert!(parse_config(conf).is_ok());
    }

    #[test]
    fn empty_config_is_invalid() {
        let result = parse_config("");

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ConfigParseError::EmptyConfig);
    }
}
