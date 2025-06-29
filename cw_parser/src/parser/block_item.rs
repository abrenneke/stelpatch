use std::ops::Range;

use crate::{AstConditionalBlock, AstExpression, AstNode, AstValue};

pub enum AstBlockItem<'a> {
    Expression(AstExpression<'a>),
    ArrayItem(AstValue<'a>),
    Conditional(AstConditionalBlock<'a>),
}

impl<'a> AstNode for AstBlockItem<'a> {
    fn span_range(&self) -> Range<usize> {
        match self {
            Self::Expression(e) => e.span_range(),
            Self::ArrayItem(v) => v.span_range(),
            Self::Conditional(c) => c.span_range(),
        }
    }
}
