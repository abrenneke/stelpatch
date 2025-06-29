#[cfg(test)]
mod strings_test {
    use winnow::{LocatingSlice, Parser};

    use super::super::super::*;

    #[test]
    fn test_unquoted_string_valid_input() {
        let mut input = LocatingSlice::new("hello123");
        let result = unquoted_string.parse_next(&mut input).unwrap();
        assert_eq!(
            result,
            AstString {
                value: AstToken {
                    value: "hello123",
                    span: 0..8,
                },
                is_quoted: false,
            }
        );
    }

    #[test]
    fn test_unquoted_string_invalid_input() {
        let mut input = LocatingSlice::new("invalid*identifier");
        let result = unquoted_string.parse_next(&mut input);
        assert!(result.is_err());
    }

    #[test]
    fn test_quoted_string_valid_input() {
        let mut input = LocatingSlice::new("\"hello world\"");
        let result = quoted_string.parse_next(&mut input).unwrap();
        assert_eq!(
            result,
            AstString {
                value: AstToken {
                    value: "hello world",
                    span: 0..13,
                },
                is_quoted: true,
            }
        );
    }

    #[test]
    fn test_quoted_string_valid_input_with_special_characters() {
        let mut input = LocatingSlice::new("\"a:b.c|d/e$f'g\"");
        let result = quoted_string.parse_next(&mut input).unwrap();
        assert_eq!(
            result,
            AstString {
                value: AstToken {
                    value: "a:b.c|d/e$f'g",
                    span: 0..15,
                },
                is_quoted: true,
            }
        );
    }

    #[test]
    fn test_quoted_string_invalid_input() {
        let mut input = LocatingSlice::new("\"invalid\"quote\"");
        let result = quoted_string.parse_next(&mut input);
        assert!(result.is_err());
    }

    #[test]
    fn test_quoted_or_unquoted_string_valid_input_unquoted() {
        let mut input = LocatingSlice::new("hello123");
        let result = quoted_or_unquoted_string.parse_next(&mut input).unwrap();
        assert_eq!(
            result,
            AstString {
                value: AstToken {
                    value: "hello123",
                    span: 0..8,
                },
                is_quoted: false,
            }
        );
    }

    #[test]
    fn test_quoted_or_unquoted_string_valid_input_quoted() {
        let mut input = LocatingSlice::new("\"hello world\"");
        let result = quoted_or_unquoted_string.parse_next(&mut input).unwrap();
        assert_eq!(
            result,
            AstString {
                value: AstToken {
                    value: "hello world",
                    span: 0..13,
                },
                is_quoted: true,
            }
        );
    }

    #[test]
    fn test_quoted_or_unquoted_string_valid_input_quoted_with_special_characters() {
        let mut input = LocatingSlice::new("\"a:b.c|d/e$f'g\"");
        let result = quoted_or_unquoted_string.parse_next(&mut input).unwrap();
        assert_eq!(
            result,
            AstString {
                value: AstToken {
                    value: "a:b.c|d/e$f'g",
                    span: 0..15,
                },
                is_quoted: true,
            }
        );
    }

    #[test]
    fn test_quoted_or_unquoted_string_invalid_input_unquoted() {
        let mut input = LocatingSlice::new("invalid*identifier");
        let result = quoted_or_unquoted_string.parse_next(&mut input);
        assert!(result.is_err());
    }

    #[test]
    fn test_quoted_empty_string() {
        let mut input = LocatingSlice::new("\"\"");
        let result = quoted_string.parse_next(&mut input).unwrap();
        assert_eq!(
            result,
            AstString {
                value: AstToken {
                    value: "",
                    span: 0..2,
                },
                is_quoted: true,
            }
        );
    }

    #[test]
    fn dynamic_script_value() {
        let mut input = LocatingSlice::new("$FLAG$");
        let result = unquoted_string.parse_next(&mut input).unwrap();
        assert_eq!(
            result,
            AstString {
                value: AstToken {
                    value: "$FLAG$",
                    span: 0..6,
                },
                is_quoted: false,
            }
        );
    }
}
