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

    output
}
