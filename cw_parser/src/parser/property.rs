use std::ops::Range;

use crate::{AstNode, AstOperator, AstString, AstValue};

/// A property in an entity, like { a = b } or { a > b }
#[derive(PartialEq, Eq, Debug)]
pub struct AstProperty<'a> {
    pub key: AstString<'a>,
    pub operator: AstOperator<'a>,
    pub value: AstValue<'a>,
    pub span: Range<usize>,
}

impl<'a> AstProperty<'a> {
    pub fn new(key: AstString<'a>, operator: AstOperator<'a>, value: AstValue<'a>) -> Self {
        let span = key.value.span.start..value.span_range().end;

        Self {
            key,
            operator,
            value,
            span,
        }
    }
}

impl<'a> AstNode for AstProperty<'a> {
    fn span_range(&self) -> Range<usize> {
        self.span.clone()
    }
}
