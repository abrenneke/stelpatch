//! Rule options and constraints for CWT analysis
//!
//! This module contains structures and utilities for handling CWT rule options,
//! cardinality constraints, and other rule-specific configuration.

use cw_parser::{CwtCommentRangeBound, CwtOptionExpression, cwt::AstCwtRule};
use std::collections::HashMap;

/// Options that can be applied to CWT rules
#[derive(Debug, Clone, Default)]
pub struct RuleOptions {
    /// Cardinality constraint (min..max)
    pub cardinality: Option<CardinalityConstraint>,
    /// Scope constraint
    pub scope: Option<Vec<String>>,
    /// Push scope
    pub push_scope: Option<String>,
    /// Replace scope mappings
    pub replace_scope: Option<HashMap<String, String>>,
    /// Documentation comment
    pub documentation: Option<String>,
}

/// Cardinality constraint for CWT rules
#[derive(Debug, Clone)]
pub struct CardinalityConstraint {
    pub min: Option<u32>, // None means -inf
    pub max: Option<u32>, // None means inf
    pub is_warning: bool, // ~ prefix means warning-only
}

impl RuleOptions {
    /// Create new default rule options
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse rule options from a CWT rule AST node
    pub fn from_rule(rule: &AstCwtRule) -> Self {
        let mut options = RuleOptions::default();

        // Process all CWT options from the parsed AST
        for cwt_option in &rule.options {
            match cwt_option.key {
                "cardinality" => {
                    let (min, max, is_warning) = cwt_option.value.as_range().unwrap();
                    options.cardinality = Some(CardinalityConstraint {
                        min: match min {
                            CwtCommentRangeBound::Number(n) => Some(n.parse().unwrap()),
                            CwtCommentRangeBound::Infinity => None,
                        },
                        max: match max {
                            CwtCommentRangeBound::Number(n) => Some(n.parse().unwrap()),
                            CwtCommentRangeBound::Infinity => None,
                        },
                        is_warning,
                    });
                }
                "push_scope" => {
                    let scope = cwt_option.value.as_identifier().unwrap();
                    options.push_scope = Some(scope.to_string());
                }
                "replace_scope" => {
                    let replacements = cwt_option.value.as_assignments().unwrap();
                    let mut replace_map = HashMap::new();
                    for replacement in replacements {
                        let (from, to) = replacement.as_assignment().unwrap();
                        replace_map.insert(from.to_string(), to.as_string().unwrap().to_string());
                    }
                    options.replace_scope = Some(replace_map);
                }
                "scope" => {
                    let scopes = match &cwt_option.value {
                        CwtOptionExpression::List(scopes) => scopes
                            .iter()
                            .map(|s| s.as_string().unwrap().to_string())
                            .collect(),
                        CwtOptionExpression::String(scope) => vec![scope.to_string()],
                        _ => vec![],
                    };
                    options.scope = Some(scopes);
                }
                _ => {
                    // Handle other option types as needed
                }
            }
        }

        // Extract documentation from the rule
        if !rule.documentation.is_empty() {
            options.documentation = Some(
                rule.documentation
                    .iter()
                    .map(|d| d.text.to_string())
                    .collect::<Vec<String>>()
                    .join("\n"),
            );
        }

        // Default cardinality if none specified
        if options.cardinality.is_none() {
            options.cardinality = Some(CardinalityConstraint {
                min: Some(1),
                max: Some(1),
                is_warning: false,
            });
        }

        options
    }

    /// Check if this rule has cardinality constraints
    pub fn has_cardinality(&self) -> bool {
        self.cardinality.is_some()
    }

    /// Check if this rule has scope constraints
    pub fn has_scope(&self) -> bool {
        self.scope.is_some()
    }

    /// Check if this rule pushes scope
    pub fn has_push_scope(&self) -> bool {
        self.push_scope.is_some()
    }

    /// Check if this rule replaces scope
    pub fn has_replace_scope(&self) -> bool {
        self.replace_scope.is_some()
    }

    /// Check if this rule has documentation
    pub fn has_documentation(&self) -> bool {
        self.documentation.is_some()
    }
}

impl CardinalityConstraint {
    /// Create a new cardinality constraint
    pub fn new(min: u32, max: Option<u32>) -> Self {
        Self {
            min: Some(min),
            max,
            is_warning: false,
        }
    }

    /// Create a soft cardinality constraint (warning-only)
    pub fn new_soft(min: u32, max: Option<u32>) -> Self {
        Self {
            min: Some(min),
            max,
            is_warning: true,
        }
    }

    /// Create an optional cardinality constraint (0..1)
    pub fn optional() -> Self {
        Self {
            min: Some(0),
            max: Some(1),
            is_warning: false,
        }
    }

    /// Create a required cardinality constraint (1..1)
    pub fn required() -> Self {
        Self {
            min: Some(1),
            max: Some(1),
            is_warning: false,
        }
    }

    /// Create an array cardinality constraint (0..*)
    pub fn array() -> Self {
        Self {
            min: Some(0),
            max: None,
            is_warning: false,
        }
    }

    /// Create a non-empty array cardinality constraint (1..*)
    pub fn non_empty_array() -> Self {
        Self {
            min: Some(1),
            max: None,
            is_warning: false,
        }
    }

    /// Check if this constraint allows zero occurrences
    pub fn allows_zero(&self) -> bool {
        self.min == Some(0)
    }

    /// Check if this constraint allows multiple occurrences
    pub fn allows_multiple(&self) -> bool {
        self.max.is_none() || self.max.unwrap() > 1
    }

    /// Check if this constraint is satisfied by the given count
    pub fn is_satisfied(&self, count: u32) -> bool {
        count >= self.min.unwrap() && (self.max.is_none() || count <= self.max.unwrap())
    }

    /// Check if this constraint is optional (0..1)
    pub fn is_optional(&self) -> bool {
        self.min == Some(0) && self.max == Some(1)
    }

    /// Check if this constraint is required (1..1)
    pub fn is_required(&self) -> bool {
        self.min == Some(1) && self.max == Some(1)
    }

    /// Check if this constraint is an array (allows multiple)
    pub fn is_array(&self) -> bool {
        self.allows_multiple()
    }
}

impl std::fmt::Display for CardinalityConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let prefix = if self.is_warning { "~" } else { "" };
        match (self.min, self.max) {
            (Some(min), Some(max)) if min == max => write!(f, "{}{}", prefix, min),
            (Some(min), Some(max)) => write!(f, "{}{min}..{max}", prefix),
            (Some(min), None) => write!(f, "{}{min}..inf", prefix),
            (None, Some(max)) => write!(f, "{}-inf..{max}", prefix),
            (None, None) => write!(f, "{}-inf..inf", prefix),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cardinality_constraint_creation() {
        let optional = CardinalityConstraint::optional();
        assert!(optional.is_optional());
        assert!(optional.allows_zero());
        assert!(!optional.allows_multiple());

        let required = CardinalityConstraint::required();
        assert!(required.is_required());
        assert!(!required.allows_zero());
        assert!(!required.allows_multiple());

        let array = CardinalityConstraint::array();
        assert!(array.is_array());
        assert!(array.allows_zero());
        assert!(array.allows_multiple());

        let non_empty_array = CardinalityConstraint::non_empty_array();
        assert!(non_empty_array.is_array());
        assert!(!non_empty_array.allows_zero());
        assert!(non_empty_array.allows_multiple());
    }

    #[test]
    fn test_cardinality_constraint_satisfaction() {
        let optional = CardinalityConstraint::optional();
        assert!(optional.is_satisfied(0));
        assert!(optional.is_satisfied(1));
        assert!(!optional.is_satisfied(2));

        let required = CardinalityConstraint::required();
        assert!(!required.is_satisfied(0));
        assert!(required.is_satisfied(1));
        assert!(!required.is_satisfied(2));

        let array = CardinalityConstraint::array();
        assert!(array.is_satisfied(0));
        assert!(array.is_satisfied(1));
        assert!(array.is_satisfied(100));
    }

    #[test]
    fn test_cardinality_constraint_display() {
        let optional = CardinalityConstraint::optional();
        assert_eq!(optional.to_string(), "0..1");

        let required = CardinalityConstraint::required();
        assert_eq!(required.to_string(), "1");

        let array = CardinalityConstraint::array();
        assert_eq!(array.to_string(), "0..inf");

        let soft_required = CardinalityConstraint::new_soft(1, Some(1));
        assert_eq!(soft_required.to_string(), "~1");
    }
}
