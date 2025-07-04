use std::ops::Range;

use winnow::{
    LocatingSlice, ModalResult, Parser,
    ascii::digit1,
    combinator::{alt, opt, peek},
    error::StrContext,
    token::literal,
};

use crate::{
    AstComment, AstNode, AstToken, get_comments, opt_trailing_comment, opt_ws_and_comments,
    value_terminator,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AstNumber<'a> {
    pub value: AstToken<'a>,
    pub is_percentage: bool,
    pub leading_comments: Vec<AstComment<'a>>,
    pub trailing_comment: Option<AstComment<'a>>,
}

impl<'a> AstNumber<'a> {
    pub fn new(value: &'a str, span: Range<usize>) -> Self {
        Self {
            value: AstToken::new(value, span),
            is_percentage: false,
            leading_comments: vec![],
            trailing_comment: None,
        }
    }
}

impl<'a> AstNode<'a> for AstNumber<'a> {
    fn span_range(&self) -> Range<usize> {
        self.value.span.clone()
    }

    fn leading_comments(&self) -> &[AstComment<'a>] {
        &self.leading_comments
    }

    fn trailing_comment(&self) -> Option<&AstComment<'a>> {
        self.trailing_comment.as_ref()
    }
}

/// A number is a sequence of digits, optionally preceded by a sign and optionally followed by a decimal point and more digits, followed by whitespace.
pub(crate) fn number_val<'a>(input: &mut LocatingSlice<&'a str>) -> ModalResult<AstNumber<'a>> {
    let leading_comments = opt_ws_and_comments.parse_next(input)?;

    let (value, span) = (opt(alt(('-', '+'))), digit1, opt(('.', digit1)))
        .take()
        .with_span()
        .context(StrContext::Label("number_val"))
        .parse_next(input)?;

    // Look for a % sign, if it's there, consume it
    let is_percentage: ModalResult<()> = peek(literal("%")).void().parse_next(input);
    if is_percentage.is_ok() {
        literal("%").void().parse_next(input)?;
    }

    peek(value_terminator)
        .context(StrContext::Label("number_val terminator"))
        .parse_next(input)?;

    let trailing_comment = opt_trailing_comment.parse_next(input)?;

    Ok(AstNumber {
        value: AstToken::new(value, span),
        is_percentage: is_percentage.is_ok(),
        leading_comments: get_comments(&leading_comments),
        trailing_comment,
    })
}

#[cfg(test)]
mod tests {
    #![cfg(test)]
    use pretty_assertions::assert_eq;
    use winnow::{LocatingSlice, Parser};

    use super::super::super::*;

    #[test]
    fn test_number_val_valid_input() {
        let mut input = LocatingSlice::new("123  ");
        let result = number_val.parse_next(&mut input).unwrap();
        assert_eq!(result, AstNumber::new("123", 0..3));
    }

    #[test]
    fn test_number_val_negative_input() {
        let mut input = LocatingSlice::new("-12.34  ");
        let result = number_val.parse_next(&mut input).unwrap();
        assert_eq!(result, AstNumber::new("-12.34", 0..6));
    }

    #[test]
    fn test_number_val_positive_input() {
        let mut input = LocatingSlice::new("+12.34  ");
        let result = number_val.parse_next(&mut input).unwrap();
        assert_eq!(result, AstNumber::new("+12.34", 0..6));
    }

    #[test]
    fn test_number_val_decimal_input() {
        let mut input = LocatingSlice::new("3.14159  ");
        let result = number_val.parse_next(&mut input).unwrap();
        assert_eq!(result, AstNumber::new("3.14159", 0..7));
    }

    #[test]
    fn test_number_val_valid_input_with_comments() {
        let mut input = LocatingSlice::new("123# This is a comment");
        let result = number_val.parse_next(&mut input).unwrap();
        assert_eq!(
            result,
            AstNumber {
                value: AstToken::new("123", 0..3),
                is_percentage: false,
                leading_comments: vec![],
                trailing_comment: Some(AstComment::new(" This is a comment", 3..22)),
            }
        );
    }

    #[test]
    fn test_number_val_must_end_with_whitespace() {
        let mut input = LocatingSlice::new("123$");
        let result = number_val.parse_next(&mut input);
        assert!(result.is_err());
    }

    #[test]
    fn test_number_val_invalid_input() {
        let mut input = LocatingSlice::new("abc  ");
        let result = number_val.parse_next(&mut input);
        assert!(result.is_err());
    }

    #[test]
    fn does_not_parse_var_starts_with_number() {
        let mut input = LocatingSlice::new("1abc  ");
        let result = number_val.parse_next(&mut input);
        assert!(result.is_err());
    }

    #[test]
    fn number_with_comments() {
        let mut input = LocatingSlice::new(
            r#"
            # This is a leading comment
            # This is another leading comment
            123.4 # This is a trailing comment
        "#,
        );

        let result = number_val.parse_next(&mut input).unwrap();

        assert_eq!(
            result,
            AstNumber {
                value: AstToken::new("123.4", 99..104),
                is_percentage: false,
                leading_comments: vec![
                    AstComment::new(" This is a leading comment", 13..40),
                    AstComment::new(" This is another leading comment", 53..86),
                ],
                trailing_comment: Some(AstComment::new(" This is a trailing comment", 105..133)),
            }
        );
    }

    #[test]
    fn percentage() {
        let input = LocatingSlice::new("0.5%");
        let result = number_val.parse(input).unwrap();
        assert_eq!(
            result,
            AstNumber {
                value: AstToken::new("0.5", 0..3),
                is_percentage: true,
                leading_comments: vec![],
                trailing_comment: None,
            }
        );
    }
}
