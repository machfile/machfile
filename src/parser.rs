use std::{collections::HashMap, ffi::OsString, fs::read_to_string, path::PathBuf, str::FromStr};

use serde::Deserialize;

#[derive(Debug, PartialEq)]
pub enum ConfigParseError {
    NoConfigFile,
    CorruptConfigFile,
    EmptyConfig,
    InvalidTaskDefinition,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(flatten)]
    pub commands: HashMap<String, CommandConfig>,
}

#[derive(Debug, Deserialize)]
pub struct CommandConfig {
    pub command: Option<String>,
    pub desc: Option<String>,
    pub deps: Option<Vec<String>>,
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
    let Ok(config) = toml::from_str::<Config>(config_str) else {
        return Err(ConfigParseError::InvalidTaskDefinition);
    };

    if config.commands.is_empty() {
        return Err(ConfigParseError::EmptyConfig);
    }

    Ok(config)
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
