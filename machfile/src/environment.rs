use std::{collections::HashMap, error::Error, fmt, path::Path};

#[derive(Debug, PartialEq)]
pub enum EnvironmentParseError {
    NoEnvironmentFile,
    CorruptEnvironmentFile,
    InvalidQuotedValue(String),
    WhitespaceAroundEquals,
}

impl fmt::Display for EnvironmentParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EnvironmentParseError::NoEnvironmentFile => write!(f, "no configuration file found"),
            EnvironmentParseError::CorruptEnvironmentFile => {
                write!(f, "configuration file could not be parsed")
            }
            EnvironmentParseError::InvalidQuotedValue(key) => {
                write!(f, "key '{key}' has invalid quoted value")
            }
            EnvironmentParseError::WhitespaceAroundEquals => {
                write!(f, "whitespace around = is not allowed")
            }
        }
    }
}

impl Error for EnvironmentParseError {}

#[derive(Debug, Default)]
pub struct EnvironmentFile {
    pub values: HashMap<String, String>,
}

impl EnvironmentFile {
    pub fn parse_file(path: &Path) -> Result<Self, EnvironmentParseError> {
        // TODO: read from file
        Self::parse("")
    }

    fn parse(content: &str) -> Result<Self, EnvironmentParseError> {
        let mut result = Self::default();

        for line in content.lines() {
            if line.trim().is_empty() || line.chars().find(|c| !c.is_whitespace()) == Some('#') {
                // skip empty lines and comments
                continue;
            }

            let Some(equal_sign) = line.find("=") else {
                return Err(EnvironmentParseError::CorruptEnvironmentFile);
            };

            if line.chars().nth(equal_sign - 1).unwrap().is_whitespace()
                || line.chars().nth(equal_sign + 1).unwrap().is_whitespace()
            {
                return Err(EnvironmentParseError::WhitespaceAroundEquals);
            }

            let parts = line.split_once('=').unwrap();

            let parsed_value = match result.parse_value(parts.1) {
                Ok(val) => val,
                Err(error) => match error {
                    EnvironmentParseError::InvalidQuotedValue(_) => {
                        return Err(EnvironmentParseError::InvalidQuotedValue(
                            parts.0.to_string(),
                        ));
                    }
                    err => return Err(err),
                },
            };

            result.values.insert(parts.0.to_string(), parsed_value);
        }

        Ok(result)
    }

    fn parse_value(&self, value: &str) -> Result<String, EnvironmentParseError> {
        let mut treated = value.trim().to_string();

        if treated.starts_with('"') {
            if !treated.ends_with('"') {
                return Err(EnvironmentParseError::InvalidQuotedValue(String::new()));
            }

            treated = treated[1..treated.len() - 1].to_string();

            treated = treated.replace("\\t", "\t");
            treated = treated.replace("\\n", "\n");
            treated = treated.replace("\\\\", "\\");
        }

        if treated.starts_with('\'') {
            if !treated.ends_with('\'') {
                return Err(EnvironmentParseError::InvalidQuotedValue(String::new()));
            }
        } else {
            // TODO: variable expansion
        }

        Ok(treated)
    }
}

#[cfg(test)]
mod tests {
    use std::env;

    use super::*;

    #[test]
    fn empty_file_returns_empty_hashmap() {
        let result = EnvironmentFile::parse("");

        assert!(result.is_ok());
        assert!(result.unwrap().values.is_empty());
    }

    #[test]
    fn invalid_line_returns_error() {
        let result = EnvironmentFile::parse("AAAA");

        assert!(result.is_err());
    }

    #[test]
    fn spaces_around_equal_sign_produces_error() {
        let mut result = EnvironmentFile::parse("A =B");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            EnvironmentParseError::WhitespaceAroundEquals
        );

        result = EnvironmentFile::parse("A= B");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            EnvironmentParseError::WhitespaceAroundEquals
        );
    }

    #[test]
    fn single_string_value_parses_correctly() {
        let result = EnvironmentFile::parse("AB=A");

        assert!(result.is_ok());
        let content = result.unwrap().values;
        assert!(content.contains_key("AB"));
        assert_eq!(content.get("AB").unwrap(), "A");
    }

    #[test]
    fn single_number_value_parses_correctly() {
        let result = EnvironmentFile::parse("CD=3");

        assert!(result.is_ok());
        let content = result.unwrap().values;
        assert!(content.contains_key("CD"));
        assert_eq!(content.get("CD").unwrap(), "3");
    }

    #[test]
    fn invalid_line_after_valid_line_returns_error() {
        let result = EnvironmentFile::parse("A=B\nAAAA");

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            EnvironmentParseError::CorruptEnvironmentFile
        ));
    }

    #[test]
    fn empty_lines_are_skipped() {
        let result = EnvironmentFile::parse("\nAB=A");

        assert!(result.is_ok());
        let content = result.unwrap().values;
        assert_eq!(content.keys().count(), 1);
        assert!(content.contains_key("AB"));
        assert_eq!(content.get("AB").unwrap(), "A");
    }

    #[test]
    fn lines_with_only_whitespace_are_skipped() {
        let result = EnvironmentFile::parse("AB=A\n  ");

        assert!(result.is_ok());
        let content = result.unwrap().values;
        assert_eq!(content.keys().count(), 1);
        assert!(content.contains_key("AB"));
        assert_eq!(content.get("AB").unwrap(), "A");
    }

    #[test]
    fn comment_lines_are_skipped() {
        let result = EnvironmentFile::parse("# COMMENT\n # AA\nAB=A");

        assert!(result.is_ok());
        let content = result.unwrap().values;
        assert_eq!(content.keys().count(), 1);
        assert!(content.contains_key("AB"));
        assert_eq!(content.get("AB").unwrap(), "A");
    }

    #[test]
    fn multiple_values_parses_correctly() {
        let result = EnvironmentFile::parse("AB=A\nCD=BD");

        assert!(result.is_ok());
        let content = result.unwrap().values;
        assert_eq!(content.keys().count(), 2);
        assert!(content.contains_key("AB"));
        assert_eq!(content.get("AB").unwrap(), "A");
        assert!(content.contains_key("CD"));
        assert_eq!(content.get("CD").unwrap(), "BD");
    }

    #[test]
    fn repetitions_of_the_same_key_override_value() {
        let result = EnvironmentFile::parse("AB=A\nAB=BD");

        assert!(result.is_ok());
        let content = result.unwrap().values;
        assert_eq!(content.keys().count(), 1);
        assert!(content.contains_key("AB"));
        assert_eq!(content.get("AB").unwrap(), "BD");
    }

    #[test]
    fn unquoted_values_are_trimmed() {
        let result = EnvironmentFile::default().parse_value(" ASDD  ");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "ASDD");
    }

    #[test]
    fn missing_closing_quote_produces_error() {
        let result = EnvironmentFile::default().parse_value("\"aaa");
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert_eq!(
            error,
            EnvironmentParseError::InvalidQuotedValue(String::new())
        );
    }

    #[test]
    fn missing_closing_quote_produces_throws_error_with_key() {
        let result = EnvironmentFile::parse("A=\"aaa");
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert_eq!(
            error,
            EnvironmentParseError::InvalidQuotedValue("A".to_string())
        );
    }

    #[test]
    fn double_quotes_are_removed_from_value() {
        let result = EnvironmentFile::default().parse_value("\"ASDF\"");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "ASDF");
    }

    #[test]
    fn double_quotes_preserve_whitespace() {
        let result = EnvironmentFile::default().parse_value("\" AA  \"");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), " AA  ");
    }

    #[test]
    fn double_quotes_allow_newline_escape_sequence() {
        let result = EnvironmentFile::default().parse_value("\"A\\nA\"");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "A\nA");
    }

    #[test]
    fn double_quotes_allow_tab_escape_sequence() {
        let result = EnvironmentFile::default().parse_value("\"A\\tA\"");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "A\tA");
    }

    #[test]
    fn double_quotes_allow_backslash_escape_sequence() {
        let result = EnvironmentFile::default().parse_value("\"A\\\\A\"");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "A\\A");
    }

    #[test]
    fn variable_expansion_works_in_unquoted_values() {
        let result = EnvironmentFile::parse("A=AA\nB=$A");
        assert!(result.is_ok());
        let content = result.unwrap().values;
        assert_eq!(content.get("B").unwrap(), "AA");
    }

    #[test]
    fn variable_expansion_works_in_double_quoted_values() {
        let result = EnvironmentFile::parse("A=AA\nB=\"$A\"");
        assert!(result.is_ok());
        let content = result.unwrap().values;
        assert_eq!(content.get("B").unwrap(), "AA");
    }

    #[test]
    fn variable_expansion_does_not_works_in_single_quoted_values() {
        let result = EnvironmentFile::parse("A=AA\nB='$A'");
        assert!(result.is_ok());
        let content = result.unwrap().values;
        assert_eq!(content.get("B").unwrap(), "$A");
    }

    #[test]
    fn variable_expansion_inserts_empty_string_on_missing_key() {
        let result = EnvironmentFile::parse("B=a $C b");
        assert!(result.is_ok());
        let content = result.unwrap().values;
        assert_eq!(content.get("B").unwrap(), "a  b");
    }

    #[test]
    fn variable_expansion_works_with_existing_environment_variables() {
        unsafe {
            env::set_var("VARIABLE_EXPANSION_TEST", "AA");
        }

        let result = EnvironmentFile::parse("B=$VARIABLE_EXPANSION_TEST");
        assert!(result.is_ok());
        let content = result.unwrap().values;
        assert_eq!(content.get("B").unwrap(), "AA");
    }
}
