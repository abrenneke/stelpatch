use std::ops::Range;

use winnow::{
    LocatingSlice, ModalResult, Parser,
    combinator::{alt, delimited, opt},
    error::{ErrMode, StrContext},
    token::take_while,
};

use crate::{AstComment, AstNode, AstString, opt_trailing_comment, quoted_or_unquoted_string};

use super::{AstCwtComment, CwtReferenceType};

/// Standalone identifier
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AstCwtIdentifier<'a> {
    pub identifier_type: CwtReferenceType<'a>,
    pub before_identifier: Option<Box<AstCwtIdentifier<'a>>>,
    pub name: Box<AstCwtIdentifierKey<'a>>,
    pub span: Range<usize>,
    /// Is there a ! prepended to the name?
    pub is_not: bool,
    pub leading_comments: Vec<AstCwtComment<'a>>,
    pub trailing_comment: Option<AstComment<'a>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AstCwtIdentifierOrString<'a> {
    Identifier(AstCwtIdentifier<'a>),
    String(AstString<'a>),
}

impl<'a> AstCwtIdentifierOrString<'a> {
    pub fn name(&self) -> &str {
        match self {
            AstCwtIdentifierOrString::Identifier(id) => id.name.raw_value(),
            AstCwtIdentifierOrString::String(s) => s.raw_value(),
        }
    }

    pub fn as_identifier(&self) -> Option<&AstCwtIdentifier<'a>> {
        match self {
            AstCwtIdentifierOrString::Identifier(id) => Some(id),
            AstCwtIdentifierOrString::String(_) => None,
        }
    }

    pub fn as_string(&self) -> Option<&AstString<'a>> {
        match self {
            AstCwtIdentifierOrString::Identifier(_) => None,
            AstCwtIdentifierOrString::String(s) => Some(s),
        }
    }
}

impl<'a> AstNode<'a> for AstCwtIdentifierOrString<'a> {
    fn span_range(&self) -> Range<usize> {
        match self {
            AstCwtIdentifierOrString::Identifier(id) => id.span_range(),
            AstCwtIdentifierOrString::String(s) => s.span_range(),
        }
    }

    fn leading_comments(&self) -> &[AstComment<'a>] {
        match self {
            AstCwtIdentifierOrString::Identifier(id) => id.leading_comments(),
            AstCwtIdentifierOrString::String(s) => s.leading_comments(),
        }
    }

    fn trailing_comment(&self) -> Option<&AstComment<'a>> {
        match self {
            AstCwtIdentifierOrString::Identifier(id) => id.trailing_comment(),
            AstCwtIdentifierOrString::String(s) => s.trailing_comment(),
        }
    }
}

/// Keys of identifiers can be complex, e.g. `alias[scope:enum[value]]`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AstCwtIdentifierKey<'a> {
    /// The part before a colon, if any
    pub scope: Option<AstString<'a>>,

    /// The main key
    pub key: AstCwtIdentifierOrString<'a>,
}

impl<'a> AstCwtIdentifierKey<'a> {
    pub fn new(scope: Option<AstString<'a>>, key: AstCwtIdentifierOrString<'a>) -> Self {
        Self { scope, key }
    }

    pub fn raw_value(&self) -> &'a str {
        match &self.key {
            AstCwtIdentifierOrString::Identifier(id) => id.name.raw_value(),
            AstCwtIdentifierOrString::String(s) => s.raw_value(),
        }
    }
}

impl<'a> AstNode<'a> for AstCwtIdentifierKey<'a> {
    fn span_range(&self) -> Range<usize> {
        self.key.span_range()
    }

    fn leading_comments(&self) -> &[AstComment<'a>] {
        self.key.leading_comments()
    }

    fn trailing_comment(&self) -> Option<&AstComment<'a>> {
        self.key.trailing_comment()
    }
}

impl<'a> AstCwtIdentifier<'a> {
    /// Check if this is a type key identifier
    pub fn is_type_key(&self) -> bool {
        self.identifier_type.is_type_ref()
    }

    pub fn new(
        identifier_type: CwtReferenceType<'a>,
        name: AstCwtIdentifierKey<'a>,
        span: Range<usize>,
    ) -> Self {
        Self {
            identifier_type,
            name: Box::new(name),
            before_identifier: None,
            is_not: false,
            span,
            leading_comments: Vec::new(),
            trailing_comment: None,
        }
    }
}

impl<'a> AstNode<'a> for AstCwtIdentifier<'a> {
    fn span_range(&self) -> Range<usize> {
        self.span.clone()
    }

    fn leading_comments(&self) -> &[AstComment<'a>] {
        &[] // TODO!
    }

    fn trailing_comment(&self) -> Option<&AstComment<'a>> {
        self.trailing_comment.as_ref()
    }
}

fn is_valid_scope_char(c: char) -> bool {
    c.is_ascii_alphanumeric()
        || match c {
            '_' => true,
            _ => false,
        }
}

fn scope<'a>(input: &mut LocatingSlice<&'a str>) -> ModalResult<AstString<'a>> {
    let ((scope, span), _) =
        (take_while(1.., is_valid_scope_char).with_span(), ":").parse_next(input)?;
    Ok(AstString::new(scope, false, span))
}

/// Helper function to create a parser for standard `prefix[content]` patterns
fn identifier_parser<'a>(
    prefix: &'static str,
    reference_type: CwtReferenceType<'a>,
) -> impl Parser<
    LocatingSlice<&'a str>,
    (
        AstCwtIdentifierOrString<'a>,
        Option<AstString<'a>>,
        bool,
        CwtReferenceType<'a>,
    ),
    ErrMode<winnow::error::ContextError>,
> {
    move |input: &mut LocatingSlice<&'a str>| {
        let (_, _, is_not, scope, key, _) = (
            prefix,
            "[",
            opt("!"),
            opt(scope),
            cwt_identifier_or_string,
            "]",
        )
            .parse_next(input)?;
        Ok((key, scope, is_not.is_some(), reference_type.clone()))
    }
}

pub(crate) fn cwt_identifier<'a>(
    input: &mut LocatingSlice<&'a str>,
) -> ModalResult<AstCwtIdentifier<'a>> {
    // Wonky, probably wrong, syntax, but we need to parse it
    let (before_part, mut after_part) =
        (opt((cwt_identifier_part, '.')), cwt_identifier_part).parse_next(input)?;

    let before_part = if let Some((before_part, _)) = before_part {
        Some(Box::new(before_part))
    } else {
        None
    };

    after_part.before_identifier = before_part;

    Ok(after_part)
}

/// Parse a standalone identifier
pub(crate) fn cwt_identifier_part<'a>(
    input: &mut LocatingSlice<&'a str>,
) -> ModalResult<AstCwtIdentifier<'a>> {
    let ((key, scope, is_not, identifier_type), span) = alt((
        // Type key identifier: <identifier>
        delimited("<", (opt("!"), quoted_or_unquoted_string), ">").map(|(is_not, name)| {
            (
                AstCwtIdentifierOrString::String(name),
                None,
                is_not.is_some(),
                CwtReferenceType::TypeRef,
            )
        }),
        identifier_parser("alias", CwtReferenceType::Alias),
        identifier_parser("icon", CwtReferenceType::Icon),
        identifier_parser("filepath", CwtReferenceType::Filepath),
        // Standard bracket identifiers
        identifier_parser("value_set", CwtReferenceType::ValueSet),
        identifier_parser("value", CwtReferenceType::Value),
        identifier_parser("enum", CwtReferenceType::Enum),
        identifier_parser("scope", CwtReferenceType::Scope),
        identifier_parser("alias_name", CwtReferenceType::AliasName),
        identifier_parser("alias_match_left", CwtReferenceType::AliasMatchLeft),
        identifier_parser("single_alias_right", CwtReferenceType::SingleAlias),
        identifier_parser("alias_keys_field", CwtReferenceType::AliasKeysField),
        identifier_parser("scope_group", CwtReferenceType::ScopeGroup),
        identifier_parser("colour", CwtReferenceType::Colour),
        identifier_parser(
            "stellaris_name_format",
            CwtReferenceType::StellarisNameFormat,
        ),
        identifier_parser("type", CwtReferenceType::Type),
        identifier_parser("subtype", CwtReferenceType::Subtype),
        identifier_parser("complex_enum", CwtReferenceType::ComplexEnum),
    ))
    .with_span()
    .context(StrContext::Label("cwt_identifier"))
    .parse_next(input)?;

    let trailing_comment = opt_trailing_comment.parse_next(input)?;

    Ok(AstCwtIdentifier {
        identifier_type,
        name: Box::new(AstCwtIdentifierKey::new(scope, key)),
        before_identifier: None,
        is_not,
        span,
        leading_comments: Vec::new(), // Will be populated by cwt_entity
        trailing_comment,
    })
}

pub(crate) fn cwt_identifier_or_string<'a>(
    input: &mut LocatingSlice<&'a str>,
) -> ModalResult<AstCwtIdentifierOrString<'a>> {
    alt((
        cwt_identifier.map(AstCwtIdentifierOrString::Identifier),
        quoted_or_unquoted_string.map(AstCwtIdentifierOrString::String),
    ))
    .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use winnow::{LocatingSlice, Parser};

    #[test]
    fn test_cwt_identifier_icon() {
        let mut input = LocatingSlice::new("icon[gfx/interface/icons/buildings]");
        let result = cwt_identifier.parse_next(&mut input).unwrap();

        match result.identifier_type {
            CwtReferenceType::Icon => {
                assert_eq!(result.name.raw_value(), "gfx/interface/icons/buildings");
            }
            _ => panic!("Expected Icon identifier type"),
        }
    }

    #[test]
    fn test_cwt_identifier_type_ref() {
        let mut input = LocatingSlice::new("<test_type>");
        let result = cwt_identifier.parse_next(&mut input).unwrap();

        match result.identifier_type {
            CwtReferenceType::TypeRef => {
                assert_eq!(result.name.raw_value(), "test_type");
                assert!(result.is_type_key());
            }
            _ => panic!("Expected TypeRef identifier type"),
        }
    }

    #[test]
    fn test_cwt_identifier_enum() {
        let mut input = LocatingSlice::new("enum[test_enum]");
        let result = cwt_identifier.parse_next(&mut input).unwrap();

        match result.identifier_type {
            CwtReferenceType::Enum => {
                assert_eq!(result.name.raw_value(), "test_enum");
            }
            _ => panic!("Expected Enum identifier type"),
        }
    }

    #[test]
    fn test_cwt_identifier_value_set() {
        let mut input = LocatingSlice::new("value_set[test_values]");
        let result = cwt_identifier.parse_next(&mut input).unwrap();

        match result.identifier_type {
            CwtReferenceType::ValueSet => {
                assert_eq!(result.name.raw_value(), "test_values");
            }
            _ => panic!("Expected ValueSet identifier type"),
        }
    }

    #[test]
    fn test_cwt_identifier_scope() {
        let mut input = LocatingSlice::new("scope[country]");
        let result = cwt_identifier.parse_next(&mut input).unwrap();

        match result.identifier_type {
            CwtReferenceType::Scope => {
                assert_eq!(result.name.raw_value(), "country");
            }
            _ => panic!("Expected Scope identifier type"),
        }
    }

    #[test]
    fn test_cwt_identifier_alias_name() {
        let mut input = LocatingSlice::new("alias_name[test_alias]");
        let result = cwt_identifier.parse_next(&mut input).unwrap();

        match result.identifier_type {
            CwtReferenceType::AliasName => {
                assert_eq!(result.name.raw_value(), "test_alias");
            }
            _ => panic!("Expected AliasName identifier type"),
        }
    }

    #[test]
    fn test_scope_group() {
        let mut input = LocatingSlice::new("scope_group[celestial_coordinate]");
        let result = cwt_identifier.parse_next(&mut input).unwrap();

        match result.identifier_type {
            CwtReferenceType::ScopeGroup => {
                assert_eq!(result.name.raw_value(), "celestial_coordinate");
            }
            _ => panic!("Expected ScopeGroup identifier type"),
        }
    }

    #[test]
    fn scope_with_colon() {
        let mut input = LocatingSlice::new("alias[modifier_rule:foo]");
        let result = cwt_identifier.parse_next(&mut input).unwrap();

        assert_eq!(result.name.scope.unwrap().raw_value(), "modifier_rule");
        assert_eq!(result.name.key.as_string().unwrap().raw_value(), "foo");
    }

    #[test]
    fn nested() {
        let mut input = LocatingSlice::new("alias[modifier_rule:enum[complex_maths_enum]]");
        let _result = cwt_identifier.parse_next(&mut input).unwrap();
    }
}
