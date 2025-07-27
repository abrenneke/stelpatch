use cw_parser::AstValue;
use lasso::Spur;

use crate::{CaseInsensitiveInterner, Entity, EntityVisitor};

/// A value is anything after an =
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum Value {
    String(Spur),
    Number(Spur),
    Entity(Entity),
    Color(Color),
    Maths(Spur),
}

impl Default for Value {
    fn default() -> Self {
        Self::String(Spur::default())
    }
}

impl ToString for Value {
    fn to_string(&self) -> String {
        match self {
            Self::String(v) => format!("{:?}", v),
            Self::Number(v) => format!("{:?}", v),
            Self::Entity(v) => format!("{}", v.to_string()),
            Self::Color(c) => match &c.a {
                Some(a) => format!("{} {{ {} {} {} {} }}", c.color_type, c.r, c.g, c.b, a),
                None => format!("{} {{ {} {} {} }}", c.color_type, c.r, c.g, c.b),
            },
            Self::Maths(v) => format!("{:?}", v),
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

    pub fn color(&self) -> &Color {
        if let Value::Color(c) = self {
            c
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

    pub fn as_string(&self) -> Option<&Spur> {
        if let Value::String(s) = self {
            Some(s)
        } else {
            None
        }
    }
}

pub(crate) struct ValueVisitor<'a, 'interner> {
    value: &'a mut Value,
    interner: &'interner CaseInsensitiveInterner,
}

impl<'a, 'interner> ValueVisitor<'a, 'interner> {
    pub fn new(value: &'a mut Value, interner: &'interner CaseInsensitiveInterner) -> Self {
        Self { value, interner }
    }
}

impl<'a, 'b, 'ast, 'interner> cw_parser::AstVisitor<'b, 'ast> for ValueVisitor<'a, 'interner>
where
    'b: 'ast,
{
    fn visit_string(&mut self, node: &cw_parser::AstString<'b>) -> () {
        *self.value = Value::String(self.interner.get_or_intern(node.raw_value()));
    }

    fn visit_number(&mut self, node: &cw_parser::AstNumber<'b>) -> () {
        *self.value = Value::Number(self.interner.get_or_intern(node.value.value));
    }

    fn visit_color(&mut self, node: &cw_parser::AstColor<'b>) -> () {
        *self.value = Value::Color(Color {
            color_type: node.color_type.to_string(),
            r: match &node.r {
                AstValue::Number(n) => n.value.value.to_string(),
                AstValue::String(s) => s.value.to_string(),
                _ => panic!("Expected number or string"),
            },
            g: match &node.g {
                AstValue::Number(n) => n.value.value.to_string(),
                AstValue::String(s) => s.value.to_string(),
                _ => panic!("Expected number or string"),
            },
            b: match &node.b {
                AstValue::Number(n) => n.value.value.to_string(),
                AstValue::String(s) => s.value.to_string(),
                _ => panic!("Expected number or string"),
            },
            a: node.a.as_ref().map(|a| match a {
                AstValue::Number(n) => n.value.value.to_string(),
                AstValue::String(s) => s.value.to_string(),
                _ => panic!("Expected number or string"),
            }),
        });
    }

    fn visit_maths(&mut self, node: &cw_parser::AstMaths<'b>) -> () {
        *self.value = Value::Maths(self.interner.get_or_intern(node.value.value));
    }

    fn visit_entity(&mut self, node: &cw_parser::AstEntity<'b>) -> () {
        let mut entity = Entity::new();
        let mut entity_visitor = EntityVisitor::new(&mut entity, self.interner);
        entity_visitor.visit_entity(node);
        *self.value = Value::Entity(entity);
    }
}
