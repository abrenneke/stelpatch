use cw_parser::AstVisitor;

use crate::{util::indent, visitors::entity::ItemVisitor};

pub struct ConditionalVisitor<'a> {
    output: &'a mut String,
}

impl<'a> ConditionalVisitor<'a> {
    pub fn new(output: &'a mut String) -> Self {
        Self { output }
    }
}

impl<'a> AstVisitor<'a> for ConditionalVisitor<'a> {
    fn visit_conditional_block(&mut self, node: &cw_parser::AstConditionalBlock<'a>) -> () {
        for comment in node.leading_comments.iter() {
            self.output.push_str(&format!("#{}\n", comment.text));
        }

        self.output.push_str("[[");

        if node.is_not {
            self.output.push_str("!");
        }

        self.output.push_str(&node.key.raw_value());

        self.output.push_str("]\n");

        let mut buf = String::new();

        for item in &node.items {
            let mut visitor = ItemVisitor::new(&mut buf);
            visitor.visit_entity_item(item);
        }

        if !buf.is_empty() {
            self.output.push_str(&format!("\n{}\n", &indent(&buf)));
        }

        self.output.push_str("]");

        if let Some(trailing_comment) = node.trailing_comment.as_ref() {
            self.output
                .push_str(&format!(" #{}\n", trailing_comment.text));
        } else {
            self.output.push_str("\n");
        }
    }
}
