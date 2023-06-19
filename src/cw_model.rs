use std::{
    collections::HashMap,
    fmt::{Debug, Display},
    path::Path,
    str::FromStr,
};

use anyhow::anyhow;
use indent::indent_all_by;
use lasso::{Spur, ThreadedRodeo};

use crate::{
    cw_parser::parser::{
        ParsedEntity, ParsedModule, ParsedProperties, ParsedPropertyInfo, ParsedPropertyInfoList,
        ParsedValue,
    },
    playset::{diff::EntityMergeMode, statics::get_merge_mode_for_namespace},
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
    pub conditional_blocks: HashMap<Spur, ConditionalBlock>,
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Properties {
    pub kv: HashMap<Spur, PropertyInfoList>,
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
    String(Spur),
    Number(Spur),
    Boolean(bool),
    Entity(Entity),
    Color((Spur, Spur, Spur, Spur, Option<Spur>)),
    Maths(Spur),
}

/// A conditional block looks like [[PARAM_NAME] key = value] and is dumb
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConditionalBlock {
    pub key: (bool, Spur),
    pub items: Vec<Value>,
    pub properties: Properties,
}

/// A Module is a single file inside of a Namespace. Another module in the same namespace with the same name will overwrite
/// the previous module in the game's load order. Entities in a module are unique in a namespace. An entity defined in one module
/// and defined in another module with a different name will be overwritten by the second module in the game's load order. If two
/// modules at the same point in the load order define the same entity, the entity will be overwritten by the second module's name alphabetically.
/// This is why some modules start with 00_, 01_, etc. to ensure they are loaded first and get overridden first.
#[derive(Debug, PartialEq, Clone)]
pub struct Module {
    pub filename: Spur,
    pub namespace: Spur,
    pub properties: Properties,
    pub values: Vec<Value>,
}

/// A Namespace is the path to the folder containing module files in the `common` directory. Maybe other directories too.
/// E.g. common/armies is the namespace, and contains modules with unique names. All modules in a namespace are combined together following
/// the rules above in Module.
#[derive(Debug, PartialEq, Clone)]
pub struct Namespace {
    pub namespace: Spur,
    pub properties: Properties,
    pub values: Vec<Value>,
    pub modules: HashMap<Spur, Module>,
    pub merge_mode: EntityMergeMode,
}

impl Namespace {
    pub fn new(
        namespace: &str,
        merge_mode: Option<EntityMergeMode>,
        interner: &ThreadedRodeo,
    ) -> Self {
        let ns = Self {
            namespace: interner.get_or_intern(namespace),
            properties: Properties::new_module(),
            values: Vec::new(),
            modules: HashMap::new(),
            merge_mode: merge_mode
                .unwrap_or_else(|| get_merge_mode_for_namespace(&namespace.clone())),
        };

        ns
    }

    pub fn insert(&mut self, module: Module) -> &Self {
        // TODO: properties should follow the merge mode, technically, but it's unlikely a single
        // mod will define the same property twice in the same namespace, so for now we can treat it like
        // EntityMergeMode::LIOS
        self.properties.kv.extend(module.properties.kv);
        self.values.extend(module.values);

        // self.modules.insert(module.path(), module);

        self
    }

    pub fn get_module(&self, module_name: &str, interner: &ThreadedRodeo) -> Option<&Module> {
        self.modules.get(&interner.get_or_intern(module_name))
    }

    pub fn get_only(&self, key: &str, interner: &ThreadedRodeo) -> Option<&Value> {
        if let Some(value) = self.properties.kv.get(&interner.get_or_intern(key)) {
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

impl ToStringWithInterner for Entity {
    fn to_string_with_interner(&self, interner: &ThreadedRodeo) -> String {
        let mut buf = String::from("{\n");
        for value in &self.items {
            let stringified =
                indent_all_by(4, format!("{}\n", value.to_string_with_interner(interner)));
            buf.push_str(&stringified);
        }

        for (key, value) in &self.properties.kv {
            for item in value.clone().into_iter() {
                let stringified = indent_all_by(
                    4,
                    format!(
                        "{:?} {}\n",
                        interner.resolve(key),
                        item.to_string_with_interner(interner)
                    ),
                );
                buf.push_str(&stringified);
            }
        }

        for (_, conditional_block) in &self.conditional_blocks {
            let stringified = indent_all_by(
                4,
                format!("{}\n", conditional_block.to_string_with_interner(interner)),
            );
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

    pub fn with_property(mut self, key: &str, value: Value, interner: &ThreadedRodeo) -> Self {
        self.properties
            .kv
            .entry(interner.get_or_intern(key))
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
        interner: &ThreadedRodeo,
    ) -> Self {
        let items = self
            .properties
            .kv
            .entry(interner.get_or_intern(key))
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
        interner: &ThreadedRodeo,
    ) -> Self {
        self.properties
            .kv
            .entry(interner.get_or_intern(key))
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
        self.conditional_blocks.insert(value.key.1, value);
        self
    }
}

impl ToStringWithInterner for PropertyInfo {
    fn to_string_with_interner(&self, interner: &ThreadedRodeo) -> String {
        format!(
            "{} {}",
            self.operator,
            self.value.to_string_with_interner(interner)
        )
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

impl ToStringWithInterner for PropertyInfoList {
    fn to_string_with_interner(&self, interner: &ThreadedRodeo) -> String {
        let mut buf = String::new();
        for item in self.clone().into_iter() {
            buf.push_str(&format!("{}\n", item.to_string_with_interner(interner)));
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

pub trait ToStringWithInterner {
    fn to_string_with_interner(&self, interner: &ThreadedRodeo) -> String;
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

impl ToStringWithInterner for Value {
    fn to_string_with_interner(&self, interner: &ThreadedRodeo) -> String {
        match self {
            Self::String(v) => format!("{}", interner.resolve(v)),
            Self::Number(v) => format!("{}", interner.resolve(v)),
            Self::Boolean(v) => format!("{}", v.to_string()),
            Self::Entity(v) => format!("{}", v.to_string_with_interner(interner)),
            Self::Color((color_type, a, b, c, d)) => match d {
                Some(d) => format!(
                    "{} {{ {} {} {} {} }}",
                    interner.resolve(color_type),
                    interner.resolve(a),
                    interner.resolve(b),
                    interner.resolve(c),
                    interner.resolve(d)
                ),
                None => format!(
                    "{} {{ {} {} {} }}",
                    interner.resolve(color_type),
                    interner.resolve(a),
                    interner.resolve(b),
                    interner.resolve(c)
                ),
            },
            Self::Maths(v) => format!("{}", interner.resolve(v)),
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

    pub fn string(&self) -> &Spur {
        if let Value::String(s) = self {
            s
        } else {
            panic!("Expected string")
        }
    }

    pub fn number(&self) -> &Spur {
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

    pub fn color(&self) -> (Spur, Spur, Spur, Spur, Option<Spur>) {
        if let Value::Color((color_type, h, s, v, a)) = self {
            (*color_type, *h, *s, *v, *a)
        } else {
            panic!("Expected hsv")
        }
    }

    pub fn maths(&self) -> &Spur {
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

impl ToStringWithInterner for Module {
    fn to_string_with_interner(&self, interner: &ThreadedRodeo) -> String {
        let mut buf = String::from("");
        for value in &self.values {
            let value = format!("{}\n", value.to_string_with_interner(interner));
            buf.push_str(&value);
        }
        for (key, value) in &self.properties.kv {
            let value = format!(
                "{} = {}\n",
                interner.resolve(key),
                value.to_string_with_interner(interner)
            );
            buf.push_str(&value);
        }
        buf
    }
}

impl Module {
    pub fn new(filename: &str, namespace: &str, interner: &ThreadedRodeo) -> Self {
        Self {
            filename: interner.get_or_intern(filename),
            namespace: interner.get_or_intern(namespace.replace("\\", "/")),
            properties: Properties::new_module(),
            values: Vec::new(),
        }
    }

    pub fn add_property(&mut self, key: Spur, value: PropertyInfoList) {
        self.properties.kv.insert(key, value);
    }

    // pub fn add_entity(&mut self, key: String, value: Value) {
    //     self.entities.insert(key, value);
    // }

    pub fn add_value(&mut self, value: Value) {
        self.values.push(value);
    }

    pub fn get_property(&self, key: &Spur) -> Option<&PropertyInfoList> {
        self.properties.kv.get(key)
    }

    pub fn get_only_property(&self, key: &Spur) -> Option<&Value> {
        if let Some(properties) = self.properties.kv.get(key) {
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

    pub fn path(&self, interner: &ThreadedRodeo) -> String {
        format!(
            "{}/{}",
            interner.resolve(&self.namespace),
            interner.resolve(&self.filename)
        )
    }
}

impl ParsedModule<'_> {
    pub fn into_module(self, interner: &ThreadedRodeo) -> Module {
        let mut module = Module::new(&self.filename, &self.namespace, interner);

        for value in self.values {
            module.add_value(value.into_value(interner));
        }

        module.properties = self.properties.into_properties(interner);

        module
    }
}

impl ParsedProperties<'_> {
    pub fn into_properties(self, interner: &ThreadedRodeo) -> Properties {
        let mut properties = Properties::new();
        properties.is_module = self.is_module;

        for (key, value) in self.kv {
            properties.kv.insert(
                interner.get_or_intern(key),
                value.into_property_info_list(interner),
            );
        }

        properties
    }
}

impl Module {
    /// Parses a cw module from a file.
    // pub async fn parse_from_file_async(
    //     file_path: &Path,
    // ) -> Result<ParsedModule<'a>, anyhow::Error> {
    //     let (namespace, module_name) = Self::get_module_info(file_path);
    //     let input = tokio::fs::read_to_string(file_path).await?;
    //     parse_module(&input, namespace, module_name)
    // }

    /// Parses a cw module from a file.

    pub fn parse(
        content: &str,
        namespace: &str,
        module_name: &str,
        interner: &ThreadedRodeo,
    ) -> Result<Self, anyhow::Error> {
        let mut parsed_module = ParsedModule::new(namespace, module_name);

        let (properties, values) =
            crate::cw_parser::parser::module::<nom::error::Error<_>>(&content, &module_name)
                .map(|(_, module)| module)
                .map_err(|e| anyhow!(e.to_string()))?;

        parsed_module.properties = properties;
        parsed_module.values = values;

        Ok(parsed_module.into_module(interner))
    }

    pub fn parse_from_file(
        file_path: &Path,
        interner: &ThreadedRodeo,
    ) -> Result<Self, anyhow::Error> {
        let input = std::fs::read_to_string(file_path)?;
        let (namespace, module_name) = ParsedModule::get_module_info(file_path);

        let mut parsed_module = ParsedModule::new(&namespace, &module_name);

        let (properties, values) =
            crate::cw_parser::parser::module::<nom::error::Error<_>>(&input, &module_name)
                .map(|(_, module)| module)
                .map_err(|e| anyhow!(e.to_string()))?;

        parsed_module.properties = properties;
        parsed_module.values = values;

        let module = parsed_module.into_module(interner);
        Ok(module)
    }
}

impl ParsedValue<'_> {
    pub fn into_value(self, interner: &ThreadedRodeo) -> Value {
        match self {
            ParsedValue::Entity(e) => Value::Entity(e.into_entity(interner)),
            ParsedValue::String(s) => Value::String(interner.get_or_intern(s)),
            ParsedValue::Number(n) => Value::Number(interner.get_or_intern(n)),
            ParsedValue::Boolean(b) => Value::Boolean(b),
            ParsedValue::Color((color_type, a, b, c, d)) => Value::Color((
                interner.get_or_intern(color_type),
                interner.get_or_intern(a),
                interner.get_or_intern(b),
                interner.get_or_intern(c),
                d.map(|d| interner.get_or_intern(d)),
            )),
            ParsedValue::Maths(m) => Value::Maths(interner.get_or_intern(m)),
        }
    }
}

impl ParsedEntity<'_> {
    pub fn into_entity(self, interner: &ThreadedRodeo) -> Entity {
        let mut entity = Entity::new(
            self.items.len(),
            self.properties.kv.len(),
            self.conditional_blocks.len(),
        );

        for value in self.items {
            entity.items.push(value.into_value(interner));
        }

        for (key, value) in self.properties.kv {
            entity.properties.kv.insert(
                interner.get_or_intern(key),
                value.into_property_info_list(interner),
            );
        }

        entity
    }
}

impl ParsedPropertyInfo<'_> {
    pub fn into_property_info(self, interner: &ThreadedRodeo) -> PropertyInfo {
        PropertyInfo {
            value: self.value.into_value(interner),
            operator: self.operator,
        }
    }
}

impl ParsedPropertyInfoList<'_> {
    pub fn into_property_info_list(self, interner: &ThreadedRodeo) -> PropertyInfoList {
        let mut list = PropertyInfoList::with_capacity(self.len());

        for value in self.0 {
            list.push(value.into_property_info(interner));
        }

        list
    }
}

impl ToStringWithInterner for ConditionalBlock {
    fn to_string_with_interner(&self, interner: &ThreadedRodeo) -> String {
        let mut buf = String::from("[[");

        let (is_not, key) = &self.key;
        if *is_not {
            buf.push_str("!");
        }

        buf.push_str(interner.resolve(key));
        buf.push_str("]\n");

        for value in &self.items {
            let value = format!("{}\n", value.to_string_with_interner(interner));
            buf.push_str(&value);
        }
        for (key, value) in &self.properties.kv {
            let value = format!(
                "{} {}\n",
                interner.resolve(key),
                value.to_string_with_interner(interner)
            );
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
