use std::ops::Range;

use winnow::{
    LocatingSlice, ModalResult, Parser,
    combinator::{alt, delimited, opt, repeat_till},
    error::StrContext,
    token::literal,
};

use crate::{
    AstComment, AstNode, AstString, opt_ws_and_comments, quoted_or_unquoted_string,
    unquoted_string, with_opt_trailing_ws,
};

use super::{
    CwtComment, CwtIdentifier, CwtRule, CwtValue, cwt_identifier, cwt_rule, cwt_value,
    get_cwt_comments, opt_cwt_ws_and_comments,
};

/// CWT entity types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CwtEntity<'a> {
    /// Regular rule: key = value
    Rule(CwtRule<'a>),
    /// Block entity: { ... }
    Block(CwtBlock<'a>),
    /// Standalone identifier: <identifier>
    Identifier(CwtIdentifier<'a>),
    /// A quoted or unquoted string by itself, for e.g. enum values
    String(AstString<'a>),
}

/// Type definition in CWT
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CwtTypeDefinition<'a> {
    pub name: AstString<'a>,
    pub config: CwtTypeConfig<'a>,
    pub span: Range<usize>,
    pub leading_comments: Vec<CwtComment<'a>>,
}

/// Type configuration options
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CwtTypeConfig<'a> {
    pub path: Option<&'a str>,
    pub name_field: Option<&'a str>,
    pub skip_root_key: Vec<&'a str>,
    pub path_strict: bool,
    pub path_file: Option<&'a str>,
    pub path_extension: Option<&'a str>,
    pub type_per_file: bool,
    pub starts_with: Option<&'a str>,
    pub severity: Option<&'a str>,
    pub unique: bool,
    pub subtypes: Vec<CwtSubtype<'a>>,
    pub localisation: Vec<CwtLocalisation<'a>>,
    pub modifiers: Vec<CwtModifier<'a>>,
    pub span: Range<usize>,
}

/// Subtype definition
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CwtSubtype<'a> {
    pub name: &'a str,
    pub conditions: Vec<CwtRule<'a>>,
    pub span: Range<usize>,
}

/// Localisation requirement
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CwtLocalisation<'a> {
    pub name: &'a str,
    pub pattern: &'a str,
    pub required: bool,
    pub primary: bool,
    pub span: Range<usize>,
}

/// Modifier definition
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CwtModifier<'a> {
    pub pattern: &'a str,
    pub category: &'a str,
    pub span: Range<usize>,
}

/// Enum definition
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CwtEnumDefinition<'a> {
    pub name: AstString<'a>,
    pub content: CwtValue<'a>,
    pub span: Range<usize>,
    pub leading_comments: Vec<CwtComment<'a>>,
}

/// Complex enum definition
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CwtComplexEnumDefinition<'a> {
    pub name: AstString<'a>,
    pub path: AstString<'a>,
    pub name_structure: CwtComplexEnumNameStructure<'a>,
    pub start_from_root: bool,
    pub span: Range<usize>,
    pub leading_comments: Vec<CwtComment<'a>>,
}

/// Name structure for complex enums
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CwtComplexEnumNameStructure<'a> {
    /// Direct scalar mapping
    Scalar(AstString<'a>),
    /// Nested structure
    Nested {
        path: Vec<AstString<'a>>,
        target: AstString<'a>,
    },
}

/// Alias definition
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CwtAliasDefinition<'a> {
    pub category_and_name: AstString<'a>,
    pub category: &'a str,
    pub name: &'a str,
    pub definition: CwtValue<'a>,
    pub span: Range<usize>,
    pub leading_comments: Vec<CwtComment<'a>>,
}

/// Single alias definition
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CwtSingleAliasDefinition<'a> {
    pub name: AstString<'a>,
    pub definition: CwtBlock<'a>,
    pub span: Range<usize>,
    pub leading_comments: Vec<CwtComment<'a>>,
}

/// Subtype definition
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CwtSubtypeDefinition<'a> {
    pub is_not: bool,
    pub name: AstString<'a>,
    pub conditions: CwtBlock<'a>,
    pub span: Range<usize>,
    pub leading_comments: Vec<CwtComment<'a>>,
}

/// Block entity containing rules and sub-entities
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CwtBlock<'a> {
    pub items: Vec<CwtEntity<'a>>,
    pub span: Range<usize>,
    pub leading_comments: Vec<CwtComment<'a>>,
    pub trailing_comments: Vec<CwtComment<'a>>,
}

impl<'a> CwtBlock<'a> {
    pub fn new(span: Range<usize>) -> Self {
        Self {
            items: Vec::new(),
            span,
            leading_comments: Vec::new(),
            trailing_comments: Vec::new(),
        }
    }

    pub fn with_item(mut self, item: CwtEntity<'a>) -> Self {
        self.items.push(item);
        self
    }

    pub fn with_leading_comment(mut self, comment: CwtComment<'a>) -> Self {
        self.leading_comments.push(comment);
        self
    }

    pub fn with_trailing_comment(mut self, comment: CwtComment<'a>) -> Self {
        self.trailing_comments.push(comment);
        self
    }

    /// Find all rules with the given key name
    pub fn find_rules(&self, key: &str) -> Vec<&CwtRule<'a>> {
        self.items
            .iter()
            .filter_map(|item| match item {
                CwtEntity::Rule(rule) if rule.key.name() == key => Some(rule),
                _ => None,
            })
            .collect()
    }

    /// Find the first rule with the given key name
    pub fn find_rule(&self, key: &str) -> Option<&CwtRule<'a>> {
        self.items.iter().find_map(|item| match item {
            CwtEntity::Rule(rule) if rule.key.name() == key => Some(rule),
            _ => None,
        })
    }

    /// Get all rules in the block
    pub fn rules(&self) -> impl Iterator<Item = &CwtRule<'a>> {
        self.items.iter().filter_map(|item| match item {
            CwtEntity::Rule(rule) => Some(rule),
            _ => None,
        })
    }

    /// Check if the block is empty
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Get the number of items in the block
    pub fn len(&self) -> usize {
        self.items.len()
    }
}

impl<'a> AstNode<'a> for CwtEntity<'a> {
    fn span_range(&self) -> Range<usize> {
        match self {
            CwtEntity::Rule(rule) => rule.span.clone(),
            CwtEntity::Block(block) => block.span.clone(),
            CwtEntity::Identifier(identifier) => identifier.span.clone(),
            CwtEntity::String(string) => string.span_range(),
        }
    }

    fn leading_comments(&self) -> &[AstComment<'a>] {
        // CWT comments don't map directly to AstComment
        // This is a design limitation - we'd need to convert CwtComment to AstComment
        &[]
    }

    fn trailing_comment(&self) -> Option<&AstComment<'a>> {
        // CWT comments don't map directly to AstComment
        // This is a design limitation - we'd need to convert CwtComment to AstComment
        None
    }
}

impl<'a> AstNode<'a> for CwtBlock<'a> {
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

/// Parse a CWT block
pub(crate) fn cwt_block<'a>(input: &mut LocatingSlice<&'a str>) -> ModalResult<CwtBlock<'a>> {
    let ((_, (items, _)), span) = (
        "{",
        repeat_till(0.., cwt_entity, (opt_ws_and_comments, "}")),
    )
        .with_span()
        .context(StrContext::Label("cwt_block"))
        .parse_next(input)?;

    Ok(CwtBlock {
        items,
        span,
        leading_comments: Vec::new(), // Will be populated by cwt_entity if this is a standalone block
        trailing_comments: Vec::new(),
    })
}

/// Parse a CWT entity
pub(crate) fn cwt_entity<'a>(input: &mut LocatingSlice<&'a str>) -> ModalResult<CwtEntity<'a>> {
    let leading_comments_data = opt_cwt_ws_and_comments.parse_next(input)?;
    let leading_comments = get_cwt_comments(&leading_comments_data);

    let mut entity = alt((
        cwt_block.map(CwtEntity::Block),
        cwt_rule.map(CwtEntity::Rule),
        cwt_identifier.map(CwtEntity::Identifier),
        quoted_or_unquoted_string.map(CwtEntity::String),
    ))
    .context(StrContext::Label("cwt_entity"))
    .parse_next(input)?;

    // Attach leading comments to the entity
    match &mut entity {
        CwtEntity::Block(block) => {
            block.leading_comments.extend(leading_comments);
        }
        CwtEntity::Rule(rule) => {
            // For rules, we store the first comment as documentation
            if let Some(first_comment) = leading_comments.into_iter().next() {
                rule.documentation = Some(first_comment);
            }
        }
        CwtEntity::Identifier(identifier) => {
            identifier.leading_comments = leading_comments;
        }
        CwtEntity::String(string) => {
            // TODO: Handle leading comments for strings
        }
    }

    Ok(entity)
}

/// Parse a type definition: type[name] = { ... }
pub(crate) fn cwt_type_definition<'a>(
    input: &mut LocatingSlice<&'a str>,
) -> ModalResult<CwtTypeDefinition<'a>> {
    let ((name, _, config), span) = (
        with_opt_trailing_ws(delimited("type[", quoted_or_unquoted_string, "]")),
        with_opt_trailing_ws(literal("=")),
        cwt_block,
    )
        .with_span()
        .context(StrContext::Label("cwt_type_definition"))
        .parse_next(input)?;

    Ok(CwtTypeDefinition {
        name,
        config: CwtTypeConfig {
            path: None,
            name_field: None,
            skip_root_key: Vec::new(),
            path_strict: false,
            path_file: None,
            path_extension: None,
            type_per_file: false,
            starts_with: None,
            severity: None,
            unique: false,
            subtypes: Vec::new(),
            localisation: Vec::new(),
            modifiers: Vec::new(),
            span: config.span.clone(),
        },
        span,
        leading_comments: Vec::new(), // Will be populated by cwt_entity
    })
}

/// Parse an enum definition: enum[name] = { ... }
pub(crate) fn cwt_enum_definition<'a>(
    input: &mut LocatingSlice<&'a str>,
) -> ModalResult<CwtEnumDefinition<'a>> {
    let ((name, _, content), span) = (
        with_opt_trailing_ws(delimited("enum[", quoted_or_unquoted_string, "]")),
        with_opt_trailing_ws(literal("=")),
        cwt_value,
    )
        .with_span()
        .context(StrContext::Label("cwt_enum_definition"))
        .parse_next(input)?;

    Ok(CwtEnumDefinition {
        name: name,
        content,
        span,
        leading_comments: Vec::new(), // Will be populated by cwt_entity
    })
}

/// Parse a complex enum definition: complex_enum[name] = { ... }
pub(crate) fn cwt_complex_enum_definition<'a>(
    input: &mut LocatingSlice<&'a str>,
) -> ModalResult<CwtComplexEnumDefinition<'a>> {
    let ((name, _, _config), span) = (
        with_opt_trailing_ws(delimited("complex_enum[", quoted_or_unquoted_string, "]")),
        with_opt_trailing_ws(literal("=")),
        cwt_block,
    )
        .with_span()
        .context(StrContext::Label("cwt_complex_enum_definition"))
        .parse_next(input)?;

    Ok(CwtComplexEnumDefinition {
        name,
        path: AstString::new("", false, 0..0), // TODO: Parse from config
        name_structure: CwtComplexEnumNameStructure::Scalar(AstString::new("", false, 0..0)),
        start_from_root: false,
        span,
        leading_comments: Vec::new(), // Will be populated by cwt_entity
    })
}
/// Parse an alias definition: alias[category:name] = { ... } or alias[category:name] = value
pub(crate) fn cwt_alias_definition<'a>(
    input: &mut LocatingSlice<&'a str>,
) -> ModalResult<CwtAliasDefinition<'a>> {
    let ((category_and_name, _, definition), span) = (
        with_opt_trailing_ws(delimited(
            with_opt_trailing_ws("alias["),
            with_opt_trailing_ws(unquoted_string),
            "]",
        )),
        with_opt_trailing_ws(literal("=")),
        cwt_value,
    )
        .with_span()
        .context(StrContext::Label("cwt_alias_definition"))
        .parse_next(input)?;

    let (category, name) = category_and_name.raw_value().split_once(':').unwrap();

    Ok(CwtAliasDefinition {
        category_and_name,
        category,
        name,
        definition,
        span,
        leading_comments: Vec::new(), // Will be populated by cwt_entity
    })
}

/// Parse a single alias definition: single_alias[name] = { ... }
pub(crate) fn cwt_single_alias_definition<'a>(
    input: &mut LocatingSlice<&'a str>,
) -> ModalResult<CwtSingleAliasDefinition<'a>> {
    let ((name, _, definition), span) = (
        with_opt_trailing_ws(delimited("single_alias[", quoted_or_unquoted_string, "]")),
        with_opt_trailing_ws(literal("=")),
        cwt_block,
    )
        .with_span()
        .context(StrContext::Label("cwt_single_alias_definition"))
        .parse_next(input)?;

    Ok(CwtSingleAliasDefinition {
        name,
        definition,
        span,
        leading_comments: Vec::new(), // Will be populated by cwt_entity
    })
}

/// Parse a subtype definition: subtype[name] = { ... }
pub(crate) fn cwt_subtype_definition<'a>(
    input: &mut LocatingSlice<&'a str>,
) -> ModalResult<CwtSubtypeDefinition<'a>> {
    let (((is_not, name), _, conditions), span) = (
        with_opt_trailing_ws(delimited(
            "subtype[",
            (opt(literal("!")), quoted_or_unquoted_string),
            "]",
        )),
        with_opt_trailing_ws(literal("=")),
        cwt_block,
    )
        .with_span()
        .context(StrContext::Label("cwt_subtype_definition"))
        .parse_next(input)?;

    Ok(CwtSubtypeDefinition {
        is_not: is_not.is_some(),
        name,
        conditions,
        span,
        leading_comments: Vec::new(), // Will be populated by cwt_entity
    })
}

#[cfg(test)]
mod tests {
    use crate::{AstString, CwtCommentType, CwtOperator, CwtReferenceType, CwtValue};

    use super::*;
    use pretty_assertions::assert_eq;
    use winnow::{LocatingSlice, Parser};

    #[test]
    fn test_cwt_block_empty() {
        let mut input = LocatingSlice::new("{}");
        let result = cwt_block.parse_next(&mut input).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_cwt_type_definition() {
        let mut input = LocatingSlice::new("type[test] = {}");
        let result = cwt_type_definition.parse_next(&mut input).unwrap();
        assert_eq!(result.name.raw_value(), "test");
    }

    #[test]
    fn test_cwt_enum_definition() {
        let mut input = LocatingSlice::new("enum[test] = { value1 value2 }");
        let result = cwt_enum_definition.parse_next(&mut input).unwrap();
        assert_eq!(result.name.raw_value(), "test");
        // Check that the content is a block with the expected values
        if let CwtValue::Block(block) = &result.content {
            assert_eq!(block.items.len(), 2);
            let identifiers: Vec<_> = block
                .items
                .iter()
                .filter_map(|item| match item {
                    CwtEntity::String(id) => Some(id.raw_value()),
                    _ => None,
                })
                .collect();
            assert_eq!(identifiers.len(), 2);
            assert_eq!(identifiers[0], "value1");
            assert_eq!(identifiers[1], "value2");
        } else {
            panic!("Expected block content");
        }
    }

    #[test]
    fn test_cwt_enum_definition_single_value() {
        let mut input = LocatingSlice::new("enum[weight_or_base] = float");
        let result = cwt_enum_definition.parse_next(&mut input).unwrap();
        assert_eq!(result.name.raw_value(), "weight_or_base");
        // Single value can be parsed as different types depending on the value
        match &result.content {
            CwtValue::Simple(simple_value) => {
                // "float" is recognized as a simple value type
                assert_eq!(
                    simple_value.value_type,
                    crate::cwt::CwtSimpleValueType::Float
                );
            }
            CwtValue::Block(block) => {
                // Single values might be parsed as a block with one identifier
                assert_eq!(block.len(), 1);
                if let CwtEntity::Identifier(id) = &block.items[0] {
                    assert_eq!(id.name.raw_value(), "float");
                } else {
                    panic!("Expected identifier in block");
                }
            }
            other => {
                panic!("Unexpected content type: {:?}", other);
            }
        }
    }

    #[test]
    fn test_cwt_alias_definition() {
        let mut input = LocatingSlice::new("alias[effect:test] = {}");
        let result = cwt_alias_definition.parse_next(&mut input).unwrap();
        assert_eq!(result.category, "effect");
        assert_eq!(result.name, "test");
    }

    #[test]
    fn test_cwt_single_alias_definition() {
        let mut input = LocatingSlice::new("single_alias[test] = {}");
        let result = cwt_single_alias_definition.parse_next(&mut input).unwrap();
        assert_eq!(result.name.raw_value(), "test");
    }

    #[test]
    fn test_cwt_block_with_multiple_rules() {
        let mut input = LocatingSlice::new("{ key1 = value1 key2 = value2 key3 = value3 }");
        let result = cwt_block.parse_next(&mut input).unwrap();

        assert_eq!(result.len(), 3);
        assert!(!result.is_empty());

        let rules: Vec<_> = result.rules().collect();
        assert_eq!(rules.len(), 3);
        assert_eq!(rules[0].key.name(), "key1");
        assert_eq!(rules[1].key.name(), "key2");
        assert_eq!(rules[2].key.name(), "key3");
    }

    #[test]
    fn test_cwt_block_with_nested_blocks() {
        let mut input = LocatingSlice::new("{ outer_key = value { inner_key = inner_value } }");
        let result = cwt_block.parse_next(&mut input).unwrap();

        assert_eq!(result.len(), 2);

        // Check that we have one rule and one nested block
        let rules: Vec<_> = result.rules().collect();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].key.name(), "outer_key");

        // The second item should be a nested block
        match &result.items[1] {
            CwtEntity::Block(nested_block) => {
                assert_eq!(nested_block.len(), 1);
                let nested_rules: Vec<_> = nested_block.rules().collect();
                assert_eq!(nested_rules.len(), 1);
                assert_eq!(nested_rules[0].key.name(), "inner_key");
            }
            _ => panic!("Expected a nested block"),
        }
    }

    #[test]
    fn test_cwt_block_with_comments() {
        let mut block = CwtBlock::new(0..10);
        let comment = CwtComment::new("# Test comment", CwtCommentType::Regular, 0..14);

        block = block
            .with_leading_comment(comment.clone())
            .with_trailing_comment(comment);

        assert_eq!(block.leading_comments.len(), 1);
        assert_eq!(block.trailing_comments.len(), 1);
    }
}
