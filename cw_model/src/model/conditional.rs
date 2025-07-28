use cw_parser::{AstConditionalBlock, AstExpression, AstValue, AstVisitor};
use lasso::Spur;

use crate::{
    CaseInsensitiveInterner, Properties, PropertyInfo, PropertyInfoList, PropertyVisitor, Value,
    ValueVisitor,
};

/// A conditional block looks like [[PARAM_NAME] key = value] and is dumb
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConditionalBlock {
    pub is_not: bool,
    pub key: Spur,
    pub items: Vec<Value>,
    pub properties: Properties,
}

impl ConditionalBlock {
    pub fn new() -> Self {
        Self {
            is_not: false,
            key: Spur::default(),
            items: Vec::new(),
            properties: Properties::new(),
        }
    }
}

impl Default for ConditionalBlock {
    fn default() -> Self {
        Self::new()
    }
}

impl ToString for ConditionalBlock {
    fn to_string(&self) -> String {
        let mut buf = String::from("[[");

        if self.is_not {
            buf.push_str("!");
        }

        buf.push_str(format!("{:?}", self.key).as_str());
        buf.push_str("]\n");

        for value in &self.items {
            let value = format!("{}\n", value.to_string());
            buf.push_str(&value);
        }
        for (key, value) in &self.properties.kv {
            let value = format!("{:?} {}\n", key, value.to_string());
            buf.push_str(&value);
        }

        buf.push_str("]\n");
        buf
    }
}

pub(crate) struct ConditionalBlockVisitor<'a, 'interner> {
    conditional_block: &'a mut ConditionalBlock,
    interner: &'interner CaseInsensitiveInterner,
}

impl<'a, 'interner> ConditionalBlockVisitor<'a, 'interner> {
    pub fn new(
        conditional_block: &'a mut ConditionalBlock,
        interner: &'interner CaseInsensitiveInterner,
    ) -> Self {
        Self {
            conditional_block,
            interner,
        }
    }
}

impl<'a, 'b, 'ast, 'interner> AstVisitor<'b, 'ast> for ConditionalBlockVisitor<'a, 'interner>
where
    'b: 'ast,
{
    fn visit_conditional_block(&mut self, node: &AstConditionalBlock<'b>) -> () {
        self.conditional_block.is_not = node.is_not;
        self.conditional_block.key = self.interner.get_or_intern(node.key.raw_value());

        self.walk_conditional_block(node);
    }

    fn visit_expression(&mut self, node: &AstExpression<'b>) -> () {
        let mut property = PropertyInfo::default();
        let mut property_visitor = PropertyVisitor::new(&mut property, self.interner);
        property_visitor.visit_expression(node);
        self.conditional_block
            .properties
            .kv
            .entry(self.interner.get_or_intern(node.key.raw_value()))
            .or_insert_with(PropertyInfoList::new)
            .0
            .push(property);
    }

    fn visit_value(&mut self, node: &AstValue<'b>) -> () {
        let mut value = Value::default();
        let mut value_visitor = ValueVisitor::new(&mut value, self.interner);
        value_visitor.visit_value(node);
        self.conditional_block.items.push(value);
    }
}
