use std::collections::HashMap;

fn parse_environment_file(content: &str) -> Result<HashMap<String, String>, ()> {
    let mut result = HashMap::new();

    for line in content.lines() {
        if line.is_empty() || line.chars().find(|c| !c.is_whitespace()) == Some('#') {
            // skip empty lines and comments
            continue;
        }

        let Some(parts) = line.split_once('=') else {
            return Err(());
        };

        result.insert(parts.0.to_string(), parts.1.to_string());
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_file_returns_empty_hashmap() {
        let result = parse_environment_file("");

        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn invalid_line_returns_error() {
        let result = parse_environment_file("AAAA");

        assert!(result.is_err());
    }

    #[test]
    fn single_string_value_parses_correctly() {
        let result = parse_environment_file("AB=A");

        assert!(result.is_ok());
        let content = result.unwrap();
        assert!(content.contains_key("AB"));
        assert_eq!(content.get("AB").unwrap(), "A");
    }

    #[test]
    fn single_number_value_parses_correctly() {
        let result = parse_environment_file("CD=3");

        assert!(result.is_ok());
        let content = result.unwrap();
        assert!(content.contains_key("CD"));
        assert_eq!(content.get("CD").unwrap(), "3");
    }

    #[test]
    fn invalid_line_after_valid_line_returns_error() {
        let result = parse_environment_file("A=B\nAAAA");

        assert!(result.is_err());
    }

    #[test]
    fn empty_lines_are_skipped() {
        let result = parse_environment_file("\nAB=A");

        assert!(result.is_ok());
        let content = result.unwrap();
        assert_eq!(content.keys().count(), 1);
        assert!(content.contains_key("AB"));
        assert_eq!(content.get("AB").unwrap(), "A");
    }

    #[test]
    fn comment_lines_are_skipped() {
        let result = parse_environment_file("# COMMENT\n # AA\nAB=A");

        assert!(result.is_ok());
        let content = result.unwrap();
        assert_eq!(content.keys().count(), 1);
        assert!(content.contains_key("AB"));
        assert_eq!(content.get("AB").unwrap(), "A");
    }

    #[test]
    fn multiple_values_parses_correctly() {
        let result = parse_environment_file("AB=A\nCD=BD");

        assert!(result.is_ok());
        let content = result.unwrap();
        assert_eq!(content.keys().count(), 2);
        assert!(content.contains_key("AB"));
        assert_eq!(content.get("AB").unwrap(), "A");
        assert!(content.contains_key("CD"));
        assert_eq!(content.get("CD").unwrap(), "BD");
    }

    #[test]
    fn repetitions_of_the_same_key_override_value() {
        let result = parse_environment_file("AB=A\nAB=BD");

        assert!(result.is_ok());
        let content = result.unwrap();
        assert_eq!(content.keys().count(), 1);
        assert!(content.contains_key("AB"));
        assert_eq!(content.get("AB").unwrap(), "BD");
    }
}
