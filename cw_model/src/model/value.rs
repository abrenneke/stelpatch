use crate::{Entity, EntityVisitor};

/// A value is anything after an =
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum Value {
    String(String),
    Number(String),
    Entity(Entity),
    Color(Color),
    Maths(String),
}

impl Default for Value {
    fn default() -> Self {
        Self::String(String::new())
    }
}

impl ToString for Value {
    fn to_string(&self) -> String {
        match self {
            Self::String(v) => format!("{}", v),
            Self::Number(v) => format!("{}", v),
            Self::Entity(v) => format!("{}", v.to_string()),
            Self::Color(c) => match &c.a {
                Some(a) => format!("{} {{ {} {} {} {} }}", c.color_type, c.r, c.g, c.b, a),
                None => format!("{} {{ {} {} {} }}", c.color_type, c.r, c.g, c.b),
            },
            Self::Maths(v) => format!("{}", v),
        }
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Color {
    color_type: String,
    r: String,
    g: String,
    b: String,
    a: Option<String>,
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

    pub fn color(&self) -> &Color {
        if let Value::Color(c) = self {
            c
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

    pub fn is_color(&self) -> bool {
        matches!(self, Value::Color(_))
    }

    pub fn is_maths(&self) -> bool {
        matches!(self, Value::Maths(_))
    }

    pub fn as_entity(&self) -> Option<&Entity> {
        if let Value::Entity(e) = self {
            Some(e)
        } else {
            None
        }
    }
}

pub(crate) struct ValueVisitor<'a> {
    value: &'a mut Value,
}

impl<'a> ValueVisitor<'a> {
    pub fn new(value: &'a mut Value) -> Self {
        Self { value }
    }
}

impl<'a, 'b> cw_parser::AstVisitor<'b> for ValueVisitor<'a> {
    fn visit_string(&mut self, node: &cw_parser::AstString<'b>) -> () {
        *self.value = Value::String(node.value.value.to_string());
    }

    fn visit_number(&mut self, node: &cw_parser::AstNumber<'b>) -> () {
        *self.value = Value::Number(node.value.value.to_string());
    }

    fn visit_color(&mut self, node: &cw_parser::AstColor<'b>) -> () {
        *self.value = Value::Color(Color {
            color_type: node.color_type.to_string(),
            r: node.r.value.to_string(),
            g: node.g.value.to_string(),
            b: node.b.value.to_string(),
            a: node.a.as_ref().map(|a| a.value.to_string()),
        });
    }

    fn visit_maths(&mut self, node: &cw_parser::AstMaths<'b>) -> () {
        *self.value = Value::Maths(node.value.value.to_string());
    }

    fn visit_entity(&mut self, node: &cw_parser::AstEntity<'b>) -> () {
        let mut entity = Entity::new();
        let mut entity_visitor = EntityVisitor::new(&mut entity);
        entity_visitor.visit_entity(node);
        *self.value = Value::Entity(entity);
    }
}
