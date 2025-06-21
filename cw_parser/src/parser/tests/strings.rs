#[cfg(test)]
mod strings_test {
    use super::super::super::*;

    #[test]
    fn test_unquoted_string_valid_input() {
        let mut input = "hello123";
        let result = unquoted_string.parse_next(&mut input).unwrap();
        assert_eq!(result, "hello123");
    }

    #[test]
    fn test_unquoted_string_invalid_input() {
        let mut input = "invalid*identifier";
        let result = unquoted_string.parse_next(&mut input);
        assert!(result.is_err());
    }

    #[test]
    fn test_quoted_string_valid_input() {
        let mut input = "\"hello world\"";
        let result = quoted_string.parse_next(&mut input).unwrap();
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_quoted_string_valid_input_with_special_characters() {
        let mut input = "\"a:b.c|d/e$f'g\"";
        let result = quoted_string.parse_next(&mut input).unwrap();
        assert_eq!(result, "a:b.c|d/e$f'g");
    }

    #[test]
    fn test_quoted_string_invalid_input() {
        let mut input = "\"invalid\"quote\"";
        let result = quoted_string.parse_next(&mut input);
        assert!(result.is_err());
    }

    #[test]
    fn test_quoted_or_unquoted_string_valid_input_unquoted() {
        let mut input = "hello123";
        let result = quoted_or_unquoted_string.parse_next(&mut input).unwrap();
        assert_eq!(result, "hello123");
    }

    #[test]
    fn test_quoted_or_unquoted_string_valid_input_quoted() {
        let mut input = "\"hello world\"";
        let result = quoted_or_unquoted_string.parse_next(&mut input).unwrap();
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_quoted_or_unquoted_string_valid_input_quoted_with_special_characters() {
        let mut input = "\"a:b.c|d/e$f'g\"";
        let result = quoted_or_unquoted_string.parse_next(&mut input).unwrap();
        assert_eq!(result, "a:b.c|d/e$f'g");
    }

    #[test]
    fn test_quoted_or_unquoted_string_invalid_input_unquoted() {
        let mut input = "invalid*identifier";
        let result = quoted_or_unquoted_string.parse_next(&mut input);
        assert!(result.is_err());
    }

    #[test]
    fn test_quoted_empty_string() {
        let mut input = "\"\"";
        let result = quoted_string.parse_next(&mut input).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn dynamic_script_value() {
        let mut input = "$FLAG$";
        let result = unquoted_string.parse_next(&mut input).unwrap();
        assert_eq!(result, "$FLAG$");
    }
}
