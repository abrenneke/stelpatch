use std::ops::Range;

use winnow::{
    LocatingSlice, ModalResult, Parser,
    combinator::{delimited, repeat},
    error::StrContext,
};

use crate::{
    AstComment, AstNode, AstString, opt_trailing_comment, opt_ws_and_comments, quoted_string,
};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AstArrayValue<'a> {
    values: Vec<AstString<'a>>,
    span: Range<usize>,

    pub leading_comments: Vec<AstComment<'a>>,
    pub trailing_comment: Option<AstComment<'a>>,
}

impl<'a> AstNode<'a> for AstArrayValue<'a> {
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

impl<'a> AstArrayValue<'a> {
    pub fn new(values: Vec<AstString<'a>>, span: Range<usize>) -> Self {
        Self {
            values,
            span,
            leading_comments: vec![],
            trailing_comment: None,
        }
    }

    pub fn values(&self) -> &[AstString<'a>] {
        &self.values
    }
}

pub(crate) fn array_value<'a>(
    input: &mut LocatingSlice<&'a str>,
) -> ModalResult<AstArrayValue<'a>> {
    let leading_comments = opt_ws_and_comments.parse_next(input)?;

    let (values, span) = delimited('{', repeat(1.., quoted_string), (opt_ws_and_comments, '}'))
        .with_span()
        .context(StrContext::Label("array_value"))
        .parse_next(input)?;

    let trailing_comment = opt_trailing_comment.parse_next(input)?;

    Ok(AstArrayValue {
        values,
        span,
        leading_comments,
        trailing_comment,
    })
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
