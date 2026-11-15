use std::{
    collections::{HashMap, VecDeque},
    path::Path,
};

use log::error;
use serde::Deserialize;

use crate::utils::{split_command_string, validate_task_name};

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
        // make sure task name is valid
        if let Err(e) = validate_task_name(name) {
            return Err(ConfigParseError::InvalidTaskDefinition(e.to_string()));
        }

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
                    "missing script".to_owned(),
                ));
            }

            for command in lines {
                let mut parts = VecDeque::from(split_command_string(command));

                if parts.is_empty() {
                    // empty line gets skipped
                    continue;
                }

                task_commands.push(ScriptCommand {
                    command: parts.pop_front().unwrap().to_string(),
                    args: parts.into(),
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
    allow_args: Option<bool>,
    forward_env_file: Option<String>,
}

impl RawTaskOptions {
    fn parse(self, path: &Path) -> Result<TaskOptions, ConfigParseError> {
        let mut working_directory = path.to_path_buf();
        if let Some(work_dir) = self.working_directory {
            working_directory.push(work_dir);
            if !working_directory.is_dir() {
                return Err(ConfigParseError::InvalidTaskDefinition(
                    "invalid working_directory".into(),
                ));
            }
        }

        let mut environment: HashMap<String, String> = self.environment.unwrap_or_default();

        // Forward env file if specified
        if let Some(env_file_path) = &self.forward_env_file {
            let env_file_path = path.join(env_file_path);
            if !env_file_path.exists() {
                return Err(ConfigParseError::InvalidTaskDefinition(format!(
                    "env file does not exist: {}",
                    env_file_path.display()
                )));
            }

            if let Err(_) = std::fs::read_to_string(&env_file_path) {
                return Err(ConfigParseError::InvalidTaskDefinition(format!(
                    "could not read env file: {}",
                    env_file_path.display()
                )));
            }

            let env = crate::environment::Environment::load_file(&env_file_path);
            if let Ok(env) = env {
                // Merge env file values into environment (env file values override config values)
                environment.extend(env.values);
            } else {
                // Parsing env file failures are non-fatal; continue with partial env
                log::warn!(
                    "Warning: env file could not be parsed: {}",
                    env.unwrap_err()
                );
            }
        }

        let allow_args = self.allow_args.unwrap_or_default();

        Ok(TaskOptions {
            working_directory,
            environment,
            allow_args,
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
        let opts = RawTaskOptions::default();
        let path = Path::new("/tmp");

        let result = opts.parse(&path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().working_directory.as_path(), path);
    }

    #[test]
    fn raw_task_options_constructs_relative_working_dir_correctly() {
        let opts = RawTaskOptions {
            working_directory: Some("src".to_string()),
            ..Default::default()
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
            ..Default::default()
        };
        let calling_directory_buf = current_dir().unwrap();
        let path = calling_directory_buf.as_path();

        let result = opts.parse(&path);
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert_eq!(
            error,
            ConfigParseError::InvalidTaskDefinition("invalid working_directory".to_string())
        );
    }

    #[test]
    fn raw_task_options_errors_with_non_existing_path_as_working_dir() {
        let opts = RawTaskOptions {
            working_directory: Some("this_path_should_not_exist_in_the_repo".to_string()),
            ..Default::default()
        };
        let calling_directory_buf = current_dir().unwrap();
        let path = calling_directory_buf.as_path();

        let result = opts.parse(&path);
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert_eq!(
            error,
            ConfigParseError::InvalidTaskDefinition("invalid working_directory".to_string())
        );
    }

    #[test]
    fn raw_task_options_constructs_absolute_dir_correctly() {
        let opts = RawTaskOptions {
            working_directory: Some("/tmp".to_string()),
            ..Default::default()
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
            environment: Some(test_map),
            ..Default::default()
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
    fn single_quoted_arguments_are_treated_as_one_argument() {
        let raw_task = RawTaskConfig {
            script: Some("touch 'arg arg'".to_string()),
            desc: None,
            deps: None,
            options: None,
        };
        let result = raw_task.parse("name", Path::new("/tmp"));

        assert!(result.is_ok());
        let task = result.unwrap();
        assert!(task.script.is_some());
        let script = task.script.unwrap();
        // First command
        assert_eq!(script[0].command, "touch".to_string());
        assert_eq!(script[0].args.len(), 1);
        assert_eq!(script[0].args[0], "arg arg".to_string());
    }

    #[test]
    fn double_quoted_arguments_are_treated_as_one_argument() {
        let raw_task = RawTaskConfig {
            script: Some("touch \"arg arg\"".to_string()),
            desc: None,
            deps: None,
            options: None,
        };
        let result = raw_task.parse("name", Path::new("/tmp"));

        assert!(result.is_ok());
        let task = result.unwrap();
        assert!(task.script.is_some());
        let script = task.script.unwrap();
        // First command
        assert_eq!(script[0].command, "touch".to_string());
        assert_eq!(script[0].args.len(), 1);
        assert_eq!(script[0].args[0], "arg arg".to_string());
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

    #[test]
    fn raw_task_options_forward_env_file_parses_correctly() {
        let temp_dir = std::env::temp_dir();
        let env_file = temp_dir.join("mach_test_parses_XXXXXX");
        std::fs::write(
            &env_file, "FOO=bar
",
        )
        .unwrap();

        let opts = RawTaskOptions {
            forward_env_file: Some(env_file.to_str().unwrap().to_string()),
            ..Default::default()
        };

        let result = opts.parse(&temp_dir);
        assert!(result.is_ok());
        let task_options = result.unwrap();
        assert!(task_options.environment.contains_key("FOO"));
        assert_eq!(task_options.environment.get("FOO").unwrap(), "bar");

        // Cleanup
        std::fs::remove_file(&env_file).unwrap();
    }

    #[test]
    fn raw_task_options_forward_env_file_merges_env_values() {
        let temp_dir = std::env::temp_dir();
        let env_file = temp_dir.join("mach_test_env_XXXXXX");
        let env_content = "FOO=bar
BAZ=qux
";
        std::fs::write(&env_file, env_content).unwrap();

        let opts = RawTaskOptions {
            forward_env_file: Some(env_file.to_str().unwrap().to_string()),
            ..Default::default()
        };

        let result = opts.parse(&temp_dir);
        assert!(result.is_ok());
        let task_options = result.unwrap();

        assert!(task_options.environment.contains_key("FOO"));
        assert_eq!(task_options.environment.get("FOO").unwrap(), "bar");
        assert!(task_options.environment.contains_key("BAZ"));
        assert_eq!(task_options.environment.get("BAZ").unwrap(), "qux");

        // Cleanup
        std::fs::remove_file(&env_file).unwrap();
    }

    #[test]
    fn raw_task_options_forward_env_file_merges_with_existing_env() {
        let temp_dir = std::env::temp_dir();
        let env_file = temp_dir.join("mach_test_env_merge_XXXXXX");
        std::fs::write(&env_file, "FOO=bar\n").unwrap();

        let opts = RawTaskOptions {
            forward_env_file: Some(env_file.to_str().unwrap().to_string()),
            environment: Some(std::collections::HashMap::from([(
                "EXISTING".to_string(),
                "value".to_string(),
            )])),
            ..Default::default()
        };

        let result = opts.parse(&temp_dir);
        assert!(result.is_ok());
        let task_options = result.unwrap();

        assert!(task_options.environment.contains_key("EXISTING"));
        assert_eq!(task_options.environment.get("EXISTING").unwrap(), "value");
        assert!(task_options.environment.contains_key("FOO"));
        assert_eq!(task_options.environment.get("FOO").unwrap(), "bar");

        // Cleanup
        std::fs::remove_file(&env_file).unwrap();
    }

    #[test]
    fn raw_task_options_forward_env_file_nonexistent_path_errors() {
        let opts = RawTaskOptions {
            forward_env_file: Some("env_file.env".to_string()),
            ..Default::default()
        };

        let result = opts.parse(&Path::new("/tmp"));
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("env file does not exist"));
    }

    #[test]
    fn raw_task_options_forward_env_file_with_empty_env_file() {
        let temp_dir = std::env::temp_dir();
        let env_file = temp_dir.join("mach_test_empty_env_XXXXXX");
        std::fs::write(&env_file, "").unwrap();

        let opts = RawTaskOptions {
            forward_env_file: Some(env_file.to_str().unwrap().to_string()),
            ..Default::default()
        };

        let result = opts.parse(&temp_dir);
        assert!(result.is_ok());
        let task_options = result.unwrap();
        assert!(task_options.environment.is_empty());

        // Cleanup
        std::fs::remove_file(&env_file).unwrap();
    }

    #[test]
    fn raw_task_options_forward_env_file_with_comments_and_empty_lines() {
        let temp_dir = std::env::temp_dir();
        let env_file = temp_dir.join("mach_test_comments_env_XXXXXX");
        let env_content = "# This is a comment\n\nFOO=bar\n  \nBAZ=qux\n# Another comment\n";
        std::fs::write(&env_file, env_content).unwrap();

        let opts = RawTaskOptions {
            forward_env_file: Some(env_file.to_str().unwrap().to_string()),
            ..Default::default()
        };

        let result = opts.parse(&temp_dir);
        assert!(result.is_ok());
        let task_options = result.unwrap();

        assert!(task_options.environment.contains_key("FOO"));
        assert_eq!(task_options.environment.get("FOO").unwrap(), "bar");
        assert!(task_options.environment.contains_key("BAZ"));
        assert_eq!(task_options.environment.get("BAZ").unwrap(), "qux");

        // Cleanup
        std::fs::remove_file(&env_file).unwrap();
    }

    #[test]
    fn raw_task_options_forward_env_file_handles_corrupt_env_file() {
        let temp_dir = std::env::temp_dir();
        let env_file = temp_dir.join("mach_test_corrupt_env_XXXXXX");
        std::fs::write(&env_file, "INVALID_LINE_WITHOUT_EQUALS").unwrap();

        let opts = RawTaskOptions {
            forward_env_file: Some(env_file.to_str().unwrap().to_string()),
            ..Default::default()
        };

        let result = opts.parse(&temp_dir);
        // Parsing env file failures are non-fatal; the task config should still be valid
        assert!(result.is_ok());

        // Cleanup
        std::fs::remove_file(&env_file).unwrap();
    }

    #[test]
    fn raw_task_options_forward_env_file_with_quoted_values() {
        let temp_dir = std::env::temp_dir();
        let env_file = temp_dir.join("mach_test_quoted_env_XXXXXX");
        let env_content = r#"QUOTED="hello world"
SINGLE_QUOTED='single quotes'
"#;
        std::fs::write(&env_file, env_content).unwrap();

        let opts = RawTaskOptions {
            forward_env_file: Some(env_file.to_str().unwrap().to_string()),
            ..Default::default()
        };

        let result = opts.parse(&temp_dir);
        assert!(result.is_ok());
        let task_options = result.unwrap();

        assert!(task_options.environment.contains_key("QUOTED"));
        assert_eq!(
            task_options.environment.get("QUOTED").unwrap(),
            "hello world"
        );
        assert!(task_options.environment.contains_key("SINGLE_QUOTED"));
        assert_eq!(
            task_options.environment.get("SINGLE_QUOTED").unwrap(),
            "single quotes"
        );

        // Cleanup
        std::fs::remove_file(&env_file).unwrap();
    }

    #[test]
    fn raw_task_options_forward_env_file_variable_expansion() {
        let temp_dir = std::env::temp_dir();
        let env_file = temp_dir.join("mach_test_variable_env_XXXXXX");
        let env_content = "BASE=expanded\nFINAL=\"$BASE value\"\n";
        std::fs::write(&env_file, env_content).unwrap();

        let opts = RawTaskOptions {
            forward_env_file: Some(env_file.to_str().unwrap().to_string()),
            ..Default::default()
        };

        let result = opts.parse(&temp_dir);
        assert!(result.is_ok());
        let task_options = result.unwrap();

        assert!(task_options.environment.contains_key("BASE"));
        assert_eq!(task_options.environment.get("BASE").unwrap(), "expanded");
        assert!(task_options.environment.contains_key("FINAL"));
        assert_eq!(
            task_options.environment.get("FINAL").unwrap(),
            "expanded value"
        );

        // Cleanup
        std::fs::remove_file(&env_file).unwrap();
    }

    #[test]
    fn raw_task_options_forward_env_file_overrides_existing_env() {
        let temp_dir = std::env::temp_dir();
        let env_file = temp_dir.join("mach_test_override_env_XXXXXX");
        std::fs::write(&env_file, "OVERRIDE=from_file\n").unwrap();

        let opts = RawTaskOptions {
            forward_env_file: Some(env_file.to_str().unwrap().to_string()),
            environment: Some(std::collections::HashMap::from([(
                "OVERRIDE".to_string(),
                "from_config".to_string(),
            )])),
            ..Default::default()
        };

        let result = opts.parse(&temp_dir);
        assert!(result.is_ok());
        let task_options = result.unwrap();

        // Env file values override config values (last one wins)
        assert_eq!(
            task_options.environment.get("OVERRIDE").unwrap(),
            "from_file"
        );

        // Cleanup
        std::fs::remove_file(&env_file).unwrap();
    }
}
