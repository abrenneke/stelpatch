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

impl<'a, 'ast> AstVisitor<'a, 'ast> for ModuleVisitor<'a>
where
    'a: 'ast,
{
    fn visit_module(&mut self, node: &cw_parser::AstModule<'a>) -> () {
        self.output.push_str(
            &node
                .leading_comments
                .iter()
                .map(|c| format!("#{}\n", c.text))
                .collect::<Vec<_>>()
                .join(""),
        );

        self.walk_module(node);
    }

    fn visit_value(&mut self, node: &cw_parser::AstValue<'a>) -> () {
        let mut visitor = ValueVisitor::new(self.output);
        visitor.visit_value(node);
    }

    fn visit_expression(&mut self, node: &cw_parser::AstExpression<'a>) -> () {
        let mut visitor = ExpressionVisitor::new(self.output);
        visitor.visit_expression(node);
    }
}
