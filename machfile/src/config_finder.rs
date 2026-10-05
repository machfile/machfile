use std::path::{Path, PathBuf};

use crate::utils::ConfigParseError;

pub fn get_mach_file_path(directory: &Path) -> Result<PathBuf, ConfigParseError> {
    let mut ancestors = directory.ancestors();

    if let Some(path) = ancestors.next() {
        if let Some(config_path) = check_dir_for_config(path)? {
            return Ok(config_path);
        }
    } else {
        return Err(ConfigParseError::NoConfigFile);
    }

    let mut is_git_repository = false;
    let mut config_path: Option<PathBuf> = None;

    for dir in ancestors {
        // If we have not found a config yet, check for it
        if config_path.is_none() {
            config_path = check_dir_for_config(dir)?;
        }

        if dir_is_git_root(dir) {
            is_git_repository = true;
            break;
        }
    }

    if let Some(path) = config_path {
        Ok(path)
    } else if is_git_repository {
        Err(ConfigParseError::NoConfigFileInRepo)
    } else {
        Err(ConfigParseError::NoConfigFile)
    }
}

/// Checks if the provided directory contains a mach configuration file with any of the supported extensions (toml, yaml, yml)
/// and returns the path to it if found.
///
/// # Arguments
///
/// * `dir`: Path to the directory to check.
///
/// returns: Result<Option<PathBuf>, ConfigParseError>
///
/// # Errors
///
/// - Returns [`ConfigParseError::MultipleConfigFiles`] if multiple config files with different extensions are found in the same directory.
pub fn check_dir_for_config(dir: &Path) -> Result<Option<PathBuf>, ConfigParseError> {
    let mut path = dir.to_path_buf();
    path.push("mach");

    let mut config_path = None;

    for ext in ["toml", "yaml", "yml"] {
        let path_with_extension = path.with_extension(ext);

        if path_with_extension.is_file() {
            if config_path.is_some() {
                return Err(ConfigParseError::MultipleConfigFiles);
            } else {
                config_path = Some(path_with_extension);
            }
        }
    }

    Ok(config_path)
}

fn dir_is_git_root(dir: &Path) -> bool {
    let mut path = dir.to_path_buf();
    path.push(".git");

    path.is_dir()
}

#[cfg(test)]
mod tests {
    use assert_fs::prelude::*;

    use super::*;

    #[test]
    fn check_dir_for_config_returns_none_on_empty_dir() {
        let dir = assert_fs::TempDir::new().unwrap();

        assert!(check_dir_for_config(dir.path()).unwrap().is_none());
        dir.close().unwrap();
    }

    #[test]
    fn check_dir_for_config_finds_toml() {
        let dir = assert_fs::TempDir::new().unwrap();
        let config = dir.child("mach.toml");
        config.touch().unwrap();

        assert!(check_dir_for_config(dir.path()).unwrap().is_some());
        dir.close().unwrap();
    }

    #[test]
    fn check_dir_for_config_finds_yaml() {
        let dir = assert_fs::TempDir::new().unwrap();
        let config = dir.child("mach.yaml");
        config.touch().unwrap();

        assert!(check_dir_for_config(dir.path()).unwrap().is_some());
        dir.close().unwrap();

        let dir = assert_fs::TempDir::new().unwrap();
        let config = dir.child("mach.yml");
        config.touch().unwrap();

        assert!(check_dir_for_config(dir.path()).unwrap().is_some());
        dir.close().unwrap();
    }

    #[test]
    fn check_dir_for_config_returns_error_on_multiple_configs() {
        let dir = assert_fs::TempDir::new().unwrap();
        let config = dir.child("mach.toml");
        config.touch().unwrap();
        let config2 = dir.child("mach.yaml");
        config2.touch().unwrap();

        assert_eq!(
            check_dir_for_config(dir.path()).unwrap_err(),
            ConfigParseError::MultipleConfigFiles
        );
        dir.close().unwrap();
    }

    #[test]
    fn dir_is_git_root_returns_false_on_empty_dir() {
        let dir = assert_fs::TempDir::new().unwrap();

        assert!(!dir_is_git_root(dir.path()));
        dir.close().unwrap();
    }

    #[test]
    fn dir_is_git_root_returns_true_on_dir_with_git_directory() {
        let dir = assert_fs::TempDir::new().unwrap();
        let git_dir = dir.child(".git");
        git_dir.create_dir_all().unwrap();

        assert!(dir_is_git_root(dir.path()));
        dir.close().unwrap();
    }

    #[test]
    fn directory_in_repo_without_config_returns_correct_error() {
        let dir = assert_fs::TempDir::new().unwrap();
        let git_dir = dir.child(".git");
        git_dir.create_dir_all().unwrap();
        let sub_dir = dir.child("subdirectory");
        sub_dir.create_dir_all().unwrap();

        let res = get_mach_file_path(sub_dir.path());
        assert!(res.is_err());
        assert_eq!(res, Err(ConfigParseError::NoConfigFileInRepo));
        dir.close().unwrap();
    }

    #[test]
    fn directory_in_repo_returns_direct_file_if_found() {
        let dir = assert_fs::TempDir::new().unwrap();
        let git_dir = dir.child(".git");
        git_dir.create_dir_all().unwrap();
        let sub_dir = dir.child("subdirectory");
        sub_dir.create_dir_all().unwrap();
        let config = sub_dir.child("mach.toml");
        config.touch().unwrap();

        let res = get_mach_file_path(sub_dir.path());
        assert!(res.is_ok());
        assert_eq!(res, Ok(PathBuf::from(config.path())));
        dir.close().unwrap();
    }

    #[test]
    fn directory_in_repo_returns_parent_file_if_found() {
        let dir = assert_fs::TempDir::new().unwrap();
        let git_dir = dir.child(".git");
        git_dir.create_dir_all().unwrap();
        let config = dir.child("mach.toml");
        config.touch().unwrap();

        let sub_dir = dir.child("subdirectory");
        sub_dir.create_dir_all().unwrap();

        let res = get_mach_file_path(sub_dir.path());
        assert!(res.is_ok());
        assert_eq!(res, Ok(PathBuf::from(config.path())));
        dir.close().unwrap();
    }
}
