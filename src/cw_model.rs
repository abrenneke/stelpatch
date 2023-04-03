use std::{
    collections::HashMap,
    fmt::{Debug, Display},
    str::FromStr,
};

use anyhow::anyhow;
use indent::indent_all_by;

#[derive(PartialEq, Clone)]
pub struct Entity {
    /// Array items in the entity, like { a b c }
    pub items: Vec<Value>,

    /// Key value pairs in the entity, like { a = b } or { a > b }
    pub properties: HashMap<String, PropertyInfoList>,
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
        buf.push_str("}\n");
        write!(f, "{}", buf)
    }
}

impl Entity {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            properties: HashMap::new(),
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
}

#[derive(PartialEq, Clone)]
pub struct PropertyInfo {
    pub operator: Operator,
    pub value: Value,
}

impl Display for PropertyInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.operator, self.value)
    }
}

#[derive(PartialEq, Clone)]
pub struct PropertyInfoList(Vec<PropertyInfo>);

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

#[derive(PartialEq, Clone)]
pub enum Value {
    String(String),
    Number(f32),
    Boolean(bool),
    Entity(Entity),
    Define(String),
    Color((String, f32, f32, f32, Option<f32>)),
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

impl From<f32> for Value {
    fn from(v: f32) -> Self {
        Self::Number(v)
    }
}

impl From<i32> for Value {
    fn from(v: i32) -> Self {
        Self::Number(v as f32)
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

    pub fn number(&self) -> &f32 {
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

    pub fn color(&self) -> (String, f32, f32, f32, Option<f32>) {
        if let Value::Color((color_type, h, s, v, a)) = self {
            (color_type.to_owned(), *h, *s, *v, *a)
        } else {
            panic!("Expected hsv")
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
}

#[derive(Debug, PartialEq)]
pub struct Module {
    pub filename: String,
    pub type_path: String,
    pub entities: HashMap<String, Value>,
    pub defines: HashMap<String, Value>,
    pub properties: HashMap<String, PropertyInfoList>,
}

impl Display for Module {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut buf = String::from("");
        for (key, value) in &self.defines {
            let value = format!("{} = {}\n", key, value);
            buf.push_str(&value);
        }
        for (key, value) in &self.properties {
            let value = format!("{} = {}\n", key, value);
            buf.push_str(&value);
        }
        for (key, value) in &self.entities {
            let value = format!("{} = {}\n", key, value);
            buf.push_str(&value);
        }
        write!(f, "{}", buf)
    }
}
