#[cfg(test)]
mod tests {
    use super::super::super::*;
    use nom_supreme::error::ErrorTree;

    #[test]
    fn test_define_valid_input_with_unquoted_string() {
        let (_, result) = define::<ErrorTree<_>>("@my_var = value").unwrap();
        assert_eq!(result.key, "@my_var");
        assert_eq!(result.operator, cw_model::Operator::Equals);
        assert_eq!(result.value, cw_model::Value::String("value".to_string()));
        assert_eq!(result.is_define, true);
    }

    #[test]
    fn test_define_valid_input_with_quoted_string() {
        let (_, result) = define::<ErrorTree<_>>("@my_var = \"value\"").unwrap();
        assert_eq!(result.key, "@my_var");
        assert_eq!(result.operator, cw_model::Operator::Equals);
        assert_eq!(result.value, cw_model::Value::String("value".to_string()));
        assert_eq!(result.is_define, true);
    }

    #[test]
    fn test_define_valid_input_with_number() {
        let (_, result) = define::<ErrorTree<_>>("@my_var = 123").unwrap();
        assert_eq!(result.key, "@my_var");
        assert_eq!(result.operator, cw_model::Operator::Equals);
        assert_eq!(result.value, cw_model::Value::Number(123.0));
        assert_eq!(result.is_define, true);
    }

    #[test]
    fn test_define_valid_input_with_float() {
        let (_, result) = define::<ErrorTree<_>>("@my_var = 123.4").unwrap();
        assert_eq!(result.key, "@my_var");
        assert_eq!(result.operator, cw_model::Operator::Equals);
        assert_eq!(result.value, cw_model::Value::Number(123.4));
        assert_eq!(result.is_define, true);
    }

    #[test]
    fn test_define_valid_input_with_color() {
        let (_, result) = define::<ErrorTree<_>>("@my_var = rgb { 1 2 3 }").unwrap();
        assert_eq!(result.key, "@my_var");
        assert_eq!(result.operator, cw_model::Operator::Equals);
        assert_eq!(
            result.value,
            cw_model::Value::Color(("rgb".to_string(), 1.0, 2.0, 3.0, None))
        );
        assert_eq!(result.is_define, true);
    }

    #[test]
    fn test_define_invalid_input_missing_value() {
        let result = define::<ErrorTree<_>>("@my_var =");
        assert!(result.is_err());
    }

    #[test]
    fn test_define_invalid_input_missing_key() {
        let result = define::<ErrorTree<_>>("@ = 123");
        assert!(result.is_err());
    }

    #[test]
    fn test_define_invalid_input_reference_value() {
        let result = define::<ErrorTree<_>>("@my_var = *other_var");
        assert!(result.is_err());
    }
}
