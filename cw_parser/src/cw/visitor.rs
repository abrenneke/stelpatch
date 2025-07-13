use crate::{
    AstBlockItem, AstColor, AstComment, AstConditionalBlock, AstEntity, AstEntityItem,
    AstExpression, AstMaths, AstModule, AstNumber, AstOperator, AstString, AstToken, AstValue,
    CommentOrWhitespace,
};

/// Visitor trait for traversing the AST
pub trait AstVisitor<'a, 'ast>
where
    'a: 'ast,
{
    fn visit_module(&mut self, node: &'ast AstModule<'a>) -> () {
        self.walk_module(node)
    }

    fn visit_entity(&mut self, node: &'ast AstEntity<'a>) -> () {
        self.walk_entity(node)
    }

    fn visit_entity_item(&mut self, node: &'ast AstEntityItem<'a>) -> () {
        self.walk_entity_item(node)
    }

    fn visit_expression(&mut self, node: &'ast AstExpression<'a>) -> () {
        self.walk_expression(node)
    }

    fn visit_value(&mut self, node: &'ast AstValue<'a>) -> () {
        self.walk_value(node)
    }

    fn visit_string(&mut self, node: &'ast AstString<'a>) -> () {
        self.walk_string(node)
    }

    fn visit_number(&mut self, node: &'ast AstNumber<'a>) -> () {
        self.walk_number(node)
    }

    fn visit_color(&mut self, node: &'ast AstColor<'a>) -> () {
        self.walk_color(node)
    }

    fn visit_maths(&mut self, node: &'ast AstMaths<'a>) -> () {
        self.walk_maths(node)
    }

    fn visit_operator(&mut self, node: &'ast AstOperator<'a>) -> () {
        self.walk_operator(node)
    }

    fn visit_token(&mut self, node: &'ast AstToken<'a>) -> () {
        self.walk_token(node)
    }

    fn visit_block_item(&mut self, node: &'ast AstBlockItem<'a>) -> () {
        self.walk_block_item(node)
    }

    fn visit_conditional_block(&mut self, node: &'ast AstConditionalBlock<'a>) -> () {
        self.walk_conditional_block(node)
    }

    fn visit_comment(&mut self, node: &'ast AstComment<'a>) -> () {
        self.walk_comment(node)
    }

    // Default walking implementations
    fn walk_module(&mut self, node: &'ast AstModule<'a>) -> () {
        for item in &node.items {
            self.visit_entity_item(item);
        }
    }

    fn walk_entity(&mut self, node: &'ast AstEntity<'a>) -> () {
        for item in &node.items {
            self.visit_entity_item(item);
        }
    }

    fn walk_entity_item(&mut self, node: &'ast AstEntityItem<'a>) -> () {
        match node {
            AstEntityItem::Expression(prop) => self.visit_expression(prop),
            AstEntityItem::Item(value) => self.visit_value(value),
            AstEntityItem::Conditional(cond) => self.visit_conditional_block(cond),
        }
    }

    fn walk_expression(&mut self, node: &'ast AstExpression<'a>) -> () {
        self.visit_string(&node.key);
        self.visit_operator(&node.operator);
        self.visit_value(&node.value);
    }

    fn walk_value(&mut self, node: &'ast AstValue<'a>) -> () {
        match node {
            AstValue::String(s) => self.visit_string(s),
            AstValue::Number(n) => self.visit_number(n),
            AstValue::Entity(e) => self.visit_entity(e),
            AstValue::Color(c) => self.visit_color(c),
            AstValue::Maths(m) => self.visit_maths(m),
        }
    }

    fn walk_block_item(&mut self, node: &'ast AstBlockItem<'a>) -> () {
        match node {
            AstBlockItem::Expression(expr) => self.visit_expression(expr),
            AstBlockItem::ArrayItem(value) => self.visit_value(value),
            AstBlockItem::Conditional(cond) => self.visit_conditional_block(cond),
            AstBlockItem::Whitespace(_) => {}
        }
    }

    fn walk_whitespace_array(&mut self, node: &'ast [CommentOrWhitespace<'a>]) -> () {
        for item in node {
            match item {
                CommentOrWhitespace::Comment(comment) => self.visit_comment(comment),
                CommentOrWhitespace::Whitespace { .. } => {}
            };
        }
    }

    // Terminal node defaults - these should be implemented based on Result type
    fn walk_string(&mut self, _node: &'ast AstString<'a>) -> () {}

    fn walk_comment(&mut self, _node: &'ast AstComment<'a>) -> () {}

    fn walk_number(&mut self, _node: &'ast AstNumber<'a>) -> () {}

    fn walk_color(&mut self, _node: &'ast AstColor<'a>) -> () {}

    fn walk_maths(&mut self, _node: &'ast AstMaths<'a>) -> () {}

    fn walk_operator(&mut self, _node: &'ast AstOperator<'a>) -> () {}

    fn walk_token(&mut self, _node: &'ast AstToken<'a>) -> () {}

    fn walk_conditional_block(&mut self, node: &'ast AstConditionalBlock<'a>) -> () {
        for item in &node.items {
            self.visit_entity_item(item);
        }
    }
}
