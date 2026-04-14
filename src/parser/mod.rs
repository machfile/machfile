//! Mach configuration file parsing
//!
//! This module includes the structs related to the mach configuration and the functions that find
//! and parse the supported configuration files

use std::{
    collections::{HashMap, VecDeque},
    env,
    ffi::OsString,
    fs::read_to_string,
    path::{Path, PathBuf},
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

/// The main struct holding all [tasks](`TaskConfig`)
#[derive(Debug, Clone)]
pub struct Config {
    pub path: PathBuf,
    pub tasks: HashMap<String, TaskConfig>,
}

impl Config {
    fn from_raw(raw: RawConfig, config_path: &Path) -> Result<Self, ConfigParseError> {
        let path = config_path.parent().unwrap().to_path_buf();
        let mut config = Self {
            path: path.clone(),
            tasks: HashMap::new(),
        };

        for (name, raw_task) in raw.tasks {
            let task = raw_task.parse(&name, &path)?;
            config.tasks.insert(name, task);
        }

        Ok(config)
    }
}

/// The individual task
///
/// Composed of the `TaskOptions` and optionally a description, script and task dependencies
#[derive(Debug, Clone)]
pub struct TaskConfig {
    pub options: TaskOptions,
    /// Vector of `ScriptCommand` that are run in order when the task is executed
    pub script: Option<Vec<ScriptCommand>>,
    /// The task description shown in the help message
    pub desc: Option<String>,
    /// Vector of the names of other tasks. The dependencies are executed in order before the tasks
    /// script is run
    pub deps: Option<Vec<String>>,
}

/// Configuration options for the environment in which the [`ScriptCommand`]s will be run
#[derive(Debug, Clone)]
pub struct TaskOptions {
    /// Working directory in which all commands will be executed
    pub working_directory: PathBuf,
    /// Environmental variables that will be added to the environment in which the commands will be
    /// executed
    pub environment: HashMap<String, String>,
}

/// Individual command that forms a [task](`TaskConfig`)
#[derive(Debug, Clone)]
pub struct ScriptCommand {
    /// Name of the executable that will be called
    pub command: String,
    /// Vector of the arguments that will be provided to the command
    pub args: Vec<String>,
}

impl ScriptCommand {
    #[must_use]
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
    fn parse(self, name: &str, path: &Path) -> Result<TaskConfig, ConfigParseError> {
        let options = self.options.unwrap_or_default().parse(path)?;

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

#[derive(Debug, Deserialize, Default)]
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

/// Load a mach configuration into memory
///
/// If an override is provided, the provided override is loaded. Otherwise, the function tries to
/// find a configuration file in the current directory, and in the case of the working directory
/// being inside a git repository, a configuration file will be searched upwards until the git
/// boundary.
///
/// # Errors
///
/// - Returns [`ConfigParseError::NoConfigFile`] if an invalid override is provided
/// - Returns [`ConfigParseError::CorruptConfigFile`] if the text content of the file can't be
///   loaded
/// - Bubbles up any [`ConfigParseError`] from [`parse_config`]
///
/// # Panics
///
/// This function panics if it can't detect the current working directory
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

    match parse_config(&file_content, &config_file) {
        Err(err) => Err(err),
        Ok(conf) => {
            APP_CONFIG
                .set(conf)
                .expect("Race condition when setting config");
            Ok(())
        }
    }
}

/// Gets the configuration from memory
pub fn get_config() -> Option<&'static Config> {
    APP_CONFIG.get()
}

/// Tries to parse the provided string as `Config`
///
/// # Errors
///
/// - Returns [`ConfigParseError::InvalidTaskDefinition`] if the TOML can't be parsed
/// - Returns [`ConfigParseError::EmptyConfig`] if the TOML does not contain any task
pub fn parse_config(config_str: &str, config_path: &Path) -> Result<Config, ConfigParseError> {
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
    fn parse_config_succeeds_on_valid_config() {
        let conf = "[task]\nscript = \"hello world\"";

        assert!(parse_config(conf, &PathBuf::from("/tmp/mach.toml")).is_ok());
    }

    #[test]
    fn parse_config_fails_on_empty_config() {
        let result = parse_config("", &PathBuf::from("/tmp/mach.toml"));

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ConfigParseError::EmptyConfig);
    }

    #[test]
    fn parse_config_correctly_loads_simple_task_command() {
        let conf = "[task]\nscript = \"pwd\"";

        let config = {
            let conf = parse_config(conf, &PathBuf::from("/tmp/mach.toml"));
            assert!(conf.is_ok());

            conf.unwrap()
        };

        let task_opt = config.tasks.get("task");
        assert!(task_opt.is_some());
        let task = task_opt.unwrap();

        assert!(task.script.is_some());
        let script = task.clone().script.unwrap();

        assert_eq!(script.len(), 1);
        assert_eq!(script[0].command, "pwd");
    }

    #[test]
    fn parse_config_correctly_loads_simple_task_args() {
        let conf = "[task]\nscript = \"echo 'test'\"";

        let config = {
            let conf = parse_config(conf, &PathBuf::from("/tmp/mach.toml"));
            assert!(conf.is_ok());

            conf.unwrap()
        };

        let task_opt = config.tasks.get("task");
        assert!(task_opt.is_some());
        let task = task_opt.unwrap();

        assert!(task.script.is_some());
        let script = task.clone().script.unwrap();

        assert_eq!(script.len(), 1);
        assert_eq!(script[0].args.len(), 1);
    }

    #[test]
    fn parse_config_correctly_sets_working_directory_without_override() {
        let conf = "[task]\nscript = \"pwd\"";

        let config = {
            let conf = parse_config(conf, &PathBuf::from("/tmp/mach.toml"));
            assert!(conf.is_ok());

            conf.unwrap()
        };

        let task_opt = config.tasks.get("task");
        assert!(task_opt.is_some());
        let task = task_opt.unwrap();

        assert_eq!(task.options.working_directory, PathBuf::from("/tmp"));
    }

    #[test]
    fn parse_config_correctly_sets_working_directory_with_relative_override() {
        let conf = "[task]\nscript = \"pwd\"\noptions.working_directory = \"src\"";

        let config = {
            let conf = parse_config(conf, &PathBuf::from("./mach.toml"));
            assert!(conf.is_ok());

            conf.unwrap()
        };

        let task_opt = config.tasks.get("task");
        assert!(task_opt.is_some());
        let task = task_opt.unwrap();

        assert_eq!(task.options.working_directory, PathBuf::from("./src"));
    }

    #[test]
    fn parse_config_correctly_sets_working_directory_with_absolute_override() {
        let conf = "[task]\nscript = \"pwd\"\noptions.working_directory = \"/tmp\"";

        let config = {
            let conf = parse_config(conf, &PathBuf::from("./mach.toml"));
            assert!(conf.is_ok());

            conf.unwrap()
        };

        let task_opt = config.tasks.get("task");
        assert!(task_opt.is_some());
        let task = task_opt.unwrap();

        assert_eq!(task.options.working_directory, PathBuf::from("/tmp"));
    }
}
