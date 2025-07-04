use std::ops::Range;

use winnow::{LocatingSlice, ModalResult, Parser, combinator::cut_err, error::StrContext};

use crate::{
    AstComment, AstNode, AstOperator, AstString, AstValue, operator, quoted_or_unquoted_string,
    script_value,
};

#[derive(Debug, PartialEq, Eq)]
pub struct AstExpression<'a> {
    pub key: AstString<'a>,
    pub operator: AstOperator<'a>,
    pub value: AstValue<'a>,
    pub span: Range<usize>,
}

impl<'a> AstExpression<'a> {
    pub fn new(key: AstString<'a>, operator: AstOperator<'a>, value: AstValue<'a>) -> Self {
        let span = key.span_range().start..value.span_range().end;
        Self {
            key,
            operator,
            value,
            span,
        }
    }
}

impl<'a> AstNode<'a> for AstExpression<'a> {
    fn span_range(&self) -> Range<usize> {
        self.span.clone()
    }

    fn leading_comments(&self) -> &[AstComment<'a>] {
        &[]
    }

    fn trailing_comment(&self) -> Option<&AstComment<'a>> {
        None
    }
}

pub(crate) fn expression<'a>(input: &mut LocatingSlice<&'a str>) -> ModalResult<AstExpression<'a>> {
    let ((key, op, value), span) = (
        quoted_or_unquoted_string.context(StrContext::Label("key")),
        operator.context(StrContext::Label("operator")),
        cut_err(script_value).context(StrContext::Label("expression value")),
    )
        .with_span()
        .context(StrContext::Label("expression"))
        .parse_next(input)?;

    Ok(AstExpression {
        key,
        operator: op,
        value,
        span,
    })
}
