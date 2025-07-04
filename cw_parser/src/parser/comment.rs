use std::ops::Range;

use crate::AstNode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AstComment<'a> {
    pub text: &'a str,
    pub span: Range<usize>,
}

impl<'a> AstNode<'a> for AstComment<'a> {
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

impl<'a> AstComment<'a> {
    pub fn new(text: &'a str, span: Range<usize>) -> Self {
        Self { text, span }
    }
}
