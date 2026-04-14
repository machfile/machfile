use std::{
    collections::{HashMap, VecDeque},
    env,
    ffi::OsString,
    fs::read_to_string,
    path::PathBuf,
    process::Command,
    sync::OnceLock,
};

use config_finder::get_mach_file_path;
use log::error;
use serde::Deserialize;

mod config_finder;

static APP_CONFIG: OnceLock<Config> = OnceLock::new();

#[derive(Debug, PartialEq)]
pub enum ConfigParseError {
    NoConfigFile,
    NoConfigFileInRepo,
    CorruptConfigFile,
    EmptyConfig,
    InvalidTaskDefinition(String),
}

#[derive(Debug, Clone)]
pub struct Config {
    pub path: PathBuf,
    pub tasks: HashMap<String, TaskConfig>,
}

impl Config {
    fn from_raw(raw: RawConfig, config_path: PathBuf) -> Result<Self, ConfigParseError> {
        let path = config_path.parent().unwrap().to_path_buf();
        let mut config = Self {
            path,
            tasks: HashMap::new(),
        };

        for (name, raw_task) in raw.tasks {
            let task = raw_task.parse(&name)?;
            config.tasks.insert(name, task);
        }

        Ok(config)
    }
}

#[derive(Debug, Clone)]
pub struct TaskConfig {
    pub script: Option<Vec<ScriptCommand>>,
    pub desc: Option<String>,
    pub deps: Option<Vec<String>>,
    pub options: Option<TaskOptions>,
}

#[derive(Debug, Clone)]
pub struct TaskOptions {
    pub working_directory: Option<PathBuf>,
    pub environment: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct ScriptCommand {
    pub command: String,
    pub args: Vec<String>,
}

impl ScriptCommand {
    pub fn create_sys_command(&self) -> Command {
        let mut command = Command::new(self.command.clone());
        command.args(self.args.clone());
        command
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
    options: Option<RawTaskOptions>,
}

impl RawTaskConfig {
    fn parse(self, name: &str) -> Result<TaskConfig, ConfigParseError> {
        let options = if let Some(raw) = self.options {
            let opts = raw.parse()?;
            Some(opts)
        } else {
            None
        };

        let mut task = TaskConfig {
            script: None,
            desc: self.desc,
            deps: self.deps,
            options,
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

#[derive(Debug, Deserialize)]
struct RawTaskOptions {
    working_directory: Option<String>,
    environment: Option<HashMap<String, String>>,
}

impl RawTaskOptions {
    fn parse(self) -> Result<TaskOptions, ConfigParseError> {
        let working_directory = if let Some(work_dir) = self.working_directory {
            let path = PathBuf::from(work_dir);
            if path.is_dir() {
                Some(path)
            } else {
                return Err(ConfigParseError::InvalidTaskDefinition(
                    "Invalid working_directory".into(),
                ));
            }
        } else {
            None
        };

        let environment: HashMap<String, String> = self.environment.unwrap_or_default();

        Ok(TaskOptions {
            working_directory,
            environment,
        })
    }
}

pub fn load_config(config_override: Option<OsString>) -> Result<(), ConfigParseError> {
    let config_file = if let Some(conf_override) = config_override {
        let path = PathBuf::from(conf_override);
        if path.is_file() {
            println!("Loading override from {}", path.display());
            Ok(path)
        } else {
            println!("Invalid config file override: {}", path.display());
            Err(ConfigParseError::NoConfigFile)
        }
    } else {
        get_mach_file_path(&env::current_dir().expect("Failed to get current working dir"))
    }?;

    // TODO: read file content safely
    let Ok(file_content) = read_to_string(config_file.clone()) else {
        return Err(ConfigParseError::CorruptConfigFile);
    };

    match parse_config(&file_content, config_file) {
        Err(err) => Err(err),
        Ok(conf) => {
            APP_CONFIG
                .set(conf)
                .expect("Race condition when setting config");
            Ok(())
        }
    }
}

pub fn get_config() -> &'static Config {
    APP_CONFIG.get().unwrap()
}

fn parse_config(config_str: &str, config_path: PathBuf) -> Result<Config, ConfigParseError> {
    let Ok(config) = toml::from_str::<RawConfig>(config_str) else {
        return Err(ConfigParseError::InvalidTaskDefinition(
            "Invalid TOML".to_owned(),
        ));
    };

    if config.tasks.is_empty() {
        return Err(ConfigParseError::EmptyConfig);
    }

    Config::from_raw(config, config_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_config_parses() {
        let conf = "[task]\ncommand = \"hello world\"";

        assert!(parse_config(conf, PathBuf::from("/tmp/mach.toml")).is_ok());
    }

    #[test]
    fn empty_config_is_invalid() {
        let result = parse_config("", PathBuf::from("/tmp/mach.toml"));

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ConfigParseError::EmptyConfig);
    }
}
