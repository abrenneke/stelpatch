use crate::{
    AstBlockItem, AstColor, AstComment, AstConditionalBlock, AstEntity, AstEntityItem,
    AstExpression, AstMaths, AstModule, AstNumber, AstOperator, AstString, AstToken, AstValue,
    CommentOrWhitespace,
};

/// Visitor trait for traversing the AST
pub trait AstVisitor<'a> {
    fn visit_module(&mut self, node: &AstModule<'a>) -> () {
        self.walk_module(node)
    }

    fn visit_entity(&mut self, node: &AstEntity<'a>) -> () {
        self.walk_entity(node)
    }

    fn visit_entity_item(&mut self, node: &AstEntityItem<'a>) -> () {
        self.walk_entity_item(node)
    }

    fn visit_expression(&mut self, node: &AstExpression<'a>) -> () {
        self.walk_expression(node)
    }

    fn visit_value(&mut self, node: &AstValue<'a>) -> () {
        self.walk_value(node)
    }

    fn visit_string(&mut self, node: &AstString<'a>) -> () {
        self.walk_string(node)
    }

    fn visit_number(&mut self, node: &AstNumber<'a>) -> () {
        self.walk_number(node)
    }

    fn visit_color(&mut self, node: &AstColor<'a>) -> () {
        self.walk_color(node)
    }

    fn visit_maths(&mut self, node: &AstMaths<'a>) -> () {
        self.walk_maths(node)
    }

    fn visit_operator(&mut self, node: &AstOperator<'a>) -> () {
        self.walk_operator(node)
    }

    fn visit_token(&mut self, node: &AstToken<'a>) -> () {
        self.walk_token(node)
    }

    fn visit_block_item(&mut self, node: &AstBlockItem<'a>) -> () {
        self.walk_block_item(node)
    }

    fn visit_conditional_block(&mut self, node: &AstConditionalBlock<'a>) -> () {
        self.walk_conditional_block(node)
    }

    fn visit_comment(&mut self, node: &AstComment<'a>) -> () {
        self.walk_comment(node)
    }

    // Default walking implementations
    fn walk_module(&mut self, node: &AstModule<'a>) -> () {
        for item in &node.items {
            self.visit_entity_item(item);
        }
    }

    fn walk_entity(&mut self, node: &AstEntity<'a>) -> () {
        for item in &node.items {
            self.visit_entity_item(item);
        }
    }

    fn walk_entity_item(&mut self, node: &AstEntityItem<'a>) -> () {
        match node {
            AstEntityItem::Expression(prop) => self.visit_expression(prop),
            AstEntityItem::Item(value) => self.visit_value(value),
            AstEntityItem::Conditional(cond) => self.visit_conditional_block(cond),
        }
    }

    fn walk_expression(&mut self, node: &AstExpression<'a>) -> () {
        self.visit_string(&node.key);
        self.visit_operator(&node.operator);
        self.visit_value(&node.value);
    }

    fn walk_value(&mut self, node: &AstValue<'a>) -> () {
        match node {
            AstValue::String(s) => self.visit_string(s),
            AstValue::Number(n) => self.visit_number(n),
            AstValue::Entity(e) => self.visit_entity(e),
            AstValue::Color(c) => self.visit_color(c),
            AstValue::Maths(m) => self.visit_maths(m),
        }
    }

    fn walk_block_item(&mut self, node: &AstBlockItem<'a>) -> () {
        match node {
            AstBlockItem::Expression(expr) => self.visit_expression(expr),
            AstBlockItem::ArrayItem(value) => self.visit_value(value),
            AstBlockItem::Conditional(cond) => self.visit_conditional_block(cond),
            AstBlockItem::Whitespace(_) => {}
        }
    }

    fn walk_whitespace_array(&mut self, node: &[CommentOrWhitespace<'a>]) -> () {
        for item in node {
            match item {
                CommentOrWhitespace::Comment(comment) => self.visit_comment(comment),
                CommentOrWhitespace::Whitespace { .. } => {}
            };
        }
    }

    // Terminal node defaults - these should be implemented based on Result type
    fn walk_string(&mut self, _node: &AstString<'a>) -> () {}

    fn walk_comment(&mut self, _node: &AstComment<'a>) -> () {}

    fn walk_number(&mut self, _node: &AstNumber<'a>) -> () {}

    fn walk_color(&mut self, _node: &AstColor<'a>) -> () {}

    fn walk_maths(&mut self, _node: &AstMaths<'a>) -> () {}

    fn walk_operator(&mut self, _node: &AstOperator<'a>) -> () {}

    fn walk_token(&mut self, _node: &AstToken<'a>) -> () {}

    fn walk_conditional_block(&mut self, node: &AstConditionalBlock<'a>) -> () {
        for item in &node.items {
            self.visit_entity_item(item);
        }
    }
}
