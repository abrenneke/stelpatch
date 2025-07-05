use cw_parser::{AstString, AstVisitor};

pub struct StringVisitor<'a> {
    output: &'a mut String,
}

impl<'a> StringVisitor<'a> {
    pub fn new(output: &'a mut String) -> Self {
        Self { output }
    }
}

impl<'a> AstVisitor<'a> for StringVisitor<'a> {
    fn visit_string(&mut self, node: &AstString<'a>) -> () {
        if node.leading_newlines > 0 {
            self.output.push_str(&"\n".repeat(node.leading_newlines));
        }

        for comment in node.leading_comments.iter() {
            self.output.push_str(&format!("#{}\n", comment.text));
        }

        let formatted = if node.is_quoted {
            format!("\"{}\"", escape_string(&node.value.value))
        } else {
            escape_string(&node.value.value)
        };

        self.output.push_str(&formatted);

        if let Some(trailing_comment) = node.trailing_comment.as_ref() {
            self.output
                .push_str(&format!(" #{}\n", trailing_comment.text));
        }
    }
}

fn escape_string<'a>(input: &'a str) -> String {
    // TODO if no escape is needed, return input as is
    let mut output = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '"' => output.push_str("\\\""),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            '\\' => output.push_str("\\\\"),
            _ => output.push(c),
        }
    }

    output
}
