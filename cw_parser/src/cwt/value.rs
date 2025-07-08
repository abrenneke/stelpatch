use std::ops::Range;

use winnow::{
    LocatingSlice, ModalResult, Parser,
    ascii::digit1,
    combinator::{alt, delimited, opt, separated_pair},
    error::StrContext,
    token::literal,
};

use crate::{AstComment, AstNode, AstString, quoted_or_unquoted_string};

use super::{AstCwtBlock, AstCwtIdentifier, cwt_block, cwt_identifier};

/// CWT value types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CwtValue<'a> {
    /// Simple rule types: bool, int, float, scalar, etc.
    Simple(CwtSimpleValue<'a>),
    /// Identifier types: regular, <type_key>, enum[key], scope[key], etc.
    Identifier(AstCwtIdentifier<'a>),
    /// Block value: { ... }
    Block(AstCwtBlock<'a>),
    /// A quoted or unquoted string by itself, for e.g. enum values
    String(AstString<'a>),
}

/// Simple CWT value types
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CwtSimpleValue<'a> {
    pub value_type: CwtSimpleValueType,
    pub range: Option<CwtRange<'a>>,
    pub span: Range<usize>,
}

/// Types of simple CWT values
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CwtSimpleValueType {
    Bool,
    Int,
    Float,
    Scalar,
    PercentageField,
    Localisation,
    LocalisationSynced,
    LocalisationInline,
    DateField,
    VariableField,
    IntVariableField,
    ValueField,
    IntValueField,
    ScopeField,
    Filepath,
    Icon,
}

/// Unified CWT reference types - used in both values and standalone identifiers
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CwtReferenceType<'a> {
    /// Type reference: <type_key>
    TypeRef,
    /// Type reference with prefix/suffix: prefix_<type_key>_suffix
    TypeRefWithPrefixSuffix(&'a str, &'a str),
    /// Enum reference: enum[key]
    Enum,
    /// Scope reference: scope[key]
    Scope,
    /// Scope group reference: scope_group[key]
    ScopeGroup,
    /// Alias reference: alias[key]
    Alias,
    /// Alias name reference: alias_name[key]
    AliasName,
    /// Alias match left reference: alias_match_left[key]
    AliasMatchLeft,
    /// Value reference: value[key]
    Value,
    /// Value set reference: value_set[key]
    ValueSet,
    /// Single alias reference: single_alias_right[key]
    SingleAlias,
    /// Alias keys field: alias_keys_field[key]
    AliasKeysField,
    /// Icon reference: icon[path]
    Icon,
    /// Filepath reference: filepath[path]
    Filepath,
    /// Colour reference: colour[hsv|rgb]
    Colour,
    /// Stellaris name format reference: stellaris_name_format[key]
    StellarisNameFormat,
    /// Type reference: type[key]
    Type,
    /// Subtype reference: subtype[name]
    Subtype,
    /// Complex enum reference: complex_enum[key]
    ComplexEnum,
}

/// Types of alias references
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CwtAliasType {
    Name,
    MatchLeft,
}

impl<'a> CwtReferenceType<'a> {
    /// Check if this is a type reference
    pub fn is_type_ref(&self) -> bool {
        matches!(self, Self::TypeRef | Self::TypeRefWithPrefixSuffix(..))
    }

    /// Check if this is an alias reference
    pub fn is_alias(&self) -> bool {
        matches!(self, Self::AliasName | Self::AliasMatchLeft)
    }
}

/// Range specification for int and float values
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CwtRange<'a> {
    pub min: CwtRangeBound<'a>,
    pub max: CwtRangeBound<'a>,
    pub span: Range<usize>,
}

/// Range bounds
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CwtRangeBound<'a> {
    /// Integer value
    Int(&'a str),
    /// Float value
    Float(&'a str),
    /// Infinity (-inf or inf)
    Infinity(bool), // true for positive infinity, false for negative infinity
}

impl<'a> CwtValue<'a> {
    pub fn new_simple(
        value_type: CwtSimpleValueType,
        range: Option<CwtRange<'a>>,
        span: Range<usize>,
    ) -> Self {
        Self::Simple(CwtSimpleValue {
            value_type,
            range,
            span,
        })
    }

    pub fn new_identifier(identifier: AstCwtIdentifier<'a>) -> Self {
        Self::Identifier(identifier)
    }

    pub fn new_block(block: AstCwtBlock<'a>) -> Self {
        Self::Block(block)
    }

    /// Check if this is a simple value
    pub fn is_simple(&self) -> bool {
        matches!(self, Self::Simple(_))
    }

    /// Check if this is an identifier value
    pub fn is_identifier(&self) -> bool {
        matches!(self, Self::Identifier(_))
    }

    /// Check if this is a block value
    pub fn is_block(&self) -> bool {
        matches!(self, Self::Block(_))
    }

    /// Try to get the value as a simple value
    pub fn as_simple(&self) -> Option<&CwtSimpleValue<'a>> {
        match self {
            Self::Simple(s) => Some(s),
            _ => None,
        }
    }

    /// Try to get the value as an identifier value
    pub fn as_identifier(&self) -> Option<&AstCwtIdentifier<'a>> {
        match self {
            Self::Identifier(i) => Some(i),
            _ => None,
        }
    }

    /// Try to get the value as a block value
    pub fn as_block(&self) -> Option<&AstCwtBlock<'a>> {
        match self {
            Self::Block(b) => Some(b),
            _ => None,
        }
    }
}

impl<'a> AstNode<'a> for CwtValue<'a> {
    fn span_range(&self) -> Range<usize> {
        match self {
            Self::Simple(s) => s.span.clone(),
            Self::Identifier(i) => i.span.clone(),
            Self::Block(b) => b.span_range(),
            Self::String(s) => s.span_range(),
        }
    }

    fn leading_comments(&self) -> &[AstComment<'a>] {
        match self {
            Self::Simple(_) => &[],
            Self::Identifier(i) => i.leading_comments(),
            Self::Block(b) => b.leading_comments(),
            Self::String(s) => s.leading_comments(),
        }
    }

    fn trailing_comment(&self) -> Option<&AstComment<'a>> {
        match self {
            Self::Simple(_) => None,
            Self::Identifier(_) => None,
            Self::Block(b) => b.trailing_comment(),
            Self::String(s) => s.trailing_comment(),
        }
    }
}

impl<'a> AstNode<'a> for CwtSimpleValue<'a> {
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

/// Parse a CWT value
pub(crate) fn cwt_value<'a>(input: &mut LocatingSlice<&'a str>) -> ModalResult<CwtValue<'a>> {
    alt((
        cwt_identifier.map(CwtValue::Identifier),
        simple_value.map(CwtValue::Simple),
        cwt_block.map(CwtValue::Block),
        quoted_or_unquoted_string.map(CwtValue::String),
    ))
    .context(StrContext::Label("cwt_value"))
    .parse_next(input)
}

/// Parse a simple CWT value type
pub(crate) fn simple_value<'a>(
    input: &mut LocatingSlice<&'a str>,
) -> ModalResult<CwtSimpleValue<'a>> {
    let ((value_type, range), span) = alt((
        // Types with potential ranges - order matters! Longer matches first
        (literal("int_variable_field"), opt(cwt_range_inline))
            .map(|(_, range)| (CwtSimpleValueType::IntVariableField, range)),
        (literal("int_value_field"), opt(cwt_range_inline))
            .map(|(_, range)| (CwtSimpleValueType::IntValueField, range)),
        (literal("variable_field"), opt(cwt_range_inline))
            .map(|(_, range)| (CwtSimpleValueType::VariableField, range)),
        (literal("value_field"), opt(cwt_range_inline))
            .map(|(_, range)| (CwtSimpleValueType::ValueField, range)),
        (literal("int"), opt(cwt_range_inline)).map(|(_, range)| (CwtSimpleValueType::Int, range)),
        (literal("float"), opt(cwt_range_inline))
            .map(|(_, range)| (CwtSimpleValueType::Float, range)),
        // Types without ranges - order matters! Longer matches first
        literal("percentage_field").value((CwtSimpleValueType::PercentageField, None)),
        literal("localisation_synced").value((CwtSimpleValueType::LocalisationSynced, None)),
        literal("localisation_inline").value((CwtSimpleValueType::LocalisationInline, None)),
        literal("localisation").value((CwtSimpleValueType::Localisation, None)),
        literal("date_field").value((CwtSimpleValueType::DateField, None)),
        literal("scope_field").value((CwtSimpleValueType::ScopeField, None)),
        literal("filepath").value((CwtSimpleValueType::Filepath, None)),
        literal("scalar").value((CwtSimpleValueType::Scalar, None)),
        literal("bool").value((CwtSimpleValueType::Bool, None)),
        literal("icon").value((CwtSimpleValueType::Icon, None)),
    ))
    .with_span()
    .context(StrContext::Label("simple_value"))
    .parse_next(input)?;

    Ok(CwtSimpleValue {
        value_type,
        range,
        span,
    })
}

/// Parse an inline range specification: [min..max]
pub(crate) fn cwt_range_inline<'a>(
    input: &mut LocatingSlice<&'a str>,
) -> ModalResult<CwtRange<'a>> {
    let ((min, max), span) = delimited(
        "[",
        separated_pair(range_bound, alt(("...", "..")), range_bound),
        "]",
    )
    .with_span()
    .context(StrContext::Label("cwt_range_inline"))
    .parse_next(input)?;

    Ok(CwtRange { min, max, span })
}

/// Parse a range bound (int, float, or infinity)
pub(crate) fn range_bound<'a>(
    input: &mut LocatingSlice<&'a str>,
) -> ModalResult<CwtRangeBound<'a>> {
    alt((
        literal("inf").value(CwtRangeBound::Infinity(true)),
        literal("-inf").value(CwtRangeBound::Infinity(false)),
        (
            opt(alt((literal("-"), literal("+")))),
            digit1,
            opt((literal("."), digit1)),
        )
            .take()
            .map(|s: &str| {
                if s.contains('.') {
                    CwtRangeBound::Float(s)
                } else {
                    CwtRangeBound::Int(s)
                }
            }),
    ))
    .context(StrContext::Label("range_bound"))
    .parse_next(input)
}

#[cfg(test)]
mod tests {
    use crate::{AstCwtIdentifierKey, AstCwtIdentifierOrString, AstString};

    use super::*;
    use pretty_assertions::assert_eq;
    use winnow::LocatingSlice;

    macro_rules! parse_test {
        ($parser:ident, $input:expr) => {{
            let mut input = LocatingSlice::new($input);
            $parser(&mut input).unwrap()
        }};
    }

    #[test]
    fn test_simple_value_types() {
        // Test bool
        let result = parse_test!(simple_value, "bool");
        assert_eq!(result.value_type, CwtSimpleValueType::Bool);
        assert!(result.range.is_none());

        // Test int
        let result = parse_test!(simple_value, "int");
        assert_eq!(result.value_type, CwtSimpleValueType::Int);
        assert!(result.range.is_none());

        // Test float
        let result = parse_test!(simple_value, "float");
        assert_eq!(result.value_type, CwtSimpleValueType::Float);
        assert!(result.range.is_none());

        // Test scalar
        let result = parse_test!(simple_value, "scalar");
        assert_eq!(result.value_type, CwtSimpleValueType::Scalar);

        // Test percentage_field
        let result = parse_test!(simple_value, "percentage_field");
        assert_eq!(result.value_type, CwtSimpleValueType::PercentageField);

        // Test localisation variants
        let result = parse_test!(simple_value, "localisation");
        assert_eq!(result.value_type, CwtSimpleValueType::Localisation);

        let result = parse_test!(simple_value, "localisation_synced");
        assert_eq!(result.value_type, CwtSimpleValueType::LocalisationSynced);

        let result = parse_test!(simple_value, "localisation_inline");
        assert_eq!(result.value_type, CwtSimpleValueType::LocalisationInline);

        // Test field types
        let result = parse_test!(simple_value, "date_field");
        assert_eq!(result.value_type, CwtSimpleValueType::DateField);

        let result = parse_test!(simple_value, "variable_field");
        assert_eq!(result.value_type, CwtSimpleValueType::VariableField);

        let result = parse_test!(simple_value, "int_variable_field");
        assert_eq!(result.value_type, CwtSimpleValueType::IntVariableField);

        let result = parse_test!(simple_value, "value_field");
        assert_eq!(result.value_type, CwtSimpleValueType::ValueField);

        let result = parse_test!(simple_value, "int_value_field");
        assert_eq!(result.value_type, CwtSimpleValueType::IntValueField);

        let result = parse_test!(simple_value, "scope_field");
        assert_eq!(result.value_type, CwtSimpleValueType::ScopeField);

        // Test file types
        let result = parse_test!(simple_value, "filepath");
        assert_eq!(result.value_type, CwtSimpleValueType::Filepath);

        let result = parse_test!(simple_value, "icon");
        assert_eq!(result.value_type, CwtSimpleValueType::Icon);
    }

    #[test]
    fn test_complex_value_types() {
        // Test type reference
        let result = parse_test!(cwt_identifier, "<building>");
        match result.identifier_type {
            CwtReferenceType::TypeRef => {
                assert_eq!(result.name.raw_value(), "building");
            }
            _ => panic!("Expected TypeRef"),
        }

        // Test enum reference
        let result = parse_test!(cwt_identifier, "enum[government_type]");
        match result.identifier_type {
            CwtReferenceType::Enum => {
                assert_eq!(result.name.raw_value(), "government_type");
            }
            _ => panic!("Expected Enum"),
        }

        // Test scope reference
        let result = parse_test!(cwt_identifier, "scope[country]");
        match result.identifier_type {
            CwtReferenceType::Scope => {
                assert_eq!(result.name.raw_value(), "country");
            }
            _ => panic!("Expected Scope"),
        }

        // Test scope group reference
        let result = parse_test!(cwt_identifier, "scope_group[celestial_coordinate]");
        match result.identifier_type {
            CwtReferenceType::ScopeGroup => {
                assert_eq!(result.name.raw_value(), "celestial_coordinate");
            }
            _ => panic!("Expected ScopeGroup"),
        }

        // Test value reference
        let result = parse_test!(cwt_identifier, "value[resource]");
        match result.identifier_type {
            CwtReferenceType::Value => {
                assert_eq!(result.name.raw_value(), "resource");
            }
            _ => panic!("Expected Value"),
        }

        // Test value set reference
        let result = parse_test!(cwt_identifier, "value_set[ethics]");
        match result.identifier_type {
            CwtReferenceType::ValueSet => {
                assert_eq!(result.name.raw_value(), "ethics");
            }
            _ => panic!("Expected ValueSet"),
        }

        // Test single alias reference
        let result = parse_test!(cwt_identifier, "single_alias_right[test]");
        match result.identifier_type {
            CwtReferenceType::SingleAlias => {
                assert_eq!(result.name.raw_value(), "test");
            }
            _ => panic!("Expected SingleAlias"),
        }

        // Test alias keys field
        let result = parse_test!(cwt_identifier, "alias_keys_field[test]");
        match result.identifier_type {
            CwtReferenceType::AliasKeysField => {
                assert_eq!(result.name.raw_value(), "test");
            }
            _ => panic!("Expected AliasKeysField"),
        }

        // Test alias name
        let result = parse_test!(cwt_identifier, "alias_name[test]");
        match result.identifier_type {
            CwtReferenceType::AliasName => {
                assert_eq!(result.name.raw_value(), "test");
            }
            _ => panic!("Expected AliasName"),
        }

        // Test alias match left
        let result = parse_test!(cwt_identifier, "alias_match_left[test]");
        match result.identifier_type {
            CwtReferenceType::AliasMatchLeft => {
                assert_eq!(result.name.raw_value(), "test");
            }
            _ => panic!("Expected AliasMatchLeft"),
        }
    }

    #[test]
    fn test_alias_match_left_trigger() {
        // Test specifically for alias_match_left[trigger] which was failing
        let result = parse_test!(cwt_identifier, "alias_match_left[trigger]");
        match result.identifier_type {
            CwtReferenceType::AliasMatchLeft => {
                assert_eq!(result.name.raw_value(), "trigger");
            }
            _ => panic!("Expected AliasMatchLeft type"),
        }
    }

    #[test]
    fn test_range_bounds() {
        // Test integer bound
        let result = parse_test!(range_bound, "42");
        match result {
            CwtRangeBound::Int(val) => assert_eq!(val, "42"),
            _ => panic!("Expected Int"),
        }

        // Test negative integer bound
        let result = parse_test!(range_bound, "-10");
        match result {
            CwtRangeBound::Int(val) => assert_eq!(val, "-10"),
            _ => panic!("Expected Int"),
        }

        // Test float bound
        let result = parse_test!(range_bound, "3.14");
        match result {
            CwtRangeBound::Float(val) => assert_eq!(val, "3.14"),
            _ => panic!("Expected Float"),
        }

        // Test negative float bound
        let result = parse_test!(range_bound, "-2.5");
        match result {
            CwtRangeBound::Float(val) => assert_eq!(val, "-2.5"),
            _ => panic!("Expected Float"),
        }

        // Test positive infinity
        let result = parse_test!(range_bound, "inf");
        match result {
            CwtRangeBound::Infinity(true) => (),
            _ => panic!("Expected positive infinity"),
        }

        // Test negative infinity
        let result = parse_test!(range_bound, "-inf");
        match result {
            CwtRangeBound::Infinity(false) => (),
            _ => panic!("Expected negative infinity"),
        }
    }

    #[test]
    fn test_cwt_range() {
        // Test integer range
        let result = parse_test!(cwt_range_inline, "[0..100]");
        match (&result.min, &result.max) {
            (CwtRangeBound::Int(min), CwtRangeBound::Int(max)) => {
                assert_eq!(*min, "0");
                assert_eq!(*max, "100");
            }
            _ => panic!("Expected integer range"),
        }

        // Test float range
        let result = parse_test!(cwt_range_inline, "[0.0..1.0]");
        match (&result.min, &result.max) {
            (CwtRangeBound::Float(min), CwtRangeBound::Float(max)) => {
                assert_eq!(*min, "0.0");
                assert_eq!(*max, "1.0");
            }
            _ => panic!("Expected float range"),
        }

        // Test mixed range with infinity
        let result = parse_test!(cwt_range_inline, "[0..inf]");
        match (&result.min, &result.max) {
            (CwtRangeBound::Int(min), CwtRangeBound::Infinity(true)) => {
                assert_eq!(*min, "0");
            }
            _ => panic!("Expected mixed range with infinity"),
        }

        // Test range with negative infinity
        let result = parse_test!(cwt_range_inline, "[-inf..0]");
        match (&result.min, &result.max) {
            (CwtRangeBound::Infinity(false), CwtRangeBound::Int(max)) => {
                assert_eq!(*max, "0");
            }
            _ => panic!("Expected range with negative infinity"),
        }

        let result = parse_test!(cwt_range_inline, "[0...100]");
        match (&result.min, &result.max) {
            (CwtRangeBound::Int(min), CwtRangeBound::Int(max)) => {
                assert_eq!(*min, "0");
                assert_eq!(*max, "100");
            }
            _ => panic!("Expected integer range"),
        }
    }

    #[test]
    fn test_simple_value_with_range() {
        // Test int with range
        let result = parse_test!(simple_value, "int[0..100]");
        assert_eq!(result.value_type, CwtSimpleValueType::Int);
        assert!(result.range.is_some());
        let range = result.range.unwrap();
        match (&range.min, &range.max) {
            (CwtRangeBound::Int(min), CwtRangeBound::Int(max)) => {
                assert_eq!(*min, "0");
                assert_eq!(*max, "100");
            }
            _ => panic!("Expected integer range"),
        }

        // Test float with range
        let result = parse_test!(simple_value, "float[0.0..1.0]");
        assert_eq!(result.value_type, CwtSimpleValueType::Float);
        assert!(result.range.is_some());
        let range = result.range.unwrap();
        match (&range.min, &range.max) {
            (CwtRangeBound::Float(min), CwtRangeBound::Float(max)) => {
                assert_eq!(*min, "0.0");
                assert_eq!(*max, "1.0");
            }
            _ => panic!("Expected float range"),
        }
    }

    #[test]
    fn test_cwt_value_enum() {
        // Test simple value parsing
        let result = parse_test!(cwt_value, "bool");
        assert!(result.is_simple());
        if let CwtValue::Simple(simple) = result {
            assert_eq!(simple.value_type, CwtSimpleValueType::Bool);
        }

        // Test identifier value parsing
        let result = parse_test!(cwt_value, "<building>");
        assert!(result.is_identifier());
        if let CwtValue::Identifier(identifier) = result {
            match identifier.identifier_type {
                CwtReferenceType::TypeRef => {
                    assert_eq!(identifier.name.raw_value(), "building");
                }
                _ => panic!("Expected TypeRef"),
            }
        }
    }

    #[test]
    fn test_cwt_value_methods() {
        // Test simple value methods
        let simple = CwtValue::new_simple(CwtSimpleValueType::Bool, None, 0..4);
        assert!(simple.is_simple());
        assert!(!simple.is_identifier());
        assert!(simple.as_simple().is_some());
        assert!(simple.as_identifier().is_none());

        // Test identifier value methods
        let identifier_val = CwtValue::new_identifier(AstCwtIdentifier {
            identifier_type: CwtReferenceType::TypeRef,
            name: Box::new(AstCwtIdentifierKey::new(
                None,
                AstCwtIdentifierOrString::String(AstString::new("test", false, 0..6)),
            )),
            before_identifier: None,
            span: 0..6,
            leading_comments: Vec::new(),
            trailing_comment: None,
            is_not: false,
        });
        assert!(!identifier_val.is_simple());
        assert!(identifier_val.is_identifier());
        assert!(identifier_val.as_simple().is_none());
        assert!(identifier_val.as_identifier().is_some());
    }

    #[test]
    fn test_ast_node_implementations() {
        // Test that CwtValue implements AstNode correctly
        let simple = CwtValue::new_simple(CwtSimpleValueType::Bool, None, 5..9);
        assert_eq!(simple.span_range(), 5..9);
        assert_eq!(simple.leading_comments().len(), 0);
        assert!(simple.trailing_comment().is_none());

        let identifier_val = CwtValue::new_identifier(AstCwtIdentifier {
            identifier_type: CwtReferenceType::TypeRef,
            name: Box::new(AstCwtIdentifierKey::new(
                None,
                AstCwtIdentifierOrString::String(AstString::new("test", false, 10..16)),
            )),
            before_identifier: None,
            span: 10..16,
            leading_comments: Vec::new(),
            trailing_comment: None,
            is_not: false,
        });
        assert_eq!(identifier_val.span_range(), 10..16);
        assert_eq!(identifier_val.leading_comments().len(), 0);
        assert!(identifier_val.trailing_comment().is_none());
    }

    #[test]
    fn test_alias_types() {
        assert_eq!(CwtAliasType::Name, CwtAliasType::Name);
        assert_eq!(CwtAliasType::MatchLeft, CwtAliasType::MatchLeft);
        assert_ne!(CwtAliasType::Name, CwtAliasType::MatchLeft);
    }

    #[test]
    fn test_range_construction() {
        let range = CwtRange {
            min: CwtRangeBound::Int("0"),
            max: CwtRangeBound::Int("100"),
            span: 0..7,
        };
        assert_eq!(range.span, 0..7);
        match (&range.min, &range.max) {
            (CwtRangeBound::Int(min), CwtRangeBound::Int(max)) => {
                assert_eq!(*min, "0");
                assert_eq!(*max, "100");
            }
            _ => panic!("Expected integer bounds"),
        }
    }

    #[test]
    fn test_simple_value_types_ordering() {
        // Test that longer keywords are matched before shorter ones
        let result = parse_test!(simple_value, "localisation_synced");
        assert_eq!(result.value_type, CwtSimpleValueType::LocalisationSynced);

        let result = parse_test!(simple_value, "localisation_inline");
        assert_eq!(result.value_type, CwtSimpleValueType::LocalisationInline);

        let result = parse_test!(simple_value, "int_variable_field");
        assert_eq!(result.value_type, CwtSimpleValueType::IntVariableField);

        let result = parse_test!(simple_value, "int_value_field");
        assert_eq!(result.value_type, CwtSimpleValueType::IntValueField);
    }

    #[test]
    fn test_complex_alphanumeric_keys() {
        // Test that alphanumeric keys work correctly
        let result = parse_test!(cwt_identifier, "<building123>");
        match result.identifier_type {
            CwtReferenceType::TypeRef => {
                assert_eq!(result.name.raw_value(), "building123");
            }
            _ => panic!("Expected TypeRef"),
        }

        let result = parse_test!(cwt_identifier, "enum[test123]");
        match result.identifier_type {
            CwtReferenceType::Enum => {
                assert_eq!(result.name.raw_value(), "test123");
            }
            _ => panic!("Expected Enum"),
        }
    }

    #[test]
    fn test_cwt_value_alias_match_left() {
        // Test that cwt_value correctly chooses identifier parser for alias_match_left[trigger]
        let result = parse_test!(cwt_value, "alias_match_left[trigger]");
        assert!(result.is_identifier());
        if let CwtValue::Identifier(identifier) = result {
            match identifier.identifier_type {
                CwtReferenceType::AliasMatchLeft => {
                    assert_eq!(identifier.name.raw_value(), "trigger");
                }
                _ => panic!("Expected AliasMatchLeft type"),
            }
        }
    }

    #[test]
    fn range_decimals() {
        let result = parse_test!(cwt_value, "float[0.0..255.0]");
        assert!(result.is_simple());
        if let CwtValue::Simple(simple) = result {
            assert_eq!(simple.value_type, CwtSimpleValueType::Float);
            assert!(simple.range.is_some());
            let range = simple.range.unwrap();
            match (&range.min, &range.max) {
                (CwtRangeBound::Float(min), CwtRangeBound::Float(max)) => {
                    assert_eq!(*min, "0.0");
                    assert_eq!(*max, "255.0");
                }
                _ => panic!("Expected float range"),
            }
        }
    }
}
