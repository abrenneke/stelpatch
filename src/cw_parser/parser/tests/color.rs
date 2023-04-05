#[cfg(test)]
mod color_tests {
    use super::super::super::*;
    use nom_supreme::error::ErrorTree;

    #[test]
    fn test_color_valid_input_rgb() {
        let result = color::<ErrorTree<_>>("rgb { 255 128 0 }").unwrap();
        assert_eq!(
            result,
            (
                "",
                (
                    "rgb".to_string(),
                    "255".to_string(),
                    "128".to_string(),
                    "0.0".to_string(),
                    None
                )
            )
        );
    }

    #[test]
    fn test_color_valid_input_hsv() {
        let result = color::<ErrorTree<_>>("hsv { 120 0.5 1 }").unwrap();
        assert_eq!(
            result,
            (
                "",
                (
                    "hsv".to_string(),
                    "120".to_string(),
                    "0.5".to_string(),
                    "1.0".to_string(),
                    None
                )
            )
        );
    }

    #[test]
    fn test_color_valid_input_rgb_with_alpha() {
        let result = color::<ErrorTree<_>>("rgb { 255 128 0 0.5 }").unwrap();
        assert_eq!(
            result,
            (
                "",
                (
                    "rgb".to_string(),
                    "255".to_string(),
                    "128".to_string(),
                    "0".to_string(),
                    Some("0.5".to_string())
                )
            )
        );
    }

    #[test]
    fn test_color_valid_input_hsv_with_alpha() {
        let result = color::<ErrorTree<_>>("hsv { 120 0.5 1 0.8 }").unwrap();
        assert_eq!(
            result,
            (
                "",
                (
                    "hsv".to_string(),
                    "120".to_string(),
                    "0.5".to_string(),
                    "1".to_string(),
                    Some("0.8".to_string())
                )
            )
        );
    }

    #[test]
    fn test_color_valid_input_rgb_no_spaces() {
        let result = color::<ErrorTree<_>>("rgb{255 128 0}").unwrap();
        assert_eq!(
            result,
            (
                "",
                (
                    "rgb".to_string(),
                    "255.0".to_string(),
                    "128.0".to_string(),
                    "0.0".to_string(),
                    None
                )
            )
        );
    }

    #[test]
    fn test_color_valid_input_hsv_no_spaces() {
        let result = color::<ErrorTree<_>>("hsv{120 0.5 1}").unwrap();
        assert_eq!(
            result,
            (
                "",
                (
                    "hsv".to_string(),
                    "120.0".to_string(),
                    "0.5".to_string(),
                    "1.0".to_string(),
                    None
                )
            )
        );
    }

    #[test]
    fn test_color_valid_input_rgb_comments() {
        let result = color::<ErrorTree<_>>("rgb { 255 #red\n 128 #green\n 0 #blue\n }").unwrap();
        assert_eq!(
            result,
            (
                "",
                (
                    "rgb".to_string(),
                    "255.0".to_string(),
                    "128.0".to_string(),
                    "0.0".to_string(),
                    None
                )
            )
        );
    }

    #[test]
    fn test_color_valid_input_hsv_comments() {
        let result =
            color::<ErrorTree<_>>("hsv { 120 #hue\n 0.5 #saturation\n 1 #value\n }").unwrap();
        assert_eq!(
            result,
            (
                "",
                (
                    "hsv".to_string(),
                    "120.0".to_string(),
                    "0.5".to_string(),
                    "1.0".to_string(),
                    None
                )
            )
        );
    }

    #[test]
    fn test_color_invalid_input_missing_component() {
        let result = color::<ErrorTree<_>>("rgb { 255 128 }");
        assert!(result.is_err());
    }

    #[test]
    fn test_color_invalid_input_invalid_type() {
        let result = color::<ErrorTree<_>>("cmyk { 100 50 0 }");
        assert!(result.is_err());
    }
}
