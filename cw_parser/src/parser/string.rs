use std::{
    hash::{Hash, Hasher},
    ops::Range,
};

use winnow::{
    LocatingSlice, ModalResult, Parser,
    ascii::escaped,
    combinator::{alt, delimited},
    error::StrContext,
    token::{none_of, one_of, take_while},
};

use crate::{
    AstComment, AstNode, AstToken, opt_trailing_comment, opt_ws_and_comments, terminated_value,
};

/// AST representation of a string with position info
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AstString<'a> {
    pub value: AstToken<'a>,
    pub is_quoted: bool,

    pub leading_comments: Vec<AstComment<'a>>,
    pub trailing_comment: Option<AstComment<'a>>,
}

impl<'a> ToString for AstString<'a> {
    fn to_string(&self) -> String {
        self.value.to_string()
    }
}

impl<'a> AsRef<str> for AstString<'a> {
    fn as_ref(&self) -> &str {
        self.value.as_ref()
    }
}

impl<'a> AstString<'a> {
    pub fn new(value: &'a str, is_quoted: bool, span: Range<usize>) -> Self {
        Self {
            value: AstToken::new(value, span),
            is_quoted,
            leading_comments: vec![],
            trailing_comment: None,
        }
    }

    /// Check if two strings have the same semantic value (ignoring quotes)
    pub fn semantic_eq(&self, other: &AstString<'a>) -> bool {
        self.value.value == other.value.value
    }

    /// Get the raw string value
    pub fn raw_value(&self) -> &'a str {
        self.value.value
    }

    /// Check if this is an identifier (unquoted string)
    pub fn is_identifier(&self) -> bool {
        !self.is_quoted
    }
}

impl<'a> AstNode<'a> for AstString<'a> {
    fn span_range(&self) -> Range<usize> {
        self.value.span.clone()
    }

    fn leading_comments(&self) -> &[AstComment<'a>] {
        &self.leading_comments
    }

    fn trailing_comment(&self) -> Option<&AstComment<'a>> {
        self.trailing_comment.as_ref()
    }
}

impl<'a> Hash for AstString<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

fn is_valid_identifier_start_char(c: char) -> bool {
    c.is_ascii_alphanumeric()
        || match c {
            '_' | '-' | '$' | '@' => true,
            _ => false,
        }
}

fn is_valid_identifier_char<'a>(c: char) -> bool {
    c.is_ascii_alphanumeric()
        || match c {
            '_' | ':' | '.' | '@' | '-' | '|' | '/' | '$' | '\'' => true,
            _ => false,
        }
}

/// An unquoted string (i.e. identifier) - a sequence of valid identifier characters, spaces not allowed.
pub(crate) fn unquoted_string<'a>(
    input: &mut LocatingSlice<&'a str>,
) -> ModalResult<AstString<'a>> {
    let leading_comments = opt_ws_and_comments.parse_next(input)?;

    let (s, range) = terminated_value(
        (
            one_of(is_valid_identifier_start_char),
            take_while(0.., is_valid_identifier_char),
        )
            .take(),
    )
    .with_span()
    .context(StrContext::Label("unquoted_string"))
    .parse_next(input)?;

    let trailing_comment = opt_trailing_comment.parse_next(input)?;

    Ok(AstString {
        value: AstToken::new(s, range),
        is_quoted: false,
        leading_comments,
        trailing_comment,
    })
}

/// A string that is quoted with double quotes. Allows spaces and other characters that would otherwise be invalid in an unquoted string.
pub(crate) fn quoted_string<'a>(input: &mut LocatingSlice<&'a str>) -> ModalResult<AstString<'a>> {
    let leading_comments = opt_ws_and_comments.parse_next(input)?;

    let (s, range) = terminated_value(delimited(
        '"',
        escaped(
            none_of(['\\', '"']),
            '\\',
            alt(("\"".value("\""), "\\".value("\\"), "n".value("\n"))),
        )
        .map(|()| ())
        .take(),
        '"',
    ))
    .with_span()
    .context(StrContext::Label("quoted_string"))
    .parse_next(input)?;

    let trailing_comment = opt_trailing_comment.parse_next(input)?;

    Ok(AstString {
        value: AstToken::new(s, range),
        is_quoted: true,
        leading_comments,
        trailing_comment,
    })
}

/// A string that is either quoted or unquoted.
pub(crate) fn quoted_or_unquoted_string<'a>(
    input: &mut LocatingSlice<&'a str>,
) -> ModalResult<AstString<'a>> {
    alt((quoted_string, unquoted_string))
        .context(StrContext::Label("quoted_or_unquoted_string"))
        .parse_next(input)
}

#[cfg(test)]
mod tests {
    use winnow::{LocatingSlice, Parser};

    use super::super::super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_unquoted_string_valid_input() {
        let mut input = LocatingSlice::new("hello123");
        let result = unquoted_string.parse_next(&mut input).unwrap();
        assert_eq!(
            result,
            AstString {
                value: AstToken::new("hello123", 0..8),
                is_quoted: false,
                leading_comments: vec![],
                trailing_comment: None,
            }
        );
    }

    #[test]
    fn test_unquoted_string_invalid_input() {
        let mut input = LocatingSlice::new("invalid*identifier");
        let result = unquoted_string.parse_next(&mut input);
        assert!(result.is_err());
    }

    #[test]
    fn test_quoted_string_valid_input() {
        let mut input = LocatingSlice::new("\"hello world\"");
        let result = quoted_string.parse_next(&mut input).unwrap();
        assert_eq!(
            result,
            AstString {
                value: AstToken::new("hello world", 0..13),
                is_quoted: true,
                leading_comments: vec![],
                trailing_comment: None,
            }
        );
    }

    #[test]
    fn test_quoted_string_valid_input_with_special_characters() {
        let mut input = LocatingSlice::new("\"a:b.c|d/e$f'g\"");
        let result = quoted_string.parse_next(&mut input).unwrap();
        assert_eq!(
            result,
            AstString {
                value: AstToken::new("a:b.c|d/e$f'g", 0..15),
                is_quoted: true,
                leading_comments: vec![],
                trailing_comment: None,
            }
        );
    }

    #[test]
    fn test_quoted_string_invalid_input() {
        let mut input = LocatingSlice::new("\"invalid\"quote\"");
        let result = quoted_string.parse_next(&mut input);
        assert!(result.is_err());
    }

    #[test]
    fn test_quoted_or_unquoted_string_valid_input_unquoted() {
        let mut input = LocatingSlice::new("hello123");
        let result = quoted_or_unquoted_string.parse_next(&mut input).unwrap();
        assert_eq!(
            result,
            AstString {
                value: AstToken::new("hello123", 0..8),
                is_quoted: false,
                leading_comments: vec![],
                trailing_comment: None,
            }
        );
    }

    #[test]
    fn test_quoted_or_unquoted_string_valid_input_quoted() {
        let mut input = LocatingSlice::new("\"hello world\"");
        let result = quoted_or_unquoted_string.parse_next(&mut input).unwrap();
        assert_eq!(
            result,
            AstString {
                value: AstToken::new("hello world", 0..13),
                is_quoted: true,
                leading_comments: vec![],
                trailing_comment: None,
            }
        );
    }

    #[test]
    fn test_quoted_or_unquoted_string_valid_input_quoted_with_special_characters() {
        let mut input = LocatingSlice::new("\"a:b.c|d/e$f'g\"");
        let result = quoted_or_unquoted_string.parse_next(&mut input).unwrap();
        assert_eq!(
            result,
            AstString {
                value: AstToken::new("a:b.c|d/e$f'g", 0..15),
                is_quoted: true,
                leading_comments: vec![],
                trailing_comment: None,
            }
        );
    }

    #[test]
    fn test_quoted_or_unquoted_string_invalid_input_unquoted() {
        let mut input = LocatingSlice::new("invalid*identifier");
        let result = quoted_or_unquoted_string.parse_next(&mut input);
        assert!(result.is_err());
    }

    #[test]
    fn test_quoted_empty_string() {
        let mut input = LocatingSlice::new("\"\"");
        let result = quoted_string.parse_next(&mut input).unwrap();
        assert_eq!(
            result,
            AstString {
                value: AstToken::new("", 0..2),
                is_quoted: true,
                leading_comments: vec![],
                trailing_comment: None,
            }
        );
    }

    #[test]
    fn dynamic_script_value() {
        let mut input = LocatingSlice::new("$FLAG$");
        let result = unquoted_string.parse_next(&mut input).unwrap();
        assert_eq!(
            result,
            AstString {
                value: AstToken::new("$FLAG$", 0..6),
                is_quoted: false,
                leading_comments: vec![],
                trailing_comment: None,
            }
        );
    }

    #[test]
    fn string_with_comments() {
        let mut input = LocatingSlice::new(
            r#"
            # This is a leading comment
            # This is another leading comment
            "Hello" # This is a trailing comment
        "#,
        );

        let result = quoted_or_unquoted_string.parse_next(&mut input).unwrap();
        assert_eq!(
            result,
            AstString {
                value: AstToken::new("Hello", 99..106),
                is_quoted: true,
                leading_comments: vec![
                    AstComment::new(" This is a leading comment", 13..40),
                    AstComment::new(" This is another leading comment", 53..86),
                ],
                trailing_comment: Some(AstComment::new(" This is a trailing comment", 107..135)),
            }
        );
    }

    #[test]
    fn comment() {
        let input = LocatingSlice::new(r#""- This: \\[This.GetName]""#);
        let result = quoted_string.parse(input).unwrap();
        assert_eq!(
            result,
            AstString::new("- This: \\\\[This.GetName]", true, 0..26)
        );
    }
}
