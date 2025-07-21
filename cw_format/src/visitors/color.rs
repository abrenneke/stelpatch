use cw_parser::{AstColor, AstVisitor};

use crate::util::TAB;

pub struct ColorVisitor<'a> {
    output: &'a mut String,
}

impl<'a> ColorVisitor<'a> {
    pub fn new(output: &'a mut String) -> Self {
        Self { output }
    }
}

impl<'a, 'ast> AstVisitor<'a, 'ast> for ColorVisitor<'a>
where
    'a: 'ast,
{
    fn visit_color(&mut self, node: &AstColor<'a>) -> () {
        // TODO
        // if node.leading_newlines > 0 {
        //     self.output.push_str(&"\n".repeat(node.leading_newlines));
        // }

        todo!()
        // for comment in node.leading_comments.iter() {
        //     self.output.push_str(comment.text);
        //     self.output.push_str("\n");
        // }

        // // If anything has a comment, print the long form, otherwise the short form
        // let long_form = !node.r.leading_comments.is_empty()
        //     || !node.g.leading_comments.is_empty()
        //     || !node.b.leading_comments.is_empty()
        //     || (node.a.is_some() && !node.a.as_ref().unwrap().leading_comments.is_empty());

        // if long_form {
        //     self.output
        //         .push_str(&format!("{} {{\n", node.color_type.value));

        //     self.output
        //         .push_str(&format!("{}{}\n", TAB, node.r.value.value));
        //     self.output
        //         .push_str(&format!("{}{}\n", TAB, node.g.value.value));
        //     self.output
        //         .push_str(&format!("{}{}\n", TAB, node.b.value.value));

        //     if let Some(a) = node.a.as_ref() {
        //         self.output.push_str(&format!("{}{}\n", TAB, a.value.value));
        //     }

        //     self.output.push_str("}\n");
        // } else {
        //     self.output.push_str(&format!(
        //         "{} {{ {} {} {} }}\n",
        //         node.color_type.value, node.r.value.value, node.g.value.value, node.b.value.value
        //     ));
        // }

        // if let Some(trailing_comment) = node.trailing_comment.as_ref() {
        //     self.output
        //         .push_str(&format!(" #{}\n", trailing_comment.text));
        // }
    }
}
