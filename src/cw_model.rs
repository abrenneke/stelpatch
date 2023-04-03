use std::{collections::HashMap, fmt::Debug, str::FromStr};

use anyhow::anyhow;
use indent::indent_all_by;

#[derive(PartialEq, Clone)]
pub struct Entity {
    /// Array items in the entity, like { a b c }
    pub items: Vec<Value>,

    /// Key value pairs in the entity, like { a = b } or { a > b }
    pub properties: HashMap<String, Vec<PropertyInfo>>,
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

impl ToString for Entity {
    fn to_string(&self) -> String {
        let mut buf = String::from("{\n");
        for value in &self.items {
            let stringified = indent_all_by(4, format!("{}\n", value.to_string()));
            buf.push_str(&stringified);
        }

        for (key, value) in &self.properties {
            for item in value {
                let stringified = indent_all_by(4, format!("{} {}\n", key, item.to_string()));
                buf.push_str(&stringified);
            }
        }
        buf.push_str("}\n");
        buf
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
            .or_insert_with(Vec::new)
            .push(PropertyInfo {
                operator: Operator::Equals,
                value,
            });
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
            .or_insert_with(Vec::new)
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

impl ToString for PropertyInfo {
    fn to_string(&self) -> String {
        format!("{} {}", self.operator.to_string(), self.value.to_string())
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
}

impl ToString for Operator {
    fn to_string(&self) -> String {
        match self {
            Self::GreaterThan => ">".to_string(),
            Self::GreaterThanOrEqual => ">=".to_string(),
            Self::LessThan => "<".to_string(),
            Self::LessThanOrEqual => "<=".to_string(),
            Self::Equals => "=".to_string(),
            Self::NotEqual => "!=".to_string(),
        }
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

impl ToString for Value {
    fn to_string(&self) -> String {
        match self {
            Self::String(v) => v.to_string(),
            Self::Number(v) => v.to_string(),
            Self::Boolean(v) => v.to_string(),
            Self::Entity(v) => v.to_string(),
            Self::Define(v) => v.to_string(),
            Self::Color((color_type, a, b, c, d)) => match d {
                Some(d) => format!("{} {{ {} {} {} {} }}", color_type, a, b, c, d),
                None => format!("{} {{ {} {} {} }}", color_type, a, b, c),
            },
        }
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

#[derive(Debug)]
pub struct Module {
    pub filename: String,
    pub type_path: String,
    pub entities: HashMap<String, Value>,
    pub defines: HashMap<String, Value>,
    pub properties: HashMap<String, Vec<PropertyInfo>>,
}

impl ToString for Module {
    fn to_string(&self) -> String {
        let mut buf = String::from("");
        for (key, value) in &self.entities {
            let value = format!("{} {}\n", key, value.to_string());
            buf.push_str(&value);
        }
        buf
    }
}
