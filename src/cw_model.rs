use std::{
    collections::HashMap,
    fmt::{Debug, Display},
    str::FromStr,
};

use anyhow::anyhow;
use indent::indent_all_by;
use serde::{Deserialize, Serialize};

use crate::playset::{diff::EntityMergeMode, statics::get_merge_mode_for_namespace};

/// An entity is an object with items, key value pairs, and conditional blocks. The majority of values in a module are entities.
/// Entities are like { key = value } or { a b c } or { a > b } or
#[derive(PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Entity {
    /// Array items in the entity, like { a b c }
    pub items: Vec<Value>,

    /// Key value pairs in the entity, like { a = b } or { a > b }
    pub properties: HashMap<String, PropertyInfoList>,

    /// Conditional blocks in the entity, like [[CONDITION] { a b c }]
    pub conditional_blocks: HashMap<String, ConditionalBlock>,
}

/// An entity with a name, like a = { key = value }
#[derive(PartialEq, Clone, Serialize, Deserialize)]
pub struct NamedEntity(pub Entity, pub String);

/// An operator that can appear between a key and a value in an entity, like a > b. Usually this is = but it depends on the implementation.
/// For our purposes it doesn't really matter, we just have to remember what it is.
#[derive(PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
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
#[derive(PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct PropertyInfo {
    pub operator: Operator,
    pub value: Value,
}

/// Since a property can have multiple values, we have to store them in a list.
/// For example, for an entity { key = value1 key = value2 }, "key" would have two property info items.
#[derive(PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct PropertyInfoList(pub Vec<PropertyInfo>);

/// A value is anything after an =
#[derive(PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum Value {
    String(String),
    Number(String),
    Boolean(bool),
    Entity(Entity),
    Define(String),
    Color((String, String, String, String, Option<String>)),
    Maths(String),
}

/// A conditional block looks like [[PARAM_NAME] key = value] and is dumb
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConditionalBlock {
    pub key: (bool, String),
    pub items: Vec<Value>,
    pub properties: HashMap<String, PropertyInfoList>,
}

/// A Module is a single file inside of a Namespace. Another module in the same namespace with the same name will overwrite
/// the previous module in the game's load order. Entities in a module are unique in a namespace. An entity defined in one module
/// and defined in another module with a different name will be overwritten by the second module in the game's load order. If two
/// modules at the same point in the load order define the same entity, the entity will be overwritten by the second module's name alphabetically.
/// This is why some modules start with 00_, 01_, etc. to ensure they are loaded first and get overridden first.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Module {
    pub filename: String,
    pub namespace: String,
    // pub entities: HashMap<String, Value>,
    pub defines: HashMap<String, Value>,
    pub properties: HashMap<String, PropertyInfoList>,
    pub values: Vec<Value>,
}

/// A Namespace is the path to the folder containing module files in the `common` directory. Maybe other directories too.
/// E.g. common/armies is the namespace, and contains modules with unique names. All modules in a namespace are combined together following
/// the rules above in Module.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Namespace {
    pub namespace: String,
    // pub entities: HashMap<String, Value>,
    pub defines: HashMap<String, Value>,
    pub properties: HashMap<String, PropertyInfoList>,
    pub values: Vec<Value>,
    pub modules: HashMap<String, Module>,
    pub merge_mode: EntityMergeMode,
}

impl Namespace {
    pub fn new(namespace: &str, merge_mode: Option<EntityMergeMode>) -> Self {
        let ns = Self {
            namespace: namespace.to_string(),
            // entities: HashMap::new(),
            defines: HashMap::new(),
            properties: HashMap::new(),
            values: Vec::new(),
            modules: HashMap::new(),
            merge_mode: merge_mode
                .unwrap_or_else(|| get_merge_mode_for_namespace(&namespace.clone())),
        };

        ns
    }

    pub fn insert(&mut self, module: &Module) -> &Self {
        let local_module = module.to_owned();

        // self.entities.extend(local_module.entities.clone());
        self.defines.extend(local_module.defines.clone());

        // TODO: properties should follow the merge mode, technically, but it's unlikely a single
        // mod will define the same property twice in the same namespace, so for now we can treat it like
        // EntityMergeMode::LIOS
        self.properties.extend(local_module.properties.clone());
        self.values.extend(local_module.values.clone());

        self.modules.insert(local_module.path(), local_module);

        self
    }

    pub fn get_module(&self, module_name: &str) -> Option<&Module> {
        self.modules.get(module_name)
    }

    pub fn get_only(&self, key: &str) -> Option<&Value> {
        if let Some(value) = self.properties.get(key) {
            if value.0.len() == 1 {
                return Some(&value.0[0].value);
            }
        }
        None
    }

    // pub fn get_entity(&self, entity_name: &str) -> Option<&Entity> {
    // self.entities.get(entity_name).map(|v| v.entity())
    // }
}

impl Debug for Entity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{{")?;
        for (key, value) in &self.properties {
            writeln!(f, "    {} {:?}", key, value)?;
        }
        writeln!(f, "}}")?;
        Ok(())
    }
}

impl Display for Entity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut buf = String::from("{\n");
        for value in &self.items {
            let stringified = indent_all_by(4, format!("{}\n", value.to_string()));
            buf.push_str(&stringified);
        }

        for (key, value) in &self.properties {
            for item in value.clone().into_iter() {
                let stringified = indent_all_by(4, format!("{} {}\n", key, item.to_string()));
                buf.push_str(&stringified);
            }
        }

        for (_, conditional_block) in &self.conditional_blocks {
            let stringified = indent_all_by(4, format!("{}\n", conditional_block.to_string()));
            buf.push_str(&stringified);
        }

        buf.push_str("}\n");
        write!(f, "{}", buf)
    }
}

impl Entity {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            properties: HashMap::new(),
            conditional_blocks: HashMap::new(),
        }
    }

    pub fn with_property(mut self, key: &str, value: Value) -> Self {
        self.properties
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
        self.conditional_blocks
            .insert(value.key.1.to_owned(), value);
        self
    }
}

impl Display for PropertyInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.operator, self.value)
    }
}

impl PropertyInfoList {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn with_property(mut self, operator: Operator, value: Value) -> Self {
        self.push(PropertyInfo { operator, value });
        self
    }

    pub fn push(&mut self, property: PropertyInfo) {
        self.0.push(property);
    }

    pub fn iter(&self) -> std::slice::Iter<PropertyInfo> {
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

impl Display for PropertyInfoList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for item in self.clone().into_iter() {
            write!(f, "{}\n", item)?;
        }
        Ok(())
    }
}

impl IntoIterator for PropertyInfoList {
    type Item = PropertyInfo;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
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
            Self::GreaterThan => ">".to_string(),
            Self::GreaterThanOrEqual => ">=".to_string(),
            Self::LessThan => "<".to_string(),
            Self::LessThanOrEqual => "<=".to_string(),
            Self::Equals => "=".to_string(),
            Self::NotEqual => "!=".to_string(),
            Self::MinusEquals => "-=".to_string(),
            Self::PlusEquals => "+=".to_string(),
            Self::MultiplyEquals => "*=".to_string(),
        };
        write!(f, "{}", s)
    }
}

impl Debug for Operator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl FromStr for Operator {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            ">" => Ok(Operator::GreaterThan),
            ">=" => Ok(Operator::GreaterThanOrEqual),
            "<" => Ok(Operator::LessThan),
            "<=" => Ok(Operator::LessThanOrEqual),
            "=" => Ok(Operator::Equals),
            "!=" => Ok(Operator::NotEqual),
            "-=" => Ok(Operator::MinusEquals),
            "+=" => Ok(Operator::PlusEquals),
            "*=" => Ok(Operator::MultiplyEquals),
            _ => Err(anyhow!("Invalid operator: {}", s)),
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::String(v) => v.to_string(),
            Self::Number(v) => v.to_string(),
            Self::Boolean(v) => v.to_string(),
            Self::Entity(v) => v.to_string(),
            Self::Define(v) => v.to_string(),
            Self::Color((color_type, a, b, c, d)) => match d {
                Some(d) => format!("{} {{ {} {} {} {} }}", color_type, a, b, c, d),
                None => format!("{} {{ {} {} {} }}", color_type, a, b, c),
            },
            Self::Maths(v) => v.to_string(),
        };
        write!(f, "{}", s)
    }
}

impl Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
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

    pub fn define(&self) -> &String {
        if let Value::Define(d) = self {
            d
        } else {
            panic!("Expected define")
        }
    }

    pub fn color(&self) -> (String, String, String, String, Option<String>) {
        if let Value::Color((color_type, h, s, v, a)) = self {
            (
                color_type.to_owned(),
                h.to_owned(),
                s.to_owned(),
                v.to_owned(),
                a.to_owned(),
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

    pub fn is_define(&self) -> bool {
        matches!(self, Value::Define(_))
    }

    pub fn is_color(&self) -> bool {
        matches!(self, Value::Color((_, _, _, _, _)))
    }

    pub fn is_maths(&self) -> bool {
        matches!(self, Value::Maths(_))
    }
}

impl Display for Module {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut buf = String::from("");
        for value in &self.values {
            let value = format!("{}\n", value);
            buf.push_str(&value);
        }
        for (key, value) in &self.defines {
            let value = format!("{} = {}\n", key, value);
            buf.push_str(&value);
        }
        for (key, value) in &self.properties {
            let value = format!("{} = {}\n", key, value);
            buf.push_str(&value);
        }
        // for (key, value) in &self.entities {
        //     let value = format!("{} = {}\n", key, value);
        //     buf.push_str(&value);
        // }
        write!(f, "{}", buf)
    }
}

impl Module {
    pub fn new(filename: String, namespace: String) -> Self {
        Self {
            filename,
            namespace: namespace.replace("\\", "/"),
            // entities: HashMap::new(),
            defines: HashMap::new(),
            properties: HashMap::new(),
            values: Vec::new(),
        }
    }

    pub fn add_define(&mut self, key: String, value: Value) {
        self.defines.insert(key, value);
    }

    pub fn add_property(&mut self, key: String, value: PropertyInfoList) {
        self.properties.insert(key, value);
    }

    // pub fn add_entity(&mut self, key: String, value: Value) {
    //     self.entities.insert(key, value);
    // }

    pub fn add_value(&mut self, value: Value) {
        self.values.push(value);
    }

    pub fn get_define(&self, key: &str) -> Option<&Value> {
        self.defines.get(key)
    }

    pub fn get_property(&self, key: &str) -> Option<&PropertyInfoList> {
        self.properties.get(key)
    }

    pub fn get_only_property(&self, key: &str) -> Option<&Value> {
        if let Some(properties) = self.properties.get(key) {
            if properties.len() == 1 {
                return Some(&properties.0[0].value);
            } else {
                panic!("Expected only one property");
            }
        }
        None
    }

    // pub fn get_entity(&self, key: &str) -> Option<&Value> {
    //     self.entities.get(key)
    // }

    pub fn path(&self) -> String {
        format!("{}/{}", self.namespace, self.filename)
    }
}

impl Display for ConditionalBlock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut buf = String::from("[[");

        let (is_not, key) = &self.key;
        if *is_not {
            buf.push_str("!");
        }

        buf.push_str(key);
        buf.push_str("]\n");

        for value in &self.items {
            let value = format!("{}\n", value);
            buf.push_str(&value);
        }
        for (key, value) in &self.properties {
            let value = format!("{} {}\n", key, value);
            buf.push_str(&value);
        }

        buf.push_str("]\n");

        write!(f, "{}", buf)
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
