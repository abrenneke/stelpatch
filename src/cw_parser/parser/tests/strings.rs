#[cfg(test)]
mod strings_test {
    use super::super::super::*;
    use nom_supreme::error::ErrorTree;

    #[test]
    fn test_valid_identifier_char_valid_character() {
        let result = valid_identifier_char::<ErrorTree<_>>("a").unwrap();
        assert_eq!(result, ("", 'a'));
    }

    #[test]
    fn test_valid_identifier_char_valid_digit() {
        let result = valid_identifier_char::<ErrorTree<_>>("0").unwrap();
        assert_eq!(result, ("", '0'));
    }

    #[test]
    fn test_valid_identifier_char_valid_special_character() {
        let result = valid_identifier_char::<ErrorTree<_>>("_").unwrap();
        assert_eq!(result, ("", '_'));
    }

    #[test]
    fn test_valid_identifier_char_invalid_character() {
        let result = valid_identifier_char::<ErrorTree<_>>(" ");
        assert!(result.is_err());
    }

    #[test]
    fn test_unquoted_string_valid_input() {
        let result = unquoted_string::<ErrorTree<_>>("hello123").unwrap();
        assert_eq!(result, ("", "hello123"));
    }

    #[test]
    fn test_unquoted_string_invalid_input() {
        let result = unquoted_string::<ErrorTree<_>>("invalid*identifier");
        assert!(result.is_err());
    }

    #[test]
    fn test_quoted_string_valid_input() {
        let result = quoted_string::<ErrorTree<_>>("\"hello world\"").unwrap();
        assert_eq!(result, ("", "hello world"));
    }

    #[test]
    fn test_quoted_string_valid_input_with_special_characters() {
        let result = quoted_string::<ErrorTree<_>>("\"a:b.c|d/e$f'g\"").unwrap();
        assert_eq!(result, ("", "a:b.c|d/e$f'g"));
    }

    #[test]
    fn test_quoted_string_invalid_input() {
        let result = quoted_string::<ErrorTree<_>>("\"invalid\"quote\"");
        assert!(result.is_err());
    }

    #[test]
    fn test_quoted_or_unquoted_string_valid_input_unquoted() {
        let result = quoted_or_unquoted_string::<ErrorTree<_>>("hello123").unwrap();
        assert_eq!(result, ("", "hello123"));
    }

    #[test]
    fn test_quoted_or_unquoted_string_valid_input_quoted() {
        let result = quoted_or_unquoted_string::<ErrorTree<_>>("\"hello world\"").unwrap();
        assert_eq!(result, ("", "hello world"));
    }

    #[test]
    fn test_quoted_or_unquoted_string_valid_input_quoted_with_special_characters() {
        let result = quoted_or_unquoted_string::<ErrorTree<_>>("\"a:b.c|d/e$f'g\"").unwrap();
        assert_eq!(result, ("", "a:b.c|d/e$f'g"));
    }

    #[test]
    fn test_quoted_or_unquoted_string_invalid_input_unquoted() {
        let result = quoted_or_unquoted_string::<ErrorTree<_>>("invalid*identifier");
        assert!(result.is_err());
    }
}
