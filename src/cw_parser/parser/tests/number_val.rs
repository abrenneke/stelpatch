#[cfg(test)]
mod number_val_tests {
    use super::super::super::*;
    use nom_supreme::error::ErrorTree;

    #[test]
    fn test_number_val_valid_input() {
        let result = number_val::<ErrorTree<_>>("123  ").unwrap();
        assert_eq!(result, ("  ", "123"));
    }

    #[test]
    fn test_number_val_negative_input() {
        let result = number_val::<ErrorTree<_>>("-12.34  ").unwrap();
        assert_eq!(result, ("  ", "-12.34"));
    }

    #[test]
    fn test_number_val_positive_input() {
        let result = number_val::<ErrorTree<_>>("+12.34  ").unwrap();
        assert_eq!(result, ("  ", "+12.34"));
    }

    #[test]
    fn test_number_val_decimal_input() {
        let result = number_val::<ErrorTree<_>>("3.14159  ").unwrap();
        assert_eq!(result, ("  ", "3.14159"));
    }

    #[test]
    fn test_number_val_valid_input_with_comments() {
        let result = number_val::<ErrorTree<_>>("123# This is a comment").unwrap();
        assert_eq!(result, ("# This is a comment", "123"));
    }

    #[test]
    fn test_number_val_must_end_with_whitespace() {
        let input = "123$";
        let result = number_val::<ErrorTree<_>>(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_number_val_invalid_input() {
        let input = "abc  ";
        let result = number_val::<ErrorTree<_>>(input);
        assert!(result.is_err());
    }

    #[test]
    fn does_not_parse_var_starts_with_number() {
        let input = "1abc  ";
        let result = number_val::<ErrorTree<_>>(input);
        assert!(result.is_err());
    }
}
