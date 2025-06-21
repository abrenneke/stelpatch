#![cfg(test)]
use winnow::Parser;

use crate::parser::inline_maths;

#[test]
fn inline_maths_test() {
    let input = "@[ stabilitylevel2 + 10 ]";

    let result = inline_maths.parse(input).unwrap();

    assert_eq!(result, "@[ stabilitylevel2 + 10 ]");
}

#[test]
fn inline_maths_alt_test() {
    let input = "@\\[ stabilitylevel2 + 10 ]";

    let result = inline_maths.parse(input).unwrap();

    assert_eq!(result, "@\\[ stabilitylevel2 + 10 ]");
}
