use std::ops::Range;

use winnow::{
    LocatingSlice, ModalResult, Parser,
    combinator::{eof, opt, repeat_till},
    error::StrContext,
    token::literal,
};

use crate::{AstComment, AstNode, CwParseError, ParseError, opt_ws_and_comments};

use super::{
    CwtAliasDefinition, CwtBlock, CwtComment, CwtComplexEnumDefinition, CwtEntity,
    CwtEnumDefinition, CwtRule, CwtSingleAliasDefinition, CwtTypeDefinition, cwt_entity,
    get_cwt_comments, opt_cwt_ws_and_comments,
};

use self_cell::self_cell;

/// CWT module representing a complete CWT file
#[derive(Debug, PartialEq)]
pub struct CwtModule<'a> {
    pub items: Vec<CwtEntity<'a>>,
    pub span: Range<usize>,
    pub leading_comments: Vec<CwtComment<'a>>,
    pub trailing_comments: Vec<CwtComment<'a>>,
}

pub type CwtModuleResult<'a> = Result<CwtModule<'a>, CwParseError>;

self_cell!(
    pub struct CwtModuleCell {
        owner: String,

        #[covariant]
        dependent: CwtModuleResult,
    }

    impl {Debug, PartialEq}
);

impl Clone for CwtModuleCell {
    fn clone(&self) -> Self {
        let owner_clone = self.borrow_owner().to_owned();
        Self::from_input(owner_clone)
    }
}

impl CwtModuleCell {
    pub fn from_input(input: String) -> Self {
        Self::new(input, |input| {
            let mut module = CwtModule::new();
            module.parse_input(&input).map(|_| module)
        })
    }
}

impl<'a> CwtModule<'a> {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            span: 0..0,
            leading_comments: Vec::new(),
            trailing_comments: Vec::new(),
        }
    }

    pub fn from_input(input: &'a str) -> Result<Self, CwParseError> {
        let mut module = Self::new();
        module.parse_input(input)?;
        Ok(module)
    }

    pub fn parse_input(&mut self, input: &'a str) -> Result<(), CwParseError> {
        let mut input_slice = LocatingSlice::new(input);

        let module = cwt_module
            .parse_next(&mut input_slice)
            .map_err(|e| ParseError::from_winnow_error_with_slice(e, input_slice, input))?;

        self.items = module.items;
        self.span = module.span;
        self.leading_comments = module.leading_comments;
        self.trailing_comments = module.trailing_comments;

        Ok(())
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

    /// Get all rules in the module
    pub fn rules(&self) -> impl Iterator<Item = &CwtRule<'a>> {
        self.items.iter().filter_map(|item| match item {
            CwtEntity::Rule(rule) => Some(rule),
            _ => None,
        })
    }

    /// Get all blocks in the module
    pub fn blocks(&self) -> impl Iterator<Item = &CwtBlock<'a>> {
        self.items.iter().filter_map(|item| match item {
            CwtEntity::Block(block) => Some(block),
            _ => None,
        })
    }

    /// Check if the module is empty
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Get the number of items in the module
    pub fn len(&self) -> usize {
        self.items.len()
    }
}

impl<'a> AstNode<'a> for CwtModule<'a> {
    fn span_range(&self) -> Range<usize> {
        self.span.clone()
    }

    fn leading_comments(&self) -> &[AstComment<'a>] {
        &[]
    }

    fn trailing_comment(&self) -> Option<&AstComment<'a>> {
        self.trailing_comments
            .last()
            .map(|_| {
                // CWT comments don't map directly to AstComment, so we return None
                // This is a design choice - we might want to convert CwtComment to AstComment
                None
            })
            .flatten()
    }
}

/// Parse a CWT module (entire file)
pub(crate) fn cwt_module<'a>(input: &mut LocatingSlice<&'a str>) -> ModalResult<CwtModule<'a>> {
    // Skip BOM if present
    opt(literal("\u{feff}")).parse_next(input)?;

    // Parse leading comments for the entire module
    let leading_comments_data = opt_cwt_ws_and_comments.parse_next(input)?;
    let leading_comments = get_cwt_comments(&leading_comments_data);

    // Parse all entities (each entity parser will handle its own leading comments)
    let ((entities, _), span): ((Vec<CwtEntity<'a>>, _), _) =
        repeat_till(0.., cwt_entity, (opt_ws_and_comments, eof))
            .with_span()
            .context(StrContext::Label("cwt_module"))
            .parse_next(input)?;

    let span = 0..span.end;

    Ok(CwtModule {
        items: entities,
        span,
        leading_comments,
        trailing_comments: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use crate::CwtValue;

    use super::*;
    use pretty_assertions::assert_eq;
    use winnow::{LocatingSlice, Parser};

    #[test]
    fn test_cwt_module_empty() {
        let mut input = LocatingSlice::new("");
        let result = cwt_module.parse_next(&mut input).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_simple_rule_parsing() {
        let input = r#"key = value"#;
        let result = CwtModule::from_input(input).unwrap();
        assert_eq!(result.len(), 1);
        let rule = result.find_rule("key");
        assert!(rule.is_some());
        assert_eq!(rule.unwrap().key.name(), "key");
    }

    #[test]
    fn test_simple_block_parsing() {
        let input = r#"key = {}"#;
        let result = CwtModule::from_input(input).unwrap();
        assert_eq!(result.len(), 1);
        let rule = result.find_rule("key");
        assert!(rule.is_some());
        assert_eq!(rule.unwrap().key.name(), "key");
    }

    #[test]
    fn test_types_rule_parsing() {
        let input = r#"types = {}"#;
        let result = CwtModule::from_input(input).unwrap();
        assert_eq!(result.len(), 1);
        let rule = result.find_rule("types");
        assert!(rule.is_some());
        assert_eq!(rule.unwrap().key.name(), "types");
    }

    #[test]
    fn types_definition() {
        let input = r#"
types = {
	type[traitcolors] = {
		path = "game/common/named_colors"
	}
	type[ambient_object] = {
		name_field = "name"
		path = "game/common/ambient_objects"
		subtype[show_name] = {
			show_name = yes
		}
		localisation = {
			subtype[show_name] = {
				## required
				name = "$"
			}
		}
		subtype[selectable] = {
			selectable = yes
		}
	}
	type[asteroid_belt_type] = { #reason for this rename is the way vanilla names stuff, creating ambiguity.
		path = "game/common/asteroid_belts"
	}
	type[attitude] = {
		path = "game/common/attitudes"
		localisation = {
			## required
			name = "attitude_$"
		}
	}
	type[button_effect] = {
		path = "game/common/button_effects"
	}
	type[country_customization] = {
		path = "game/common/country_customization"
	}
	type[system_type] = {
		path = "game/common/system_types"
		localisation = {
			## required
			name = "$"
		}
	}
	## type_key_filter = part
	type[start_screen_message] = {
		path = "game/common/start_screen_messages"
	}
	type[precursors] = {
		path = "common/precursor_civilizations"
	}
	## replace_scope = { root = system this = system }
	type[sector_type] = {
		path = "game/common/sector_types"
		localisation = {
			## required
			name = "$"
		}
	}
	## replace_scope = { root = sector this = sector }
	type[sector_focus] = {
		path = "game/common/sector_focuses"
		subtype[hidden] = {
			hidden = yes
		}
		localisation = {
			subtype[!hidden] = {
				## required
				name = "st_$"
				## required
				desc = "st_$_desc"
			}
		}
	}
	
}"#;

        let _result = CwtModule::from_input(input).unwrap();
    }

    #[test]
    fn test_real_cwt_file() {
        let input = include_str!("../../test_data/cwt/00_small_types_consolidated.cwt");

        let _result = CwtModule::from_input(input).unwrap();
    }

    #[test]
    fn test_real_cwt_file_2() {
        let input = include_str!("../../test_data/cwt/agreements.cwt");

        let _result = CwtModule::from_input(input).unwrap();
    }

    #[test]
    fn test_real_cwt_file_3() {
        let input = include_str!("../../test_data/cwt/all_modifiers_consolidated.cwt");
        let _result = CwtModule::from_input(input).unwrap();
    }

    #[test]
    fn test_alias_match_left() {
        let input = r#"
button_effect = {
	potential = {
		alias_name[trigger] = alias_match_left[trigger]
	}
}
"#;

        let _result = CwtModule::from_input(input).unwrap();
    }

    #[test]
    fn test_alias_match_left_with_quotes() {
        let input = r#"
button_effect = {
    potential = {
        "alias_name[trigger]" = "alias_match_left[trigger]"
    }
}
"#;

        let _result = CwtModule::from_input(input).unwrap();
    }

    #[test]
    fn test_simple_rule_with_alias_match_left() {
        let input = "key = alias_match_left[trigger]";
        let result = CwtModule::from_input(input).unwrap();
        assert_eq!(result.len(), 1);
        let rule = result.find_rule("key").unwrap();
        assert_eq!(rule.key.name(), "key");

        // Check that the value is parsed as a complex value
        match &rule.value {
            crate::cwt::CwtValue::Identifier(identifier) => match identifier.identifier_type {
                crate::cwt::CwtReferenceType::AliasName
                | crate::cwt::CwtReferenceType::AliasMatchLeft => {
                    assert!(matches!(
                        identifier.identifier_type,
                        crate::cwt::CwtReferenceType::AliasMatchLeft
                    ));
                    assert_eq!(identifier.name.raw_value(), "trigger");
                }
                _ => panic!("Expected alias complex value type"),
            },
            _ => panic!("Expected complex value"),
        }
    }

    #[test]
    fn test_ai_budget() {
        let input = r#"
## replace_scope = { this = country root = country }
ai_budget = {
	## cardinality = 0..1
	weight = {
		enum[weight_or_base] = float
		alias_name[modifier_rule] = alias_match_left[modifier_rule]
	}

}        
"#;

        let _result = CwtModule::from_input(input).unwrap();
    }

    #[test]
    fn test_agreement_terms() {
        let input = r#"
agreement_term_value = {
	subtype[resource] = {
		resources = {
			<resource>
		}
	}
}
    "#;

        let _result = CwtModule::from_input(input).unwrap();
    }

    #[test]
    fn test_opinion_modifier() {
        let input = r#"
opinion_modifier = {
	subtype[!triggered_opinion_modifier] = {
		min = int[-9999...-1]
		max = int[1...999]
	}
}
        "#;

        let _result = CwtModule::from_input(input).unwrap();
    }

    #[test]
    fn test_astral_rift() {
        let input = r#"
astral_rift = {
	flags = {
		value_set[astral_rift_flag]
	}
}"#;

        let _result = CwtModule::from_input(input).unwrap();
    }

    #[test]
    fn test_building() {
        let input = r#"
building = {    
    icon = icon[gfx/interface/icons/buildings] #todo: filepath
}
        
"#;
        let _result = CwtModule::from_input(input).unwrap();
    }

    #[test]
    fn test_top_level_alias() {
        let input = r#"
alias[alert_category:category] = <alert_icon>
        "#;

        let _result = CwtModule::from_input(input).unwrap();
    }

    #[test]
    fn test_diplo_phase() {
        let input = r#"
diplo_phrase = {
	<diplomatic_action> = {
		enum[diplo_phrase_types] = {
			localisation = {}
		}
	}
}

"#;

        let _result = CwtModule::from_input(input).unwrap();
    }

    #[test]
    fn test_scope_group() {
        let input = r#"
alias[fleet_action:move_to] = scope_group[celestial_coordinate]
"#;

        let _result = CwtModule::from_input(input).unwrap();
    }

    #[test]
    fn test_job() {
        let input = r#"
job = {
	swappable_data = {
		swap_type = {
			icon = filepath[gfx/interface/icons/jobs/job_,.dds]
		}
	}
}
"#;

        let _result = CwtModule::from_input(input).unwrap();
    }

    #[test]
    fn test_planet_class() {
        let input = r#"
planet_class = {
	atmosphere_color = colour[hsv]
}"#;

        let _result = CwtModule::from_input(input).unwrap();
    }

    #[test]
    fn test_int_value_field() {
        let input = "alias[trigger:pop_amount] == int_value_field";
        let _result = CwtModule::from_input(input).unwrap();
    }
}
