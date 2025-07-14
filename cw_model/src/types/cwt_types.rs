//! CWT Type System
//!
//! This module provides a direct representation of CWT types and definitions,
//! closely aligned with the CWT specification rather than inferred types.

use crate::{SeverityLevel, TypeKeyFilter};
use cw_parser::{AstCwtRule, CwtCommentRangeBound};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Trait for generating unique fingerprints for types to enable deduplication
pub trait TypeFingerprint {
    /// Generate a unique fingerprint string for this type
    fn fingerprint(&self) -> String;
}

/// The main CWT type system - directly represents CWT type concepts
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CwtType {
    /// Unknown type
    Unknown,

    /// Simple primitive types (bool, int, float, scalar, etc.)
    Simple(SimpleType),

    /// Reference types (<type>, enum[key], scope[key], etc.)
    Reference(ReferenceType),

    /// Block/object types with properties
    Block(BlockType),

    /// Array types
    Array(ArrayType),

    /// Union types (multiple alternatives)
    Union(Vec<CwtType>),

    /// Literal string values
    Literal(String),

    /// Set of literal values
    LiteralSet(HashSet<String>),

    /// Comparable types (for triggers with == operator)
    Comparable(Box<CwtType>),
}

impl CwtType {
    pub fn reference_id(&self) -> Option<String> {
        match self {
            CwtType::Reference(ref_type) => Some(ref_type.id()),
            _ => None,
        }
    }
}

/// Simple CWT primitive types - directly maps to CWT simple values
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SimpleType {
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
    Color,
    Maths,
}

impl SimpleType {
    pub fn id(&self) -> &'static str {
        match self {
            SimpleType::Bool => "bool",
            SimpleType::Int => "int",
            SimpleType::Float => "float",
            SimpleType::Scalar => "scalar",
            SimpleType::PercentageField => "percentage_field",
            SimpleType::Localisation => "localisation",
            SimpleType::LocalisationSynced => "localisation_synced",
            SimpleType::LocalisationInline => "localisation_inline",
            SimpleType::DateField => "date_field",
            SimpleType::VariableField => "variable_field",
            SimpleType::IntVariableField => "int_variable_field",
            SimpleType::ValueField => "value_field",
            SimpleType::IntValueField => "int_value_field",
            SimpleType::ScopeField => "scope_field",
            SimpleType::Filepath => "filepath",
            SimpleType::Icon => "icon",
            SimpleType::Color => "color",
            SimpleType::Maths => "maths",
        }
    }
}

/// CWT reference types - directly maps to CWT reference syntax
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ReferenceType {
    /// Type reference: <type_key>
    Type { key: String },

    /// Type reference with prefix/suffix: prefix_<type_key>_suffix
    TypeWithAffix {
        key: String,
        prefix: Option<String>,
        suffix: Option<String>,
    },

    /// Enum reference: enum[key]
    Enum { key: String },

    /// Complex enum reference: complex_enum[key]
    ComplexEnum { key: String },

    /// Scope reference: scope[key]
    Scope { key: String },

    /// Scope group reference: scope_group[key]
    ScopeGroup { key: String },

    /// Alias reference: alias[key]
    Alias { key: String },

    /// Alias name reference: alias_name[key]
    AliasName { key: String },

    /// Alias match left reference: alias_match_left[key]
    AliasMatchLeft { key: String },

    /// Single alias reference: single_alias_right[key]
    SingleAlias { key: String },

    /// Alias keys field: alias_keys_field[key]
    AliasKeysField { key: String },

    /// Value reference: value[key]
    Value { key: String },

    /// Value set reference: value_set[key]
    ValueSet { key: String },

    /// Icon reference: icon[path]
    Icon { path: String },

    /// Filepath reference: filepath[path]
    Filepath { path: String },

    /// Colour reference: colour[format]
    Colour { format: String },

    /// Stellaris name format reference: stellaris_name_format[key]
    StellarisNameFormat { key: String },

    /// Subtype reference: subtype[name]
    Subtype { name: String },
}

impl ReferenceType {
    pub fn id(&self) -> String {
        match self {
            ReferenceType::Type { key } => key.clone(),
            ReferenceType::TypeWithAffix {
                key,
                prefix,
                suffix,
            } => format!(
                "{}<{}>{}",
                prefix.as_ref().unwrap_or(&"".to_string()),
                key,
                suffix.as_ref().unwrap_or(&"".to_string())
            ),
            ReferenceType::Enum { key } => format!("enum[{}]", key),
            ReferenceType::ComplexEnum { key } => format!("complex_enum[{}]", key),
            ReferenceType::Scope { key } => format!("scope[{}]", key),
            ReferenceType::ScopeGroup { key } => format!("scope_group[{}]", key),
            ReferenceType::Alias { key } => format!("alias[{}]", key),
            ReferenceType::AliasName { key } => format!("alias_name[{}]", key),
            ReferenceType::AliasMatchLeft { key } => format!("alias_match_left[{}]", key),
            ReferenceType::SingleAlias { key } => format!("single_alias[{}]", key),
            ReferenceType::AliasKeysField { key } => format!("alias_keys_field[{}]", key),
            ReferenceType::Value { key } => format!("value[{}]", key),
            ReferenceType::ValueSet { key } => format!("value_set[{}]", key),
            ReferenceType::Icon { path } => format!("icon[{}]", path),
            ReferenceType::Filepath { path } => format!("filepath[{}]", path),
            ReferenceType::Colour { format } => format!("colour[{}]", format),
            ReferenceType::StellarisNameFormat { key } => format!("stellaris_name_format[{}]", key),
            ReferenceType::Subtype { name } => format!("subtype[{}]", name),
        }
    }
}

/// Block/object types with properties and subtypes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockType {
    /// Regular properties
    pub properties: HashMap<String, Property>,

    /// Subtypes - conditional property sets
    pub subtypes: HashMap<String, Subtype>,

    /// Subtype properties - subtype_name -> property_name -> Property
    pub subtype_properties: HashMap<String, HashMap<String, Property>>,

    /// Pattern properties - properties that can match multiple keys but maintain unified cardinality
    pub pattern_properties: Vec<PatternProperty>,

    /// Localisation requirements
    pub localisation: Option<LocalisationSpec>,

    /// Modifier generation rules
    pub modifiers: Option<ModifierSpec>,

    /// Additional flags, like an array
    pub additional_flags: Vec<CwtType>,
}

/// A property that can match multiple keys using patterns
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatternProperty {
    /// Type of pattern
    pub pattern_type: PatternType,

    /// Value type for matching keys
    pub value_type: CwtType,

    /// CWT options/directives (includes cardinality, range, etc.)
    pub options: CwtOptions,

    /// Documentation
    pub documentation: Option<String>,
}

/// Types of patterns that can match multiple keys
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PatternType {
    /// alias_name[category] - matches any alias name from the category
    AliasName { category: String },

    /// enum[key] - matches any enum value from the key
    Enum { key: String },

    /// <type_key> - matches any type from the key
    Type { key: String },
}

impl PatternType {
    pub fn id(&self) -> String {
        match self {
            PatternType::AliasName { category } => format!("alias_name[{}]", category),
            PatternType::Enum { key } => format!("enum[{}]", key),
            PatternType::Type { key } => format!("<{}>", key),
        }
    }
}

/// Aliases can be defined in (at least) two ways:
/// 1. alias[foo:x] = bar
/// 2. alias[foo:<type_name>] = bar
///
/// The first is a simple alias, the second is a dynamic alias.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AliasPattern {
    /// Full text for hashing, e.g. "foo:<type_name>" or "foo:x"
    pub full_text: String,

    /// Category of the alias, e.g. "foo"
    pub category: String,

    /// Name of the alias, either a static name or a dynamic name
    pub name: AliasName,
}

impl AliasPattern {
    pub fn new_basic(category: &str, name: &str) -> Self {
        Self {
            full_text: format!("{}:{}", category, name),
            category: category.to_string(),
            name: AliasName::Static(name.to_string()),
        }
    }

    pub fn new_type_ref(category: &str, name: &str) -> Self {
        Self {
            full_text: format!("{}:{}", category, name),
            category: category.to_string(),
            name: AliasName::TypeRef(name.to_string()),
        }
    }

    pub fn new_enum(category: &str, name: &str) -> Self {
        Self {
            full_text: format!("{}:{}", category, name),
            category: category.to_string(),
            name: AliasName::Enum(name.to_string()),
        }
    }
}

impl std::fmt::Display for AliasPattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.full_text)
    }
}

impl std::hash::Hash for AliasPattern {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.full_text.hash(state);
    }
}

impl PartialEq for AliasPattern {
    fn eq(&self, other: &Self) -> bool {
        self.full_text == other.full_text
    }
}

impl Eq for AliasPattern {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AliasName {
    Static(String),
    TypeRef(String),
    Enum(String),
}

/// A property in a block type
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Property {
    /// Type of this property
    pub property_type: CwtType,

    /// CWT options/directives (includes cardinality, range, etc.)
    pub options: CwtOptions,

    /// Documentation
    pub documentation: Option<String>,
}

/// Subtype definition - properties that apply under certain conditions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Subtype {
    /// Condition that activates this subtype
    pub condition: SubtypeCondition,

    /// CWT schema condition properties with cardinality constraints
    /// These define the rules for when this subtype matches (e.g., is_origin = no with cardinality 0..1)
    pub condition_properties: HashMap<String, Property>,

    /// Game data properties that are allowed when this subtype is active
    /// These are discovered from analyzing actual game files (e.g., traits, playable, etc.)
    pub allowed_properties: HashMap<String, Property>,

    /// Options for this subtype
    pub options: CwtOptions,

    /// Whether this subtype is inverted (e.g., !hidden)
    pub is_inverted: bool,
}

/// Conditions for subtype activation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SubtypeCondition {
    /// Property equals specific value
    PropertyEquals { key: String, value: String },

    /// Property does not equal specific value
    PropertyNotEquals { key: String, value: String },

    /// Property exists
    PropertyExists { key: String },

    /// Property does not exist
    PropertyNotExists { key: String },

    /// Type key starts with prefix
    KeyStartsWith { prefix: String },

    /// Type key matches filter
    KeyMatches { filter: String },

    /// Complex expression
    Expression(String),
}

/// Array type
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArrayType {
    /// Element type
    pub element_type: Box<CwtType>,
}

/// Cardinality constraints
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cardinality {
    /// Minimum occurrences
    pub min: Option<u32>,
    /// Maximum occurrences (None = unlimited)
    pub max: Option<u32>,
    /// Soft constraint (prefixed with ~)
    pub soft: bool,
}

/// Range constraints for numeric types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Range {
    /// Minimum value
    pub min: RangeBound,
    /// Maximum value
    pub max: RangeBound,
}

/// Range boundary values
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RangeBound {
    Integer(i64),
    Float(f64),
    NegInfinity,
    PosInfinity,
}

/// CWT options/directives that can apply to any type
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CwtOptions {
    /// Required field
    pub required: bool,
    /// Primary field
    pub primary: bool,
    /// Optional field (explicit)
    pub optional: bool,
    /// Severity level
    pub severity: Option<SeverityLevel>,
    /// Display name
    pub display_name: Option<String>,
    /// Abbreviation
    pub abbreviation: Option<String>,
    /// Starts with constraint
    pub starts_with: Option<String>,
    /// Push scope
    pub push_scope: Option<String>,
    /// Replace scope mappings
    pub replace_scope: Option<HashMap<String, String>>,
    /// Scope constraint
    pub scope: Option<Vec<String>>,
    /// Type key filter
    pub type_key_filter: Option<TypeKeyFilter>,
    /// Graph related types
    pub graph_related_types: Option<Vec<String>>,
    /// Unique constraint
    pub unique: bool,
    /// Skip root key configurations
    pub skip_root_key: Option<Vec<String>>,
    /// Path constraints
    pub path_strict: bool,
    pub path_file: Option<String>,
    pub path_extension: Option<String>,
    /// Type per file
    pub type_per_file: bool,
    /// Range constraints (for numeric types)
    pub range: Option<Range>,
    /// Cardinality constraints
    pub cardinality: Option<Cardinality>,
}

/// Localisation specification
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocalisationSpec {
    /// Required localisation keys
    pub required: HashMap<String, String>,
    /// Optional localisation keys
    pub optional: HashMap<String, String>,
    /// Primary localisation key
    pub primary: Option<String>,
    /// Subtype-specific localisation
    pub subtypes: HashMap<String, HashMap<String, String>>,
}

/// Modifier generation specification
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModifierSpec {
    /// Modifier patterns
    pub modifiers: HashMap<String, String>,
    /// Subtype-specific modifiers
    pub subtypes: HashMap<String, HashMap<String, String>>,
}

/// CWT type definition - top-level type in the registry
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CwtTypeDefinition {
    /// Type name/key
    pub name: String,
    /// Type specification
    pub type_spec: CwtType,
    /// Path configuration
    pub path: Option<String>,
    /// Name field override
    pub name_field: Option<String>,
    /// Skip root key configuration
    pub skip_root_key: Option<SkipRootKeySpec>,
    /// Type-level options
    pub options: CwtOptions,
}

/// Skip root key specification
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SkipRootKeySpec {
    /// Skip specific key
    Specific(String),
    /// Skip any key
    Any,
    /// Skip except specific keys
    Except(Vec<String>),
    /// Skip multiple levels
    Multiple(Vec<String>),
}

/// CWT enum definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CwtEnumDefinition {
    /// Enum name/key
    pub name: String,
    /// Simple enum values
    pub values: HashSet<String>,
    /// Complex enum configuration
    pub complex: Option<ComplexEnumSpec>,
}

/// Complex enum specification
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComplexEnumSpec {
    /// Path to scan
    pub path: String,
    /// Name extraction structure
    pub name_structure: CwtType,
    /// Start from file root
    pub start_from_root: bool,
}

/// CWT alias definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CwtAliasDefinition {
    /// Alias name/key
    pub name: String,
    /// Alias category
    pub category: String,
    /// Alias type specification
    pub type_spec: CwtType,
}

/// CWT value set definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CwtValueSetDefinition {
    /// Value set name/key
    pub name: String,
    /// Set of values
    pub values: HashSet<String>,
}

// TypeFingerprint implementations
impl TypeFingerprint for CwtType {
    fn fingerprint(&self) -> String {
        match self {
            CwtType::Unknown => "unknown".to_string(),
            CwtType::Simple(simple) => format!("simple:{}", simple.fingerprint()),
            CwtType::Reference(reference) => format!("reference:{}", reference.fingerprint()),
            CwtType::Block(block) => format!("block:{}", block.fingerprint()),
            CwtType::Array(array) => format!("array:{}", array.fingerprint()),
            CwtType::Union(types) => {
                let mut type_fingerprints: Vec<String> =
                    types.iter().map(|t| t.fingerprint()).collect();
                type_fingerprints.sort();
                format!("union:{}", type_fingerprints.join("|"))
            }
            CwtType::Literal(value) => format!("literal:{}", value),
            CwtType::LiteralSet(values) => {
                let mut sorted_values: Vec<String> = values.iter().cloned().collect();
                sorted_values.sort();
                format!("literal_set:{}", sorted_values.join("|"))
            }
            CwtType::Comparable(base_type) => format!("comparable:{}", base_type.fingerprint()),
        }
    }
}

impl TypeFingerprint for SimpleType {
    fn fingerprint(&self) -> String {
        match self {
            SimpleType::Bool => "bool".to_string(),
            SimpleType::Int => "int".to_string(),
            SimpleType::Float => "float".to_string(),
            SimpleType::Scalar => "scalar".to_string(),
            SimpleType::PercentageField => "percentage_field".to_string(),
            SimpleType::Localisation => "localisation".to_string(),
            SimpleType::LocalisationSynced => "localisation_synced".to_string(),
            SimpleType::LocalisationInline => "localisation_inline".to_string(),
            SimpleType::DateField => "date_field".to_string(),
            SimpleType::VariableField => "variable_field".to_string(),
            SimpleType::IntVariableField => "int_variable_field".to_string(),
            SimpleType::ValueField => "value_field".to_string(),
            SimpleType::IntValueField => "int_value_field".to_string(),
            SimpleType::ScopeField => "scope_field".to_string(),
            SimpleType::Filepath => "filepath".to_string(),
            SimpleType::Icon => "icon".to_string(),
            SimpleType::Color => "color".to_string(),
            SimpleType::Maths => "maths".to_string(),
        }
    }
}

impl TypeFingerprint for ReferenceType {
    fn fingerprint(&self) -> String {
        match self {
            ReferenceType::Type { key } => format!("type:{}", key),
            ReferenceType::TypeWithAffix {
                key,
                prefix,
                suffix,
            } => {
                format!(
                    "type_with_affix:{}:{}:{}",
                    prefix.as_deref().unwrap_or(""),
                    key,
                    suffix.as_deref().unwrap_or("")
                )
            }
            ReferenceType::Enum { key } => format!("enum:{}", key),
            ReferenceType::ComplexEnum { key } => format!("complex_enum:{}", key),
            ReferenceType::Scope { key } => format!("scope:{}", key),
            ReferenceType::ScopeGroup { key } => format!("scope_group:{}", key),
            ReferenceType::Alias { key } => format!("alias:{}", key),
            ReferenceType::AliasName { key } => format!("alias_name:{}", key),
            ReferenceType::AliasMatchLeft { key } => format!("alias_match_left:{}", key),
            ReferenceType::SingleAlias { key } => format!("single_alias:{}", key),
            ReferenceType::AliasKeysField { key } => format!("alias_keys_field:{}", key),
            ReferenceType::Value { key } => format!("value:{}", key),
            ReferenceType::ValueSet { key } => format!("value_set:{}", key),
            ReferenceType::Icon { path } => format!("icon:{}", path),
            ReferenceType::Filepath { path } => format!("filepath:{}", path),
            ReferenceType::Colour { format } => format!("colour:{}", format),
            ReferenceType::StellarisNameFormat { key } => format!("stellaris_name_format:{}", key),
            ReferenceType::Subtype { name } => format!("subtype:{}", name),
        }
    }
}

impl TypeFingerprint for BlockType {
    fn fingerprint(&self) -> String {
        let mut parts = Vec::new();

        // Properties
        if !self.properties.is_empty() {
            let mut prop_fingerprints: Vec<String> = self
                .properties
                .iter()
                .map(|(k, v)| format!("{}:{}", k, v.fingerprint()))
                .collect();
            prop_fingerprints.sort();
            parts.push(format!("props:{}", prop_fingerprints.join(",")));
        }

        // Subtypes
        if !self.subtypes.is_empty() {
            let mut subtype_fingerprints: Vec<String> = self
                .subtypes
                .iter()
                .map(|(k, v)| format!("{}:{}", k, v.fingerprint()))
                .collect();
            subtype_fingerprints.sort();
            parts.push(format!("subtypes:{}", subtype_fingerprints.join(",")));
        }

        // Pattern properties
        if !self.pattern_properties.is_empty() {
            let mut pattern_prop_fingerprints: Vec<String> = self
                .pattern_properties
                .iter()
                .map(|p| {
                    format!(
                        "{}:{}",
                        p.pattern_type.fingerprint(),
                        p.value_type.fingerprint()
                    )
                })
                .collect();
            pattern_prop_fingerprints.sort();
            parts.push(format!(
                "pattern_props:{}",
                pattern_prop_fingerprints.join(",")
            ));
        }

        // Localisation
        if let Some(loc) = &self.localisation {
            parts.push(format!("localisation:{}", loc.fingerprint()));
        }

        // Modifiers
        if let Some(mod_spec) = &self.modifiers {
            parts.push(format!("modifiers:{}", mod_spec.fingerprint()));
        }

        parts.join(";")
    }
}

impl TypeFingerprint for PatternProperty {
    fn fingerprint(&self) -> String {
        format!(
            "{}:{}",
            self.pattern_type.fingerprint(),
            self.value_type.fingerprint()
        )
    }
}

impl TypeFingerprint for PatternType {
    fn fingerprint(&self) -> String {
        match self {
            PatternType::AliasName { category } => format!("alias_name:{}", category),
            PatternType::Enum { key } => format!("enum:{}", key),
            PatternType::Type { key } => format!("<{}>", key),
        }
    }
}

impl TypeFingerprint for ArrayType {
    fn fingerprint(&self) -> String {
        self.element_type.fingerprint()
    }
}

impl TypeFingerprint for Property {
    fn fingerprint(&self) -> String {
        format!(
            "{}:{}",
            self.property_type.fingerprint(),
            self.options.fingerprint()
        )
    }
}

impl TypeFingerprint for Subtype {
    fn fingerprint(&self) -> String {
        let mut allowed_prop_fingerprints: Vec<String> = self
            .allowed_properties
            .iter()
            .map(|(k, v)| format!("{}:{}", k, v.fingerprint()))
            .collect();
        allowed_prop_fingerprints.sort();

        let mut condition_prop_fingerprints: Vec<String> = self
            .condition_properties
            .iter()
            .map(|(k, v)| format!("{}:{}", k, v.fingerprint()))
            .collect();
        condition_prop_fingerprints.sort();

        format!(
            "{}:{}:{}:{}",
            self.condition.fingerprint(),
            condition_prop_fingerprints.join(","),
            allowed_prop_fingerprints.join(","),
            self.options.fingerprint()
        )
    }
}

impl TypeFingerprint for SubtypeCondition {
    fn fingerprint(&self) -> String {
        match self {
            SubtypeCondition::PropertyEquals { key, value } => format!("prop_eq:{}:{}", key, value),
            SubtypeCondition::PropertyNotEquals { key, value } => {
                format!("prop_ne:{}:{}", key, value)
            }
            SubtypeCondition::PropertyExists { key } => format!("prop_exists:{}", key),
            SubtypeCondition::PropertyNotExists { key } => format!("prop_not_exists:{}", key),
            SubtypeCondition::KeyStartsWith { prefix } => format!("key_starts_with:{}", prefix),
            SubtypeCondition::KeyMatches { filter } => format!("key_matches:{}", filter),
            SubtypeCondition::Expression(expr) => format!("expr:{}", expr),
        }
    }
}

impl TypeFingerprint for Cardinality {
    fn fingerprint(&self) -> String {
        format!(
            "{}:{}:{}",
            self.min
                .map(|v| v.to_string())
                .unwrap_or_else(|| "none".to_string()),
            self.max
                .map(|v| v.to_string())
                .unwrap_or_else(|| "none".to_string()),
            if self.soft { "soft" } else { "hard" }
        )
    }
}

impl TypeFingerprint for Range {
    fn fingerprint(&self) -> String {
        format!("{}:{}", self.min.fingerprint(), self.max.fingerprint())
    }
}

impl TypeFingerprint for RangeBound {
    fn fingerprint(&self) -> String {
        match self {
            RangeBound::Integer(i) => format!("int:{}", i),
            RangeBound::Float(f) => format!("float:{}", f),
            RangeBound::NegInfinity => "neg_inf".to_string(),
            RangeBound::PosInfinity => "pos_inf".to_string(),
        }
    }
}

impl TypeFingerprint for CwtOptions {
    fn fingerprint(&self) -> String {
        let mut parts = Vec::new();

        if self.required {
            parts.push("required".to_string());
        }
        if self.primary {
            parts.push("primary".to_string());
        }
        if self.optional {
            parts.push("optional".to_string());
        }
        if self.unique {
            parts.push("unique".to_string());
        }
        if self.path_strict {
            parts.push("path_strict".to_string());
        }
        if self.type_per_file {
            parts.push("type_per_file".to_string());
        }

        if let Some(severity) = &self.severity {
            parts.push(format!("severity:{:?}", severity));
        }
        if let Some(display_name) = &self.display_name {
            parts.push(format!("display_name:{}", display_name));
        }
        if let Some(abbreviation) = &self.abbreviation {
            parts.push(format!("abbreviation:{}", abbreviation));
        }
        if let Some(starts_with) = &self.starts_with {
            parts.push(format!("starts_with:{}", starts_with));
        }
        if let Some(push_scope) = &self.push_scope {
            parts.push(format!("push_scope:{}", push_scope));
        }
        if let Some(replace_scope) = &self.replace_scope {
            let mut scope_parts: Vec<String> = replace_scope
                .iter()
                .map(|(k, v)| format!("{}:{}", k, v))
                .collect();
            scope_parts.sort();
            parts.push(format!("replace_scope:{}", scope_parts.join(",")));
        }
        if let Some(scope) = &self.scope {
            let mut scope_vec = scope.clone();
            scope_vec.sort();
            parts.push(format!("scope:{}", scope_vec.join(",")));
        }
        if let Some(type_key_filter) = &self.type_key_filter {
            parts.push(format!("type_key_filter:{:?}", type_key_filter));
        }
        if let Some(graph_related_types) = &self.graph_related_types {
            let mut grt = graph_related_types.clone();
            grt.sort();
            parts.push(format!("graph_related_types:{}", grt.join(",")));
        }
        if let Some(skip_root_key) = &self.skip_root_key {
            let mut srk = skip_root_key.clone();
            srk.sort();
            parts.push(format!("skip_root_key:{}", srk.join(",")));
        }
        if let Some(path_file) = &self.path_file {
            parts.push(format!("path_file:{}", path_file));
        }
        if let Some(path_extension) = &self.path_extension {
            parts.push(format!("path_extension:{}", path_extension));
        }
        if let Some(range) = &self.range {
            parts.push(format!("range:{}", range.fingerprint()));
        }
        if let Some(cardinality) = &self.cardinality {
            parts.push(format!("cardinality:{}", cardinality.fingerprint()));
        }

        parts.sort();
        parts.join(";")
    }
}

impl TypeFingerprint for LocalisationSpec {
    fn fingerprint(&self) -> String {
        let mut parts = Vec::new();

        if !self.required.is_empty() {
            let mut req_parts: Vec<String> = self
                .required
                .iter()
                .map(|(k, v)| format!("{}:{}", k, v))
                .collect();
            req_parts.sort();
            parts.push(format!("required:{}", req_parts.join(",")));
        }

        if !self.optional.is_empty() {
            let mut opt_parts: Vec<String> = self
                .optional
                .iter()
                .map(|(k, v)| format!("{}:{}", k, v))
                .collect();
            opt_parts.sort();
            parts.push(format!("optional:{}", opt_parts.join(",")));
        }

        if let Some(primary) = &self.primary {
            parts.push(format!("primary:{}", primary));
        }

        if !self.subtypes.is_empty() {
            let mut subtype_parts: Vec<String> = self
                .subtypes
                .iter()
                .map(|(k, v)| {
                    let mut inner: Vec<String> =
                        v.iter().map(|(ik, iv)| format!("{}:{}", ik, iv)).collect();
                    inner.sort();
                    format!("{}:{}", k, inner.join(","))
                })
                .collect();
            subtype_parts.sort();
            parts.push(format!("subtypes:{}", subtype_parts.join(",")));
        }

        parts.join(";")
    }
}

impl TypeFingerprint for ModifierSpec {
    fn fingerprint(&self) -> String {
        let mut parts = Vec::new();

        if !self.modifiers.is_empty() {
            let mut mod_parts: Vec<String> = self
                .modifiers
                .iter()
                .map(|(k, v)| format!("{}:{}", k, v))
                .collect();
            mod_parts.sort();
            parts.push(format!("modifiers:{}", mod_parts.join(",")));
        }

        if !self.subtypes.is_empty() {
            let mut subtype_parts: Vec<String> = self
                .subtypes
                .iter()
                .map(|(k, v)| {
                    let mut inner: Vec<String> =
                        v.iter().map(|(ik, iv)| format!("{}:{}", ik, iv)).collect();
                    inner.sort();
                    format!("{}:{}", k, inner.join(","))
                })
                .collect();
            subtype_parts.sort();
            parts.push(format!("subtypes:{}", subtype_parts.join(",")));
        }

        parts.join(";")
    }
}

impl TypeFingerprint for CwtTypeDefinition {
    fn fingerprint(&self) -> String {
        let mut parts = vec![
            format!("name:{}", self.name),
            format!("type_spec:{}", self.type_spec.fingerprint()),
            format!("options:{}", self.options.fingerprint()),
        ];

        if let Some(path) = &self.path {
            parts.push(format!("path:{}", path));
        }
        if let Some(name_field) = &self.name_field {
            parts.push(format!("name_field:{}", name_field));
        }
        if let Some(skip_root_key) = &self.skip_root_key {
            parts.push(format!("skip_root_key:{}", skip_root_key.fingerprint()));
        }

        parts.join(";")
    }
}

impl TypeFingerprint for SkipRootKeySpec {
    fn fingerprint(&self) -> String {
        match self {
            SkipRootKeySpec::Specific(key) => format!("specific:{}", key),
            SkipRootKeySpec::Any => "any".to_string(),
            SkipRootKeySpec::Except(keys) => {
                let mut sorted_keys = keys.clone();
                sorted_keys.sort();
                format!("except:{}", sorted_keys.join(","))
            }
            SkipRootKeySpec::Multiple(keys) => {
                let mut sorted_keys = keys.clone();
                sorted_keys.sort();
                format!("multiple:{}", sorted_keys.join(","))
            }
        }
    }
}

impl TypeFingerprint for CwtEnumDefinition {
    fn fingerprint(&self) -> String {
        let mut parts = vec![format!("name:{}", self.name)];

        let mut sorted_values: Vec<String> = self.values.iter().cloned().collect();
        sorted_values.sort();
        parts.push(format!("values:{}", sorted_values.join(",")));

        if let Some(complex) = &self.complex {
            parts.push(format!("complex:{}", complex.fingerprint()));
        }

        parts.join(";")
    }
}

impl TypeFingerprint for ComplexEnumSpec {
    fn fingerprint(&self) -> String {
        format!(
            "{}:{}:{}",
            self.path,
            self.name_structure.fingerprint(),
            self.start_from_root
        )
    }
}

impl TypeFingerprint for CwtAliasDefinition {
    fn fingerprint(&self) -> String {
        format!(
            "{}:{}:{}",
            self.name,
            self.category,
            self.type_spec.fingerprint()
        )
    }
}

impl TypeFingerprint for CwtValueSetDefinition {
    fn fingerprint(&self) -> String {
        let mut sorted_values: Vec<String> = self.values.iter().cloned().collect();
        sorted_values.sort();
        format!("{}:{}", self.name, sorted_values.join(","))
    }
}

// Helper functions for working with fingerprints
impl CwtType {
    /// Check if two types are equivalent by comparing their fingerprints
    pub fn is_equivalent_to(&self, other: &CwtType) -> bool {
        self.fingerprint() == other.fingerprint()
    }

    /// Get a hash-based fingerprint for more efficient storage/comparison
    pub fn fingerprint_hash(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        self.fingerprint().hash(&mut hasher);
        hasher.finish()
    }
}

// Convenience constructors
impl CwtType {
    /// Create a simple type
    pub fn simple(simple_type: SimpleType) -> Self {
        Self::Simple(simple_type)
    }

    /// Create a type reference
    pub fn type_ref(key: impl Into<String>) -> Self {
        Self::Reference(ReferenceType::Type { key: key.into() })
    }

    /// Create an enum reference
    pub fn enum_ref(key: impl Into<String>) -> Self {
        Self::Reference(ReferenceType::Enum { key: key.into() })
    }

    /// Create a value set reference
    pub fn value_set(key: impl Into<String>) -> Self {
        Self::Reference(ReferenceType::ValueSet { key: key.into() })
    }

    /// Create a literal value
    pub fn literal(value: impl Into<String>) -> Self {
        Self::Literal(value.into())
    }

    /// Create a block type
    pub fn block() -> BlockType {
        BlockType {
            properties: HashMap::new(),
            subtypes: HashMap::new(),
            pattern_properties: Vec::new(),
            subtype_properties: HashMap::new(),
            localisation: None,
            modifiers: None,
            additional_flags: Vec::new(),
        }
    }

    /// Create an array type
    pub fn array(element_type: CwtType) -> Self {
        Self::Array(ArrayType {
            element_type: Box::new(element_type),
        })
    }

    /// Create a union type
    pub fn union(types: Vec<CwtType>) -> Self {
        Self::Union(types)
    }

    /// Create a comparable type
    pub fn comparable(base_type: CwtType) -> Self {
        Self::Comparable(Box::new(base_type))
    }
}

impl Property {
    /// Create a simple property
    pub fn simple(property_type: CwtType) -> Self {
        Self {
            property_type,
            options: CwtOptions::default(),
            documentation: None,
        }
    }

    /// Create a required property
    pub fn required(property_type: CwtType) -> Self {
        Self {
            property_type,
            options: CwtOptions {
                required: true,
                cardinality: Some(Cardinality::required()),
                ..Default::default()
            },
            documentation: None,
        }
    }

    /// Create an optional property
    pub fn optional(property_type: CwtType) -> Self {
        Self {
            property_type,
            options: CwtOptions {
                optional: true,
                cardinality: Some(Cardinality::optional()),
                ..Default::default()
            },
            documentation: None,
        }
    }

    /// Add documentation
    pub fn with_documentation(mut self, doc: impl Into<String>) -> Self {
        self.documentation = Some(doc.into());
        self
    }

    /// Add options
    pub fn with_options(mut self, options: CwtOptions) -> Self {
        self.options = options;
        self
    }
}

impl Cardinality {
    /// Exactly once (required)
    pub fn required() -> Self {
        Self {
            min: Some(1),
            max: Some(1),
            soft: false,
        }
    }

    /// Zero or one (optional)
    pub fn optional() -> Self {
        Self {
            min: Some(0),
            max: Some(1),
            soft: false,
        }
    }

    /// One or more (required repeating)
    pub fn required_repeating() -> Self {
        Self {
            min: Some(1),
            max: None,
            soft: false,
        }
    }

    /// Zero or more (optional repeating)
    pub fn optional_repeating() -> Self {
        Self {
            min: Some(0),
            max: None,
            soft: false,
        }
    }

    /// Custom cardinality
    pub fn custom(min: Option<u32>, max: Option<u32>, soft: bool) -> Self {
        Self { min, max, soft }
    }
}

impl Range {
    /// Integer range
    pub fn int_range(min: i64, max: i64) -> Self {
        Self {
            min: RangeBound::Integer(min),
            max: RangeBound::Integer(max),
        }
    }

    /// Float range
    pub fn float_range(min: f64, max: f64) -> Self {
        Self {
            min: RangeBound::Float(min),
            max: RangeBound::Float(max),
        }
    }

    /// Unbounded range
    pub fn unbounded() -> Self {
        Self {
            min: RangeBound::NegInfinity,
            max: RangeBound::PosInfinity,
        }
    }
}

impl CwtOptions {
    /// Extract CWT options from a rule
    pub fn from_rule(rule: &AstCwtRule) -> Self {
        let mut options = CwtOptions::default();

        // Parse CWT options from the rule
        for option in &rule.options {
            match option.key {
                "display_name" => {
                    options.display_name =
                        Some(option.value.as_string_or_identifier().unwrap().to_string());
                }
                "abbreviation" => {
                    options.abbreviation =
                        Some(option.value.as_string_or_identifier().unwrap().to_string());
                }
                "push_scope" => {
                    options.push_scope =
                        Some(option.value.as_string_or_identifier().unwrap().to_string());
                }
                "starts_with" => {
                    options.starts_with =
                        Some(option.value.as_string_or_identifier().unwrap().to_string());
                }
                "severity" => {
                    options.severity = Some(option.value.as_identifier().unwrap().parse().unwrap());
                }
                "type_key_filter" => {
                    options.type_key_filter = Some(TypeKeyFilter::Specific(
                        option.value.as_string_or_identifier().unwrap().to_string(),
                    ));
                }
                "required" => {
                    options.required = true;
                }
                "primary" => {
                    options.primary = true;
                }
                "optional" => {
                    options.optional = true;
                }
                "unique" => {
                    options.unique = true;
                }
                "path_strict" => {
                    options.path_strict = true;
                }
                "type_per_file" => {
                    options.type_per_file = true;
                }
                "path_file" => {
                    options.path_file =
                        Some(option.value.as_string_or_identifier().unwrap().to_string());
                }
                "path_extension" => {
                    options.path_extension =
                        Some(option.value.as_string_or_identifier().unwrap().to_string());
                }
                "cardinality" => {
                    if let Some(range) = option.value.as_range() {
                        let (min_bound, max_bound, soft) = range;
                        let min = match min_bound {
                            CwtCommentRangeBound::Number(n) => Some(n.parse().unwrap_or(0)),
                            CwtCommentRangeBound::Infinity => None,
                        };
                        let max = match max_bound {
                            CwtCommentRangeBound::Number(n) => Some(n.parse().unwrap_or(1)),
                            CwtCommentRangeBound::Infinity => None,
                        };
                        let cardinality = Cardinality { min, max, soft };
                        options.cardinality = Some(cardinality);
                    }
                }
                _ => {}
            }
        }

        options
    }

    /// Merge options, preferring non-default values from other
    pub fn merge(self, other: CwtOptions) -> CwtOptions {
        CwtOptions {
            required: self.required || other.required,
            primary: self.primary || other.primary,
            optional: self.optional || other.optional,
            severity: self.severity.or(other.severity),
            display_name: self.display_name.or(other.display_name),
            abbreviation: self.abbreviation.or(other.abbreviation),
            starts_with: self.starts_with.or(other.starts_with),
            push_scope: self.push_scope.or(other.push_scope),
            replace_scope: self.replace_scope.or(other.replace_scope),
            scope: self.scope.or(other.scope),
            type_key_filter: self.type_key_filter.or(other.type_key_filter),
            graph_related_types: self.graph_related_types.or(other.graph_related_types),
            unique: self.unique || other.unique,
            skip_root_key: self.skip_root_key.or(other.skip_root_key),
            path_strict: self.path_strict || other.path_strict,
            path_file: self.path_file.or(other.path_file),
            path_extension: self.path_extension.or(other.path_extension),
            type_per_file: self.type_per_file || other.type_per_file,
            range: self.range.or(other.range),
            cardinality: self.cardinality.or(other.cardinality),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fingerprint_uniqueness() {
        // Test that different types have different fingerprints
        let type1 = CwtType::simple(SimpleType::Int);
        let type2 = CwtType::simple(SimpleType::Float);
        let type3 = CwtType::type_ref("test_type");

        assert_ne!(type1.fingerprint(), type2.fingerprint());
        assert_ne!(type1.fingerprint(), type3.fingerprint());
        assert_ne!(type2.fingerprint(), type3.fingerprint());
    }

    #[test]
    fn test_fingerprint_consistency() {
        // Test that the same type produces the same fingerprint
        let type1 = CwtType::simple(SimpleType::Int);
        let type2 = CwtType::simple(SimpleType::Int);

        assert_eq!(type1.fingerprint(), type2.fingerprint());
        assert!(type1.is_equivalent_to(&type2));
    }

    #[test]
    fn test_union_fingerprint_ordering() {
        // Test that union types with same elements in different order have same fingerprint
        let type1 = CwtType::simple(SimpleType::Int);
        let type2 = CwtType::simple(SimpleType::Float);

        let union1 = CwtType::union(vec![type1.clone(), type2.clone()]);
        let union2 = CwtType::union(vec![type2.clone(), type1.clone()]);

        assert_eq!(union1.fingerprint(), union2.fingerprint());
        assert!(union1.is_equivalent_to(&union2));
    }

    #[test]
    fn test_literal_set_fingerprint_ordering() {
        // Test that literal sets with same values in different order have same fingerprint
        let mut values1 = HashSet::new();
        values1.insert("a".to_string());
        values1.insert("b".to_string());
        values1.insert("c".to_string());

        let mut values2 = HashSet::new();
        values2.insert("c".to_string());
        values2.insert("a".to_string());
        values2.insert("b".to_string());

        let set1 = CwtType::LiteralSet(values1);
        let set2 = CwtType::LiteralSet(values2);

        assert_eq!(set1.fingerprint(), set2.fingerprint());
        assert!(set1.is_equivalent_to(&set2));
    }

    #[test]
    fn test_complex_type_fingerprint() {
        // Test fingerprint for complex block type
        let mut block = CwtType::block();
        block.properties.insert(
            "key1".to_string(),
            Property::simple(CwtType::simple(SimpleType::Int)),
        );
        block.properties.insert(
            "key2".to_string(),
            Property::simple(CwtType::simple(SimpleType::Float)),
        );

        let complex_type = CwtType::Block(block);
        let fingerprint = complex_type.fingerprint();

        // Should contain identifiable parts
        assert!(fingerprint.contains("props:"));
        assert!(fingerprint.contains("key1"));
        assert!(fingerprint.contains("key2"));
    }

    #[test]
    fn test_fingerprint_hash() {
        // Test that fingerprint hash is consistent
        let type1 = CwtType::simple(SimpleType::Int);
        let hash1 = type1.fingerprint_hash();
        let hash2 = type1.fingerprint_hash();

        assert_eq!(hash1, hash2);
    }
}
