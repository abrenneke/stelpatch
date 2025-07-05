use std::collections::HashMap;

use crate::{Entity, Operator, Value, ValueVisitor};

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Properties {
    pub kv: HashMap<String, PropertyInfoList>,
    pub is_module: bool,
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

/// Info about the value of an entity's property. The property info contains the "= b" part of "a = b".
#[derive(PartialEq, Eq, Clone)]
pub struct PropertyInfo {
    pub operator: Operator,
    pub value: Value,
}

impl Default for PropertyInfo {
    fn default() -> Self {
        Self {
            operator: Operator::Equals,
            value: Value::default(),
        }
    }
}

impl ToString for PropertyInfo {
    fn to_string(&self) -> String {
        format!("{} {}", self.operator, self.value.to_string())
    }
}

/// Since a property can have multiple values, we have to store them in a list.
/// For example, for an entity { key = value1 key = value2 }, "key" would have two property info items.
#[derive(PartialEq, Eq, Clone)]
pub struct PropertyInfoList(pub Vec<PropertyInfo>);

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

impl std::fmt::Debug for PropertyInfoList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for item in self.clone().into_iter() {
            write!(f, "{:?}\n", item)?;
        }
        Ok(())
    }
}

impl std::fmt::Debug for PropertyInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} {:?}", self.operator, self.value)
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

pub(crate) struct PropertyVisitor<'a> {
    property: &'a mut PropertyInfo,
}

impl<'a> PropertyVisitor<'a> {
    pub fn new(property: &'a mut PropertyInfo) -> Self {
        Self { property }
    }
}

impl<'a, 'b> cw_parser::AstVisitor<'b> for PropertyVisitor<'a> {
    fn visit_operator(&mut self, node: &cw_parser::AstOperator<'b>) -> () {
        self.property.operator = node.operator.into();
    }

    fn visit_value(&mut self, node: &cw_parser::AstValue<'b>) -> () {
        let mut value = Value::default();
        let mut value_visitor = ValueVisitor::new(&mut value);
        value_visitor.visit_value(node);
        self.property.value = value;
    }
}
