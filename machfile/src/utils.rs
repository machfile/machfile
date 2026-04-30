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

impl fmt::Display for ConfigParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigParseError::NoConfigFile => write!(f, "no configuration file found"),
            ConfigParseError::NoConfigFileInRepo => {
                write!(f, "no configuration file found in git repository")
            }
            ConfigParseError::CorruptConfigFile => {
                write!(f, "configuration file could not be parsed")
            }
            ConfigParseError::EmptyConfig => write!(f, "configuration file is empty"),
            ConfigParseError::InvalidTaskDefinition(e) => {
                write!(f, "task configuration is invalid: {e}")
            }
            ConfigParseError::MultipleConfigFiles => {
                write!(f, "several configuration files were found")
            }
            ConfigParseError::UnsupportedConfigFileExtension => {
                write!(f, "unsupported configuration file extension")
            }
            ConfigParseError::CircularDependencies => {
                write!(f, "tasks present circular dependencies")
            }
        }
    }
}

impl Error for ConfigParseError {}

/// This struct is used for errors during command execution
#[derive(Debug)]
pub struct CommandError {
    pub message: String,
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "command encountered an unexpected error: {}",
            self.message
        )
    }
}

impl Error for CommandError {}

#[derive(Debug)]
pub enum TaskNameError {
    LeadingUnderscore,
    LeadingHypen,
    InvalidCharacter(char),
    TooShort,
}

impl fmt::Display for TaskNameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskNameError::LeadingUnderscore => write!(f, "leading underscore is not allowed"),
            TaskNameError::LeadingHypen => write!(f, "leading hypen is not allowed"),
            TaskNameError::InvalidCharacter(i) => write!(f, "invalid character: '{i}''"),
            TaskNameError::TooShort => write!(f, "task name is to short"),
        }
    }
}

impl Error for TaskNameError {}

pub fn split_command_string(cmd_string: &str) -> Vec<String> {
    let mut arguments = vec![];

    let mut current_str = String::new();
    let mut is_double_quoted = false;
    let mut is_single_quoted = false;
    for c in cmd_string.chars() {
        if c == '"' && !is_single_quoted {
            is_double_quoted = !is_double_quoted;
            continue;
        }
        if c == '\'' && !is_double_quoted {
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

pub fn validate_task_name(name: &str) -> Result<(), TaskNameError> {
    if name.len() < 2 {
        return Err(TaskNameError::TooShort);
    }

    let invalid_characters = vec!['$', '\\', '{', '}', '\'', '"', '&'];

    let mut chars = name.chars();
    let mut char = chars.next().unwrap();
    if char == '_' {
        return Err(TaskNameError::LeadingUnderscore);
    }
    if char == '-' {
        return Err(TaskNameError::LeadingHypen);
    }

    loop {
        for c in &invalid_characters {
            if char == *c {
                return Err(TaskNameError::InvalidCharacter(char));
            }
        }

        if let Some(next) = chars.next() {
            char = next;
            continue;
        }

        break;
    }

    Ok(())
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

    #[test]
    fn split_command_string_allows_single_quotes_inside_double_qoutes() {
        let input = "asdf \"a' 'b\" ccc";
        let result = split_command_string(input);

        assert_eq!(result.len(), 3);
        assert_eq!(result[0], "asdf".to_string());
        assert_eq!(result[1], "a' 'b".to_string());
        assert_eq!(result[2], "ccc".to_string());
    }

    #[test]
    fn split_command_string_allows_double_quotes_inside_single_qoutes() {
        let input = "asdf 'a\" \"b' ccc";
        let result = split_command_string(input);

        assert_eq!(result.len(), 3);
        assert_eq!(result[0], "asdf".to_string());
        assert_eq!(result[1], "a\" \"b".to_string());
        assert_eq!(result[2], "ccc".to_string());
    }

    #[test]
    fn validate_task_name_rejects_empty_names() {
        let result = validate_task_name("");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TaskNameError::TooShort));
    }

    #[test]
    fn validate_task_name_rejects_leading_underscore() {
        let result = validate_task_name("_test");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TaskNameError::LeadingUnderscore
        ));
    }

    #[test]
    fn validate_task_name_rejects_leading_hypen() {
        let result = validate_task_name("-test");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TaskNameError::LeadingHypen));
    }

    #[test]
    fn validate_task_name_rejects_dollar_sign() {
        let result = validate_task_name("test$ab");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TaskNameError::InvalidCharacter('$')
        ));
    }

    #[test]
    fn validate_task_name_rejects_forbidden_characters_in_first_position() {
        let result = validate_task_name("$ab");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TaskNameError::InvalidCharacter('$')
        ));
    }

    #[test]
    fn validate_task_name_rejects_backslash() {
        let result = validate_task_name("tes\\ab");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TaskNameError::InvalidCharacter('\\')
        ));
    }

    #[test]
    fn validate_task_name_rejects_opening_brackets() {
        let result = validate_task_name("ab{asa");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TaskNameError::InvalidCharacter('{')
        ));
    }

    #[test]
    fn validate_task_name_rejects_closing_brackets() {
        let result = validate_task_name("tes}ab");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TaskNameError::InvalidCharacter('}')
        ));
    }

    #[test]
    fn validate_task_name_rejects_single_quote() {
        let result = validate_task_name("test'ab");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TaskNameError::InvalidCharacter('\'')
        ));
    }

    #[test]
    fn validate_task_name_rejects_double_quote() {
        let result = validate_task_name("test\"ab");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TaskNameError::InvalidCharacter('\"')
        ));
    }

    #[test]
    fn validate_task_name_rejects_ampersand() {
        let result = validate_task_name("tes&ab");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TaskNameError::InvalidCharacter('&')
        ));
    }

    #[test]
    fn validate_task_name_accepts_valid_names() {
        let names = vec!["task1", "task", "options", "run", "build", "do"];
        for name in &names {
            assert!(validate_task_name(name).is_ok());
        }
    }
}
