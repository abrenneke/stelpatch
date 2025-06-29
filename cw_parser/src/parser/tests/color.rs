#[cfg(test)]
mod color_tests {
    use pretty_assertions::assert_eq;
    use winnow::{LocatingSlice, Parser};

    use super::super::super::*;

    #[test]
    fn test_color_valid_input_rgb() {
        let result = color
            .parse(LocatingSlice::new("rgb { 255 128 0 }"))
            .unwrap();
        assert_eq!(
            result,
            AstColor {
                color_type: AstToken::new("rgb", 0..3),
                r: AstNumber::new("255", 6..9),
                g: AstNumber::new("128", 10..13),
                b: AstNumber::new("0", 14..15),
                a: None,
                span: 0..17,
            }
        );
    }

    #[test]
    fn test_color_valid_input_hsv() {
        let result = color
            .parse(LocatingSlice::new("hsv { 120 0.5 1 }"))
            .unwrap();
        assert_eq!(
            result,
            AstColor {
                color_type: AstToken::new("hsv", 0..3),
                r: AstNumber::new("120", 6..9),
                g: AstNumber::new("0.5", 10..13),
                b: AstNumber::new("1", 14..15),
                a: None,
                span: 0..17,
            }
        );
    }

    #[test]
    fn test_color_valid_input_rgb_with_alpha() {
        let result = color
            .parse(LocatingSlice::new("rgb { 255 128 0 0.5 }"))
            .unwrap();
        assert_eq!(
            result,
            AstColor {
                color_type: AstToken::new("rgb", 0..3),
                r: AstNumber::new("255", 6..9),
                g: AstNumber::new("128", 10..13),
                b: AstNumber::new("0", 14..15),
                a: Some(AstNumber::new("0.5", 16..19)),
                span: 0..21,
            }
        );
    }

    #[test]
    fn test_color_valid_input_hsv_with_alpha() {
        let result = color
            .parse(LocatingSlice::new("hsv { 120 0.5 1 0.8 }"))
            .unwrap();
        assert_eq!(
            result,
            AstColor {
                color_type: AstToken::new("hsv", 0..3),
                r: AstNumber::new("120", 6..9),
                g: AstNumber::new("0.5", 10..13),
                b: AstNumber::new("1", 14..15),
                a: Some(AstNumber::new("0.8", 16..19)),
                span: 0..21,
            }
        );
    }

    #[test]
    fn test_color_valid_input_rgb_no_spaces() {
        let result = color.parse(LocatingSlice::new("rgb{255 128 0}")).unwrap();
        assert_eq!(
            result,
            AstColor {
                color_type: AstToken::new("rgb", 0..3),
                r: AstNumber::new("255", 4..7),
                g: AstNumber::new("128", 8..11),
                b: AstNumber::new("0", 12..13),
                a: None,
                span: 0..14,
            }
        );
    }

    #[test]
    fn test_color_valid_input_hsv_no_spaces() {
        let result = color.parse(LocatingSlice::new("hsv{120 0.5 1}")).unwrap();
        assert_eq!(
            result,
            AstColor {
                color_type: AstToken::new("hsv", 0..3),
                r: AstNumber::new("120", 4..7),
                g: AstNumber::new("0.5", 8..11),
                b: AstNumber::new("1", 12..13),
                a: None,
                span: 0..14,
            }
        );
    }

    #[test]
    fn test_color_valid_input_rgb_comments() {
        let result = color
            .parse(LocatingSlice::new(
                "rgb { 255 #red\n 128 #green\n 0 #blue\n }",
            ))
            .unwrap();
        assert_eq!(
            result,
            AstColor {
                color_type: AstToken::new("rgb", 0..3),
                r: AstNumber::new("255", 6..9),
                g: AstNumber::new("128", 16..19),
                b: AstNumber::new("0", 28..29),
                a: None,
                span: 0..38,
            }
        );
    }

    #[test]
    fn test_color_valid_input_hsv_comments() {
        let result = color
            .parse(LocatingSlice::new(
                "hsv { 120 #hue\n 0.5 #saturation\n 1 #value\n }",
            ))
            .unwrap();
        assert_eq!(
            result,
            AstColor {
                color_type: AstToken::new("hsv", 0..3),
                r: AstNumber::new("120", 6..9),
                g: AstNumber::new("0.5", 16..19),
                b: AstNumber::new("1", 33..34),
                a: None,
                span: 0..44,
            }
        );
    }

    #[test]
    fn test_color_invalid_input_missing_component() {
        let result = color.parse(LocatingSlice::new("rgb { 255 128 }"));
        assert!(result.is_err());
    }

    #[test]
    fn test_color_invalid_input_invalid_type() {
        let result = color.parse(LocatingSlice::new("cmyk { 100 50 0 }"));
        assert!(result.is_err());
    }
}
