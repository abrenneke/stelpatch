use std::ops::Range;

use winnow::{
    LocatingSlice, ModalResult, Parser,
    ascii::{multispace1, till_line_ending},
    combinator::{alt, opt, repeat},
    error::StrContext,
};

use crate::{AstComment, AstNode, eol};

/// CWT option value expressions
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CwtOptionExpression<'a> {
    /// Simple identifier: `country`, `required`
    Identifier(&'a str),
    /// String literal: `"Fancy name"`
    String(&'a str),
    /// Range expression: `0..1`, `~1..2`
    Range {
        min: CwtCommentRangeBound<'a>,
        max: CwtCommentRangeBound<'a>,
        lenient: bool, // true for ~1..2
    },
    /// List expression: `{ country planet }` or `{ this = planet root = ship }` (two assignment expressions)
    Block(Vec<CwtOptionExpression<'a>>),
    /// Assignment expression: `this = planet`
    Assignment {
        key: &'a str,
        value: Box<CwtOptionExpression<'a>>,
    },
}

impl<'a> CwtOptionExpression<'a> {
    /// Check if this is an identifier
    pub fn is_identifier(&self) -> bool {
        matches!(self, CwtOptionExpression::Identifier(_))
    }

    /// Check if this is a string literal
    pub fn is_string(&self) -> bool {
        matches!(self, CwtOptionExpression::String(_))
    }

    /// Check if this is a range expression
    pub fn is_range(&self) -> bool {
        matches!(self, CwtOptionExpression::Range { .. })
    }

    /// Check if this is a list expression
    pub fn is_list(&self) -> bool {
        matches!(self, CwtOptionExpression::Block(_))
    }

    /// Check if this is an assignment expression
    pub fn is_assignment(&self) -> bool {
        matches!(self, CwtOptionExpression::Assignment { .. })
    }

    /// Get the identifier value if this is an identifier
    pub fn as_identifier(&self) -> Option<&'a str> {
        match self {
            CwtOptionExpression::Identifier(s) => Some(s),
            _ => None,
        }
    }

    /// Get the string value if this is a string literal
    pub fn as_string(&self) -> Option<&'a str> {
        match self {
            CwtOptionExpression::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_string_or_identifier(&self) -> Option<&'a str> {
        match self {
            CwtOptionExpression::Identifier(s) => Some(s),
            CwtOptionExpression::String(s) => Some(s),
            _ => None,
        }
    }

    /// Get the range data if this is a range expression
    pub fn as_range(&self) -> Option<(&CwtCommentRangeBound<'a>, &CwtCommentRangeBound<'a>, bool)> {
        match self {
            CwtOptionExpression::Range { min, max, lenient } => Some((min, max, *lenient)),
            _ => None,
        }
    }

    /// Get the list items if this is a list expression
    pub fn as_list(&self) -> Option<&[CwtOptionExpression<'a>]> {
        match self {
            CwtOptionExpression::Block(items) => Some(items),
            _ => None,
        }
    }

    /// Get the assignment data if this is an assignment expression
    pub fn as_assignment(&self) -> Option<(&'a str, &CwtOptionExpression<'a>)> {
        match self {
            CwtOptionExpression::Assignment { key, value } => Some((key, value)),
            _ => None,
        }
    }
}

/// Range bound for cardinality expressions
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CwtCommentRangeBound<'a> {
    Number(&'a str),
    Infinity,
}

impl<'a> CwtCommentRangeBound<'a> {
    /// Check if this is a number
    pub fn is_number(&self) -> bool {
        matches!(self, CwtCommentRangeBound::Number(_))
    }

    /// Check if this is infinity
    pub fn is_infinity(&self) -> bool {
        matches!(self, CwtCommentRangeBound::Infinity)
    }

    /// Get the number value if this is a number
    pub fn as_number(&self) -> Option<&'a str> {
        match self {
            CwtCommentRangeBound::Number(n) => Some(n),
            _ => None,
        }
    }

    /// Convert to a comparable value for ordering (infinity is treated as u32::MAX)
    pub fn to_comparable(&self) -> &'a str {
        match self {
            CwtCommentRangeBound::Number(n) => n,
            CwtCommentRangeBound::Infinity => "inf",
        }
    }
}

/// A single CWT option within an option comment
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AstCwtCommentOption<'a> {
    pub key: &'a str,
    pub is_ne: bool,
    pub value: CwtOptionExpression<'a>,
}

impl<'a> AstCwtCommentOption<'a> {
    /// Create a new option
    pub fn new(key: &'a str, value: CwtOptionExpression<'a>) -> Self {
        Self {
            key,
            value,
            is_ne: false,
        }
    }

    pub fn new_ne(key: &'a str, value: CwtOptionExpression<'a>) -> Self {
        Self {
            key,
            value,
            is_ne: true,
        }
    }

    /// Check if this option has the given key
    pub fn has_key(&self, key: &str) -> bool {
        self.key == key
    }
}

/// Structured data for CWT option comments, preserving order and structure
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CwtOptionData<'a> {
    /// The options in order as they appear in the comment
    pub options: Vec<AstCwtCommentOption<'a>>,
}

impl<'a> CwtOptionData<'a> {
    /// Create new empty option data
    pub fn new() -> Self {
        Self {
            options: Vec::new(),
        }
    }

    /// Add an option
    pub fn add_option(&mut self, option: AstCwtCommentOption<'a>) {
        self.options.push(option);
    }

    /// Get an option by key (first match)
    pub fn get_option(&self, key: &str) -> Option<&AstCwtCommentOption<'a>> {
        self.options.iter().find(|opt| opt.has_key(key))
    }

    /// Get all options with the given key
    pub fn get_options(&self, key: &str) -> Vec<&AstCwtCommentOption<'a>> {
        self.options.iter().filter(|opt| opt.has_key(key)).collect()
    }

    /// Check if there's an option with the given key
    pub fn has_option(&self, key: &str) -> bool {
        self.options.iter().any(|opt| opt.has_key(key))
    }

    /// Get the cardinality option if present
    pub fn get_cardinality(&self) -> Option<&CwtOptionExpression<'a>> {
        self.get_option("cardinality").map(|opt| &opt.value)
    }

    /// Get the scope option if present
    pub fn get_scope(&self) -> Option<&CwtOptionExpression<'a>> {
        self.get_option("scope").map(|opt| &opt.value)
    }

    /// Get the push_scope option if present
    pub fn get_push_scope(&self) -> Option<&CwtOptionExpression<'a>> {
        self.get_option("push_scope").map(|opt| &opt.value)
    }

    /// Get the replace_scope option if present
    pub fn get_replace_scope(&self) -> Option<&CwtOptionExpression<'a>> {
        self.get_option("replace_scope").map(|opt| &opt.value)
    }

    /// Get the severity option if present
    pub fn get_severity(&self) -> Option<&CwtOptionExpression<'a>> {
        self.get_option("severity").map(|opt| &opt.value)
    }

    /// Check if this has a "required" flag
    pub fn is_required(&self) -> bool {
        self.has_option("required")
    }

    /// Check if this has a "primary" flag
    pub fn is_primary(&self) -> bool {
        self.has_option("primary")
    }
}

impl<'a> Default for CwtOptionData<'a> {
    fn default() -> Self {
        Self::new()
    }
}

/// CWT-specific comment types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CwtCommentType<'a> {
    /// Regular comment (#)
    Regular,
    /// Option comment (##) with structured data
    Option(CwtOptionData<'a>),
    /// Documentation comment (###)
    Documentation,
}

/// AST representation of a CWT comment with type information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AstCwtComment<'a> {
    pub text: &'a str,
    pub comment_type: CwtCommentType<'a>,
    pub span: Range<usize>,
}

impl<'a> AstCwtComment<'a> {
    pub fn new(text: &'a str, comment_type: CwtCommentType<'a>, span: Range<usize>) -> Self {
        Self {
            text,
            comment_type,
            span,
        }
    }

    /// Check if this is a regular comment
    pub fn is_regular(&self) -> bool {
        matches!(self.comment_type, CwtCommentType::Regular)
    }

    /// Check if this is an option comment
    pub fn is_option(&self) -> bool {
        matches!(self.comment_type, CwtCommentType::Option(_))
    }

    /// Check if this is a documentation comment
    pub fn is_documentation(&self) -> bool {
        matches!(self.comment_type, CwtCommentType::Documentation)
    }

    /// Get the option data if this is an option comment
    pub fn option_data(&self) -> Option<&CwtOptionData<'a>> {
        match &self.comment_type {
            CwtCommentType::Option(data) => Some(data),
            _ => None,
        }
    }
}

impl<'a> AstNode<'a> for AstCwtComment<'a> {
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

/// CWT-specific comment or whitespace
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CwtCommentOrWhitespace<'a> {
    Comment(AstCwtComment<'a>),
    Whitespace { blank_lines: usize },
}

/// Count blank lines in whitespace
fn cwt_ws_count_blank_lines<'a>(input: &mut LocatingSlice<&'a str>) -> ModalResult<usize> {
    let whitespace: &str = multispace1.parse_next(input)?;
    let num_newlines = whitespace.chars().filter(|c| *c == '\n').count();
    Ok(num_newlines)
}

/// Parse optional whitespace and comments for CWT
pub(crate) fn opt_cwt_ws_and_comments<'a>(
    input: &mut LocatingSlice<&'a str>,
) -> ModalResult<Vec<CwtCommentOrWhitespace<'a>>> {
    let comments_and_whitespace: Vec<CwtCommentOrWhitespace<'a>> = repeat(
        0..,
        alt((
            cwt_ws_count_blank_lines
                .map(|blank_lines| CwtCommentOrWhitespace::Whitespace { blank_lines }),
            cwt_comment.map(CwtCommentOrWhitespace::Comment),
        )),
    )
    .parse_next(input)?;

    Ok(comments_and_whitespace)
}

/// Extract just the comments from a mixed collection of comments and whitespace
pub(crate) fn get_cwt_comments<'a>(
    whitespace: &[CwtCommentOrWhitespace<'a>],
) -> Vec<AstCwtComment<'a>> {
    whitespace
        .iter()
        .filter_map(|c| match c {
            CwtCommentOrWhitespace::Comment(c) => Some(c.clone()),
            CwtCommentOrWhitespace::Whitespace { .. } => None,
        })
        .collect()
}

/// Parse a CWT comment with type detection
pub(crate) fn cwt_comment<'a>(
    input: &mut LocatingSlice<&'a str>,
) -> ModalResult<AstCwtComment<'a>> {
    let ((comment_prefix, comment_text), span) = alt((
        // Documentation comment (###)
        ("###", till_line_ending).map(|(_, text)| (CwtCommentType::Documentation, text)),
        // Option comment (##)
        ("##", till_line_ending).map(|(_, text): (_, &str)| {
            let option_data = parse_option_comment_data(text.trim());
            (CwtCommentType::Option(option_data), text)
        }),
        // Regular comment (#)
        ("#", till_line_ending).map(|(_, text)| (CwtCommentType::Regular, text)),
    ))
    .with_span()
    .context(StrContext::Label("cwt_comment"))
    .parse_next(input)?;

    // Consume the newline but don't count it for the comment text
    opt(eol).parse_next(input)?;

    Ok(AstCwtComment::new(comment_text, comment_prefix, span))
}

/// Parse structured data from option comment text
fn parse_option_comment_data<'a>(text: &'a str) -> CwtOptionData<'a> {
    let mut data = CwtOptionData::new();
    let text = text.trim();

    if text.is_empty() {
        return data;
    }

    // Look for key = value patterns
    if let Some(eq_pos) = text.find('=') {
        let key = text[..eq_pos].trim();
        let value_text = text[eq_pos + 1..].trim();

        let value = parse_option_expression(value_text);
        data.add_option(AstCwtCommentOption::new(&key, value));
    } else if let Some(ne_pos) = text.find("<>") {
        let key = text[..ne_pos].trim();
        let value_text = text[ne_pos + 2..].trim();

        let value = parse_option_expression(value_text);
        data.add_option(AstCwtCommentOption::new_ne(&key, value));
    } else if let Some(ne_pos) = text.find("!=") {
        let key = text[..ne_pos].trim();
        let value_text = text[ne_pos + 2..].trim();

        let value = parse_option_expression(value_text);
        data.add_option(AstCwtCommentOption::new_ne(&key, value));
    } else {
        // Flag with no value: required, primary
        data.add_option(AstCwtCommentOption::new(
            &text,
            CwtOptionExpression::Identifier(text),
        ));
    }

    data
}

/// Parse a CWT option expression from text
fn parse_option_expression<'a>(text: &'a str) -> CwtOptionExpression<'a> {
    let text = text.trim();

    // Try to parse as range first
    if let Some(range) = parse_range_expression(text) {
        return range;
    }

    // Try to parse as list
    if text.starts_with('{') && text.ends_with('}') {
        let inner = text.trim_start_matches('{').trim_end_matches('}').trim();
        if inner.is_empty() {
            return CwtOptionExpression::Block(vec![]);
        }

        let items = parse_list_contents(inner);
        return CwtOptionExpression::Block(items);
    }

    // Try to parse as quoted string
    if text.starts_with('"') && text.ends_with('"') {
        let inner = text.trim_matches('"');
        return CwtOptionExpression::String(inner);
    }

    // Try to parse as single assignment
    if let Some(eq_pos) = text.find('=') {
        let key = text[..eq_pos].trim();
        let value_text = text[eq_pos + 1..].trim();
        let value = Box::new(parse_option_expression(value_text));
        return CwtOptionExpression::Assignment { key, value };
    }

    // Default to identifier
    CwtOptionExpression::Identifier(text)
}

/// Parse a range expression like "0..1" or "~1..2"
fn parse_range_expression<'a>(text: &'a str) -> Option<CwtOptionExpression<'a>> {
    let text = text.trim();

    // Check for lenient cardinality (~)
    let (lenient, text) = if text.starts_with('~') {
        (true, text.strip_prefix('~')?.trim())
    } else {
        (false, text)
    };

    // Look for range pattern
    if let Some(dot_pos) = text.find("..") {
        let min_text = text[..dot_pos].trim();
        let max_text = text[dot_pos + 2..].trim();

        let min = parse_range_bound(min_text)?;
        let max = parse_range_bound(max_text)?;

        Some(CwtOptionExpression::Range { min, max, lenient })
    } else {
        None
    }
}

/// Parse a range bound (number or "inf")
fn parse_range_bound<'a>(text: &'a str) -> Option<CwtCommentRangeBound<'a>> {
    if text == "inf" {
        Some(CwtCommentRangeBound::Infinity)
    } else {
        Some(CwtCommentRangeBound::Number(text))
    }
}

/// Parse the contents of a list, handling assignments and identifiers
fn parse_list_contents<'a>(text: &'a str) -> Vec<CwtOptionExpression<'a>> {
    let mut items = Vec::new();
    let parts: Vec<&str> = text.split_whitespace().collect();

    let mut i = 0;
    while i < parts.len() {
        // Check if this is an assignment (key = value)
        if i + 2 < parts.len() && parts[i + 1] == "=" {
            let key = parts[i];
            let value = Box::new(CwtOptionExpression::Identifier(parts[i + 2]));
            items.push(CwtOptionExpression::Assignment { key, value });
            i += 3;
        } else {
            // Just a regular identifier
            items.push(CwtOptionExpression::Identifier(parts[i]));
            i += 1;
        }
    }

    items
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    // === Basic Comment Type Tests ===

    #[test]
    fn test_basic_cardinality_parsing() {
        let data = parse_option_comment_data("cardinality = 0..1");
        assert_eq!(data.options.len(), 1);

        let option = &data.options[0];
        assert_eq!(option.key, "cardinality");

        if let CwtOptionExpression::Range { min, max, lenient } = &option.value {
            assert_eq!(min, &CwtCommentRangeBound::Number("0"));
            assert_eq!(max, &CwtCommentRangeBound::Number("1"));
            assert!(!lenient);
        } else {
            panic!("Expected range expression");
        }
    }

    #[test]
    fn test_lenient_cardinality_parsing() {
        let data = parse_option_comment_data("cardinality = ~1..2");
        assert_eq!(data.options.len(), 1);

        let option = &data.options[0];
        assert_eq!(option.key, "cardinality");

        if let CwtOptionExpression::Range { min, max, lenient } = &option.value {
            assert_eq!(min, &CwtCommentRangeBound::Number("1"));
            assert_eq!(max, &CwtCommentRangeBound::Number("2"));
            assert!(lenient);
        } else {
            panic!("Expected range expression");
        }
    }

    // === Range Expression Tests ===

    #[test]
    fn test_infinity_ranges() {
        let test_cases = vec![
            ("cardinality = 0..inf", 0, true),
            ("cardinality = 1..inf", 1, true),
            ("cardinality = ~0..inf", 0, true), // lenient with infinity
        ];

        for (input, min_val, expect_inf) in test_cases {
            let data = parse_option_comment_data(input);
            let option = &data.options[0];

            if let CwtOptionExpression::Range {
                min,
                max,
                lenient: _,
            } = &option.value
            {
                assert_eq!(min, &CwtCommentRangeBound::Number(&min_val.to_string()));
                if expect_inf {
                    assert_eq!(max, &CwtCommentRangeBound::Infinity);
                }
            } else {
                panic!("Expected range expression for: {}", input);
            }
        }
    }

    #[test]
    fn test_range_edge_cases() {
        let test_cases = vec![
            ("cardinality = 0..0", 0, 0),         // Same min/max
            ("cardinality = 100..200", 100, 200), // Large numbers
            ("cardinality = ~5..10", 5, 10),      // Lenient mid-range
        ];

        for (input, min_val, max_val) in test_cases {
            let data = parse_option_comment_data(input);
            let option = &data.options[0];

            if let CwtOptionExpression::Range {
                min,
                max,
                lenient: _,
            } = &option.value
            {
                assert_eq!(min, &CwtCommentRangeBound::Number(&min_val.to_string()));
                assert_eq!(max, &CwtCommentRangeBound::Number(&max_val.to_string()));
            } else {
                panic!("Expected range expression for: {}", input);
            }
        }
    }

    // === List Expression Tests ===

    #[test]
    fn test_simple_lists() {
        let data = parse_option_comment_data("scope = { country planet }");
        assert_eq!(data.options.len(), 1);

        let option = &data.options[0];
        assert_eq!(option.key, "scope");

        if let CwtOptionExpression::Block(items) = &option.value {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0].as_identifier(), Some("country"));
            assert_eq!(items[1].as_identifier(), Some("planet"));
        } else {
            panic!("Expected list expression");
        }
    }

    #[test]
    fn test_complex_lists() {
        let test_cases = vec![
            ("type_key_filter = { one two three }", 3),
            (
                "graph_related_types = { special_project anomaly_category }",
                2,
            ),
            ("scope = { country planet fleet ship }", 4),
            ("scope = { }", 0), // Empty list
        ];

        for (input, expected_count) in test_cases {
            let data = parse_option_comment_data(input);
            let option = &data.options[0];

            if let CwtOptionExpression::Block(items) = &option.value {
                assert_eq!(items.len(), expected_count, "Failed for: {}", input);
            } else {
                panic!("Expected list expression for: {}", input);
            }
        }
    }

    // === String Literal Tests ===

    #[test]
    fn test_string_literals() {
        let test_cases = vec![
            ("display_name = \"Fancy Name\"", "Fancy Name"),
            ("display_name = \"Simple\"", "Simple"),
            ("display_name = \"With Spaces\"", "With Spaces"),
            ("display_name = \"\"", ""), // Empty string
            ("abbreviation = \"ST\"", "ST"),
        ];

        for (input, expected) in test_cases {
            let data = parse_option_comment_data(input);
            let option = &data.options[0];

            assert_eq!(
                option.value.as_string(),
                Some(expected),
                "Failed for: {}",
                input
            );
        }
    }

    // === Flag Tests ===

    #[test]
    fn test_flags() {
        let test_cases = vec![
            "required",
            "primary",
            "some_flag",
            "UPPERCASE_FLAG",
            "flag_with_underscores",
        ];

        for flag in test_cases {
            let data = parse_option_comment_data(flag);
            assert_eq!(data.options.len(), 1);

            let option = &data.options[0];
            assert_eq!(option.key, flag);
            assert_eq!(option.value.as_identifier(), Some(flag));
        }
    }

    // === Assignment Tests ===

    #[test]
    fn test_simple_assignments() {
        let test_cases = vec![
            ("push_scope = country", "push_scope", "country"),
            ("severity = warning", "severity", "warning"),
            ("starts_with = b_", "starts_with", "b_"),
            ("abbreviation = ST", "abbreviation", "ST"),
        ];

        for (input, key, value) in test_cases {
            let data = parse_option_comment_data(input);
            let option = &data.options[0];

            assert_eq!(option.key, key);
            assert_eq!(
                option.value.as_identifier(),
                Some(value),
                "Failed for: {}",
                input
            );
        }
    }

    #[test]
    fn test_complex_assignments() {
        // Test replace_scope with multiple assignments
        let data = parse_option_comment_data("replace_scope = { this = planet root = ship }");
        let option = &data.options[0];

        assert_eq!(option.key, "replace_scope");
        if let CwtOptionExpression::Block(items) = &option.value {
            // Should now parse as two assignments
            assert_eq!(items.len(), 2);

            // First assignment: this = planet
            if let Some((key, value)) = items[0].as_assignment() {
                assert_eq!(key, "this");
                assert_eq!(value.as_identifier(), Some("planet"));
            } else {
                panic!("Expected first item to be assignment");
            }

            // Second assignment: root = ship
            if let Some((key, value)) = items[1].as_assignment() {
                assert_eq!(key, "root");
                assert_eq!(value.as_identifier(), Some("ship"));
            } else {
                panic!("Expected second item to be assignment");
            }
        } else {
            panic!("Expected list expression for replace_scope");
        }
    }

    // === Helper Method Tests ===

    #[test]
    fn test_option_data_helpers() {
        let data = parse_option_comment_data("cardinality = 0..1");

        // Test basic option access
        assert!(data.has_option("cardinality"));
        assert!(!data.has_option("scope"));

        let cardinality = data.get_cardinality();
        assert!(cardinality.is_some());
        assert!(cardinality.unwrap().is_range());

        // Test specific accessors
        assert!(data.get_scope().is_none());
        assert!(data.get_push_scope().is_none());
        assert!(!data.is_required());
        assert!(!data.is_primary());
    }

    #[test]
    fn test_multiple_flag_helpers() {
        let data = parse_option_comment_data("required");
        assert!(data.is_required());

        let data = parse_option_comment_data("primary");
        assert!(data.is_primary());
    }

    #[test]
    fn test_scope_helpers() {
        let data = parse_option_comment_data("scope = { country planet }");
        let scope = data.get_scope();
        assert!(scope.is_some());
        assert!(scope.unwrap().is_list());

        let data = parse_option_comment_data("push_scope = country");
        let push_scope = data.get_push_scope();
        assert!(push_scope.is_some());
        assert!(push_scope.unwrap().is_identifier());
    }

    // === Range Bound Helper Tests ===

    #[test]
    fn test_range_bound_helpers() {
        let number_bound = CwtCommentRangeBound::Number("42");
        let inf_bound = CwtCommentRangeBound::Infinity;

        // Test type checking
        assert!(number_bound.is_number());
        assert!(!number_bound.is_infinity());
        assert!(!inf_bound.is_number());
        assert!(inf_bound.is_infinity());

        // Test value extraction
        assert_eq!(number_bound.as_number(), Some("42"));
        assert_eq!(inf_bound.as_number(), None);

        // Test comparable values
        assert_eq!(number_bound.to_comparable(), "42");
        assert_eq!(inf_bound.to_comparable(), "inf");
    }

    // === Expression Helper Tests ===

    #[test]
    fn test_expression_type_checking() {
        let identifier = CwtOptionExpression::Identifier("test");
        let string_expr = CwtOptionExpression::String("test");
        let range_expr = CwtOptionExpression::Range {
            min: CwtCommentRangeBound::Number("0"),
            max: CwtCommentRangeBound::Number("1"),
            lenient: false,
        };
        let list_expr = CwtOptionExpression::Block(vec![]);
        let assignment_expr = CwtOptionExpression::Assignment {
            key: "key",
            value: Box::new(CwtOptionExpression::Identifier("value")),
        };

        // Test type checking methods
        assert!(identifier.is_identifier());
        assert!(string_expr.is_string());
        assert!(range_expr.is_range());
        assert!(list_expr.is_list());
        assert!(assignment_expr.is_assignment());

        // Test cross-type checking
        assert!(!identifier.is_string());
        assert!(!string_expr.is_range());
        assert!(!range_expr.is_list());
        assert!(!list_expr.is_assignment());
        assert!(!assignment_expr.is_identifier());
    }

    #[test]
    fn test_expression_value_extraction() {
        let identifier = CwtOptionExpression::Identifier("test");
        let string_expr = CwtOptionExpression::String("test");
        let range_expr = CwtOptionExpression::Range {
            min: CwtCommentRangeBound::Number("0"),
            max: CwtCommentRangeBound::Number("1"),
            lenient: false,
        };
        let list_expr = CwtOptionExpression::Block(vec![]);
        let assignment_expr = CwtOptionExpression::Assignment {
            key: "key",
            value: Box::new(CwtOptionExpression::Identifier("value")),
        };

        // Test value extraction
        assert_eq!(identifier.as_identifier(), Some("test"));
        assert_eq!(string_expr.as_string(), Some("test"));
        assert!(range_expr.as_range().is_some());
        assert_eq!(list_expr.as_list(), Some(&[][..]));
        assert!(assignment_expr.as_assignment().is_some());

        // Test cross-type extraction returns None
        assert!(identifier.as_string().is_none());
        assert!(string_expr.as_identifier().is_none());
        assert!(range_expr.as_list().is_none());
        assert!(list_expr.as_assignment().is_none());
        assert!(assignment_expr.as_range().is_none());
    }

    // === Real-world Examples from CWT Spec ===

    #[test]
    fn test_real_world_examples() {
        // Examples from the CWT specification document
        let examples = vec![
            ("cardinality = 0..1", "Basic cardinality"),
            ("cardinality = ~1..2", "Lenient cardinality"),
            ("push_scope = country", "Scope push"),
            ("severity = information", "Severity setting"),
            ("scope = { country planet }", "Multiple scopes"),
            ("type_key_filter = country_event", "Type filter"),
            (
                "graph_related_types = { special_project anomaly_category }",
                "Graph types",
            ),
            ("display_name = \"Fancy name\"", "Display name"),
            ("abbreviation = ST", "Abbreviation"),
            ("starts_with = b_", "Starts with filter"),
            ("required", "Required flag"),
            ("primary", "Primary flag"),
        ];

        for (input, description) in examples {
            let data = parse_option_comment_data(input);
            assert!(
                !data.options.is_empty(),
                "Failed to parse: {} ({})",
                input,
                description
            );
        }
    }

    // === Edge Cases and Error Handling ===

    #[test]
    fn test_edge_cases() {
        // Empty input
        let data = parse_option_comment_data("");
        assert!(data.options.is_empty());

        // Whitespace only
        let data = parse_option_comment_data("   ");
        assert!(data.options.is_empty());

        // Just equals sign
        let data = parse_option_comment_data("=");
        assert_eq!(data.options.len(), 1);
        assert_eq!(data.options[0].key, "");

        // Equals at end
        let data = parse_option_comment_data("key =");
        assert_eq!(data.options.len(), 1);
        assert_eq!(data.options[0].key, "key");

        // Multiple equals signs (should be treated as complex assignment)
        let data = parse_option_comment_data("a = b = c");
        assert_eq!(data.options.len(), 1);
        assert_eq!(data.options[0].key, "a");
    }

    #[test]
    fn test_malformed_ranges() {
        // These should not parse as ranges but as identifiers
        let non_ranges = vec![
            "cardinality = 0", // No range operator
        ];

        for input in non_ranges {
            let data = parse_option_comment_data(input);
            let option = &data.options[0];
            // Should parse as identifier since range parsing failed
            assert!(
                option.value.is_identifier(),
                "Should be identifier for: {}",
                input
            );
        }
    }

    // === Performance and Stress Tests ===

    #[test]
    fn test_large_lists() {
        // Test with a large list to ensure performance is reasonable
        let large_list = format!(
            "scope = {{ {} }}",
            (0..100)
                .map(|i| format!("item_{}", i))
                .collect::<Vec<_>>()
                .join(" ")
        );

        let data = parse_option_comment_data(&large_list);
        let option = &data.options[0];

        if let CwtOptionExpression::Block(items) = &option.value {
            assert_eq!(items.len(), 100);
            assert_eq!(items[0].as_identifier(), Some("item_0"));
            assert_eq!(items[99].as_identifier(), Some("item_99"));
        } else {
            panic!("Expected list expression for large list");
        }
    }

    #[test]
    fn test_complex_nested_structures() {
        // Test complex nested structures (though our current parser doesn't deeply nest)
        let complex_input = "replace_scope = { this = planet root = ship from = country }";
        let data = parse_option_comment_data(complex_input);

        assert_eq!(data.options.len(), 1);
        let option = &data.options[0];
        assert_eq!(option.key, "replace_scope");

        // Should parse as some kind of structured expression
        assert!(option.value.is_list());
    }
}
