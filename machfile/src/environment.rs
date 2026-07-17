use std::{
    collections::HashMap,
    env::{self, current_dir},
    error::Error,
    fmt, fs,
    path::{Path, PathBuf},
};

use crate::config_finder::get_mach_file_path;

#[derive(Debug, PartialEq)]
pub enum EnvironmentParseError {
    NoEnvironmentFile,
    InvalidPath,
    CorruptEnvironmentFile,
    InvalidQuotedValue(String),
    WhitespaceAroundEquals,
}

impl fmt::Display for EnvironmentParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EnvironmentParseError::NoEnvironmentFile => write!(f, "no configuration file found"),
            EnvironmentParseError::InvalidPath => write!(f, "invalid directory path"),
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

/// The `Environment` holds the env file values inside `values`, while also exposing a
/// [`Environment::get`] method, that loads the value from the env file, falling back to the
/// existing environment.
#[derive(Debug)]
pub struct Environment {
    pub values: HashMap<String, String>,
}

impl Default for Environment {
    fn default() -> Self {
        if let Ok(cwd) = current_dir()
            && let Ok(mach_path) = get_mach_file_path(&cwd)
            && let Ok(e) = Environment::load_from_directory(&mach_path)
        {
            return e;
        };

        Environment {
            values: HashMap::new(),
        }
    }
}

impl Environment {
    /// Load the default `.env` file from the given `path`
    pub fn load_from_directory(path: &Path) -> Result<Self, EnvironmentParseError> {
        if !path.is_dir() {
            Err(EnvironmentParseError::InvalidPath)
        } else {
            let mut file_path = PathBuf::from(path);
            file_path.push(".env");
            Environment::load_file(&file_path)
        }
    }

    /// Load an env file from `path`
    pub fn load_file(path: &Path) -> Result<Self, EnvironmentParseError> {
        if !path.is_file() {
            return Err(EnvironmentParseError::NoEnvironmentFile);
        }

        let Ok(content) = fs::read_to_string(path) else {
            return Err(EnvironmentParseError::CorruptEnvironmentFile);
        };

        Self::parse(&content)
    }

    /// Get a value by name, either from the env file or the environment
    pub fn get(&self, name: &str) -> Option<String> {
        if let Some(value) = &self.values.get(name) {
            return Some(value.to_string());
        } else if let Ok(value) = env::var(name) {
            return Some(value);
        }
        None
    }

    /// Expand variables in `input`
    ///
    /// Uses variables from the environment and from the loaded
    /// env file
    #[must_use]
    pub fn expand_variables_in_str(&self, input: &str) -> String {
        let mut expanded = String::with_capacity(input.len());
        let mut chars = input.char_indices().peekable();
        while let Some((i, c)) = chars.next() {
            if c != '$' {
                expanded.push(c);
                continue;
            }

            match chars.peek() {
                Some(&(_, '{')) => {
                    // ${name}
                    chars.next();
                    let start = match chars.peek() {
                        Some(&(idx, _)) => idx,
                        None => {
                            // no closing brace; treat literally
                            expanded.push_str(&input[i..]);
                            break;
                        }
                    };
                    let mut finish = None;
                    for (i2, c2) in chars.by_ref() {
                        if c2 == '}' {
                            finish = Some(i2);
                            break;
                        }
                    }
                    if let Some(idx) = finish {
                        let variable_name = &input[start..idx];
                        if let Some(value) = self.get(variable_name) {
                            expanded.push_str(&value);
                        }
                    } else {
                        // no closing brace found: treat the remainder literally
                        expanded.push_str(&input[i..]);
                        break;
                    }
                }
                Some(&(_, c)) if is_var_start(c) => {
                    // $name
                    let start = match chars.peek() {
                        Some(&(idx, _)) => idx,
                        None => input.len(),
                    };
                    let mut finish = start;

                    // find end of variable name
                    while let Some(&(i2, c2)) = chars.peek() {
                        if is_var_char(c2) {
                            finish = i2;
                            chars.next();
                        } else {
                            break;
                        }
                    }

                    let end = if finish < input.len() {
                        let c = input[finish..].chars().next().unwrap();
                        finish + c.len_utf8()
                    } else {
                        input.len()
                    };

                    let variable_name = &input[start..end];

                    if let Some(value) = self.get(variable_name) {
                        expanded.push_str(&value);
                    }
                }
                _ => {
                    // free '$', treat literally
                    expanded.push('$');
                }
            }
        }
        expanded
    }

    fn parse(content: &str) -> Result<Self, EnvironmentParseError> {
        let mut result = Self::default();

        for line in content.lines() {
            if line.trim().is_empty() || line.chars().find(|c| !c.is_whitespace()) == Some('#') {
                // skip empty lines and comments
                continue;
            }

            let Some(parts) = line.split_once('=') else {
                return Err(EnvironmentParseError::CorruptEnvironmentFile);
            };

            if parts.1.trim().is_empty() {
                result.values.insert(parts.0.to_string(), String::new());
                continue;
            }

            if parts.0.ends_with(' ') || parts.1.starts_with(' ') {
                return Err(EnvironmentParseError::WhitespaceAroundEquals);
            }

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

        if !treated.starts_with('\'')
            && !treated.starts_with('"')
            && let Some(i) = treated.find(" # ")
        {
            treated = treated[0..i].to_string();
        }

        if treated.starts_with('"') {
            let Some(end) = find_end_of_quoted_value('"', &treated) else {
                return Err(EnvironmentParseError::InvalidQuotedValue(String::new()));
            };

            // Strip quotes
            treated = treated[1..end].to_string();

            // Replace allowed escape sequences
            treated = treated.replace("\\t", "\t");
            treated = treated.replace("\\n", "\n");
            treated = treated.replace("\\\\", "\\");
        }

        if treated.starts_with('\'') {
            let Some(end) = find_end_of_quoted_value('\'', &treated) else {
                return Err(EnvironmentParseError::InvalidQuotedValue(String::new()));
            };
            // Strip quotes
            treated = treated[1..end].to_string();
        } else {
            // Expand variables
            treated = self.expand_variables_in_str(&treated);
        }

        Ok(treated)
    }
}

fn is_var_start(c: char) -> bool {
    c == '_' || c.is_ascii_alphabetic()
}

fn is_var_char(c: char) -> bool {
    c == '_' || c.is_ascii_alphanumeric()
}

/// Find the closing quote of a value
fn find_end_of_quoted_value(quote: char, value: &str) -> Option<usize> {
    let mut chars = value.char_indices().peekable();
    let mut end = None;

    chars.next();

    while let Some((i, c)) = chars.next() {
        if c == '\\'
            && let Some((_, _)) = chars.next()
        {
            chars.next();
        }

        if c == quote {
            end = Some(i);
            break;
        }
    }

    end
}

#[cfg(test)]
mod tests {
    use std::env;

    use super::*;

    #[test]
    fn empty_file_returns_empty_hashmap() {
        let result = Environment::parse("");

        assert!(result.is_ok());
        assert!(result.unwrap().values.is_empty());
    }

    #[test]
    fn invalid_line_returns_error() {
        let result = Environment::parse("AAAA");

        assert!(result.is_err());
    }

    #[test]
    fn spaces_around_equal_sign_produces_error() {
        let mut result = Environment::parse("A =B");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            EnvironmentParseError::WhitespaceAroundEquals
        );

        result = Environment::parse("A= B");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            EnvironmentParseError::WhitespaceAroundEquals
        );
    }

    #[test]
    fn single_string_value_parses_correctly() {
        let result = Environment::parse("AB=A");

        assert!(result.is_ok());
        let content = result.unwrap().values;
        assert!(content.contains_key("AB"));
        assert_eq!(content.get("AB").unwrap(), "A");
    }

    #[test]
    fn single_number_value_parses_correctly() {
        let result = Environment::parse("CD=3");

        assert!(result.is_ok());
        let content = result.unwrap().values;
        assert!(content.contains_key("CD"));
        assert_eq!(content.get("CD").unwrap(), "3");
    }

    #[test]
    fn invalid_line_after_valid_line_returns_error() {
        let result = Environment::parse("A=B\nAAAA");

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            EnvironmentParseError::CorruptEnvironmentFile
        ));
    }

    #[test]
    fn empty_lines_are_skipped() {
        let result = Environment::parse("\nAB=A");

        assert!(result.is_ok());
        let content = result.unwrap().values;
        assert_eq!(content.keys().count(), 1);
        assert!(content.contains_key("AB"));
        assert_eq!(content.get("AB").unwrap(), "A");
    }

    #[test]
    fn lines_with_only_whitespace_are_skipped() {
        let result = Environment::parse("AB=A\n  ");

        assert!(result.is_ok());
        let content = result.unwrap().values;
        assert_eq!(content.keys().count(), 1);
        assert!(content.contains_key("AB"));
        assert_eq!(content.get("AB").unwrap(), "A");
    }

    #[test]
    fn comment_lines_are_skipped() {
        let result = Environment::parse("# COMMENT\n # AA\nAB=A");

        assert!(result.is_ok());
        let content = result.unwrap().values;
        assert_eq!(content.keys().count(), 1);
        assert!(content.contains_key("AB"));
        assert_eq!(content.get("AB").unwrap(), "A");
    }

    #[test]
    fn inline_comment_without_leading_space_is_part_of_value() {
        let result = Environment::default().parse_value("AA# test");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "AA# test");
    }

    #[test]
    fn inline_comment_without_following_space_is_part_of_value() {
        let result = Environment::default().parse_value("AA #test");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "AA #test");
    }

    #[test]
    fn inline_comment_with_space_is_stripped() {
        let result = Environment::default().parse_value("AA # test");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "AA");
    }

    #[test]
    fn multiple_values_parses_correctly() {
        let result = Environment::parse("AB=A\nCD=BD");

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
        let result = Environment::parse("AB=A\nAB=BD");

        assert!(result.is_ok());
        let content = result.unwrap().values;
        assert_eq!(content.keys().count(), 1);
        assert!(content.contains_key("AB"));
        assert_eq!(content.get("AB").unwrap(), "BD");
    }

    #[test]
    fn unquoted_values_are_trimmed() {
        let result = Environment::default().parse_value(" ASDD  ");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "ASDD");
    }

    #[test]
    fn missing_closing_quote_produces_error() {
        let result = Environment::default().parse_value("\"aaa");
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert_eq!(
            error,
            EnvironmentParseError::InvalidQuotedValue(String::new())
        );
    }

    #[test]
    fn missing_closing_quote_produces_throws_error_with_key() {
        let result = Environment::parse("A=\"aaa");
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert_eq!(
            error,
            EnvironmentParseError::InvalidQuotedValue("A".to_string())
        );
    }

    #[test]
    fn empty_value_sets_empty_string_as_final_value() {
        let result = Environment::parse("A=");

        assert!(result.is_ok());
        let content = result.unwrap().values;
        assert_eq!(content.keys().count(), 1);
        assert!(content.contains_key("A"));
        assert_eq!(content.get("A").unwrap(), "");
    }

    #[test]
    fn whitespace_value_sets_empty_string_as_final_value() {
        let result = Environment::parse("A=  ");

        assert!(result.is_ok());
        let content = result.unwrap().values;
        assert_eq!(content.keys().count(), 1);
        assert!(content.contains_key("A"));
        assert_eq!(content.get("A").unwrap(), "");
    }

    #[test]
    fn empty_single_quoted_value_sets_empty_string_as_final_value() {
        let result = Environment::parse("A=''");

        assert!(result.is_ok());
        let content = result.unwrap().values;
        assert_eq!(content.keys().count(), 1);
        assert!(content.contains_key("A"));
        assert_eq!(content.get("A").unwrap(), "");
    }

    #[test]
    fn empty_double_quoted_value_sets_empty_string_as_final_value() {
        let result = Environment::parse("A=\"\"");

        assert!(result.is_ok());
        let content = result.unwrap().values;
        assert_eq!(content.keys().count(), 1);
        assert!(content.contains_key("A"));
        assert_eq!(content.get("A").unwrap(), "");
    }

    #[test]
    fn double_quotes_are_removed_from_value() {
        let result = Environment::default().parse_value("\"ASDF\"");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "ASDF");
    }

    #[test]
    fn single_quotes_are_removed_from_value() {
        let result = Environment::default().parse_value("'ASDF'");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "ASDF");
    }

    #[test]
    fn single_quotes_strips_comment_after_final_quote() {
        let result = Environment::default().parse_value("'AA' # test");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "AA");
    }

    #[test]
    fn single_quotes_strips_whitespace_after_final_quote() {
        let result = Environment::default().parse_value("'AA'   ");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "AA");
    }

    #[test]
    fn single_quotes_strips_text_after_final_quote() {
        let result = Environment::default().parse_value("'AA'aaa");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "AA");
    }

    #[test]
    fn double_quotes_preserve_whitespace() {
        let result = Environment::default().parse_value("\" AA  \"");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), " AA  ");
    }

    #[test]
    fn double_quotes_allow_newline_escape_sequence() {
        let result = Environment::default().parse_value("\"A\\nA\"");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "A\nA");
    }

    #[test]
    fn double_quotes_allow_tab_escape_sequence() {
        let result = Environment::default().parse_value("\"A\\tA\"");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "A\tA");
    }

    #[test]
    fn double_quotes_allow_backslash_escape_sequence() {
        let result = Environment::default().parse_value("\"A\\\\A\"");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "A\\A");
    }

    #[test]
    fn double_quotes_strips_comment_after_final_quote() {
        let result = Environment::default().parse_value("\"A\\\\A\" # test");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "A\\A");
    }

    #[test]
    fn double_quotes_strips_whitespace_after_final_quote() {
        let result = Environment::default().parse_value("\"A\\\\A\"   ");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "A\\A");
    }

    #[test]
    fn double_quotes_strips_text_after_final_quote() {
        let result = Environment::default().parse_value("\"A\\\\A\"aaa");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "A\\A");
    }

    #[test]
    fn variable_expansion_works_in_unquoted_values() {
        let result = Environment::parse("A=AA\nB=$A");
        assert!(result.is_ok());
        let content = result.unwrap().values;
        assert_eq!(content.get("B").unwrap(), "AA");
    }

    #[test]
    fn variable_expansion_works_in_double_quoted_values() {
        let result = Environment::parse("A=AA\nB=\"$A\"");
        assert!(result.is_ok());
        let content = result.unwrap().values;
        assert_eq!(content.get("B").unwrap(), "AA");
    }

    #[test]
    fn variable_expansion_does_not_works_in_single_quoted_values() {
        let result = Environment::parse("A=AA\nB='$A'");
        assert!(result.is_ok());
        let content = result.unwrap().values;
        assert_eq!(content.get("B").unwrap(), "$A");
    }

    #[test]
    fn variable_expansion_inserts_empty_string_on_missing_key() {
        let result = Environment::parse("B=a $C b");
        assert!(result.is_ok());
        let content = result.unwrap().values;
        assert_eq!(content.get("B").unwrap(), "a  b");
    }

    #[test]
    fn variable_expansion_inserts_empty_string_on_missing_bracketed_key() {
        let result = Environment::parse("B=a${C}b");
        assert!(result.is_ok());
        let content = result.unwrap().values;
        assert_eq!(content.get("B").unwrap(), "ab");
    }

    #[test]
    fn variable_expansion_works_with_existing_environment_variables() {
        unsafe {
            env::set_var("VARIABLE_EXPANSION_TEST", "AA");
        }

        let result = Environment::parse("B=$VARIABLE_EXPANSION_TEST");
        assert!(result.is_ok());
        let content = result.unwrap().values;
        assert_eq!(content.get("B").unwrap(), "AA");
    }

    #[test]
    fn variable_expansion_works_with_brackets() {
        let result = Environment::parse("A=AA\nB=${A}");
        assert!(result.is_ok());
        let content = result.unwrap().values;
        assert_eq!(content.get("B").unwrap(), "AA");
    }

    #[test]
    fn variable_expansion_with_unclosed_bracket_doesnt_perform_substitution() {
        let result = Environment::parse("A=AA\nB=${A");
        assert!(result.is_ok());
        let content = result.unwrap().values;
        assert_eq!(content.get("B").unwrap(), "${A");
    }

    #[test]
    fn variable_expansion_with_brackets_allow_non_whitespace_afterwards() {
        let result = Environment::parse("A=AC\nB=${A}DC");
        assert!(result.is_ok());
        let content = result.unwrap().values;
        assert_eq!(content.get("B").unwrap(), "ACDC");
    }

    #[test]
    fn find_end_of_quoted_value_returns_none_when_missing_closing_quote() {
        let result = find_end_of_quoted_value('\'', "'aa");
        assert!(result.is_none());
    }

    #[test]
    fn find_end_of_quoted_value_works_with_single_quotes_basic() {
        let result = find_end_of_quoted_value('\'', "'aa'");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), 3);
    }

    #[test]
    fn find_end_of_quoted_value_works_with_single_quotes_with_following_text() {
        let result = find_end_of_quoted_value('\'', "'aa'bb");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), 3);
    }

    #[test]
    fn find_end_of_quoted_value_works_with_double_quotes_basic() {
        let result = find_end_of_quoted_value('"', "\"aa\"");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), 3);
    }

    #[test]
    fn find_end_of_quoted_value_works_with_double_quotes_with_following_text() {
        let result = find_end_of_quoted_value('"', "\"aa\"bb");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), 3);
    }
}
