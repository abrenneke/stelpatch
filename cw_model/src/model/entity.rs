use std::collections::HashMap;

use cw_parser::{AstEntity, AstModule, AstVisitor};
use indent::indent_all_by;

use crate::{
    ConditionalBlock, ConditionalBlockVisitor, Operator, Properties, PropertyInfo,
    PropertyInfoList, PropertyVisitor, Value, ValueVisitor,
};

/// An entity is an object with items, key value pairs, and conditional blocks. The majority of values in a module are entities.
/// Entities are like { key = value } or { a b c } or { a > b } or
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Entity {
    /// Array items in the entity, like { a b c }
    pub items: Vec<Value>,

    /// Key value pairs in the entity, like { a = b } or { a > b }
    pub properties: Properties,

    /// Conditional blocks in the entity, like [[CONDITION] { a b c }]
    pub conditional_blocks: HashMap<String, ConditionalBlock>,
}

impl ToString for Entity {
    fn to_string(&self) -> String {
        let mut buf = String::from("{\n");
        for value in &self.items {
            let stringified = indent_all_by(4, format!("{}\n", value.to_string()));
            buf.push_str(&stringified);
        }

        for (key, value) in &self.properties.kv {
            for item in value.clone().into_iter() {
                let stringified = indent_all_by(4, format!("{:?} {}\n", key, item.to_string()));
                buf.push_str(&stringified);
            }
        }

        for (_, conditional_block) in &self.conditional_blocks {
            let stringified = indent_all_by(4, format!("{}\n", conditional_block.to_string()));
            buf.push_str(&stringified);
        }

        buf.push_str("}\n");
        buf
    }
}

impl Entity {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            properties: Properties {
                kv: HashMap::new(),
                is_module: false,
            },
            conditional_blocks: HashMap::new(),
        }
    }

    pub fn with_property(mut self, key: &str, value: Value) -> Self {
        self.properties
            .kv
            .entry(key.to_string())
            .or_insert_with(PropertyInfoList::new)
            .0
            .push(PropertyInfo {
                operator: Operator::Equals,
                value,
            });
        self
    }

    pub fn with_property_values<I: IntoIterator<Item = Value>>(
        mut self,
        key: &str,
        values: I,
    ) -> Self {
        let items = self
            .properties
            .kv
            .entry(key.to_string())
            .or_insert_with(PropertyInfoList::new);
        for value in values {
            items.push(PropertyInfo {
                operator: Operator::Equals,
                value,
            });
        }
        self
    }

    pub fn with_property_with_operator(
        mut self,
        key: &str,
        operator: Operator,
        value: Value,
    ) -> Self {
        self.properties
            .kv
            .entry(key.to_string())
            .or_insert_with(PropertyInfoList::new)
            .0
            .push(PropertyInfo { operator, value });
        self
    }

    pub fn with_item(mut self, value: Value) -> Self {
        self.items.push(value);
        self
    }

    pub fn with_conditional(mut self, value: ConditionalBlock) -> Self {
        self.conditional_blocks.insert(value.key.clone(), value);
        self
    }
}

/// Different namespaces in stellaris have different merge mechanics when it comes to entities with the same name
/// in different files. This defines the merge mode to use for entities with the same name.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum EntityMergeMode {
    /// Last-in-only-served - the last entity in the list will be the one that is used
    LIOS,

    /// First-in-only-served - the first entity in the list will be the one that is used
    FIOS,

    /// FIOS, but use the specified key for duplicates instead of the entity name
    FIOSKeyed(&'static str),

    /// Entities with the same name will be merged
    Merge,

    /// Like LIOS, but for the properties of the entities instead of the entities themselves.
    MergeShallow,

    /// Entities with the same name act like a PropertyInfoList, and there are multiple for the one key
    Duplicate,

    /// Entities cannot be target overridden at all, have to only overwrite at the module level
    No,

    /// Who knows!
    Unknown,
}

pub fn entity_from_ast<'a>(ast: &AstEntity<'a>) -> Entity {
    let mut entity = Entity::new();
    let mut entity_visitor = EntityVisitor::new(&mut entity);
    entity_visitor.visit_entity(ast);
    entity
}

pub fn entity_from_module_ast<'a>(ast: &AstModule<'a>) -> Entity {
    let mut entity = Entity::new();
    let mut entity_visitor = EntityVisitor::new(&mut entity);
    entity_visitor.visit_module(ast);
    entity
}

pub(crate) struct EntityVisitor<'a> {
    entity: &'a mut Entity,
}

impl<'a> EntityVisitor<'a> {
    pub fn new(entity: &'a mut Entity) -> Self {
        Self { entity }
    }
}

impl<'a, 'b, 'ast> cw_parser::AstVisitor<'b, 'ast> for EntityVisitor<'a>
where
    'b: 'ast,
{
    fn visit_expression(&mut self, node: &cw_parser::AstExpression<'b>) -> () {
        let mut property = PropertyInfo::default();
        let mut property_visitor = PropertyVisitor::new(&mut property);
        property_visitor.visit_expression(node);
        self.entity
            .properties
            .kv
            .entry(node.key.value.to_string())
            .or_insert_with(PropertyInfoList::new)
            .0
            .push(property);
    }

    fn visit_value(&mut self, node: &cw_parser::AstValue<'b>) -> () {
        let mut value = Value::default();
        let mut value_visitor = ValueVisitor::new(&mut value);
        value_visitor.visit_value(node);
        self.entity.items.push(value);
    }

    fn visit_conditional_block(&mut self, node: &cw_parser::AstConditionalBlock<'b>) -> () {
        let mut conditional_block = ConditionalBlock::default();
        let mut conditional_block_visitor = ConditionalBlockVisitor::new(&mut conditional_block);
        conditional_block_visitor.visit_conditional_block(node);
        self.entity
            .conditional_blocks
            .insert(node.key.to_string(), conditional_block);
    }
}
