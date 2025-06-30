use crate::{
    AstString, AstToken,
    mod_definition_parser::{AstArrayValue, AstExpression, AstModDefinition, AstValue},
};

/// Visitor trait for traversing the mod definition AST
pub trait ModDefinitionAstVisitor<'a> {
    type Result: Default;

    fn visit_mod_definition(&mut self, node: &AstModDefinition<'a>) -> Self::Result {
        self.walk_mod_definition(node)
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

    fn visit_array_value(&mut self, node: &AstArrayValue<'a>) -> Self::Result {
        self.walk_array_value(node)
    }

    fn visit_token(&mut self, node: &AstToken<'a>) -> Self::Result {
        self.walk_token(node)
    }

    // Default walking implementations
    fn walk_mod_definition(&mut self, node: &AstModDefinition<'a>) -> Self::Result {
        for expression in &node.expressions {
            self.visit_expression(expression);
        }
        Self::Result::default()
    }

    fn walk_expression(&mut self, node: &AstExpression<'a>) -> Self::Result {
        self.visit_token(&node.key);
        self.visit_value(&node.value);
        Self::Result::default()
    }

    fn walk_value(&mut self, node: &AstValue<'a>) -> Self::Result {
        match node {
            AstValue::String(s) => self.visit_string(s),
            AstValue::Array(a) => self.visit_array_value(a),
        }
    }

    fn walk_array_value(&mut self, node: &AstArrayValue<'a>) -> Self::Result {
        for string in node.values() {
            self.visit_string(string);
        }
        Self::Result::default()
    }

    // Terminal node defaults - these should be implemented based on Result type
    fn walk_string(&mut self, _node: &AstString<'a>) -> Self::Result {
        Self::Result::default()
    }

    fn walk_token(&mut self, _node: &AstToken<'a>) -> Self::Result {
        Self::Result::default()
    }
}
