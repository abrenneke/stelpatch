use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Represents an inferred type based on observed data patterns
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InferredType {
    /// A literal value type like 'yes' or 'no'
    Literal(String),

    /// A union of literal types like 'yes' | 'no'
    LiteralUnion(HashSet<String>),

    /// A primitive type
    Primitive(PrimitiveType),

    /// A union of primitive types
    PrimitiveUnion(HashSet<PrimitiveType>),

    /// An object with typed properties
    Object(HashMap<String, Box<InferredType>>),

    /// An array of a specific type
    Array(Box<InferredType>),

    /// A union of different types
    Union(Vec<InferredType>),

    /// Unknown type (when we can't infer)
    Unknown,
}

/// Primitive types that can be inferred
#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub enum PrimitiveType {
    String,
    Number,
    Boolean,
    Color,
    Maths,
}
