use std::{
    hash::{Hash, Hasher},
    ops::Range,
};

use crate::{AstComment, AstNode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AstToken<'a> {
    pub value: &'a str,
    pub span: Range<usize>,

    pub leading_comments: Vec<AstComment<'a>>,
    pub trailing_comment: Option<AstComment<'a>>,
}

impl ToString for AstToken<'_> {
    fn to_string(&self) -> String {
        self.value.to_string()
    }
}

impl<'a> AstToken<'a> {
    pub fn new(value: &'a str, span: Range<usize>) -> Self {
        Self {
            value,
            span,
            leading_comments: vec![],
            trailing_comment: None,
        }
    }
}

impl<'a> AstNode<'a> for AstToken<'a> {
    fn span_range(&self) -> Range<usize> {
        self.span.clone()
    }

    fn leading_comments(&self) -> &[AstComment<'a>] {
        &self.leading_comments
    }

    fn trailing_comment(&self) -> Option<&AstComment<'a>> {
        self.trailing_comment.as_ref()
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
