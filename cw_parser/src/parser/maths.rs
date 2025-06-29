use std::ops::Range;

use winnow::{
    LocatingSlice, ModalResult, Parser,
    combinator::alt,
    error::StrContext,
    token::{literal, take_till},
};

use crate::{AstToken, with_opt_trailing_ws};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AstMaths<'a> {
    pub value: AstToken<'a>,
}

impl<'a> AstMaths<'a> {
    pub fn new(value: &'a str, span: Range<usize>) -> Self {
        Self {
            value: AstToken { value, span },
        }
    }
}

/// Insanity, inline math like @[x + 1], we don't really care about the formula inside, just that it's there.
pub(crate) fn inline_maths<'a>(input: &mut LocatingSlice<&'a str>) -> ModalResult<AstMaths<'a>> {
    (
        with_opt_trailing_ws(alt((literal("@["), literal("@\\[")))),
        take_till(0.., ']'),
        ']',
    )
        .take()
        .with_span()
        .context(StrContext::Label("inline_maths"))
        .parse_next(input)
        .map(|(value, span)| AstMaths {
            value: AstToken { value, span },
        })
}
