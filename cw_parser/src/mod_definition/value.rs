use winnow::{LocatingSlice, ModalResult, Parser, combinator::alt, error::StrContext};

use super::array::array_value;
use super::string::string_value;
use crate::{AstComment, AstNode, AstString, mod_definition::array::AstArrayValue};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum AstValue<'a> {
    String(AstString<'a>),
    Array(AstArrayValue<'a>),
}

impl<'a> AstNode<'a> for AstValue<'a> {
    fn span_range(&self) -> std::ops::Range<usize> {
        match self {
            AstValue::String(s) => s.span_range(),
            AstValue::Array(a) => a.span_range(),
        }
    }

    fn leading_comments(&self) -> &[AstComment<'a>] {
        match self {
            AstValue::String(s) => s.leading_comments(),
            AstValue::Array(a) => a.leading_comments(),
        }
    }

    fn trailing_comment(&self) -> Option<&AstComment<'a>> {
        match self {
            AstValue::String(s) => s.trailing_comment(),
            AstValue::Array(a) => a.trailing_comment(),
        }
    }
}

impl<'a> AstValue<'a> {
    pub fn as_string(&self) -> Option<&'a str> {
        match self {
            AstValue::String(s) => Some(s.raw_value()),
            AstValue::Array(_) => None,
        }
    }

    pub fn as_array(&self) -> Option<&[AstString<'a>]> {
        match self {
            AstValue::String(_) => None,
            AstValue::Array(a) => Some(a.values()),
        }
    }
}

pub(crate) fn value<'a>(input: &mut LocatingSlice<&'a str>) -> ModalResult<AstValue<'a>> {
    alt((
        string_value.map(AstValue::String),
        array_value.map(AstValue::Array),
    ))
    .context(StrContext::Label("value"))
    .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_string_value() {
        let input = "\"Expanded Stellaris Traditions\"";
        let input = LocatingSlice::new(input);
        let result = value.parse(input);
        assert_eq!(
            result,
            Ok(AstValue::String(AstString::new(
                "Expanded Stellaris Traditions",
                true,
                0..31
            )))
        );
    }

    #[test]
    fn test_array_value() {
        let input = "{\"Gameplay\"}";
        let input = LocatingSlice::new(input);
        let result = value.parse(input);
        assert_eq!(
            result,
            Ok(AstValue::Array(AstArrayValue::new(
                vec![AstString::new("Gameplay", true, 1..11)],
                0..12
            )))
        );
    }
}
