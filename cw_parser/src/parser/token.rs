use std::{
    hash::{Hash, Hasher},
    ops::Range,
};

use crate::AstNode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AstToken<'a> {
    pub value: &'a str,
    pub span: Range<usize>,
}

impl ToString for AstToken<'_> {
    fn to_string(&self) -> String {
        self.value.to_string()
    }
}

impl<'a> AstToken<'a> {
    pub fn new(value: &'a str, span: Range<usize>) -> Self {
        Self { value, span }
    }
}

impl<'a> AstNode for AstToken<'a> {
    fn span_range(&self) -> Range<usize> {
        self.span.clone()
    }
}

impl<'a> Hash for AstToken<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

impl<'a> AsRef<str> for AstToken<'a> {
    fn as_ref(&self) -> &str {
        self.value
    }
}
