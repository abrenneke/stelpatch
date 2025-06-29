use crate::{AstConditionalBlock, AstExpression, AstValue};

pub enum AstBlockItem<'a> {
    Expression(AstExpression<'a>),
    ArrayItem(AstValue<'a>),
    Conditional(AstConditionalBlock<'a>),
}
