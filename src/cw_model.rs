use std::{collections::HashMap, fmt::Debug, str::FromStr};

use anyhow::anyhow;
use indent::indent_all_by;

#[derive(PartialEq)]
pub struct Entity {
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

#[derive(PartialEq)]
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

#[derive(PartialEq, Clone, Copy)]
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

#[derive(PartialEq)]
pub enum Value {
    String(String),
    Integer(i32),
    Float(f32),
    Boolean(bool),
    Entity(Entity),
    StringArray(Vec<String>),
    Define(String),
    RGB(i32, i32, i32, Option<i32>),
    HSV(f32, f32, f32, Option<f32>),
}

impl ToString for Value {
    fn to_string(&self) -> String {
        match self {
            Self::String(v) => v.to_string(),
            Self::Integer(v) => v.to_string(),
            Self::Float(v) => v.to_string(),
            Self::Boolean(v) => v.to_string(),
            Self::Entity(v) => v.to_string(),
            Self::StringArray(v) => {
                let mut buf = String::from("{\n");
                for item in v {
                    buf.push_str(&format!("\"{}\"\n", item));
                }
                buf.push_str("}\n");
                buf
            }
            Self::Define(v) => v.to_string(),
            Self::RGB(r, g, b, a) => match a {
                Some(a) => format!("rgb {{ {} {} {} {} }}", r, g, b, a),
                None => format!("rgb {{ {} {} {} }}", r, g, b),
            },
            Self::HSV(h, s, v, a) => match a {
                Some(a) => format!("hsv {{ {} {} {} {} }}", h, s, v, a),
                None => format!("hsv {{ {} {} {} }}", h, s, v),
            },
        }
    }
}

impl Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
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

    pub fn integer(&self) -> &i32 {
        if let Value::Integer(i) = self {
            i
        } else {
            panic!("Expected integer")
        }
    }

    pub fn float(&self) -> &f32 {
        if let Value::Float(f) = self {
            f
        } else {
            panic!("Expected float")
        }
    }

    pub fn boolean(&self) -> &bool {
        if let Value::Boolean(b) = self {
            b
        } else {
            panic!("Expected boolean")
        }
    }

    pub fn string_array(&self) -> &Vec<String> {
        if let Value::StringArray(s) = self {
            s
        } else {
            panic!("Expected string array")
        }
    }

    pub fn define(&self) -> &String {
        if let Value::Define(d) = self {
            d
        } else {
            panic!("Expected define")
        }
    }

    pub fn rgb(&self) -> (i32, i32, i32, Option<i32>) {
        if let Value::RGB(r, g, b, a) = self {
            (*r, *g, *b, *a)
        } else {
            panic!("Expected rgb")
        }
    }

    pub fn hsv(&self) -> (f32, f32, f32, Option<f32>) {
        if let Value::HSV(h, s, v, a) = self {
            (*h, *s, *v, *a)
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

    pub fn is_integer(&self) -> bool {
        matches!(self, Value::Integer(_))
    }

    pub fn is_float(&self) -> bool {
        matches!(self, Value::Float(_))
    }

    pub fn is_boolean(&self) -> bool {
        matches!(self, Value::Boolean(_))
    }

    pub fn is_string_array(&self) -> bool {
        matches!(self, Value::StringArray(_))
    }

    pub fn is_define(&self) -> bool {
        matches!(self, Value::Define(_))
    }

    pub fn is_rgb(&self) -> bool {
        matches!(self, Value::RGB(_, _, _, _))
    }

    pub fn is_hsv(&self) -> bool {
        matches!(self, Value::HSV(_, _, _, _))
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
