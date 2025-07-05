mod tests;
mod util;
mod visitors;

use cw_parser::{AstModuleCell, AstVisitor};

use crate::visitors::ModuleVisitor;

pub fn format_module(input: impl Into<String>) -> String {
    let module = AstModuleCell::from_input(input.into());
    let mut output = String::new();

    let mut visitor = ModuleVisitor::new(&mut output);

    let module = module.borrow_dependent().as_ref().unwrap();

    visitor.visit_module(module);

    let output = blank_line_pass(&output);

    output
}

/// Trim all lines
fn blank_line_pass(input: &str) -> String {
    // Trim the end of every line, and trim both ends of lines that are completely whitespace
    let output = input
        .lines()
        .map(|line| {
            if line.trim().is_empty() {
                // Line is completely whitespace, trim both ends (making it empty)
                line.trim()
            } else {
                // Line has content, only trim the end
                line.trim_end()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Ensure there's exactly one newline at the end
    let output = format!("{}\n", output.trim_end());

    output
}
