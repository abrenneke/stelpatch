use std::sync::Arc;

use cw_parser::{AstEntity, AstModule, AstVisitor};
use indent::indent_all_by;

use crate::{
    CaseInsensitiveInterner, ConditionalBlock, ConditionalBlockVisitor, Operator, Properties,
    PropertyInfo, PropertyInfoList, PropertyVisitor, SpurMap, Value, ValueVisitor,
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
    pub conditional_blocks: SpurMap<ConditionalBlock>,
}

impl ToString for Entity {
    fn to_string(&self) -> String {
        let mut buf = String::from("{\n");
        for value in &self.items {
            let stringified = indent_all_by(4, format!("{}\n", value.to_string()));
            buf.push_str(&stringified);
        }

        for (key, value) in &self.properties.kv {
            for item in value.iter() {
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
                kv: SpurMap::new(),
                is_module: false,
            },
            conditional_blocks: SpurMap::new(),
        }
    }

    pub fn with_property(
        mut self,
        key: &str,
        value: Value,
        interner: &CaseInsensitiveInterner,
    ) -> Self {
        let list = self
            .properties
            .kv
            .entry(interner.get_or_intern(key))
            .or_insert_with(|| Arc::new(PropertyInfoList::new()));
        let list = Arc::make_mut(list);
        list.push(PropertyInfo {
            operator: Operator::Equals,
            value,
        });
        self
    }

    pub fn with_property_values<I: IntoIterator<Item = Value>>(
        mut self,
        key: &str,
        values: I,
        interner: &CaseInsensitiveInterner,
    ) -> Self {
        let items = self
            .properties
            .kv
            .entry(interner.get_or_intern(key))
            .or_insert_with(|| Arc::new(PropertyInfoList::new()));
        let list = Arc::make_mut(items);
        for value in values {
            list.push(PropertyInfo {
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
        interner: &CaseInsensitiveInterner,
    ) -> Self {
        let list = self
            .properties
            .kv
            .entry(interner.get_or_intern(key))
            .or_insert_with(|| Arc::new(PropertyInfoList::new()));
        let list = Arc::make_mut(list);
        list.push(PropertyInfo { operator, value });
        self
    }

    pub fn with_item(mut self, value: Value) -> Self {
        self.items.push(value);
        self
    }

    pub fn with_conditional(mut self, value: ConditionalBlock) -> Self {
        self.conditional_blocks.insert(value.key, value);
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

pub fn entity_from_ast<'a>(ast: &AstEntity<'a>, interner: &CaseInsensitiveInterner) -> Entity {
    let mut entity = Entity::new();
    let mut entity_visitor = EntityVisitor::new(&mut entity, interner);
    entity_visitor.visit_entity(ast);
    entity
}

pub fn entity_from_module_ast<'a>(
    ast: &AstModule<'a>,
    interner: &CaseInsensitiveInterner,
) -> Entity {
    let mut entity = Entity::new();
    let mut entity_visitor = EntityVisitor::new(&mut entity, interner);
    entity_visitor.visit_module(ast);
    entity
}

pub(crate) struct EntityVisitor<'a, 'interner> {
    entity: &'a mut Entity,
    interner: &'interner CaseInsensitiveInterner,
}

impl<'a, 'interner> EntityVisitor<'a, 'interner> {
    pub fn new(entity: &'a mut Entity, interner: &'interner CaseInsensitiveInterner) -> Self {
        Self { entity, interner }
    }
}

impl<'a, 'b, 'ast, 'interner> cw_parser::AstVisitor<'b, 'ast> for EntityVisitor<'a, 'interner>
where
    'b: 'ast,
{
    fn visit_expression(&mut self, node: &cw_parser::AstExpression<'b>) -> () {
        let mut property = PropertyInfo::default();
        let mut property_visitor = PropertyVisitor::new(&mut property, self.interner);
        property_visitor.visit_expression(node);
        let list = self
            .entity
            .properties
            .kv
            .entry(self.interner.get_or_intern(node.key.raw_value()))
            .or_insert_with(|| Arc::new(PropertyInfoList::new()));
        let list = Arc::make_mut(list);
        list.push(property);
    }

    fn visit_value(&mut self, node: &cw_parser::AstValue<'b>) -> () {
        let mut value = Value::default();
        let mut value_visitor = ValueVisitor::new(&mut value, self.interner);
        value_visitor.visit_value(node);
        self.entity.items.push(value);
    }

    fn visit_conditional_block(&mut self, node: &cw_parser::AstConditionalBlock<'b>) -> () {
        let mut conditional_block = ConditionalBlock::default();
        let mut conditional_block_visitor =
            ConditionalBlockVisitor::new(&mut conditional_block, self.interner);
        conditional_block_visitor.visit_conditional_block(node);
        self.entity.conditional_blocks.insert(
            self.interner.get_or_intern(node.key.clone()),
            conditional_block,
        );
    }
}
