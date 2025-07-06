use std::ops::Range;

use crate::{
    AstComment, AstConditionalBlock, AstExpression, AstNode, AstValue, CommentOrWhitespace,
};

pub enum AstBlockItem<'a> {
    Expression(AstExpression<'a>),
    ArrayItem(AstValue<'a>),
    Conditional(AstConditionalBlock<'a>),
    Whitespace(Vec<CommentOrWhitespace<'a>>),
}

impl<'a> AstNode<'a> for AstBlockItem<'a> {
    fn span_range(&self) -> Range<usize> {
        match self {
            Self::Expression(e) => e.span_range(),
            Self::ArrayItem(v) => v.span_range(),
            Self::Conditional(c) => c.span_range(),
            Self::Whitespace(_) => 0..0,
        }
    }

    fn leading_comments(&self) -> &[AstComment<'a>] {
        match self {
            Self::Expression(e) => e.leading_comments(),
            Self::ArrayItem(v) => v.leading_comments(),
            Self::Conditional(c) => c.leading_comments(),
            Self::Whitespace(_) => &[],
        }
    }

    fn trailing_comment(&self) -> Option<&AstComment<'a>> {
        match self {
            Self::Expression(e) => e.trailing_comment(),
            Self::ArrayItem(v) => v.trailing_comment(),
            Self::Conditional(c) => c.trailing_comment(),
            Self::Whitespace(_) => None,
        }
    }
}
