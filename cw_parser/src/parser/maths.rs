use std::ops::Range;

use winnow::{
    LocatingSlice, ModalResult, Parser,
    combinator::alt,
    error::StrContext,
    token::{literal, take_till},
};

use crate::{
    AstComment, AstNode, AstToken, opt_trailing_comment, opt_ws_and_comments, with_opt_trailing_ws,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AstMaths<'a> {
    pub value: AstToken<'a>,

    pub leading_comments: Vec<AstComment<'a>>,
    pub trailing_comment: Option<AstComment<'a>>,
}

impl<'a> AstMaths<'a> {
    pub fn new(value: &'a str, span: Range<usize>) -> Self {
        Self {
            value: AstToken::new(value, span),
            leading_comments: vec![],
            trailing_comment: None,
        }
    }
}

impl<'a> AstNode<'a> for AstMaths<'a> {
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

/// Insanity, inline math like @[x + 1], we don't really care about the formula inside, just that it's there.
pub(crate) fn inline_maths<'a>(input: &mut LocatingSlice<&'a str>) -> ModalResult<AstMaths<'a>> {
    let leading_comments = opt_ws_and_comments.parse_next(input)?;

    let (value, span) = (
        with_opt_trailing_ws(alt((literal("@["), literal("@\\[")))),
        take_till(0.., ']'),
        ']',
    )
        .take()
        .with_span()
        .context(StrContext::Label("inline_maths"))
        .parse_next(input)?;

    let trailing_comment = opt_trailing_comment.parse_next(input)?;

    Ok(AstMaths {
        value: AstToken::new(value, span),
        leading_comments,
        trailing_comment,
    })
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use winnow::{LocatingSlice, Parser};

    use crate::{AstMaths, parser::inline_maths};

    #[test]
    fn inline_maths_test() {
        let input = LocatingSlice::new("@[ stabilitylevel2 + 10 ]");

        let result = inline_maths.parse(input).unwrap();

        assert_eq!(result, AstMaths::new("@[ stabilitylevel2 + 10 ]", 0..25));
    }

    #[test]
    fn inline_maths_alt_test() {
        let input = LocatingSlice::new("@\\[ stabilitylevel2 + 10 ]");

        let result = inline_maths.parse(input).unwrap();

        assert_eq!(result, AstMaths::new("@\\[ stabilitylevel2 + 10 ]", 0..26));
    }
}
