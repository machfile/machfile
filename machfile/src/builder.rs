use std::{env, error::Error, fmt, fs::read_to_string, path::PathBuf};

use crate::{
    Config, MachConfig,
    config_finder::{check_dir_for_config, get_mach_file_path},
    environment::{Environment, EnvironmentParseError},
    parse_config,
    utils::ConfigParseError,
};

#[derive(Debug)]
pub enum BuilderError {
    ConfigParseError(ConfigParseError),
    EnvParseError(EnvironmentParseError),
}

impl fmt::Display for BuilderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BuilderError::ConfigParseError(e) => write!(f, "{e}"),
            BuilderError::EnvParseError(e) => write!(f, "{e}"),
        }
    }
}

impl Error for BuilderError {}

/// The `Builder` allows construction of a mach context
///
/// The resulting [`MachConfig`] bundles the [`Environment`] and the [`Config`] constructed based on
/// the provided settings.
///
/// The fastest way to construct a configuration is using  [`Builder::from_current_dir`], which will
/// build a config based on the current working directory, searching upwards if inside a git
/// repository, and loading a `.env` file located in the same directory.
#[derive(Debug, Default)]
pub struct Builder {
    directory: PathBuf,
    env_file_override: Option<PathBuf>,
    disable_env_file: bool,
    disable_auto_discover: bool,
    verbose: bool,
    config: Option<Config>,
}

impl Builder {
    /// Start constructing from current working directory
    pub fn from_current_dir() -> Self {
        let directory = env::current_dir().expect("invalid working directory");

        Self::new(directory)
    }

    /// Start constructing from `directory`
    pub fn new(directory: PathBuf) -> Self {
        Self {
            directory,
            ..Default::default()
        }
    }

    /// Disable env file loading
    pub fn disable_env_file(mut self) -> Self {
        if self.env_file_override.is_some() {
            self.env_file_override = None;
        }

        self.disable_env_file = true;
        self
    }

    /// Do not try to find a mach config by going upwards
    pub fn disable_auto_discover(mut self) -> Self {
        self.disable_auto_discover = true;
        self
    }

    /// Set verbose flag
    pub fn enable_verbose(mut self) -> Self {
        self.verbose = true;
        self
    }

    /// Remove verbose flag
    pub fn disable_verbose(mut self) -> Self {
        self.verbose = false;
        self
    }

    /// Load environment variables from the provided path
    ///
    /// Removes the `disable_env_file` flag if set
    pub fn with_env_file(mut self, env_file: PathBuf) -> Self {
        if self.disable_env_file {
            self.disable_env_file = false;
        }

        self.env_file_override = Some(env_file);
        self
    }

    /// Construct the [`MachConfig`] from the settings
    pub fn build(mut self) -> Result<MachConfig, BuilderError> {
        let config: Config = match self.config {
            Some(c) => c,
            None => match self.build_config() {
                Ok(c) => c,
                Err(e) => {
                    return Err(BuilderError::ConfigParseError(e));
                }
            },
        };

        let environment: Environment = if let Some(env_file) = self.env_file_override {
            if !env_file.is_file() {
                return Err(BuilderError::EnvParseError(
                    EnvironmentParseError::NoEnvironmentFile,
                ));
            }
            match Environment::load_file(&env_file) {
                Ok(env) => env,
                Err(e) => {
                    return Err(BuilderError::EnvParseError(e));
                }
            }
        } else {
            let mut env_path = self.directory.clone();
            env_path.set_file_name(".env");

            if env_path.is_file() {
                match Environment::load_file(&env_path) {
                    Ok(env) => env,
                    Err(e) => {
                        return Err(BuilderError::EnvParseError(e));
                    }
                }
            } else {
                Environment::default()
            }
        };

        let is_verbose = self.verbose;

        Ok(MachConfig {
            environment,
            config,
            is_verbose,
        })
    }

    /// Get configuration or create it
    pub fn get_config(&mut self) -> Result<&Config, ConfigParseError> {
        if self.config.is_none() {
            let c = self.build_config()?;
            self.config = Some(c);
        }

        Ok(self.config.as_ref().expect("value just set"))
    }

    /// Build the configuration
    fn build_config(&mut self) -> Result<Config, ConfigParseError> {
        let config = {
            let path = if self.disable_auto_discover {
                match check_dir_for_config(&self.directory) {
                    Ok(p) => {
                        if let Some(p) = p {
                            p
                        } else {
                            return Err(ConfigParseError::NoConfigFile);
                        }
                    }
                    Err(e) => {
                        return Err(e);
                    }
                }
            } else {
                match get_mach_file_path(&self.directory) {
                    Ok(p) => p,
                    Err(e) => {
                        return Err(e);
                    }
                }
            };

            let Ok(content) = read_to_string(path.clone()) else {
                return Err(ConfigParseError::CorruptConfigFile);
            };

            self.directory = path.clone();

            match parse_config(&content, &path) {
                Ok(c) => c,
                Err(e) => {
                    return Err(e);
                }
            }
        };

        Ok(config)
    }
}
