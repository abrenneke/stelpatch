use std::ops::Range;

use winnow::LocatingSlice;
use winnow::combinator::opt;
use winnow::error::StrContext;
use winnow::{ModalResult, Parser};

use crate::{AstComment, AstNode, AstToken, ws_and_comments};

use super::identifier::identifier;
use super::value::{AstValue, value};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AstExpression<'a> {
    pub key: AstToken<'a>,
    pub value: AstValue<'a>,
    pub span: Range<usize>,

    pub leading_comments: Vec<AstComment<'a>>,
    pub trailing_comment: Option<AstComment<'a>>,
}

impl<'a> AstNode<'a> for AstExpression<'a> {
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

impl<'a> AstExpression<'a> {
    pub fn new(key: AstToken<'a>, value: AstValue<'a>, span: Range<usize>) -> Self {
        Self {
            key,
            value,
            span,
            leading_comments: vec![],
            trailing_comment: None,
        }
    }
}

pub(crate) fn expression<'a>(input: &mut LocatingSlice<&'a str>) -> ModalResult<AstExpression<'a>> {
    (
        identifier,
        opt(ws_and_comments),
        '='.context(StrContext::Label("=")),
        opt(ws_and_comments),
        value,
    )
        .with_span()
        .map(|((key, _, _, _, value), span)| AstExpression {
            key,
            value,
            span,
            leading_comments: vec![],
            trailing_comment: None,
        })
        .parse_next(input)
}

#[cfg(test)]
mod tests {
    use crate::AstString;
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn test_expression() {
        let input = "name = \"Expanded Stellaris Traditions\"";
        let input = LocatingSlice::new(input);
        let result = expression.parse(input);
        assert_eq!(
            result,
            Ok(AstExpression::new(
                AstToken::new("name", 0..4),
                AstValue::String(AstString::new("Expanded Stellaris Traditions", true, 7..38)),
                0..38
            ))
        );
    }

    #[test]
    fn compact_expression() {
        let input = "name=\"Expanded Stellaris Traditions\"";
        let input = LocatingSlice::new(input);
        let result = expression.parse(input);
        assert_eq!(
            result,
            Ok(AstExpression::new(
                AstToken::new("name", 0..4),
                AstValue::String(AstString::new("Expanded Stellaris Traditions", true, 5..36)),
                0..36
            ))
        );
    }
}
