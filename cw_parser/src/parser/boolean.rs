use std::ops::Range;

use winnow::{
    LocatingSlice, ModalResult, Parser, combinator::alt, error::StrContext, token::literal,
};

use crate::{AstComment, AstNode, AstToken, opt_trailing_comment, opt_ws_and_comments};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AstBoolean<'a> {
    pub value: AstToken<'a>,

    pub leading_comments: Vec<AstComment<'a>>,
    pub trailing_comment: Option<AstComment<'a>>,
}

impl<'a> AstBoolean<'a> {
    pub fn new(value: &'a str, span: Range<usize>) -> Self {
        Self {
            value: AstToken::new(value, span),
            leading_comments: vec![],
            trailing_comment: None,
        }
    }
}

impl<'a> AstNode<'a> for AstBoolean<'a> {
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

pub(crate) fn boolean<'a>(input: &mut LocatingSlice<&'a str>) -> ModalResult<AstBoolean<'a>> {
    let leading_comments = opt_ws_and_comments.parse_next(input)?;

    let (value, span) = alt((literal("true"), literal("false")))
        .context(StrContext::Label("boolean"))
        .with_span()
        .parse_next(input)?;

    let trailing_comment = opt_trailing_comment.parse_next(input)?;

    Ok(AstBoolean {
        value: AstToken::new(value, span),
        leading_comments,
        trailing_comment,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use winnow::{LocatingSlice, Parser};

    #[test]
    fn boolean_test() {
        let input = LocatingSlice::new("true");

        let result = boolean.parse(input).unwrap();

        assert_eq!(result, AstBoolean::new("true", 0..4));
    }

    #[test]
    fn boolean_test_false() {
        let input = LocatingSlice::new("false");

        let result = boolean.parse(input).unwrap();

        assert_eq!(result, AstBoolean::new("false", 0..5));
    }

    #[test]
    fn boolean_test_with_comments() {
        let input = LocatingSlice::new(
            r#"
            # This is a comment
            # This is another comment
            true # This is a trailing comment"#,
        );

        let result = boolean.parse(input).unwrap();

        assert_eq!(
            result,
            AstBoolean {
                value: AstToken::new("true", 83..87),
                leading_comments: vec![
                    AstComment::new(" This is a comment", 13..32),
                    AstComment::new(" This is another comment", 45..70),
                ],
                trailing_comment: Some(AstComment::new(" This is a trailing comment", 88..116)),
            }
        );
    }
}
