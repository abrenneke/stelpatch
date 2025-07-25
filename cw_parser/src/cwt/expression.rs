use std::ops::Range;

use winnow::{LocatingSlice, ModalResult, Parser, combinator::alt, error::StrContext};

use crate::{AstComment, AstNode, CwtValue, cwt_value};

use super::{
    AstCwtBlock, AstCwtComment, AstCwtCommentOption, AstCwtIdentifier, AstCwtRule, CwtCommentType,
    cwt_block, cwt_identifier, cwt_rule, get_cwt_comments, opt_cwt_ws_and_comments,
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
    /// Value like in an enum: float[0.0..255.0]
    Value(CwtValue<'a>),
}

impl<'a> AstNode<'a> for AstCwtExpression<'a> {
    fn span_range(&self) -> Range<usize> {
        match self {
            AstCwtExpression::Rule(rule) => rule.span.clone(),
            AstCwtExpression::Block(block) => block.span.clone(),
            AstCwtExpression::Identifier(identifier) => identifier.span.clone(),
            AstCwtExpression::Value(value) => value.span_range(),
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
        cwt_value.map(AstCwtExpression::Value),
    ))
    .context(StrContext::Label("cwt_entity"))
    .parse_next(input)?;

    // Attach leading comments to the entity
    match &mut entity {
        AstCwtExpression::Block(block) => {
            block.leading_comments.extend(leading_comments);
        }
        AstCwtExpression::Rule(rule) => {
            // Process options from comment data and attach to rule
            let (options, documentation) = process_option_comments(leading_comments);
            rule.options.extend(options);
            rule.documentation = documentation;
        }
        AstCwtExpression::Identifier(identifier) => {
            identifier.leading_comments = leading_comments;
        }
        AstCwtExpression::Value(_value) => {
            // TODO: Handle leading comments for values
        }
    }

    Ok(entity)
}

/// Convert option comments to CwtOption structs
fn process_option_comments<'a>(
    comments: Vec<AstCwtComment<'a>>,
) -> (Vec<AstCwtCommentOption<'a>>, Vec<AstCwtComment<'a>>) {
    let mut options = Vec::new();
    let mut documentation = Vec::new();

    for comment in comments.into_iter() {
        match comment.comment_type {
            CwtCommentType::Option(option_data) => {
                // Process each option in the comment
                for comment_option in option_data.options.into_iter() {
                    options.push(comment_option);
                }
            }
            CwtCommentType::Regular => {
                // These don't contain options
            }
            CwtCommentType::Documentation => {
                documentation.push(comment.clone());
            }
        }
    }

    (options, documentation)
}
