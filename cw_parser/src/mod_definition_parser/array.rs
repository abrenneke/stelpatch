use std::ops::Range;

use winnow::{
    LocatingSlice, ModalResult, Parser,
    combinator::{alt, delimited, opt, peek, repeat},
    error::StrContext,
    token::literal,
};

use super::string::string_value;
use crate::{AstNode, AstString, ws_and_comments};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AstArrayValue<'a> {
    values: Vec<AstString<'a>>,
    span: Range<usize>,
}

impl<'a> AstNode for AstArrayValue<'a> {
    fn span_range(&self) -> Range<usize> {
        self.span.clone()
    }
}

impl<'a> AstArrayValue<'a> {
    pub fn new(values: Vec<AstString<'a>>, span: Range<usize>) -> Self {
        Self { values, span }
    }

    pub fn values(&self) -> &[AstString<'a>] {
        &self.values
    }
}

pub(crate) fn array_value<'a>(
    input: &mut LocatingSlice<&'a str>,
) -> ModalResult<AstArrayValue<'a>> {
    delimited(
        '{',
        repeat(
            1..,
            (
                opt(ws_and_comments),
                string_value,
                alt((ws_and_comments, peek(literal("}")))),
            )
                .map(|(_, s, _)| s),
        ),
        '}',
    )
    .with_span()
    .context(StrContext::Label("array_value"))
    .map(|(values, span): (Vec<_>, _)| AstArrayValue { values, span })
    .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_array_value() {
        let input = "{\"Gameplay\"}";
        let mut input = LocatingSlice::new(input);
        let result = array_value(&mut input);
        assert_eq!(
            result,
            Ok(AstArrayValue::new(
                vec![AstString::new("Gameplay", true, 1..11)],
                0..12
            ))
        );
    }

    #[test]
    fn test_array_value_with_multiple_values() {
        let input = r#"{
    "Gameplay"
    "Politics"
    "Economy"
}"#;
        let mut input = LocatingSlice::new(input);
        let result = array_value(&mut input);
        assert_eq!(
            result,
            Ok(AstArrayValue::new(
                vec![
                    AstString::new("Gameplay", true, 6..16),
                    AstString::new("Politics", true, 21..31),
                    AstString::new("Economy", true, 36..45)
                ],
                0..47
            ))
        );
    }
}
