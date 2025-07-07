use crate::{
    AstComment, AstCwtBlock, AstCwtComment, AstCwtCommentOption, AstCwtExpression,
    AstCwtIdentifier, AstCwtRule, AstCwtRuleKey, AstString, CwtModule, CwtOperator, CwtSimpleValue,
    CwtValue,
};

/// Visitor trait for traversing the CWT AST
pub trait CwtVisitor<'a> {
    fn visit_module(&mut self, node: &CwtModule<'a>) -> () {
        self.walk_module(node)
    }

    fn visit_expression(&mut self, node: &AstCwtExpression<'a>) -> () {
        self.walk_expression(node)
    }

    fn visit_rule(&mut self, node: &AstCwtRule<'a>) -> () {
        self.walk_rule(node)
    }

    fn visit_block(&mut self, node: &AstCwtBlock<'a>) -> () {
        self.walk_block(node)
    }

    fn visit_identifier(&mut self, node: &AstCwtIdentifier<'a>) -> () {
        self.walk_identifier(node)
    }

    fn visit_value(&mut self, node: &CwtValue<'a>) -> () {
        self.walk_value(node)
    }

    fn visit_simple_value(&mut self, node: &CwtSimpleValue<'a>) -> () {
        self.walk_simple_value(node)
    }

    fn visit_rule_key(&mut self, node: &AstCwtRuleKey<'a>) -> () {
        self.walk_rule_key(node)
    }

    fn visit_option(&mut self, node: &AstCwtCommentOption<'a>) -> () {
        self.walk_option(node)
    }

    fn visit_operator(&mut self, node: &CwtOperator) -> () {
        self.walk_operator(node)
    }

    fn visit_string(&mut self, node: &AstString<'a>) -> () {
        self.walk_string(node)
    }

    fn visit_cwt_doc_comment(&mut self, node: &AstCwtComment<'a>) -> () {
        self.walk_cwt_comment(node)
    }

    fn visit_ast_comment(&mut self, node: &AstComment<'a>) -> () {
        self.walk_ast_comment(node)
    }

    // Default walking implementations
    fn walk_module(&mut self, node: &CwtModule<'a>) -> () {
        // Visit leading comments
        for comment in &node.leading_comments {
            self.visit_cwt_doc_comment(comment);
        }

        // Visit all expressions in the module
        for item in &node.items {
            self.visit_expression(item);
        }

        // Visit trailing comments
        for comment in &node.trailing_comments {
            self.visit_cwt_doc_comment(comment);
        }
    }

    fn walk_expression(&mut self, node: &AstCwtExpression<'a>) -> () {
        match node {
            AstCwtExpression::Rule(rule) => self.visit_rule(rule),
            AstCwtExpression::Block(block) => self.visit_block(block),
            AstCwtExpression::Identifier(identifier) => self.visit_identifier(identifier),
            AstCwtExpression::String(string) => self.visit_string(string),
        }
    }

    fn walk_rule(&mut self, node: &AstCwtRule<'a>) -> () {
        // Visit documentation comment if present
        for doc in &node.documentation {
            self.visit_cwt_doc_comment(doc);
        }

        // Visit key
        self.visit_rule_key(&node.key);

        // Visit operator
        self.visit_operator(&node.operator);

        // Visit value
        self.visit_value(&node.value);

        // Visit options
        for option in &node.options {
            self.visit_option(option);
        }
    }

    fn walk_block(&mut self, node: &AstCwtBlock<'a>) -> () {
        // Visit leading comments
        for comment in &node.leading_comments {
            self.visit_cwt_doc_comment(comment);
        }

        // Visit all expressions in the block
        for item in &node.items {
            self.visit_expression(item);
        }

        // Visit trailing comments
        for comment in &node.trailing_comments {
            self.visit_cwt_doc_comment(comment);
        }
    }

    fn walk_identifier(&mut self, node: &AstCwtIdentifier<'a>) -> () {
        // Visit leading comments
        for comment in &node.leading_comments {
            self.visit_cwt_doc_comment(comment);
        }

        // Visit the identifier name
        self.visit_string(&node.name);

        // Visit trailing comment if present
        if let Some(comment) = &node.trailing_comment {
            self.visit_ast_comment(comment);
        }
    }

    fn walk_value(&mut self, node: &CwtValue<'a>) -> () {
        match node {
            CwtValue::Simple(simple) => self.visit_simple_value(simple),
            CwtValue::Identifier(identifier) => self.visit_identifier(identifier),
            CwtValue::Block(block) => self.visit_block(block),
            CwtValue::String(string) => self.visit_string(string),
        }
    }

    fn walk_rule_key(&mut self, node: &AstCwtRuleKey<'a>) -> () {
        match node {
            AstCwtRuleKey::Identifier(identifier) => self.visit_identifier(identifier),
            AstCwtRuleKey::String(string) => self.visit_string(string),
        }
    }

    // Terminal node defaults - these should be implemented based on visitor needs
    fn walk_simple_value(&mut self, _node: &CwtSimpleValue<'a>) -> () {}

    fn walk_option(&mut self, _node: &AstCwtCommentOption<'a>) -> () {}

    fn walk_operator(&mut self, _node: &CwtOperator) -> () {}

    fn walk_string(&mut self, _node: &AstString<'a>) -> () {}

    fn walk_cwt_comment(&mut self, _node: &AstCwtComment<'a>) -> () {}

    fn walk_ast_comment(&mut self, _node: &AstComment<'a>) -> () {}
}
