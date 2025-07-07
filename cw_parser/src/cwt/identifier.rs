use std::ops::Range;

use winnow::{
    LocatingSlice, ModalResult, Parser,
    combinator::{alt, delimited, opt},
    error::StrContext,
    token::{literal, take_until},
};

use crate::{AstComment, AstNode, AstString, opt_trailing_comment, quoted_or_unquoted_string};

use super::{CwtComment, CwtReferenceType};

/// Standalone identifier
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CwtIdentifier<'a> {
    pub identifier_type: CwtReferenceType<'a>,
    pub name: AstString<'a>,
    pub span: Range<usize>,
    /// Is there a ! prepended to the name?
    pub is_not: bool,
    pub leading_comments: Vec<CwtComment<'a>>,
    pub trailing_comment: Option<AstComment<'a>>,
}

impl<'a> CwtIdentifier<'a> {
    /// Check if this is a type key identifier
    pub fn is_type_key(&self) -> bool {
        self.identifier_type.is_type_ref()
    }

    pub fn new(
        identifier_type: CwtReferenceType<'a>,
        name: AstString<'a>,
        span: Range<usize>,
    ) -> Self {
        Self {
            identifier_type,
            name,
            is_not: false,
            span,
            leading_comments: Vec::new(),
            trailing_comment: None,
        }
    }
}

impl<'a> AstNode<'a> for CwtIdentifier<'a> {
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

pub(crate) fn quoted_or_unquoted_string_with_not<'a>(
    input: &mut LocatingSlice<&'a str>,
) -> ModalResult<(AstString<'a>, bool)> {
    let is_not = opt(literal("!")).parse_next(input)?;
    let name = quoted_or_unquoted_string.parse_next(input)?;
    Ok((name, is_not.is_some()))
}

/// Parse a standalone identifier
pub(crate) fn cwt_identifier<'a>(
    input: &mut LocatingSlice<&'a str>,
) -> ModalResult<CwtIdentifier<'a>> {
    let ((name, identifier_type), span) = alt((
        // Type key identifier: <identifier>
        delimited("<", quoted_or_unquoted_string_with_not, ">")
            .map(|name| (name, CwtReferenceType::TypeRef)),
        // Value set identifier: value_set[identifier]
        delimited("value_set[", quoted_or_unquoted_string_with_not, "]")
            .map(|name| (name, CwtReferenceType::ValueSet)),
        // Value identifier: value[identifier]
        delimited("value[", quoted_or_unquoted_string_with_not, "]")
            .map(|name| (name, CwtReferenceType::Value)),
        // Enum identifier: enum[identifier]
        delimited("enum[", quoted_or_unquoted_string_with_not, "]")
            .map(|name| (name, CwtReferenceType::Enum)),
        // Scope identifier: scope[identifier]
        delimited("scope[", quoted_or_unquoted_string_with_not, "]")
            .map(|name| (name, CwtReferenceType::Scope)),
        // Alias identifier: alias[identifier]
        delimited("alias[", quoted_or_unquoted_string_with_not, "]")
            .map(|name| (name, CwtReferenceType::Alias)),
        // Alias name identifier: alias_name[identifier]
        delimited("alias_name[", quoted_or_unquoted_string_with_not, "]")
            .map(|name| (name, CwtReferenceType::AliasName)),
        // Alias match left identifier: alias_match_left[identifier]
        delimited("alias_match_left[", quoted_or_unquoted_string_with_not, "]")
            .map(|name| (name, CwtReferenceType::AliasMatchLeft)),
        // Single alias right identifier: single_alias_right[identifier]
        delimited(
            "single_alias_right[",
            quoted_or_unquoted_string_with_not,
            "]",
        )
        .map(|name| (name, CwtReferenceType::SingleAlias)),
        // Alias keys field identifier: alias_keys_field[identifier]
        delimited("alias_keys_field[", quoted_or_unquoted_string_with_not, "]")
            .map(|name| (name, CwtReferenceType::AliasKeysField)),
        // Scope group identifier: scope_group[identifier]
        delimited("scope_group[", quoted_or_unquoted_string_with_not, "]")
            .map(|name| (name, CwtReferenceType::ScopeGroup)),
        // Colour identifier: colour[hsv|rgb]
        delimited("colour[", quoted_or_unquoted_string_with_not, "]")
            .map(|name| (name, CwtReferenceType::Colour)),
        // Stellaris name format identifier: stellaris_name_format[key]
        delimited(
            "stellaris_name_format[",
            quoted_or_unquoted_string_with_not,
            "]",
        )
        .map(|name| (name, CwtReferenceType::StellarisNameFormat)),
        // Type identifier: type[identifier]
        delimited("type[", quoted_or_unquoted_string_with_not, "]")
            .map(|name| (name, CwtReferenceType::Type)),
        // Subtype identifier: subtype[name]
        delimited("subtype[", quoted_or_unquoted_string_with_not, "]")
            .map(|name| (name, CwtReferenceType::Subtype)),
        // Complex enum identifier: complex_enum[identifier]
        delimited("complex_enum[", quoted_or_unquoted_string_with_not, "]")
            .map(|name| (name, CwtReferenceType::ComplexEnum)),
        // Icon identifier: icon[path] - paths can contain slashes
        ("icon[", take_until(1.., "]").with_span(), "]").map(|(_, (path, path_span), _)| {
            (
                (AstString::new(path, false, path_span), false),
                CwtReferenceType::Icon,
            )
        }),
        // Filepath identifier: filepath[path]
        ("filepath[", take_until(1.., "]").with_span(), "]").map(|(_, (path, path_span), _)| {
            (
                (AstString::new(path, false, path_span), false),
                CwtReferenceType::Filepath,
            )
        }),
    ))
    .with_span()
    .context(StrContext::Label("cwt_identifier"))
    .parse_next(input)?;

    let trailing_comment = opt_trailing_comment.parse_next(input)?;

    Ok(CwtIdentifier {
        identifier_type,
        name: name.0,
        is_not: name.1,
        span,
        leading_comments: Vec::new(), // Will be populated by cwt_entity
        trailing_comment,
    })
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
}
