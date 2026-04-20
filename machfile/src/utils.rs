use std::{error::Error, fmt, mem::take};

/// This enum is used to identify errors that ocurre during config parsing
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

/// This struct is used for errors during command execution
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

pub fn split_command_string(cmd_string: &str) -> Vec<String> {
    let mut arguments = vec![];

    let mut current_str = String::new();
    let mut is_escaping = false;
    let mut is_double_quoted = false;
    let mut is_single_quoted = false;
    for c in cmd_string.chars() {
        if is_escaping {
            current_str.push(c);
            is_escaping = false;
            continue;
        }

        if c == '\\' && !is_single_quoted {
            is_escaping = true;
            continue;
        }

        if c == '"' {
            is_double_quoted = !is_double_quoted;
            continue;
        }
        if c == '\'' {
            is_single_quoted = !is_single_quoted;
            continue;
        }

        if c.is_whitespace() && !is_double_quoted && !is_single_quoted {
            if !current_str.is_empty() {
                arguments.push(take(&mut current_str));
            }
            continue;
        }

        current_str.push(c);
    }

    if !current_str.is_empty() {
        arguments.push(current_str);
    }

    arguments
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_command_string_returns_empty_vector_when_called_with_empty_str() {
        let result = split_command_string("");

        assert!(result.is_empty());
    }

    #[test]
    fn split_command_string_returns_empty_vector_when_called_with_whitespace_str() {
        let result = split_command_string(" ");

        assert!(result.is_empty());

        let result = split_command_string("\t");

        assert!(result.is_empty());
    }

    #[test]
    fn split_command_string_returns_vector_of_simple_arguments_in_correct_order() {
        let input = "asdf aa bbb 1cc";
        let result = split_command_string(input);

        assert_eq!(result.len(), 4);
        assert_eq!(result[0], "asdf".to_string());
        assert_eq!(result[1], "aa".to_string());
        assert_eq!(result[2], "bbb".to_string());
        assert_eq!(result[3], "1cc".to_string());
    }

    #[test]
    fn split_command_string_ignores_repeated_whitespace() {
        let input = "asdf  aa \tbbb   1cc";
        let result = split_command_string(input);

        assert_eq!(result.len(), 4);
        assert_eq!(result[0], "asdf".to_string());
        assert_eq!(result[1], "aa".to_string());
        assert_eq!(result[2], "bbb".to_string());
        assert_eq!(result[3], "1cc".to_string());
    }

    #[test]
    fn split_command_string_correctly_treats_double_quoted_arguments() {
        let input = "asdf \"ab bc\" ccc";
        let result = split_command_string(input);

        assert_eq!(result.len(), 3);
        assert_eq!(result[0], "asdf".to_string());
        assert_eq!(result[1], "ab bc".to_string());
        assert_eq!(result[2], "ccc".to_string());
    }

    #[test]
    fn split_command_string_respects_whitespace_in_double_quoted_arguments() {
        let input = "asdf \"ab   bc\" ccc";
        let result = split_command_string(input);

        assert_eq!(result.len(), 3);
        assert_eq!(result[0], "asdf".to_string());
        assert_eq!(result[1], "ab   bc".to_string());
        assert_eq!(result[2], "ccc".to_string());
    }

    #[test]
    fn split_command_string_treats_backslash_as_escape_sequence_in_double_quoted_arguments() {
        let input = "asdf \"ab \\bc\" ccc";
        let result = split_command_string(input);

        assert_eq!(result.len(), 3);
        assert_eq!(result[0], "asdf".to_string());
        assert_eq!(result[1], "ab bc".to_string());
        assert_eq!(result[2], "ccc".to_string());
    }

    #[test]
    fn split_command_string_correctly_treats_single_quoted_arguments() {
        let input = "asdf 'ab bc' ccc";
        let result = split_command_string(input);

        assert_eq!(result.len(), 3);
        assert_eq!(result[0], "asdf".to_string());
        assert_eq!(result[1], "ab bc".to_string());
        assert_eq!(result[2], "ccc".to_string());
    }

    #[test]
    fn split_command_string_respects_whitespace_in_single_quoted_arguments() {
        let input = "asdf 'ab   bc' ccc";
        let result = split_command_string(input);

        assert_eq!(result.len(), 3);
        assert_eq!(result[0], "asdf".to_string());
        assert_eq!(result[1], "ab   bc".to_string());
        assert_eq!(result[2], "ccc".to_string());
    }

    #[test]
    fn split_command_string_respects_backslashes_in_single_quoted_arguments() {
        let input = "asdf 'ab \\bc' ccc";
        let result = split_command_string(input);

        assert_eq!(result.len(), 3);
        assert_eq!(result[0], "asdf".to_string());
        assert_eq!(result[1], "ab \\bc".to_string());
        assert_eq!(result[2], "ccc".to_string());
    }
}
