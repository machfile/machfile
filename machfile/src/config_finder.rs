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
fn check_dir_for_config(dir: &Path) -> Result<Option<PathBuf>, ConfigParseError> {
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
    use std::{fs, sync::Once};

    use super::*;

    static INIT: Once = Once::new();

    pub fn initialize() {
        INIT.call_once(|| {
            let paths = vec![
                Path::new("./tests/configs/git_dir/.git"),
                Path::new("./tests/configs/empty_git_dir/.git"),
            ];

            for path in paths {
                if path.is_dir() {
                    continue;
                } else {
                    fs::create_dir(path).unwrap_or_else(|_| panic!("Failed to create {path:?}"));
                }
            }
        });
    }

    #[test]
    fn check_dir_for_config_returns_none_on_empty_dir() {
        let path = Path::new("./tests/configs/no_config");

        assert!(check_dir_for_config(path).unwrap().is_none());
    }

    #[test]
    fn check_dir_for_config_finds_toml() {
        let path = Path::new("./tests/configs/direct_config");

        assert!(check_dir_for_config(path).unwrap().is_some());
    }

    #[test]
    fn check_dir_for_config_finds_yaml() {
        let path = Path::new("./tests/configs/direct_yaml_config");

        assert!(check_dir_for_config(path).unwrap().is_some());
    }

    #[test]
    fn check_dir_for_config_returns_error_on_multiple_configs() {
        let path = Path::new("./tests/configs/multiple_configs");

        assert_eq!(
            check_dir_for_config(path).unwrap_err(),
            ConfigParseError::MultipleConfigFiles
        );
    }

    #[test]
    fn dir_is_git_root_returns_false_on_empty_dir() {
        let path = Path::new("./tests/configs/no_config");

        assert!(!dir_is_git_root(path));
    }

    #[test]
    fn dir_is_git_root_returns_true_on_dir_with_git_directory() {
        initialize();
        let path = Path::new("./tests/configs/git_dir");

        assert!(dir_is_git_root(path));
    }

    #[test]
    fn directory_in_repo_without_config_returns_correct_error() {
        initialize();
        let path = Path::new("./tests/configs/empty_git_dir/subdirectory");

        let res = get_mach_file_path(path);
        assert!(res.is_err());
        assert_eq!(res, Err(ConfigParseError::NoConfigFileInRepo));
    }

    #[test]
    fn directory_in_repo_returns_direct_file_if_found() {
        initialize();
        let path = Path::new("./tests/configs/git_dir/subdirectory_with_config");

        let res = get_mach_file_path(path);
        assert!(res.is_ok());
        assert_eq!(
            res,
            Ok(PathBuf::from(
                "./tests/configs/git_dir/subdirectory_with_config/mach.toml"
            ))
        );
    }

    #[test]
    fn directory_in_repo_returns_parent_file_if_found() {
        initialize();
        let path = Path::new("./tests/configs/git_dir/subdirectory_without_config");

        let res = get_mach_file_path(path);
        assert!(res.is_ok());
        assert_eq!(res, Ok(PathBuf::from("./tests/configs/git_dir/mach.toml")));
    }
}
