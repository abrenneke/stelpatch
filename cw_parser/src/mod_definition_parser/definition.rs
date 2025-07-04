use std::ops::Range;

use winnow::{
    LocatingSlice, ModalResult, Parser,
    combinator::{eof, opt, repeat_till},
};

use crate::{AstComment, AstNode, ws_and_comments};
use self_cell::self_cell;

use super::expression::{AstExpression, expression};

pub type AstModDefinitionResult<'a> = Result<AstModDefinition<'a>, anyhow::Error>;

self_cell!(
    pub struct AstModDefinitionCell {
        owner: String,

        #[covariant]
        dependent: AstModDefinitionResult,
    }

    impl {Debug, PartialEq, Eq}
);

impl Clone for AstModDefinitionCell {
    fn clone(&self) -> Self {
        let cloned_str = self.borrow_owner().to_owned();

        Self::new(cloned_str, |input| {
            let input = LocatingSlice::new(input.as_ref());
            mod_definition
                .parse(input)
                .map_err(|e| anyhow::anyhow!("Failed to parse mod definition: {}", e))
        })
    }
}

impl AstModDefinitionCell {
    pub fn from_input(input: String) -> Self {
        Self::new(input, |input| {
            let input = LocatingSlice::new(input.as_ref());
            mod_definition
                .parse(input)
                .map_err(|e| anyhow::anyhow!("Failed to parse mod definition: {}", e))
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AstModDefinition<'a> {
    pub expressions: Vec<AstExpression<'a>>,
    pub span: Range<usize>,

    pub leading_comments: Vec<AstComment<'a>>,
    pub trailing_comment: Option<AstComment<'a>>,
}

impl<'a> AstModDefinition<'a> {
    pub fn new(expressions: Vec<AstExpression<'a>>, span: Range<usize>) -> Self {
        Self {
            expressions,
            span,
            leading_comments: vec![],
            trailing_comment: None,
        }
    }
}

impl<'a> AstNode<'a> for AstModDefinition<'a> {
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

// Define the parse_mod_definition function to parse the entire input string
pub(crate) fn mod_definition<'a>(
    input: &mut LocatingSlice<&'a str>,
) -> ModalResult<AstModDefinition<'a>> {
    repeat_till(
        0..,
        (opt(ws_and_comments), expression, opt(ws_and_comments)).map(|(_, e, _)| e),
        eof,
    )
    .with_span()
    .map(
        |((expressions, _), span): ((Vec<_>, _), _)| AstModDefinition {
            expressions,
            span,
            leading_comments: vec![],
            trailing_comment: None,
        },
    )
    .parse_next(input)
}

#[cfg(test)]
mod tests {
    use crate::{
        AstString, AstToken,
        mod_definition_parser::{array::AstArrayValue, value::AstValue},
    };
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    pub fn test_mod_definition() {
        let input = r#"
name="Expanded Stellaris Traditions"
tags={
	"Gameplay"
}
picture="thumbnail.png"
supported_version="3.9.*"
path="C:/Users/admin/Documents/Paradox Interactive/Stellaris/mod/expanded_stellaris_traditions"
remote_file_id="946222466"
archive="expanded_stellaris_traditions.zip"
dependencies={
	"Stellaris"
}
"#;
        let input = LocatingSlice::new(input);
        let result = mod_definition.parse(input).unwrap();

        assert_eq!(
            result,
            AstModDefinition::new(
                vec![
                    AstExpression::new(
                        AstToken::new("name", 1..5),
                        AstValue::String(AstString::new(
                            "Expanded Stellaris Traditions",
                            true,
                            6..37
                        )),
                        1..37
                    ),
                    AstExpression::new(
                        AstToken::new("tags", 38..42),
                        AstValue::Array(AstArrayValue::new(
                            vec![AstString::new("Gameplay", true, 46..56)],
                            43..58
                        )),
                        38..58
                    ),
                    AstExpression::new(
                        AstToken::new("picture", 59..66),
                        AstValue::String(AstString::new("thumbnail.png", true, 67..82)),
                        59..82
                    ),
                    AstExpression::new(
                        AstToken::new("supported_version", 83..100),
                        AstValue::String(AstString::new("3.9.*", true, 101..108)),
                        83..108
                    ),
                    AstExpression::new(
                        AstToken::new("path", 109..113),
                        AstValue::String(AstString::new(
                            "C:/Users/admin/Documents/Paradox Interactive/Stellaris/mod/expanded_stellaris_traditions",
                            true,
                            114..204
                        )),
                        109..204
                    ),
                    AstExpression::new(
                        AstToken::new("remote_file_id", 205..219),
                        AstValue::String(AstString::new("946222466", true, 220..231)),
                        205..231
                    ),
                    AstExpression::new(
                        AstToken::new("archive", 232..239),
                        AstValue::String(AstString::new(
                            "expanded_stellaris_traditions.zip",
                            true,
                            240..275
                        )),
                        232..275
                    ),
                    AstExpression::new(
                        AstToken::new("dependencies", 276..288),
                        AstValue::Array(AstArrayValue::new(
                            vec![AstString::new("Stellaris", true, 292..303)],
                            289..305
                        )),
                        276..305
                    ),
                ],
                0..306
            )
        );
    }
}
