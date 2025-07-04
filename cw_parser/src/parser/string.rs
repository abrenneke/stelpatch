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

use crate::{AstNode, AstToken, terminated_value};

/// AST representation of a string with position info
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AstString<'a> {
    pub value: AstToken<'a>,
    pub is_quoted: bool,
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

impl<'a> AstNode for AstString<'a> {
    fn span_range(&self) -> Range<usize> {
        self.value.span.clone()
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
    terminated_value(
        (
            one_of(is_valid_identifier_start_char),
            take_while(0.., is_valid_identifier_char),
        )
            .take(),
    )
    .with_span()
    .map(|(s, range)| AstString {
        value: AstToken {
            value: s,
            span: range,
        },
        is_quoted: false,
    })
    .context(StrContext::Label("unquoted_string"))
    .parse_next(input)
}

/// A string that is quoted with double quotes. Allows spaces and other characters that would otherwise be invalid in an unquoted string.
pub(crate) fn quoted_string<'a>(input: &mut LocatingSlice<&'a str>) -> ModalResult<AstString<'a>> {
    terminated_value(delimited(
        '"',
        escaped(none_of(['\\', '"']), '\\', "\"".value("\""))
            .map(|()| ())
            .take(),
        '"',
    ))
    .with_span()
    .map(|(s, range)| AstString {
        value: AstToken {
            value: s,
            span: range,
        },
        is_quoted: true,
    })
    .context(StrContext::Label("quoted_string"))
    .parse_next(input)
}

/// A string that is either quoted or unquoted.
pub(crate) fn quoted_or_unquoted_string<'a>(
    input: &mut LocatingSlice<&'a str>,
) -> ModalResult<AstString<'a>> {
    alt((quoted_string, unquoted_string))
        .context(StrContext::Label("quoted_or_unquoted_string"))
        .parse_next(input)
}
