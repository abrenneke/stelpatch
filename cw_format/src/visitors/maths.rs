use cw_parser::{AstMaths, AstVisitor};

pub struct MathsVisitor<'a> {
    output: &'a mut String,
}

impl<'a> MathsVisitor<'a> {
    pub fn new(output: &'a mut String) -> Self {
        Self { output }
    }
}

impl<'a> AstVisitor<'a> for MathsVisitor<'a> {
    type Result = ();

    fn visit_maths(&mut self, node: &AstMaths<'a>) -> Self::Result {
        self.output.push_str(&node.value.to_string());
    }
}
