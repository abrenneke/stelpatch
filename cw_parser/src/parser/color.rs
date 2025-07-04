use std::ops::Range;

use winnow::{
    LocatingSlice, ModalResult, Parser,
    combinator::{alt, cut_err, delimited, opt},
    error::StrContext,
    token::literal,
};

use crate::{
    AstComment, AstNode, AstNumber, AstToken, number_val, opt_trailing_comment,
    opt_ws_and_comments, with_opt_trailing_ws,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AstColor<'a> {
    pub color_type: AstToken<'a>,
    pub r: AstNumber<'a>,
    pub g: AstNumber<'a>,
    pub b: AstNumber<'a>,
    pub a: Option<AstNumber<'a>>,
    pub span: Range<usize>,

    pub leading_comments: Vec<AstComment<'a>>,
    pub trailing_comment: Option<AstComment<'a>>,
}

impl<'a> AstColor<'a> {
    pub fn new(
        color_type: &'a str,
        color_type_span: Range<usize>,
        r: &'a str,
        r_span: Range<usize>,
        g: &'a str,
        g_span: Range<usize>,
        b: &'a str,
        b_span: Range<usize>,
        a: Option<&'a str>,
        a_span: Option<Range<usize>>,
        span: Range<usize>,
    ) -> Self {
        Self {
            color_type: AstToken::new(color_type, color_type_span),
            r: AstNumber::new(r, r_span),
            g: AstNumber::new(g, g_span),
            b: AstNumber::new(b, b_span),
            a: a.map(|a| AstNumber::new(a, a_span.unwrap())),
            span,
            leading_comments: vec![],
            trailing_comment: None,
        }
    }
}

impl<'a> AstNode<'a> for AstColor<'a> {
    fn span_range(&self) -> Range<usize> {
        self.span.clone()
    }

    fn leading_comments(&self) -> &[AstComment<'a>] {
        &self.leading_comments
    }

    fn trailing_comment(&self) -> Option<&AstComment<'a>> {
        self.trailing_comment.as_ref()
    }
}

/// A color is either rgb { r g b a } or hsv { h s v a }. The a component is optional.
pub(crate) fn color<'a>(input: &mut LocatingSlice<&'a str>) -> ModalResult<AstColor<'a>> {
    let leading_comments = opt_ws_and_comments.parse_next(input)?;

    let (color_type, color_type_span) =
        with_opt_trailing_ws(alt((literal("rgb"), literal("hsv"))).with_span())
            .context(StrContext::Label("color type"))
            .parse_next(input)?;

    let start = color_type_span.start;

    let ((r, g, b, a), span) = delimited(
        '{',
        with_opt_trailing_ws(cut_err((
            number_val.context(StrContext::Label("color a")),
            number_val.context(StrContext::Label("color b")),
            number_val.context(StrContext::Label("color c")),
            opt(number_val).context(StrContext::Label("color d")),
        ))),
        '}',
    )
    .with_span()
    .context(StrContext::Label("color tuple"))
    .parse_next(input)?;

    let trailing_comment = opt_trailing_comment.parse_next(input)?;

    Ok(AstColor {
        color_type: AstToken::new(color_type, color_type_span),
        r,
        g,
        b,
        a,
        span: start..span.end,
        leading_comments,
        trailing_comment,
    })
}

#[cfg(test)]
mod tests {
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
                leading_comments: vec![],
                trailing_comment: None,
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
                leading_comments: vec![],
                trailing_comment: None,
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
                leading_comments: vec![],
                trailing_comment: None,
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
                leading_comments: vec![],
                trailing_comment: None,
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
                leading_comments: vec![],
                trailing_comment: None,
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
                leading_comments: vec![],
                trailing_comment: None,
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
                r: AstNumber {
                    value: AstToken::new("255", 6..9),
                    leading_comments: vec![],
                    trailing_comment: Some(AstComment::new("red", 10..14)),
                    is_percentage: false,
                },
                g: AstNumber {
                    value: AstToken::new("128", 16..19),
                    leading_comments: vec![],
                    trailing_comment: Some(AstComment::new("green", 20..26)),
                    is_percentage: false,
                },
                b: AstNumber {
                    value: AstToken::new("0", 28..29),
                    leading_comments: vec![],
                    trailing_comment: Some(AstComment::new("blue", 30..35)),
                    is_percentage: false,
                },
                a: None,
                span: 0..38,
                leading_comments: vec![],
                trailing_comment: None,
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
                r: AstNumber {
                    value: AstToken::new("120", 6..9),
                    leading_comments: vec![],
                    trailing_comment: Some(AstComment::new("hue", 10..14)),
                    is_percentage: false,
                },
                g: AstNumber {
                    value: AstToken::new("0.5", 16..19),
                    leading_comments: vec![],
                    trailing_comment: Some(AstComment::new("saturation", 20..31)),
                    is_percentage: false,
                },
                b: AstNumber {
                    value: AstToken::new("1", 33..34),
                    leading_comments: vec![],
                    trailing_comment: Some(AstComment::new("value", 35..41)),
                    is_percentage: false,
                },
                a: None,
                span: 0..44,
                leading_comments: vec![],
                trailing_comment: None,
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

    #[test]
    fn color_with_comments() {
        let mut input = LocatingSlice::new(
            r#"
            # This is a leading comment
            # This is another leading comment
            rgb { 
                # Leading r
                255 # Trailing r
                # Leading g
                128 # Trailing g
                # Leading b
                0 # Trailing b
                # Leading a
                0.5 # Trailing a
            } # This is a trailing comment
        "#,
        );

        let result = color.parse_next(&mut input).unwrap();

        assert_eq!(
            result,
            AstColor {
                leading_comments: vec![
                    AstComment::new(" This is a leading comment", 13..40),
                    AstComment::new(" This is another leading comment", 53..86),
                ],
                trailing_comment: Some(AstComment::new(" This is a trailing comment", 362..390)),
                color_type: AstToken::new("rgb", 99..102),
                r: AstNumber {
                    value: AstToken::new("255", 150..153),
                    leading_comments: vec![AstComment::new(" Leading r", 122..133),],
                    trailing_comment: Some(AstComment::new(" Trailing r", 154..166)),
                    is_percentage: false,
                },
                g: AstNumber {
                    value: AstToken::new("128", 211..214),
                    leading_comments: vec![AstComment::new(" Leading g", 183..194),],
                    trailing_comment: Some(AstComment::new(" Trailing g", 215..227)),
                    is_percentage: false,
                },
                b: AstNumber {
                    value: AstToken::new("0", 272..273),
                    leading_comments: vec![AstComment::new(" Leading b", 244..255),],
                    trailing_comment: Some(AstComment::new(" Trailing b", 274..286)),
                    is_percentage: false,
                },
                a: Some(AstNumber {
                    value: AstToken::new("0.5", 331..334),
                    leading_comments: vec![AstComment::new(" Leading a", 303..314),],
                    trailing_comment: Some(AstComment::new(" Trailing a", 335..347)),
                    is_percentage: false,
                }),
                span: 99..361,
            }
        );
    }
}
