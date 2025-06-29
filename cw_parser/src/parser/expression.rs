use std::ops::Range;

use winnow::{LocatingSlice, ModalResult, Parser, combinator::cut_err, error::StrContext};

use crate::{
    AstNode, AstOperator, AstString, AstValue, operator, quoted_or_unquoted_string, script_value,
    with_opt_trailing_ws,
};

#[derive(Debug)]
pub struct AstExpression<'a> {
    pub key: AstString<'a>,
    pub operator: AstOperator<'a>,
    pub value: AstValue<'a>,
    pub span: Range<usize>,
}

impl<'a> AstNode for AstExpression<'a> {
    fn span_range(&self) -> Range<usize> {
        self.span.clone()
    }
}

pub(crate) fn expression<'a>(input: &mut LocatingSlice<&'a str>) -> ModalResult<AstExpression<'a>> {
    let key = with_opt_trailing_ws(quoted_or_unquoted_string)
        .context(StrContext::Label("key"))
        .parse_next(input)?;
    let op = with_opt_trailing_ws(operator)
        .context(StrContext::Label("operator"))
        .parse_next(input)?;
    let value = cut_err(script_value)
        .context(StrContext::Label("expression value"))
        .parse_next(input)?;

    let span = key.span_range().start..value.span_range().end;

    Ok(AstExpression {
        key,
        operator: op,
        value,
        span,
    })
}
