use std::ops::Range;

use winnow::{LocatingSlice, ModalResult, Parser, combinator::repeat_till, error::StrContext};

use crate::{AstComment, AstNode, opt_ws_and_comments};

use super::{AstCwtComment, AstCwtExpression, AstCwtRule, cwt_expression};

/// Block entity containing rules and sub-entities
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AstCwtBlock<'a> {
    pub items: Vec<AstCwtExpression<'a>>,
    pub span: Range<usize>,
    pub leading_comments: Vec<AstCwtComment<'a>>,
    pub trailing_comments: Vec<AstCwtComment<'a>>,
}

impl<'a> AstCwtBlock<'a> {
    pub fn new(span: Range<usize>) -> Self {
        Self {
            items: Vec::new(),
            span,
            leading_comments: Vec::new(),
            trailing_comments: Vec::new(),
        }
    }

    pub fn with_item(mut self, item: AstCwtExpression<'a>) -> Self {
        self.items.push(item);
        self
    }

    pub fn with_leading_comment(mut self, comment: AstCwtComment<'a>) -> Self {
        self.leading_comments.push(comment);
        self
    }

    pub fn with_trailing_comment(mut self, comment: AstCwtComment<'a>) -> Self {
        self.trailing_comments.push(comment);
        self
    }

    /// Find all rules with the given key name
    pub fn find_rules(&self, key: &str) -> Vec<&AstCwtRule<'a>> {
        self.items
            .iter()
            .filter_map(|item| match item {
                AstCwtExpression::Rule(rule) if rule.key.name() == key => Some(rule),
                _ => None,
            })
            .collect()
    }

    /// Find the first rule with the given key name
    pub fn find_rule(&self, key: &str) -> Option<&AstCwtRule<'a>> {
        self.items.iter().find_map(|item| match item {
            AstCwtExpression::Rule(rule) if rule.key.name() == key => Some(rule),
            _ => None,
        })
    }

    /// Get all rules in the block
    pub fn rules(&self) -> impl Iterator<Item = &AstCwtRule<'a>> {
        self.items.iter().filter_map(|item| match item {
            AstCwtExpression::Rule(rule) => Some(rule),
            _ => None,
        })
    }

    /// Check if the block is empty
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Get the number of items in the block
    pub fn len(&self) -> usize {
        self.items.len()
    }
}

impl<'a> AstNode<'a> for AstCwtBlock<'a> {
    fn span_range(&self) -> Range<usize> {
        self.span.clone()
    }

    fn leading_comments(&self) -> &[AstComment<'a>] {
        &[]
    }

    fn trailing_comment(&self) -> Option<&AstComment<'a>> {
        None
    }
}

/// Parse a CWT block
pub(crate) fn cwt_block<'a>(input: &mut LocatingSlice<&'a str>) -> ModalResult<AstCwtBlock<'a>> {
    let ((_, (items, _)), span) = (
        "{",
        repeat_till(0.., cwt_expression, (opt_ws_and_comments, "}")),
    )
        .with_span()
        .context(StrContext::Label("cwt_block"))
        .parse_next(input)?;

    Ok(AstCwtBlock {
        items,
        span,
        leading_comments: Vec::new(), // Will be populated by cwt_entity if this is a standalone block
        trailing_comments: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use crate::CwtCommentType;

    use super::*;
    use pretty_assertions::assert_eq;
    use winnow::{LocatingSlice, Parser};

    #[test]
    fn test_cwt_block_empty() {
        let mut input = LocatingSlice::new("{}");
        let result = cwt_block.parse_next(&mut input).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_cwt_block_with_multiple_rules() {
        let mut input = LocatingSlice::new("{ key1 = value1 key2 = value2 key3 = value3 }");
        let result = cwt_block.parse_next(&mut input).unwrap();

        assert_eq!(result.len(), 3);
        assert!(!result.is_empty());

        let rules: Vec<_> = result.rules().collect();
        assert_eq!(rules.len(), 3);
        assert_eq!(rules[0].key.name(), "key1");
        assert_eq!(rules[1].key.name(), "key2");
        assert_eq!(rules[2].key.name(), "key3");
    }

    #[test]
    fn test_cwt_block_with_comments() {
        let mut block = AstCwtBlock::new(0..10);
        let comment = AstCwtComment::new("# Test comment", CwtCommentType::Regular, 0..14);

        block = block
            .with_leading_comment(comment.clone())
            .with_trailing_comment(comment);

        assert_eq!(block.leading_comments.len(), 1);
        assert_eq!(block.trailing_comments.len(), 1);
    }
}
