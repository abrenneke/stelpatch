use cw_parser::{AstValue, AstVisitor};

use crate::visitors::{ColorVisitor, EntityVisitor, MathsVisitor, NumberVisitor, StringVisitor};

pub struct ValueVisitor<'a> {
    output: &'a mut String,
}

impl<'a> ValueVisitor<'a> {
    pub fn new(output: &'a mut String) -> Self {
        Self { output }
    }
}

impl<'a, 'ast> AstVisitor<'a, 'ast> for ValueVisitor<'a>
where
    'a: 'ast,
{
    fn visit_value(&mut self, node: &AstValue<'a>) -> () {
        match node {
            AstValue::String(string) => {
                let mut visitor = StringVisitor::new(self.output);
                visitor.visit_string(string);
            }
            AstValue::Number(number) => {
                let mut visitor = NumberVisitor::new(self.output);
                visitor.visit_number(number);
            }
            AstValue::Entity(entity) => {
                let mut visitor = EntityVisitor::new(self.output);
                visitor.visit_entity(entity);
            }
            AstValue::Color(color) => {
                let mut visitor = ColorVisitor::new(self.output);
                visitor.visit_color(color);
            }
            AstValue::Maths(maths) => {
                let mut visitor = MathsVisitor::new(self.output);
                visitor.visit_maths(maths);
            }
        }
    }
}
