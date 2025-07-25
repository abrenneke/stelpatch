use cw_parser::AstVisitor;

use crate::{
    util::indent,
    visitors::{ConditionalVisitor, ExpressionVisitor, ValueVisitor},
};

pub struct EntityVisitor<'a> {
    output: &'a mut String,
}

impl<'a> EntityVisitor<'a> {
    pub fn new(output: &'a mut String) -> Self {
        Self { output }
    }
}

const COMPACT_ENTITY_THRESHOLD: usize = 150;

fn format_entity<'a>(output: &mut String, node: &cw_parser::AstEntity<'a>) -> () {
    // Try short mode for single-value entities
    if node.items.len() == 1 {
        let mut buf = String::new();

        buf.push_str("{ ");

        let item = node.items.first().unwrap();

        let mut visitor = ItemVisitor::new(&mut buf);
        visitor.visit_entity_item(item);

        buf.push_str(" }");

        // If it's short enough, and there are no newlines, use short mode that's
        // like { key = value } instead of {
        //   key = value
        // }
        if buf.len() < COMPACT_ENTITY_THRESHOLD && buf.find('\n').is_none() {
            output.push_str(&buf);
            return;
        }
    }

    output.push_str(&format!("{{"));

    let mut buf = String::new();
    for item in node.items.iter() {
        let mut visitor = ItemVisitor::new(&mut buf);
        visitor.visit_entity_item(item);

        // It might end with a newline because of the item, but it might not, so make sure there's
        // always a newline at the end of each item
        if !buf.ends_with('\n') {
            buf.push('\n');
        }
    }

    if !buf.is_empty() {
        output.push_str(&format!("\n{}\n", &indent(&buf)));
    }

    output.push_str("}");
}

impl<'a, 'ast> AstVisitor<'a, 'ast> for EntityVisitor<'a>
where
    'a: 'ast,
{
    fn visit_entity(&mut self, node: &cw_parser::AstEntity<'a>) -> () {
        for comment in node.leading_comments.iter() {
            self.output.push_str(&format!("#{}\n", comment.text));
        }

        format_entity(self.output, node);

        if let Some(trailing_comment) = node.trailing_comment.as_ref() {
            self.output
                .push_str(&format!(" #{}\n", trailing_comment.text));
        } else {
            self.output.push_str("\n");
        }
    }
}

pub struct ItemVisitor<'a> {
    output: &'a mut String,
}

impl<'a> ItemVisitor<'a> {
    pub fn new(output: &'a mut String) -> Self {
        Self { output }
    }
}

impl<'a, 'ast> AstVisitor<'a, 'ast> for ItemVisitor<'a>
where
    'a: 'ast,
{
    fn visit_entity_item(&mut self, node: &cw_parser::AstEntityItem<'a>) -> () {
        match node {
            cw_parser::AstEntityItem::Item(item) => {
                let mut visitor = ValueVisitor::new(self.output);
                visitor.visit_value(item);
            }
            cw_parser::AstEntityItem::Expression(expression) => {
                let mut visitor = ExpressionVisitor::new(self.output);
                visitor.visit_expression(expression);
            }
            cw_parser::AstEntityItem::Conditional(conditional) => {
                let mut visitor = ConditionalVisitor::new(self.output);
                visitor.visit_conditional_block(conditional);
            }
        }
    }
}
