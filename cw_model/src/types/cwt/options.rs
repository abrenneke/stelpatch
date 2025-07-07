//! Rule options and constraints for CWT analysis
//!
//! This module contains structures and utilities for handling CWT rule options,
//! cardinality constraints, and other rule-specific configuration.

use cw_parser::cwt::{AstCwtRule, CwtCardinalityMax, CwtOptionType};
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
    pub min: u32,
    pub max: Option<u32>, // None means infinite
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
            match &cwt_option.option_type {
                CwtOptionType::Cardinality { min, max } => {
                    options.cardinality = Some(CardinalityConstraint {
                        min: *min,
                        max: match max {
                            CwtCardinalityMax::Number(n) => Some(*n),
                            CwtCardinalityMax::Infinity => None,
                        },
                        is_warning: false,
                    });
                }
                CwtOptionType::SoftCardinality { min, max } => {
                    options.cardinality = Some(CardinalityConstraint {
                        min: *min,
                        max: match max {
                            CwtCardinalityMax::Number(n) => Some(*n),
                            CwtCardinalityMax::Infinity => None,
                        },
                        is_warning: true,
                    });
                }
                CwtOptionType::PushScope { scope } => {
                    options.push_scope = Some(scope.to_string());
                }
                CwtOptionType::ReplaceScope { replacements } => {
                    let mut replace_map = HashMap::new();
                    for replacement in replacements {
                        replace_map
                            .insert(replacement.from.to_string(), replacement.to.to_string());
                    }
                    options.replace_scope = Some(replace_map);
                }
                CwtOptionType::Scope { scopes } => {
                    options.scope = Some(scopes.iter().map(|s| s.to_string()).collect());
                }
                _ => {
                    // Handle other option types as needed
                }
            }
        }

        // Extract documentation from the rule
        if let Some(doc) = &rule.documentation {
            options.documentation = Some(doc.text.to_string());
        }

        // Default cardinality if none specified
        if options.cardinality.is_none() {
            options.cardinality = Some(CardinalityConstraint {
                min: 1,
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
            min,
            max,
            is_warning: false,
        }
    }

    /// Create a soft cardinality constraint (warning-only)
    pub fn new_soft(min: u32, max: Option<u32>) -> Self {
        Self {
            min,
            max,
            is_warning: true,
        }
    }

    /// Create an optional cardinality constraint (0..1)
    pub fn optional() -> Self {
        Self {
            min: 0,
            max: Some(1),
            is_warning: false,
        }
    }

    /// Create a required cardinality constraint (1..1)
    pub fn required() -> Self {
        Self {
            min: 1,
            max: Some(1),
            is_warning: false,
        }
    }

    /// Create an array cardinality constraint (0..*)
    pub fn array() -> Self {
        Self {
            min: 0,
            max: None,
            is_warning: false,
        }
    }

    /// Create a non-empty array cardinality constraint (1..*)
    pub fn non_empty_array() -> Self {
        Self {
            min: 1,
            max: None,
            is_warning: false,
        }
    }

    /// Check if this constraint allows zero occurrences
    pub fn allows_zero(&self) -> bool {
        self.min == 0
    }

    /// Check if this constraint allows multiple occurrences
    pub fn allows_multiple(&self) -> bool {
        self.max.is_none() || self.max.unwrap() > 1
    }

    /// Check if this constraint is satisfied by the given count
    pub fn is_satisfied(&self, count: u32) -> bool {
        count >= self.min && (self.max.is_none() || count <= self.max.unwrap())
    }

    /// Check if this constraint is optional (0..1)
    pub fn is_optional(&self) -> bool {
        self.min == 0 && self.max == Some(1)
    }

    /// Check if this constraint is required (1..1)
    pub fn is_required(&self) -> bool {
        self.min == 1 && self.max == Some(1)
    }

    /// Check if this constraint is an array (allows multiple)
    pub fn is_array(&self) -> bool {
        self.allows_multiple()
    }
}

impl std::fmt::Display for CardinalityConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let prefix = if self.is_warning { "~" } else { "" };
        match self.max {
            Some(max) if max == self.min => write!(f, "{}[{}]", prefix, self.min),
            Some(max) => write!(f, "{}[{}..{}]", prefix, self.min, max),
            None => write!(f, "{}[{}..*]", prefix, self.min),
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
        assert_eq!(optional.to_string(), "[0..1]");

        let required = CardinalityConstraint::required();
        assert_eq!(required.to_string(), "[1]");

        let array = CardinalityConstraint::array();
        assert_eq!(array.to_string(), "[0..*]");

        let soft_required = CardinalityConstraint::new_soft(1, Some(1));
        assert_eq!(soft_required.to_string(), "~[1]");
    }
}
