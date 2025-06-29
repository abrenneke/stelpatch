use winnow::{LocatingSlice, ModalResult, Parser, combinator::cut_err, error::StrContext};

use crate::{
    AstOperator, AstString, AstValue, operator, quoted_or_unquoted_string, script_value,
    with_opt_trailing_ws,
};

#[derive(Debug)]
pub struct AstExpression<'a> {
    pub key: AstString<'a>,
    pub operator: AstOperator<'a>,
    pub value: AstValue<'a>,
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

    Ok(AstExpression {
        key,
        operator: op,
        value,
    })
}
