//! CWT Type System
//!
//! This module provides a direct representation of CWT types and definitions,
//! closely aligned with the CWT specification rather than inferred types.

use crate::{SeverityLevel, TypeKeyFilter};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

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

/// Block/object types with properties and subtypes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockType {
    /// Regular properties
    pub properties: HashMap<String, Property>,

    /// Subtypes - conditional property sets
    pub subtypes: HashMap<String, Subtype>,

    /// Alias patterns - alias_name[X] = alias_match_left[X] patterns
    pub alias_patterns: HashMap<String, CwtType>,

    /// Enum patterns - enum[key] = type patterns
    pub enum_patterns: HashMap<String, CwtType>,

    /// Localisation requirements
    pub localisation: Option<LocalisationSpec>,

    /// Modifier generation rules
    pub modifiers: Option<ModifierSpec>,
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

    /// Properties that apply when this subtype is active
    pub properties: HashMap<String, Property>,

    /// Options for this subtype
    pub options: CwtOptions,
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
            alias_patterns: HashMap::new(),
            enum_patterns: HashMap::new(),
            localisation: None,
            modifiers: None,
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
