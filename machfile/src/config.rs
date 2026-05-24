use crate::{
    raw_config::RawConfig,
    utils::{CommandError, ConfigParseError},
};
use std::fmt::{Display, Formatter};
use std::{
    collections::{HashMap, HashSet}
    ,
    path::{Path, PathBuf},
    process::Command,
};

/// The main struct holding all [tasks](`Task`)
#[derive(Debug, Clone)]
pub struct Config {
    pub path: PathBuf,
    pub tasks: HashMap<String, Task>,
}

impl Config {
    /// Parse a [`RawConfig`] into a `Config`
    ///
    /// # Errors
    ///
    /// Returns [`ConfigParseError`] when there is an issue with any part of the configuration
    pub fn from_raw(raw: RawConfig, config_path: &Path) -> Result<Self, ConfigParseError> {
        let path = config_path.parent().unwrap().to_path_buf();
        let mut config = Self {
            path: path.clone(),
            tasks: HashMap::new(),
        };

        for (name, raw_task) in &raw.tasks {
            let task = raw_task.clone().parse(name, &path)?;
            config.tasks.insert(name.to_string(), task);
        }

        if config.has_circular_dependencies() {
            Err(ConfigParseError::CircularDependencies)
        } else {
            Ok(config)
        }
    }

    /// Returns a vector of task names which should be executed in order for the given `task_name`
    pub fn get_execution_chain(&self, task_name: &str) -> Vec<&Task> {
        let mut chain = Vec::new();
        self.collect_deps(task_name, &mut chain);
        chain
    }

    fn collect_deps<'a>(&'a self, task_name: &str, chain: &mut Vec<&'a Task>) {
        if let Some(task) = self.tasks.get(task_name) {
            for dep in &task.deps {
                self.collect_deps(dep, chain);
            }
            chain.push(task);
        }
    }

    fn has_circular_dependencies(&self) -> bool {
        let mut visited = HashSet::new();
        for task in self.tasks.keys() {
            if self.has_cycle(task, &mut visited) {
                return true;
            }
        }
        false
    }

    fn has_cycle(&self, task_name: &str, visited: &mut HashSet<String>) -> bool {
        if visited.contains(task_name) {
            return true;
        }
        visited.insert(task_name.to_string());

        if let Some(task) = self.tasks.get(task_name) {
            for dep in &task.deps {
                if self.has_cycle(dep, visited) {
                    return true;
                }
            }
        }
        visited.remove(task_name);
        false
    }
}

/// The individual task
///
/// Composed of the `TaskOptions` and optionally a description, script and task dependencies
#[derive(Debug, Clone, Default)]
pub struct Task {
    /// The name of the task
    pub name: String,
    pub options: TaskOptions,
    /// Vector of `ScriptCommand` that are run in order when the task is executed
    pub script: Option<Vec<ScriptCommand>>,
    /// The task description shown in the help message
    pub desc: Option<String>,
    /// Vector of the names of tasks this task depends on
    pub deps: Vec<String>,
}

impl Task {
    pub fn execute(&self) -> Result<(), CommandError> {
        if let Some(script) = &self.script {
            for command in script {
                let mut sys_command = command.create_sys_command();

                for (key, value) in &self.options.environment {
                    sys_command.env(key, value);
                }
                sys_command.current_dir(&self.options.working_directory);

                let proc = sys_command.spawn();

                if let Ok(mut proc) = proc
                    && let Ok(code) = proc.wait()
                {
                    if code.success() {
                        continue;
                    }

                    return Err(CommandError {
                        message: format!("command failed with status {code}"),
                    });
                }

                return Err(CommandError {
                    message: "command failed to spawn".to_owned(),
                });
            }
        }

        Ok(())
    }
}

/// Configuration options for the environment in which the [`ScriptCommand`]s will be run
#[derive(Debug, Clone, Default)]
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

impl Display for ScriptCommand {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.command, self.args.join(" "))
    }
}
