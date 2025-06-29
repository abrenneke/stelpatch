#![cfg(test)]
use pretty_assertions::assert_eq;
use winnow::{LocatingSlice, Parser};

use crate::{AstMaths, parser::inline_maths};

#[test]
fn inline_maths_test() {
    let input = LocatingSlice::new("@[ stabilitylevel2 + 10 ]");

    let result = inline_maths.parse(input).unwrap();

    assert_eq!(result, AstMaths::new("@[ stabilitylevel2 + 10 ]", 0..25));
}

#[test]
fn inline_maths_alt_test() {
    let input = LocatingSlice::new("@\\[ stabilitylevel2 + 10 ]");

    let result = inline_maths.parse(input).unwrap();

    assert_eq!(result, AstMaths::new("@\\[ stabilitylevel2 + 10 ]", 0..26));
}
