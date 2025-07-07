use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Represents a rich type system that can express all CWT type concepts
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InferredType {
    /// A literal value type like 'yes' or 'no'
    Literal(String),

    /// A union of literal types like 'yes' | 'no'
    LiteralUnion(HashSet<String>),

    /// A primitive type with optional constraints
    Primitive(PrimitiveType),

    /// A union of primitive types
    PrimitiveUnion(HashSet<PrimitiveType>),

    /// A reference to another type (CWT reference types)
    Reference(ReferenceType),

    /// An object with typed properties and optional subtypes
    Object(ObjectType),

    /// An array/list with cardinality constraints
    Array(ArrayType),

    /// A union of different types
    Union(Vec<InferredType>),

    /// A type with cardinality constraints
    Constrained(ConstrainedType),

    /// A comparable trigger type (uses == operator)
    Comparable(Box<InferredType>),

    /// Unknown type (when we can't infer)
    Unknown,
}

/// Expanded primitive types to match CWT simple types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub enum PrimitiveType {
    // Basic types
    String,
    Boolean,

    // Numeric types
    Integer,
    Float,

    // CWT-specific types
    Scalar,
    PercentageField,

    // Localisation types
    Localisation,
    LocalisationSynced,
    LocalisationInline,

    // Specialized field types
    DateField,
    VariableField,
    IntVariableField,
    ValueField,
    IntValueField,
    ScopeField,

    // File/Resource types
    Filepath,
    Icon,

    // Game-specific types
    Color,
    Maths,
}

/// CWT reference types - references to other defined types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ReferenceType {
    /// Type reference: <type_key>
    TypeRef {
        type_key: String,
        /// Optional prefix/suffix for type references like prefix_<type>_suffix
        prefix: Option<String>,
        suffix: Option<String>,
    },

    /// Enum reference: enum[key]
    Enum { key: String },

    /// Complex enum reference: complex_enum[key]
    ComplexEnum {
        key: String,
        /// Path to scan for enum values
        path: String,
        /// Structure to match for enum extraction
        name_structure: Option<String>,
        /// Whether to start from file root instead of first level
        start_from_root: bool,
    },

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

    /// Stellaris name format reference: stellaris_name_format[key]
    StellarisNameFormat { key: String },

    /// Colour reference: colour[hsv|rgb]
    Colour { format: String },

    /// Icon reference: icon[path] with optional prefix
    Icon {
        path: String,
        /// Optional prefix path
        prefix: Option<String>,
    },

    /// Filepath reference: filepath[path] with optional prefix and extension
    Filepath {
        path: String,
        /// Optional prefix path
        prefix: Option<String>,
        /// Optional file extension
        extension: Option<String>,
    },

    /// Subtype reference: subtype[name]
    Subtype { name: String },
}

/// Object type with properties and optional subtypes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObjectType {
    /// Regular properties
    pub properties: HashMap<String, PropertyDefinition>,

    /// Subtypes - conditional property sets
    pub subtypes: HashMap<String, SubtypeDefinition>,

    /// Whether this object can have additional properties beyond those defined
    pub extensible: bool,

    /// Localisation requirements for this object type
    pub localisation: Option<LocalisationRequirements>,

    /// Modifier generation rules for this object type
    pub modifiers: Option<ModifierGeneration>,
}

/// Property definition with metadata
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PropertyDefinition {
    /// The type of this property
    pub property_type: Box<InferredType>,

    /// Cardinality constraints
    pub cardinality: Option<Cardinality>,

    /// CWT options/metadata
    pub options: Vec<CwtOption>,

    /// Documentation string
    pub documentation: Option<String>,
}

/// Subtype definition - properties that apply when a condition is met
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubtypeDefinition {
    /// The condition that enables this subtype (e.g., "show_name = yes")
    pub condition: SubtypeCondition,

    /// Properties that apply when this subtype is active
    pub properties: HashMap<String, PropertyDefinition>,

    /// Whether this subtype is mutually exclusive with others
    pub exclusive: bool,

    /// CWT options for this subtype
    pub options: Vec<CwtOption>,

    /// Display name for this subtype
    pub display_name: Option<String>,

    /// Abbreviation for this subtype
    pub abbreviation: Option<String>,
}

/// Conditions that determine when subtypes apply
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SubtypeCondition {
    /// Property has a specific value
    PropertyEquals { property: String, value: String },

    /// Property does not have a specific value
    PropertyNotEquals { property: String, value: String },

    /// Property exists
    PropertyExists { property: String },

    /// Property does not exist
    PropertyNotExists { property: String },

    /// Type key starts with prefix
    KeyStartsWith { prefix: String },

    /// Type key filter matches
    TypeKeyFilter { filter: String },

    /// Complex boolean expression
    Expression(String),
}

/// Localisation requirements for a type
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocalisationRequirements {
    /// Required localisation keys with their patterns
    pub required: HashMap<String, String>,

    /// Optional localisation keys with their patterns
    pub optional: HashMap<String, String>,

    /// Primary localisation key
    pub primary: Option<String>,

    /// Subtype-specific localisation requirements
    pub subtypes: HashMap<String, HashMap<String, String>>,
}

/// Modifier generation rules for a type
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModifierGeneration {
    /// Modifier patterns with their scope categories
    pub modifiers: HashMap<String, String>,

    /// Subtype-specific modifier generation
    pub subtypes: HashMap<String, HashMap<String, String>>,
}

/// Array type with cardinality constraints
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArrayType {
    /// Type of elements in the array
    pub element_type: Box<InferredType>,

    /// Cardinality constraints
    pub cardinality: Cardinality,
}

/// Type with constraints (cardinality, range, etc.)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConstrainedType {
    /// The base type
    pub base_type: Box<InferredType>,

    /// Cardinality constraints
    pub cardinality: Option<Cardinality>,

    /// Range constraints (for numeric types)
    pub range: Option<Range>,

    /// CWT options/metadata
    pub options: Vec<CwtOption>,
}

/// Cardinality constraints - how many times something can appear
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cardinality {
    /// Minimum occurrences
    pub min: u32,

    /// Maximum occurrences (None = infinity)
    pub max: Option<u32>,

    /// Whether this is a soft constraint (prefixed with ~)
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
    /// Integer value
    Integer(i64),

    /// Float value
    Float(f64),

    /// Positive infinity
    PosInfinity,

    /// Negative infinity
    NegInfinity,
}

/// CWT option directives from ## comments
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CwtOption {
    /// Required field
    Required,

    /// Primary field
    Primary,

    /// Optional field (explicit)
    Optional,

    /// Severity level
    Severity(SeverityLevel),

    /// Display name
    DisplayName(String),

    /// Abbreviation
    Abbreviation(String),

    /// Starts with constraint
    StartsWith(String),

    /// Push scope
    PushScope(String),

    /// Replace scope
    ReplaceScope(HashMap<String, String>),

    /// Scope constraint
    Scope(Vec<String>),

    /// Type key filter
    TypeKeyFilter(String),

    /// Graph related types
    GraphRelatedTypes(Vec<String>),

    /// Unique constraint
    Unique,

    /// Skip root key
    SkipRootKey(Vec<String>),

    /// Path constraints
    PathStrict,
    PathFile(String),
    PathExtension(String),

    /// Type per file
    TypePerFile,
}

/// Severity levels for validation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SeverityLevel {
    Error,
    Warning,
    Information,
    Hint,
}

// Convenience constructors and methods
impl InferredType {
    /// Create a simple primitive type
    pub fn primitive(ptype: PrimitiveType) -> Self {
        Self::Primitive(ptype)
    }

    /// Create a type reference
    pub fn type_ref(type_key: impl Into<String>) -> Self {
        Self::Reference(ReferenceType::TypeRef {
            type_key: type_key.into(),
            prefix: None,
            suffix: None,
        })
    }

    /// Create an enum reference
    pub fn enum_ref(key: impl Into<String>) -> Self {
        Self::Reference(ReferenceType::Enum { key: key.into() })
    }

    /// Create a value set reference
    pub fn value_set(key: impl Into<String>) -> Self {
        Self::Reference(ReferenceType::ValueSet { key: key.into() })
    }

    /// Create a value reference
    pub fn value(key: impl Into<String>) -> Self {
        Self::Reference(ReferenceType::Value { key: key.into() })
    }

    /// Create a comparable trigger type
    pub fn comparable(base_type: InferredType) -> Self {
        Self::Comparable(Box::new(base_type))
    }

    /// Create a constrained type with cardinality
    pub fn with_cardinality(self, min: u32, max: Option<u32>) -> Self {
        Self::Constrained(ConstrainedType {
            base_type: Box::new(self),
            cardinality: Some(Cardinality {
                min,
                max,
                soft: false,
            }),
            range: None,
            options: Vec::new(),
        })
    }

    /// Create a constrained type with range
    pub fn with_range(self, min: RangeBound, max: RangeBound) -> Self {
        Self::Constrained(ConstrainedType {
            base_type: Box::new(self),
            cardinality: None,
            range: Some(Range { min, max }),
            options: Vec::new(),
        })
    }

    /// Create an array type
    pub fn array(element_type: InferredType) -> Self {
        Self::Array(ArrayType {
            element_type: Box::new(element_type),
            cardinality: Cardinality {
                min: 0,
                max: None,
                soft: false,
            },
        })
    }

    /// Create an object type
    pub fn object() -> ObjectType {
        ObjectType {
            properties: HashMap::new(),
            subtypes: HashMap::new(),
            extensible: true,
            localisation: None,
            modifiers: None,
        }
    }
}

impl PropertyDefinition {
    /// Create a simple property
    pub fn simple(property_type: InferredType) -> Self {
        Self {
            property_type: Box::new(property_type),
            cardinality: None,
            options: Vec::new(),
            documentation: None,
        }
    }

    /// Create a property with cardinality
    pub fn with_cardinality(property_type: InferredType, cardinality: Cardinality) -> Self {
        Self {
            property_type: Box::new(property_type),
            cardinality: Some(cardinality),
            options: Vec::new(),
            documentation: None,
        }
    }

    /// Add documentation to this property
    pub fn with_documentation(mut self, doc: impl Into<String>) -> Self {
        self.documentation = Some(doc.into());
        self
    }

    /// Add options to this property
    pub fn with_options(mut self, options: Vec<CwtOption>) -> Self {
        self.options = options;
        self
    }
}

impl Cardinality {
    /// Create a cardinality constraint
    pub fn new(min: u32, max: Option<u32>) -> Self {
        Self {
            min,
            max,
            soft: false,
        }
    }

    /// Create a soft cardinality constraint
    pub fn soft(min: u32, max: Option<u32>) -> Self {
        Self {
            min,
            max,
            soft: true,
        }
    }

    /// Required exactly once
    pub fn required() -> Self {
        Self::new(1, Some(1))
    }

    /// Optional (0 or 1)
    pub fn optional() -> Self {
        Self::new(0, Some(1))
    }

    /// Required at least once
    pub fn required_repeating() -> Self {
        Self::new(1, None)
    }

    /// Optional repeating
    pub fn optional_repeating() -> Self {
        Self::new(0, None)
    }
}

impl Range {
    /// Create an integer range
    pub fn int_range(min: i64, max: i64) -> Self {
        Self {
            min: RangeBound::Integer(min),
            max: RangeBound::Integer(max),
        }
    }

    /// Create a float range
    pub fn float_range(min: f64, max: f64) -> Self {
        Self {
            min: RangeBound::Float(min),
            max: RangeBound::Float(max),
        }
    }

    /// Create an unbounded range
    pub fn unbounded() -> Self {
        Self {
            min: RangeBound::NegInfinity,
            max: RangeBound::PosInfinity,
        }
    }
}
