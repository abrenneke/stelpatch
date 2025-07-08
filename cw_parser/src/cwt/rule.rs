use std::ops::Range;

use winnow::{
    LocatingSlice, ModalResult, Parser, combinator::alt, error::StrContext, token::literal,
};

use crate::{
    AstComment, AstCwtCommentOption, AstCwtIdentifierOrString, AstNode, quoted_or_unquoted_string,
    with_opt_trailing_ws,
};

use super::{AstCwtComment, CwtValue, cwt_identifier, cwt_value};

/// CWT rule with optional option directives
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AstCwtRule<'a> {
    pub key: AstCwtIdentifierOrString<'a>,
    pub operator: CwtOperator,
    pub value: CwtValue<'a>,
    pub options: Vec<AstCwtCommentOption<'a>>,
    pub documentation: Vec<AstCwtComment<'a>>,
    pub span: Range<usize>,
}

/// CWT operators
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CwtOperator {
    /// Regular assignment =
    Equals,
    /// Comparable trigger ==
    ComparableEquals,
    /// Not Equals !=
    NotEquals,
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
        key: AstCwtIdentifierOrString<'a>,
        operator: CwtOperator,
        value: CwtValue<'a>,
        options: Vec<AstCwtCommentOption<'a>>,
        documentation: Vec<AstCwtComment<'a>>,
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

/// Parse a CWT rule
pub(crate) fn cwt_rule<'a>(input: &mut LocatingSlice<&'a str>) -> ModalResult<AstCwtRule<'a>> {
    let ((identifier, operator, value), span) = (
        with_opt_trailing_ws(alt((
            cwt_identifier.map(AstCwtIdentifierOrString::Identifier),
            quoted_or_unquoted_string.map(AstCwtIdentifierOrString::String),
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
        documentation: vec![],
        span,
    })
}

/// Parse a CWT operator
pub(crate) fn cwt_operator<'a>(input: &mut LocatingSlice<&'a str>) -> ModalResult<CwtOperator> {
    alt((
        literal("==").value(CwtOperator::ComparableEquals),
        literal("=").value(CwtOperator::Equals),
        literal("!=").value(CwtOperator::NotEquals),
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
