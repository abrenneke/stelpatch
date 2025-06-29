use crate::{
    AstBlockItem, AstBoolean, AstColor, AstConditionalBlock, AstEntity, AstEntityItem,
    AstExpression, AstMaths, AstModule, AstNumber, AstOperator, AstProperty, AstString, AstToken,
    AstValue,
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

    fn visit_property(&mut self, node: &AstProperty<'a>) -> Self::Result {
        self.walk_property(node)
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

    fn visit_boolean(&mut self, node: &AstBoolean<'a>) -> Self::Result {
        self.walk_boolean(node)
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
            AstEntityItem::Property(prop) => self.visit_property(prop),
            AstEntityItem::Item(value) => self.visit_value(value),
            AstEntityItem::Conditional(cond) => self.visit_conditional_block(cond),
        }
    }

    fn walk_property(&mut self, node: &AstProperty<'a>) -> Self::Result {
        self.visit_string(&node.key);
        self.visit_operator(&node.operator);
        self.visit_value(&node.value);
        Self::Result::default()
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
            AstValue::Boolean(b) => self.visit_boolean(b),
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

    fn walk_boolean(&mut self, _node: &AstBoolean<'a>) -> Self::Result {
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

/// Helper trait for collecting results from visitor traversal
pub trait CollectResults {
    fn collect<I: IntoIterator<Item = Self>>(iter: I) -> Self;
}

impl CollectResults for () {
    fn collect<I: IntoIterator<Item = Self>>(_iter: I) -> Self {
        ()
    }
}

impl<T> CollectResults for Vec<T> {
    fn collect<I: IntoIterator<Item = Vec<T>>>(iter: I) -> Self {
        iter.into_iter().flatten().collect()
    }
}

/// Example visitor for finding all identifiers/strings in the AST
pub struct IdentifierCollector {
    pub identifiers: Vec<String>,
}

impl IdentifierCollector {
    pub fn new() -> Self {
        Self {
            identifiers: Vec::new(),
        }
    }

    pub fn collect_from_module<'a>(mut self, module: &'a AstModule<'a>) -> Vec<String> {
        self.visit_module(module);
        self.identifiers
    }
}

impl<'a> AstVisitor<'a> for IdentifierCollector {
    type Result = ();

    fn walk_string(&mut self, node: &AstString<'a>) -> Self::Result {
        self.identifiers.push(node.raw_value().to_string());
    }
}
