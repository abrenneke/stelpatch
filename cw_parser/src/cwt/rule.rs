use std::ops::Range;

use winnow::{
    LocatingSlice, ModalResult, Parser, combinator::alt, error::StrContext, token::literal,
};

use crate::{
    AstComment, AstCwtIdentifier, AstNode, AstString, quoted_or_unquoted_string,
    with_opt_trailing_ws,
};

use super::{AstCwtComment, CwtValue, cwt_identifier, cwt_value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AstCwtRuleKey<'a> {
    Identifier(AstCwtIdentifier<'a>),
    String(AstString<'a>),
}

impl<'a> AstCwtRuleKey<'a> {
    pub fn name(&self) -> &str {
        match self {
            AstCwtRuleKey::Identifier(id) => id.name.raw_value(),
            AstCwtRuleKey::String(s) => s.raw_value(),
        }
    }
}

impl<'a> AstNode<'a> for AstCwtRuleKey<'a> {
    fn span_range(&self) -> Range<usize> {
        match self {
            AstCwtRuleKey::Identifier(id) => id.span_range(),
            AstCwtRuleKey::String(s) => s.span_range(),
        }
    }

    fn leading_comments(&self) -> &[AstComment<'a>] {
        match self {
            AstCwtRuleKey::Identifier(id) => id.leading_comments(),
            AstCwtRuleKey::String(s) => s.leading_comments(),
        }
    }

    fn trailing_comment(&self) -> Option<&AstComment<'a>> {
        match self {
            AstCwtRuleKey::Identifier(id) => id.trailing_comment(),
            AstCwtRuleKey::String(s) => s.trailing_comment(),
        }
    }
}
/// CWT rule with optional option directives
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AstCwtRule<'a> {
    pub key: AstCwtRuleKey<'a>,
    pub operator: CwtOperator,
    pub value: CwtValue<'a>,
    pub options: Vec<CwtOption<'a>>,
    pub documentation: Option<AstCwtComment<'a>>,
    pub span: Range<usize>,
}

/// CWT operators
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CwtOperator {
    /// Regular assignment =
    Equals,
    /// Comparable trigger ==
    ComparableEquals,
}

/// CWT option directives (from ## comments)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CwtOption<'a> {
    pub option_type: CwtOptionType<'a>,
    pub span: Range<usize>,
}

/// Types of CWT option directives
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CwtOptionType<'a> {
    /// Cardinality constraint: cardinality = min..max
    Cardinality { min: u32, max: CwtCardinalityMax },
    /// Soft cardinality constraint: cardinality = ~min..max
    SoftCardinality { min: u32, max: CwtCardinalityMax },
    /// Push scope: push_scope = scope_name
    PushScope { scope: &'a str },
    /// Replace scope: replace_scope = { this = scope1 root = scope2 }
    ReplaceScope {
        replacements: Vec<CwtScopeReplacement<'a>>,
    },
    /// Severity level: severity = level
    Severity { level: CwtSeverityLevel },
    /// Scope constraint: scope = scope_name or scope = { scope1 scope2 }
    Scope { scopes: Vec<&'a str> },
    /// Type key filter: type_key_filter = filter_value
    TypeKeyFilter { filter: &'a str },
    /// Required option: required
    Required,
    /// Primary option: primary
    Primary,
    /// Display name: display_name = "name"
    DisplayName { name: &'a str },
    /// Abbreviation: abbreviation = "abbr"
    Abbreviation { abbr: &'a str },
    /// Starts with: starts_with = "prefix"
    StartsWith { prefix: &'a str },
    /// Graph related types: graph_related_types = { type1 type2 }
    GraphRelatedTypes { types: Vec<&'a str> },
}

/// Cardinality maximum value
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CwtCardinalityMax {
    /// Specific number
    Number(u32),
    /// Infinity
    Infinity,
}

/// Scope replacement in replace_scope
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CwtScopeReplacement<'a> {
    pub from: &'a str,
    pub to: &'a str,
}

/// Severity levels
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CwtSeverityLevel {
    Error,
    Warning,
    Information,
    Hint,
}

impl<'a> AstCwtRule<'a> {
    pub fn new(
        key: AstCwtRuleKey<'a>,
        operator: CwtOperator,
        value: CwtValue<'a>,
        options: Vec<CwtOption<'a>>,
        documentation: Option<AstCwtComment<'a>>,
    ) -> Self {
        let span = key.span_range().start..value.span_range().end;
        Self {
            key,
            operator,
            value,
            options,
            documentation,
            span,
        }
    }

    /// Check if this rule has a specific option
    pub fn has_option(&self, option_type: &str) -> bool {
        self.options.iter().any(|opt| {
            matches!(
                (&opt.option_type, option_type),
                (CwtOptionType::Cardinality { .. }, "cardinality")
                    | (CwtOptionType::SoftCardinality { .. }, "cardinality")
                    | (CwtOptionType::PushScope { .. }, "push_scope")
                    | (CwtOptionType::ReplaceScope { .. }, "replace_scope")
                    | (CwtOptionType::Severity { .. }, "severity")
                    | (CwtOptionType::Scope { .. }, "scope")
                    | (CwtOptionType::TypeKeyFilter { .. }, "type_key_filter")
                    | (CwtOptionType::Required, "required")
                    | (CwtOptionType::Primary, "primary")
                    | (CwtOptionType::DisplayName { .. }, "display_name")
                    | (CwtOptionType::Abbreviation { .. }, "abbreviation")
                    | (CwtOptionType::StartsWith { .. }, "starts_with")
                    | (
                        CwtOptionType::GraphRelatedTypes { .. },
                        "graph_related_types"
                    )
            )
        })
    }

    /// Get the cardinality option if present
    pub fn get_cardinality(&self) -> Option<&CwtOption<'a>> {
        self.options.iter().find(|opt| {
            matches!(
                opt.option_type,
                CwtOptionType::Cardinality { .. } | CwtOptionType::SoftCardinality { .. }
            )
        })
    }

    /// Check if this rule is comparable (uses == operator)
    pub fn is_comparable(&self) -> bool {
        matches!(self.operator, CwtOperator::ComparableEquals)
    }
}

impl<'a> AstNode<'a> for AstCwtRule<'a> {
    fn span_range(&self) -> Range<usize> {
        self.span.clone()
    }

    fn leading_comments(&self) -> &[AstComment<'a>] {
        self.key.leading_comments()
    }

    fn trailing_comment(&self) -> Option<&AstComment<'a>> {
        self.value.trailing_comment()
    }
}

impl<'a> AstNode<'a> for CwtOption<'a> {
    fn span_range(&self) -> Range<usize> {
        self.span.clone()
    }

    fn leading_comments(&self) -> &[AstComment<'a>] {
        &[]
    }

    fn trailing_comment(&self) -> Option<&AstComment<'a>> {
        None
    }
}

/// Parse a CWT rule
pub(crate) fn cwt_rule<'a>(input: &mut LocatingSlice<&'a str>) -> ModalResult<AstCwtRule<'a>> {
    let ((identifier, operator, value), span) = (
        with_opt_trailing_ws(alt((
            cwt_identifier.map(AstCwtRuleKey::Identifier),
            quoted_or_unquoted_string.map(AstCwtRuleKey::String),
        ))),
        with_opt_trailing_ws(cwt_operator),
        cwt_value,
    )
        .with_span()
        .context(StrContext::Label("cwt_rule"))
        .parse_next(input)?;

    Ok(AstCwtRule {
        key: identifier,
        operator,
        value,
        options: vec![], // Options are parsed separately and attached
        documentation: None,
        span,
    })
}

/// Parse a CWT operator
pub(crate) fn cwt_operator<'a>(input: &mut LocatingSlice<&'a str>) -> ModalResult<CwtOperator> {
    alt((
        literal("==").value(CwtOperator::ComparableEquals),
        literal("=").value(CwtOperator::Equals),
    ))
    .context(StrContext::Label("cwt_operator"))
    .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use winnow::{LocatingSlice, Parser};

    #[test]
    fn test_cwt_operator_equals() {
        let mut input = LocatingSlice::new("=");
        let result = cwt_operator.parse_next(&mut input).unwrap();
        assert_eq!(result, CwtOperator::Equals);
    }

    #[test]
    fn test_cwt_operator_comparable_equals() {
        let mut input = LocatingSlice::new("==");
        let result = cwt_operator.parse_next(&mut input).unwrap();
        assert_eq!(result, CwtOperator::ComparableEquals);
    }
}
