mod game_mod;
mod mod_definition;
mod module;
mod namespace;

use std::{collections::HashMap, fmt::Display};

pub use game_mod::*;
use indent::indent_all_by;
pub use mod_definition::*;
pub use module::*;
pub use namespace::*;
use std::fmt::Debug;

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

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Properties {
    pub kv: HashMap<String, PropertyInfoList>,
    pub is_module: bool,
}

/// An operator that can appear between a key and a value in an entity, like a > b. Usually this is = but it depends on the implementation.
/// For our purposes it doesn't really matter, we just have to remember what it is.
#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Operator {
    GreaterThan,
    GreaterThanOrEqual,
    LessThan,
    LessThanOrEqual,
    Equals,
    NotEqual,
    MinusEquals,
    PlusEquals,
    MultiplyEquals,
}

/// Info about the value of an entity's property. The property info contains the "= b" part of "a = b".
#[derive(PartialEq, Eq, Clone)]
pub struct PropertyInfo {
    pub operator: Operator,
    pub value: Value,
}

/// Since a property can have multiple values, we have to store them in a list.
/// For example, for an entity { key = value1 key = value2 }, "key" would have two property info items.
#[derive(PartialEq, Eq, Clone)]
pub struct PropertyInfoList(pub Vec<PropertyInfo>);

/// A value is anything after an =
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum Value {
    String(String),
    Number(String),
    Boolean(bool),
    Entity(Entity),
    Color((String, String, String, String, Option<String>)),
    Maths(String),
}

/// A conditional block looks like [[PARAM_NAME] key = value] and is dumb
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConditionalBlock {
    pub key: (bool, String),
    pub items: Vec<Value>,
    pub properties: Properties,
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

impl Properties {
    pub fn new() -> Self {
        Self {
            kv: HashMap::new(),
            is_module: false,
        }
    }

    pub fn new_module() -> Self {
        Self {
            kv: HashMap::new(),
            is_module: true,
        }
    }
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
    pub fn new(
        items_count: usize,
        properties_count: usize,
        conditional_blocks_count: usize,
    ) -> Self {
        Self {
            items: Vec::with_capacity(items_count),
            properties: Properties {
                kv: HashMap::with_capacity(properties_count),
                is_module: false,
            },
            conditional_blocks: HashMap::with_capacity(conditional_blocks_count),
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
        self.conditional_blocks.insert(value.key.1.clone(), value);
        self
    }
}

impl ToString for PropertyInfo {
    fn to_string(&self) -> String {
        format!("{} {}", self.operator, self.value.to_string())
    }
}

impl PropertyInfoList {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity))
    }

    pub fn with_property(mut self, operator: Operator, value: Value) -> Self {
        self.push(PropertyInfo { operator, value });
        self
    }

    pub fn push(&mut self, property: PropertyInfo) {
        self.0.push(property);
    }

    pub fn iter(&self) -> std::slice::Iter<'_, PropertyInfo> {
        self.0.iter()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn into_vec(self) -> Vec<PropertyInfo> {
        self.0
    }

    pub fn retain(&mut self, f: impl Fn(&PropertyInfo) -> bool) {
        self.0.retain(f);
    }

    pub fn extend(&mut self, other: Vec<PropertyInfo>) {
        self.0.extend(other);
    }
}

impl From<PropertyInfoList> for Vec<PropertyInfo> {
    fn from(list: PropertyInfoList) -> Self {
        list.0
    }
}

impl ToString for PropertyInfoList {
    fn to_string(&self) -> String {
        let mut buf = String::new();
        for item in self.clone().into_iter() {
            buf.push_str(&format!("{}\n", item.to_string()));
        }
        buf
    }
}

impl IntoIterator for PropertyInfoList {
    type Item = PropertyInfo;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

pub trait ToString {
    fn to_string(&self) -> String;
}

impl Debug for PropertyInfoList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for item in self.clone().into_iter() {
            write!(f, "{:?}\n", item)?;
        }
        Ok(())
    }
}

impl Debug for PropertyInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} {:?}", self.operator, self.value)
    }
}

impl Display for Operator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::GreaterThan => ">",
            Self::GreaterThanOrEqual => ">=",
            Self::LessThan => "<",
            Self::LessThanOrEqual => "<=",
            Self::Equals => "=",
            Self::NotEqual => "!=",
            Self::MinusEquals => "-=",
            Self::PlusEquals => "+=",
            Self::MultiplyEquals => "*=",
        };
        write!(f, "{}", s)
    }
}

impl Debug for Operator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl ToString for Value {
    fn to_string(&self) -> String {
        match self {
            Self::String(v) => format!("{}", v),
            Self::Number(v) => format!("{}", v),
            Self::Boolean(v) => format!("{}", v.to_string()),
            Self::Entity(v) => format!("{}", v.to_string()),
            Self::Color((color_type, a, b, c, d)) => match d {
                Some(d) => format!("{} {{ {} {} {} {} }}", color_type, a, b, c, d),
                None => format!("{} {{ {} {} {} }}", color_type, a, b, c),
            },
            Self::Maths(v) => format!("{}", v),
        }
    }
}

impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Self::Boolean(v)
    }
}

impl From<Entity> for Value {
    fn from(v: Entity) -> Self {
        Self::Entity(v)
    }
}

impl Value {
    pub fn entity(&self) -> &Entity {
        if let Value::Entity(e) = self {
            e
        } else {
            panic!("Expected entity")
        }
    }

    pub fn string(&self) -> &String {
        if let Value::String(s) = self {
            s
        } else {
            panic!("Expected string")
        }
    }

    pub fn number(&self) -> &String {
        if let Value::Number(i) = self {
            i
        } else {
            panic!("Expected number")
        }
    }

    pub fn boolean(&self) -> &bool {
        if let Value::Boolean(b) = self {
            b
        } else {
            panic!("Expected boolean")
        }
    }

    pub fn color(&self) -> (String, String, String, String, Option<String>) {
        if let Value::Color((color_type, h, s, v, a)) = self {
            (
                color_type.clone(),
                h.clone(),
                s.clone(),
                v.clone(),
                a.clone(),
            )
        } else {
            panic!("Expected hsv")
        }
    }

    pub fn maths(&self) -> &String {
        if let Value::Maths(m) = self {
            m
        } else {
            panic!("Expected maths")
        }
    }

    pub fn is_entity(&self) -> bool {
        matches!(self, Value::Entity(_))
    }

    pub fn is_string(&self) -> bool {
        matches!(self, Value::String(_))
    }

    pub fn is_number(&self) -> bool {
        matches!(self, Value::Number(_))
    }

    pub fn is_boolean(&self) -> bool {
        matches!(self, Value::Boolean(_))
    }

    pub fn is_color(&self) -> bool {
        matches!(self, Value::Color((_, _, _, _, _)))
    }

    pub fn is_maths(&self) -> bool {
        matches!(self, Value::Maths(_))
    }
}

impl ToString for Module {
    fn to_string(&self) -> String {
        let mut buf = String::from("");
        for value in &self.values {
            let value = format!("{}\n", value.to_string());
            buf.push_str(&value);
        }
        for (key, value) in &self.properties.kv {
            let value = format!("{} = {}\n", key, value.to_string());
            buf.push_str(&value);
        }
        buf
    }
}

impl ToString for ConditionalBlock {
    fn to_string(&self) -> String {
        let mut buf = String::from("[[");

        let (is_not, key) = &self.key;
        if *is_not {
            buf.push_str("!");
        }

        buf.push_str(key);
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

impl From<Value> for PropertyInfoList {
    fn from(v: Value) -> Self {
        Self(vec![v.into()])
    }
}

impl From<Value> for PropertyInfo {
    fn from(v: Value) -> Self {
        Self {
            operator: Operator::Equals,
            value: v,
        }
    }
}

impl From<Entity> for PropertyInfo {
    fn from(v: Entity) -> Self {
        Value::Entity(v).into()
    }
}

impl From<Entity> for PropertyInfoList {
    fn from(e: Entity) -> Self {
        Value::Entity(e).into()
    }
}
