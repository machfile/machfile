use std::{error::Error, fmt};

#[derive(Debug, PartialEq)]
pub enum ConfigParseError {
    NoConfigFile,
    NoConfigFileInRepo,
    CorruptConfigFile,
    EmptyConfig,
    InvalidTaskDefinition(String),
    MultipleConfigFiles,
    UnsupportedConfigFileExtension,
    CircularDependencies,
}

#[derive(Debug)]
pub struct CommandError {
    pub message: String,
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Command encountered an unexpected error: {}",
            self.message
        )
    }
}

impl Error for CommandError {}
