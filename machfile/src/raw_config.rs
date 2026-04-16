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
            name: name.to_string(),
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

#[cfg(test)]
mod tests {
    use std::env::current_dir;

    use crate::Config;

    use super::*;

    #[test]
    fn raw_task_options_parse_correctly_with_minimal_data() {
        let opts = RawTaskOptions {
            working_directory: None,
            environment: None,
        };
        let path = Path::new("/tmp");

        let result = opts.parse(&path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().working_directory.as_path(), path);
    }

    #[test]
    fn raw_task_options_constructs_relative_working_dir_correctly() {
        let opts = RawTaskOptions {
            working_directory: Some("src".to_string()),
            environment: None,
        };
        let calling_directory_buf = current_dir().unwrap();
        let mut final_directory_buf = calling_directory_buf.clone();

        let path = calling_directory_buf.as_path();

        final_directory_buf.push("src");

        let result = opts.parse(&path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().working_directory, final_directory_buf);
    }

    #[test]
    fn raw_task_options_errors_with_non_directory_as_working_dir() {
        let opts = RawTaskOptions {
            working_directory: Some("README.md".to_string()),
            environment: None,
        };
        let calling_directory_buf = current_dir().unwrap();
        let path = calling_directory_buf.as_path();

        let result = opts.parse(&path);
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert_eq!(
            error,
            ConfigParseError::InvalidTaskDefinition("Invalid working_directory".to_string())
        );
    }

    #[test]
    fn raw_task_options_errors_with_non_existing_path_as_working_dir() {
        let opts = RawTaskOptions {
            working_directory: Some("this_path_should_not_exist_in_the_repo".to_string()),
            environment: None,
        };
        let calling_directory_buf = current_dir().unwrap();
        let path = calling_directory_buf.as_path();

        let result = opts.parse(&path);
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert_eq!(
            error,
            ConfigParseError::InvalidTaskDefinition("Invalid working_directory".to_string())
        );
    }

    #[test]
    fn raw_task_options_constructs_absolute_dir_correctly() {
        let opts = RawTaskOptions {
            working_directory: Some("/tmp".to_string()),
            environment: None,
        };
        let calling_directory_buf = current_dir().unwrap();
        let path = calling_directory_buf.as_path();

        let result = opts.parse(&path);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().working_directory.as_path(),
            Path::new("/tmp")
        );
    }

    #[test]
    fn raw_task_options_handles_environment_dict_well() {
        let mut test_map = HashMap::new();
        test_map.insert("ALPHA".to_string(), "alpha_val".to_string());
        test_map.insert("BRAVO".to_string(), "bravo_val".to_string());

        let opts = RawTaskOptions {
            working_directory: None,
            environment: Some(test_map),
        };
        let calling_directory_buf = current_dir().unwrap();
        let path = calling_directory_buf.as_path();

        let result = opts.parse(&path);
        assert!(result.is_ok());

        let env_map = result.unwrap().environment;
        assert!(!env_map.is_empty());
        assert!(env_map.contains_key("ALPHA"));
        assert!(env_map.contains_key("BRAVO"));
        assert_eq!(env_map.get("ALPHA"), Some(&"alpha_val".to_string()));
        assert_eq!(env_map.get("BRAVO"), Some(&"bravo_val".to_string()));
    }

    #[test]
    fn raw_task_config_parses_correctly_with_minimal_data() {
        let raw_task = RawTaskConfig {
            script: Some("touch".to_string()),
            desc: None,
            deps: None,
            options: None,
        };
        let result = raw_task.parse("name", Path::new("/tmp"));

        assert!(result.is_ok());
        let task = result.unwrap();
        assert_eq!(task.name, "name".to_string());
        assert!(task.script.is_some());
        let script = task.script.unwrap();
        assert_eq!(script.len(), 1);
        assert_eq!(script[0].command, "touch".to_string());
        assert_eq!(script[0].args.len(), 0);
    }

    #[test]
    fn raw_task_config_parses_script_with_single_command_single_arg_correctly() {
        let raw_task = RawTaskConfig {
            script: Some("touch me".to_string()),
            desc: None,
            deps: None,
            options: None,
        };
        let result = raw_task.parse("name", Path::new("/tmp"));

        assert!(result.is_ok());
        let task = result.unwrap();
        assert!(task.script.is_some());
        let script = task.script.unwrap();
        assert_eq!(script.len(), 1);
        assert_eq!(script[0].command, "touch".to_string());
        assert_eq!(script[0].args.len(), 1);
        assert_eq!(script[0].args[0], "me".to_string());
    }

    #[test]
    fn raw_task_config_parses_script_with_single_command_multi_arg_correctly() {
        let raw_task = RawTaskConfig {
            script: Some("touch me one more --time".to_string()),
            desc: None,
            deps: None,
            options: None,
        };
        let result = raw_task.parse("name", Path::new("/tmp"));

        assert!(result.is_ok());
        let task = result.unwrap();
        assert!(task.script.is_some());
        let script = task.script.unwrap();
        assert_eq!(script.len(), 1);
        assert_eq!(script[0].command, "touch".to_string());
        assert_eq!(script[0].args.len(), 4);
        assert_eq!(script[0].args[0], "me".to_string());
        assert_eq!(script[0].args[1], "one".to_string());
        assert_eq!(script[0].args[2], "more".to_string());
        assert_eq!(script[0].args[3], "--time".to_string());
    }

    #[test]
    fn raw_task_config_parses_script_with_multiple_simple_commands_correctly() {
        let raw_task = RawTaskConfig {
            script: Some("touch\nwho".to_string()),
            desc: None,
            deps: None,
            options: None,
        };
        let result = raw_task.parse("name", Path::new("/tmp"));

        assert!(result.is_ok());
        let task = result.unwrap();
        assert!(task.script.is_some());
        let script = task.script.unwrap();
        assert_eq!(script.len(), 2);
        assert_eq!(script[0].command, "touch".to_string());
        assert_eq!(script[1].command, "who".to_string());
    }

    #[test]
    fn raw_task_config_parses_script_with_multi_command_multi_arg_correctly() {
        let raw_task = RawTaskConfig {
            script: Some("touch me one\nmore --time".to_string()),
            desc: None,
            deps: None,
            options: None,
        };
        let result = raw_task.parse("name", Path::new("/tmp"));

        assert!(result.is_ok());
        let task = result.unwrap();
        assert!(task.script.is_some());
        let script = task.script.unwrap();
        assert_eq!(script.len(), 2);

        // First command
        assert_eq!(script[0].command, "touch".to_string());
        assert_eq!(script[0].args.len(), 2);
        assert_eq!(script[0].args[0], "me".to_string());
        assert_eq!(script[0].args[1], "one".to_string());
        // Second command
        assert_eq!(script[1].command, "more".to_string());
        assert_eq!(script[1].args.len(), 1);
        assert_eq!(script[1].args[0], "--time".to_string());
    }

    #[test]
    fn minimal_configuration_parses_correctly() {
        let mut tasks = HashMap::new();
        tasks.insert(
            "task0".to_string(),
            RawTaskConfig {
                script: Some("hello world".to_string()),
                desc: Some("test description".to_string()),
                deps: None,
                options: None,
            },
        );
        tasks.insert(
            "task1".to_string(),
            RawTaskConfig {
                script: Some("touch test1 asdf".to_string()),
                desc: Some("test description".to_string()),
                deps: None,
                options: None,
            },
        );

        let raw = RawConfig { tasks };

        let result = Config::from_raw(raw, &Path::new("/tmp"));
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.tasks.keys().len(), 2);
        assert!(config.tasks.contains_key("task0"));
        assert!(config.tasks.contains_key("task1"));
        assert!(config.tasks.get("task0").unwrap().script.is_some());
        assert_eq!(
            config.tasks.get("task0").unwrap().script.clone().unwrap()[0].command,
            "hello".to_string()
        );
        assert!(config.tasks.get("task1").unwrap().script.is_some());
        assert_eq!(
            config.tasks.get("task1").unwrap().script.clone().unwrap()[0].command,
            "touch".to_string()
        );
    }

    #[test]
    fn simple_circular_dependencies_lead_to_parse_error() {
        let mut tasks = HashMap::new();
        tasks.insert(
            "task0".to_string(),
            RawTaskConfig {
                script: Some("hello world".to_string()),
                desc: Some("test description".to_string()),
                deps: Some(vec!["task1".to_string()]),
                options: None,
            },
        );
        tasks.insert(
            "task1".to_string(),
            RawTaskConfig {
                script: Some("touch test1 asdf".to_string()),
                desc: Some("test description".to_string()),
                deps: Some(vec!["task0".to_string()]),
                options: None,
            },
        );

        let raw = RawConfig { tasks };

        let result = Config::from_raw(raw, &Path::new("/tmp"));
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error, ConfigParseError::CircularDependencies);
    }

    #[test]
    fn complex_circular_dependencies_lead_to_parse_error() {
        let mut tasks = HashMap::new();
        tasks.insert(
            "task0".to_string(),
            RawTaskConfig {
                script: Some("hello world".to_string()),
                desc: Some("test description".to_string()),
                deps: Some(vec!["task1".to_string()]),
                options: None,
            },
        );
        tasks.insert(
            "task1".to_string(),
            RawTaskConfig {
                script: Some("touch test1 asdf".to_string()),
                desc: Some("test description".to_string()),
                deps: Some(vec!["task2".to_string()]),
                options: None,
            },
        );
        tasks.insert(
            "task2".to_string(),
            RawTaskConfig {
                script: Some("touch test1 asdf".to_string()),
                desc: Some("test description".to_string()),
                deps: Some(vec!["task0".to_string()]),
                options: None,
            },
        );

        let raw = RawConfig { tasks };

        let result = Config::from_raw(raw, &Path::new("/tmp"));
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error, ConfigParseError::CircularDependencies);
    }
}
