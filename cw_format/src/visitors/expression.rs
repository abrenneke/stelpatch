use cw_parser::AstVisitor;

use crate::visitors::{StringVisitor, ValueVisitor};

pub struct ExpressionVisitor<'a> {
    output: &'a mut String,
}

impl<'a> ExpressionVisitor<'a> {
    pub fn new(output: &'a mut String) -> Self {
        Self { output }
    }
}

impl<'a> AstVisitor<'a> for ExpressionVisitor<'a> {
    type Result = ();

    fn visit_expression(&mut self, node: &cw_parser::AstExpression<'a>) -> Self::Result {
        let mut visitor = StringVisitor::new(self.output);
        visitor.visit_string(&node.key);

        self.output.push_str(" ");
        self.output.push_str(&node.operator.operator.to_string());
        self.output.push_str(" ");

        let mut value_visitor = ValueVisitor::new(self.output);
        value_visitor.visit_value(&node.value);
    }
}
