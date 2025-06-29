use std::ops::Range;

use crate::{AstNode, AstToken};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AstBoolean<'a> {
    pub value: AstToken<'a>,
}

impl<'a> AstBoolean<'a> {
    pub fn new(value: &'a str, span: Range<usize>) -> Self {
        Self {
            value: AstToken { value, span },
        }
    }
}

impl<'a> AstNode for AstBoolean<'a> {
    fn span_range(&self) -> Range<usize> {
        self.value.span.clone()
    }
}
