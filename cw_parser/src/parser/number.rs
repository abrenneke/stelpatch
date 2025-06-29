use std::ops::Range;

use winnow::{
    LocatingSlice, ModalResult, Parser,
    ascii::digit1,
    combinator::{alt, opt, peek},
    error::StrContext,
};

use crate::{AstNode, AstToken, value_terminator};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AstNumber<'a> {
    pub value: AstToken<'a>,
}

impl<'a> AstNumber<'a> {
    pub fn new(value: &'a str, span: Range<usize>) -> Self {
        Self {
            value: AstToken { value, span },
        }
    }
}

impl<'a> AstNode for AstNumber<'a> {
    fn span_range(&self) -> Range<usize> {
        self.value.span.clone()
    }
}

/// A number is a sequence of digits, optionally preceded by a sign and optionally followed by a decimal point and more digits, followed by whitespace.
pub(crate) fn number_val<'a>(input: &mut LocatingSlice<&'a str>) -> ModalResult<AstNumber<'a>> {
    let (value, span) = (opt(alt(('-', '+'))), digit1, opt(('.', digit1)))
        .take()
        .with_span()
        .context(StrContext::Label("number_val"))
        .parse_next(input)?;

    peek(value_terminator)
        .context(StrContext::Label("number_val terminator"))
        .parse_next(input)?;

    Ok(AstNumber {
        value: AstToken { value, span },
    })
}
