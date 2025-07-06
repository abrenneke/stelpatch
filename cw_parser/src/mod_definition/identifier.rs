use winnow::{
    LocatingSlice, ModalResult, Parser,
    ascii::{alpha1, alphanumeric1},
    combinator::{alt, repeat},
    error::StrContext,
    token::literal,
};

use crate::AstToken;

pub(crate) fn identifier<'a>(input: &mut LocatingSlice<&'a str>) -> ModalResult<AstToken<'a>> {
    (
        alt((alpha1, literal("_"))),
        repeat(0.., alt((alphanumeric1, literal("_")))).map(|()| ()),
    )
        .take()
        .with_span()
        .map(|(s, span)| AstToken::new(s, span))
        .context(StrContext::Label("identifier"))
        .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_identifier() {
        let input = "name";
        let input = LocatingSlice::new(input);
        let result = identifier.parse(input);
        assert_eq!(result, Ok(AstToken::new("name", 0..4)));
    }
}
