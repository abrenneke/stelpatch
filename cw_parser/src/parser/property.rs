use crate::{AstOperator, AstString, AstValue};

/// A property in an entity, like { a = b } or { a > b }
#[derive(PartialEq, Eq, Debug)]
pub struct AstProperty<'a> {
    pub key: AstString<'a>,
    pub operator: AstOperator<'a>,
    pub value: AstValue<'a>,
}

impl<'a> AstProperty<'a> {
    pub fn new(key: AstString<'a>, operator: AstOperator<'a>, value: AstValue<'a>) -> Self {
        Self {
            key,
            operator,
            value,
        }
    }
}
