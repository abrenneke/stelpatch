use crate::{
    AstBlockItem, AstColor, AstConditionalBlock, AstEntity, AstEntityItem, AstExpression, AstMaths,
    AstModule, AstNumber, AstOperator, AstString, AstToken, AstValue,
};

/// Visitor trait for traversing the AST
pub trait AstVisitor<'a> {
    type Result: Default;

    fn visit_module(&mut self, node: &AstModule<'a>) -> Self::Result {
        self.walk_module(node)
    }

    fn visit_entity(&mut self, node: &AstEntity<'a>) -> Self::Result {
        self.walk_entity(node)
    }

    fn visit_entity_item(&mut self, node: &AstEntityItem<'a>) -> Self::Result {
        self.walk_entity_item(node)
    }

    fn visit_expression(&mut self, node: &AstExpression<'a>) -> Self::Result {
        self.walk_expression(node)
    }

    fn visit_value(&mut self, node: &AstValue<'a>) -> Self::Result {
        self.walk_value(node)
    }

    fn visit_string(&mut self, node: &AstString<'a>) -> Self::Result {
        self.walk_string(node)
    }

    fn visit_number(&mut self, node: &AstNumber<'a>) -> Self::Result {
        self.walk_number(node)
    }

    fn visit_color(&mut self, node: &AstColor<'a>) -> Self::Result {
        self.walk_color(node)
    }

    fn visit_maths(&mut self, node: &AstMaths<'a>) -> Self::Result {
        self.walk_maths(node)
    }

    fn visit_operator(&mut self, node: &AstOperator<'a>) -> Self::Result {
        self.walk_operator(node)
    }

    fn visit_token(&mut self, node: &AstToken<'a>) -> Self::Result {
        self.walk_token(node)
    }

    fn visit_block_item(&mut self, node: &AstBlockItem<'a>) -> Self::Result {
        self.walk_block_item(node)
    }

    fn visit_conditional_block(&mut self, node: &AstConditionalBlock<'a>) -> Self::Result {
        self.walk_conditional_block(node)
    }

    // Default walking implementations
    fn walk_module(&mut self, node: &AstModule<'a>) -> Self::Result {
        for item in &node.items {
            self.visit_entity_item(item);
        }
        Self::Result::default()
    }

    fn walk_entity(&mut self, node: &AstEntity<'a>) -> Self::Result {
        for item in &node.items {
            self.visit_entity_item(item);
        }
        Self::Result::default()
    }

    fn walk_entity_item(&mut self, node: &AstEntityItem<'a>) -> Self::Result {
        match node {
            AstEntityItem::Expression(prop) => self.visit_expression(prop),
            AstEntityItem::Item(value) => self.visit_value(value),
            AstEntityItem::Conditional(cond) => self.visit_conditional_block(cond),
        }
    }

    fn walk_expression(&mut self, node: &AstExpression<'a>) -> Self::Result {
        self.visit_string(&node.key);
        self.visit_operator(&node.operator);
        self.visit_value(&node.value);
        Self::Result::default()
    }

    fn walk_value(&mut self, node: &AstValue<'a>) -> Self::Result {
        match node {
            AstValue::String(s) => self.visit_string(s),
            AstValue::Number(n) => self.visit_number(n),
            AstValue::Entity(e) => self.visit_entity(e),
            AstValue::Color(c) => self.visit_color(c),
            AstValue::Maths(m) => self.visit_maths(m),
        }
    }

    fn walk_block_item(&mut self, node: &AstBlockItem<'a>) -> Self::Result {
        match node {
            AstBlockItem::Expression(expr) => self.visit_expression(expr),
            AstBlockItem::ArrayItem(value) => self.visit_value(value),
            AstBlockItem::Conditional(cond) => self.visit_conditional_block(cond),
        }
    }

    // Terminal node defaults - these should be implemented based on Result type
    fn walk_string(&mut self, _node: &AstString<'a>) -> Self::Result {
        Self::Result::default()
    }

    fn walk_number(&mut self, _node: &AstNumber<'a>) -> Self::Result {
        Self::Result::default()
    }

    fn walk_color(&mut self, _node: &AstColor<'a>) -> Self::Result {
        Self::Result::default()
    }

    fn walk_maths(&mut self, _node: &AstMaths<'a>) -> Self::Result {
        Self::Result::default()
    }

    fn walk_operator(&mut self, _node: &AstOperator<'a>) -> Self::Result {
        Self::Result::default()
    }

    fn walk_token(&mut self, _node: &AstToken<'a>) -> Self::Result {
        Self::Result::default()
    }

    fn walk_conditional_block(&mut self, _node: &AstConditionalBlock<'a>) -> Self::Result {
        // Note: Implementation depends on AstConditionalBlock structure
        Self::Result::default()
    }
}
