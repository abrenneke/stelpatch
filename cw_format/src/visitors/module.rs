use cw_parser::AstVisitor;

use crate::visitors::{ExpressionVisitor, ValueVisitor};

pub struct ModuleVisitor<'a> {
    output: &'a mut String,
}

impl<'a> ModuleVisitor<'a> {
    pub fn new(output: &'a mut String) -> Self {
        Self { output }
    }
}

impl<'a> AstVisitor<'a> for ModuleVisitor<'a> {
    type Result = ();

    fn visit_value(&mut self, node: &cw_parser::AstValue<'a>) -> Self::Result {
        let mut visitor = ValueVisitor::new(self.output);
        visitor.visit_value(node);
    }

    fn visit_expression(&mut self, node: &cw_parser::AstExpression<'a>) -> Self::Result {
        let mut visitor = ExpressionVisitor::new(self.output);
        visitor.visit_expression(node);
    }
}
