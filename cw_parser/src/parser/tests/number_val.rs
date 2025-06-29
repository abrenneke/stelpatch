#![cfg(test)]
use pretty_assertions::assert_eq;
use winnow::{LocatingSlice, Parser};

use super::super::super::*;

#[test]
fn test_number_val_valid_input() {
    let mut input = LocatingSlice::new("123  ");
    let result = number_val.parse_next(&mut input).unwrap();
    assert_eq!(
        result,
        AstNumber {
            value: AstToken {
                value: "123",
                span: 0..3,
            },
        }
    );
}

#[test]
fn test_number_val_negative_input() {
    let mut input = LocatingSlice::new("-12.34  ");
    let result = number_val.parse_next(&mut input).unwrap();
    assert_eq!(
        result,
        AstNumber {
            value: AstToken {
                value: "-12.34",
                span: 0..6,
            },
        }
    );
}

#[test]
fn test_number_val_positive_input() {
    let mut input = LocatingSlice::new("+12.34  ");
    let result = number_val.parse_next(&mut input).unwrap();
    assert_eq!(
        result,
        AstNumber {
            value: AstToken {
                value: "+12.34",
                span: 0..6,
            },
        }
    );
}

#[test]
fn test_number_val_decimal_input() {
    let mut input = LocatingSlice::new("3.14159  ");
    let result = number_val.parse_next(&mut input).unwrap();
    assert_eq!(
        result,
        AstNumber {
            value: AstToken {
                value: "3.14159",
                span: 0..7,
            },
        }
    );
}

#[test]
fn test_number_val_valid_input_with_comments() {
    let mut input = LocatingSlice::new("123# This is a comment");
    let result = number_val.parse_next(&mut input).unwrap();
    assert_eq!(
        result,
        AstNumber {
            value: AstToken {
                value: "123",
                span: 0..3,
            },
        }
    );
}

#[test]
fn test_number_val_must_end_with_whitespace() {
    let mut input = LocatingSlice::new("123$");
    let result = number_val.parse_next(&mut input);
    assert!(result.is_err());
}

#[test]
fn test_number_val_invalid_input() {
    let mut input = LocatingSlice::new("abc  ");
    let result = number_val.parse_next(&mut input);
    assert!(result.is_err());
}

#[test]
fn does_not_parse_var_starts_with_number() {
    let mut input = LocatingSlice::new("1abc  ");
    let result = number_val.parse_next(&mut input);
    assert!(result.is_err());
}
