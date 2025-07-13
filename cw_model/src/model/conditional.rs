use cw_parser::{AstConditionalBlock, AstExpression, AstValue, AstVisitor};

use crate::{Properties, PropertyInfo, PropertyInfoList, PropertyVisitor, Value, ValueVisitor};

/// A conditional block looks like [[PARAM_NAME] key = value] and is dumb
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConditionalBlock {
    pub is_not: bool,
    pub key: String,
    pub items: Vec<Value>,
    pub properties: Properties,
}

impl ConditionalBlock {
    pub fn new() -> Self {
        Self {
            is_not: false,
            key: String::new(),
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

        buf.push_str(&self.key);
        buf.push_str("]\n");

        for value in &self.items {
            let value = format!("{}\n", value.to_string());
            buf.push_str(&value);
        }
        for (key, value) in &self.properties.kv {
            let value = format!("{} {}\n", key, value.to_string());
            buf.push_str(&value);
        }

        buf.push_str("]\n");
        buf
    }
}

pub(crate) struct ConditionalBlockVisitor<'a> {
    conditional_block: &'a mut ConditionalBlock,
}

impl<'a> ConditionalBlockVisitor<'a> {
    pub fn new(conditional_block: &'a mut ConditionalBlock) -> Self {
        Self { conditional_block }
    }
}

impl<'a, 'b, 'ast> AstVisitor<'b, 'ast> for ConditionalBlockVisitor<'a>
where
    'b: 'ast,
{
    fn visit_conditional_block(&mut self, node: &AstConditionalBlock<'b>) -> () {
        self.conditional_block.is_not = node.is_not;
        self.conditional_block.key = node.key.to_string();

        self.walk_conditional_block(node);
    }

    fn visit_expression(&mut self, node: &AstExpression<'b>) -> () {
        let mut property = PropertyInfo::default();
        let mut property_visitor = PropertyVisitor::new(&mut property);
        property_visitor.visit_expression(node);
        self.conditional_block
            .properties
            .kv
            .entry(node.key.value.to_string())
            .or_insert_with(PropertyInfoList::new)
            .0
            .push(property);
    }

    fn visit_value(&mut self, node: &AstValue<'b>) -> () {
        let mut value = Value::default();
        let mut value_visitor = ValueVisitor::new(&mut value);
        value_visitor.visit_value(node);
        self.conditional_block.items.push(value);
    }
}
