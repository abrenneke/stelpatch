use std::ops::Range;

use winnow::{LocatingSlice, ModalResult, Parser, combinator::alt, error::StrContext};

use crate::{AstComment, AstNode, AstString, quoted_or_unquoted_string};

use super::{
    AstCwtBlock, AstCwtIdentifier, AstCwtRule, cwt_block, cwt_identifier, cwt_rule,
    get_cwt_comments, opt_cwt_ws_and_comments,
};

/// CWT entity types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AstCwtExpression<'a> {
    /// Regular rule: key = value
    Rule(AstCwtRule<'a>),
    /// Block entity: { ... }
    Block(AstCwtBlock<'a>),
    /// Standalone identifier: <identifier>
    Identifier(AstCwtIdentifier<'a>),
    /// A quoted or unquoted string by itself, for e.g. enum values
    String(AstString<'a>),
}

impl<'a> AstNode<'a> for AstCwtExpression<'a> {
    fn span_range(&self) -> Range<usize> {
        match self {
            AstCwtExpression::Rule(rule) => rule.span.clone(),
            AstCwtExpression::Block(block) => block.span.clone(),
            AstCwtExpression::Identifier(identifier) => identifier.span.clone(),
            AstCwtExpression::String(string) => string.span_range(),
        }
    }

    fn leading_comments(&self) -> &[AstComment<'a>] {
        // CWT comments don't map directly to AstComment
        // This is a design limitation - we'd need to convert CwtComment to AstComment
        &[]
    }

    fn trailing_comment(&self) -> Option<&AstComment<'a>> {
        // CWT comments don't map directly to AstComment
        // This is a design limitation - we'd need to convert CwtComment to AstComment
        None
    }
}

/// Parse a CWT entity
pub(crate) fn cwt_expression<'a>(
    input: &mut LocatingSlice<&'a str>,
) -> ModalResult<AstCwtExpression<'a>> {
    let leading_comments_data = opt_cwt_ws_and_comments.parse_next(input)?;
    let leading_comments = get_cwt_comments(&leading_comments_data);

    let mut entity = alt((
        cwt_block.map(AstCwtExpression::Block),
        cwt_rule.map(AstCwtExpression::Rule),
        cwt_identifier.map(AstCwtExpression::Identifier),
        quoted_or_unquoted_string.map(AstCwtExpression::String),
    ))
    .context(StrContext::Label("cwt_entity"))
    .parse_next(input)?;

    // Attach leading comments to the entity
    match &mut entity {
        AstCwtExpression::Block(block) => {
            block.leading_comments.extend(leading_comments);
        }
        AstCwtExpression::Rule(rule) => {
            // For rules, we store the first comment as documentation
            if let Some(first_comment) = leading_comments.into_iter().next() {
                rule.documentation = Some(first_comment);
            }
        }
        AstCwtExpression::Identifier(identifier) => {
            identifier.leading_comments = leading_comments;
        }
        AstCwtExpression::String(string) => {
            // TODO: Handle leading comments for strings
        }
    }

    Ok(entity)
}
