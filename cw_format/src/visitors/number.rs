use cw_parser::{AstNumber, AstVisitor};

pub struct NumberVisitor<'a> {
    output: &'a mut String,
}

impl<'a> NumberVisitor<'a> {
    pub fn new(output: &'a mut String) -> Self {
        Self { output }
    }
}

impl<'a> AstVisitor<'a> for NumberVisitor<'a> {
    fn visit_number(&mut self, node: &AstNumber<'a>) -> () {
        // TODO
        // if node.leading_newlines > 0 {
        //     self.output.push_str(&"\n".repeat(node.leading_newlines));
        // }

        for comment in node.leading_comments.iter() {
            self.output.push_str(comment.text);
            self.output.push_str("\n");
        }

        self.output.push_str(&node.value.to_string());

        if node.is_percentage {
            self.output.push_str("%");
        }

        if let Some(trailing_comment) = node.trailing_comment.as_ref() {
            self.output
                .push_str(&format!(" #{}\n", trailing_comment.text));
        }
    }
}
