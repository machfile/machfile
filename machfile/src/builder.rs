use std::{env, path::PathBuf};

use crate::{
    MachConfig, config_finder::get_mach_file_path, environment::{Environment, EnvironmentParseError}, utils::ConfigParseError
};

pub enum BuilderError {
    ConfigParseError(ConfigParseError),
    EnvParseError(EnvironmentParseError),
}

#[derive(Default)]
pub struct Builder {
    directory: PathBuf,
    env_file_override: Option<PathBuf>,
    disable_env_file: bool,
    disable_auto_discover: bool,
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

    /// Construct the `MachConfig` from the settings
    pub fn build(self) -> Result<MachConfig, BuilderError> {
        let config = {
            if self.disable_auto_discover {
                // TODO direct parse or error
            } else {
                let path = match get_mach_file_path(&self.directory) {
                    Ok(p) => p,
                    Err(e) => {
                        return Err(BuilderError::ConfigParseError(e));
                    }
                } 

                // TODO parse error
            }
        };

        let environment = if let Some(env_file) = self.env_file_override {
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
            Environment::default()
        };

        Ok(MachConfig {
            environment,
            config: (),
        })
    }
}
