//! # `machfile`
//! > Not a build system
//!
//! Mach is a simple task runner, for when a full blown build system is overkill. It can be used
//! both as a CLI tool and as a library.
//!
//! ## Configuration
//!
//! The following is an example configuration written in `toml`:
//!
//! ```toml
//! [run]
//! script = "cargo run"
//! desc = "Run with debug logging"
//! options.environment.RUST_LOG = "mach=debug,info"
//!
//! [clean]
//! script = "rm -rf target"
//! desc = "Remove cache and outputs"
//!
//! [check_target_size]
//! script = "du -d1 -h"
//! desc = "Check directory sizes of cache dirs"
//! options.working_directory = "target"
//!
//! [install]
//! script = "cargo install --path ."
//! deps = ["clean"]
//! ```
//!
//! The same configuration can be written in `yaml` as follows:
//!
//! ```yaml
//! run:
//!   script: "cargo run"
//!   desc: "Run with debug logging"
//!   options:
//!     environment:
//!       RUST_LOG: "mach=debug,info"
//!
//! clean:
//!   script: "rm -rf target"
//!   desc: "Remove cache and outputs"
//!
//! check_target_size:
//!   script: "du -d1 -h"
//!   desc: "Check directory sizes of cache dirs"
//!   options:
//!     working_directory: "target"
//!
//! install:
//!   script: "cargo install --path ."
//!   deps:
//!     - "clean"
//! ```

use log::debug;
use std::{
    env,
    ffi::OsString,
    fs::read_to_string,
    path::{Path, PathBuf},
};

// pub mod executer;
pub mod utils;

pub mod config;
pub mod config_finder;
mod raw_config;

pub use config::Config;

use config_finder::get_mach_file_path;
use raw_config::RawConfig;
use utils::ConfigParseError;

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
pub fn load_config(config_override: Option<OsString>) -> Result<Config, ConfigParseError> {
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

    debug!("Found config file: {config_file:?}");

    // TODO: read file content safely
    let Ok(file_content) = read_to_string(config_file.clone()) else {
        return Err(ConfigParseError::CorruptConfigFile);
    };

    parse_config(&file_content, &config_file)
}

/// Tries to parse the provided string as `Config`
///
/// # Errors
///
/// - Returns [`ConfigParseError::InvalidTaskDefinition`] if the Config can't be parsed
/// - Returns [`ConfigParseError::UnsupportedConfigFileExtension`] if the file extension is not supported (not `toml`, `yaml` or `yml`)
/// - Returns [`ConfigParseError::EmptyConfig`] if the TOML does not contain any task
pub fn parse_config(config_str: &str, config_path: &Path) -> Result<Config, ConfigParseError> {
    let config = match config_path.extension().and_then(|ext| ext.to_str()) {
        Some("toml") => toml::from_str::<RawConfig>(config_str).map_err(|parse_error| {
            ConfigParseError::InvalidTaskDefinition(format!("Invalid TOML: {parse_error}"))
        }),
        Some("yaml") | Some("yml") => {
            serde_saphyr::from_str::<RawConfig>(config_str).map_err(|parse_error| {
                ConfigParseError::InvalidTaskDefinition(format!("Invalid YAML: {parse_error}"))
            })
        }
        _ => Err(ConfigParseError::UnsupportedConfigFileExtension),
    }?;

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
    fn parse_config_succeeds_on_valid_yaml_config() {
        let conf = "task:\n  script: \"hello world\"";

        assert!(parse_config(conf, &PathBuf::from("/tmp/mach.yaml")).is_ok());
        assert!(parse_config(conf, &PathBuf::from("/tmp/mach.yml")).is_ok());
    }

    #[test]
    fn parse_config_fails_on_invalid_config_extension() {
        let conf = "task:\n  script: \"hello world\"";

        assert_eq!(
            parse_config(conf, &PathBuf::from("/tmp/mach.json")).unwrap_err(),
            ConfigParseError::UnsupportedConfigFileExtension
        );
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
