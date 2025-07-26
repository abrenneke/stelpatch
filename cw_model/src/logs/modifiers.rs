use lasso::{Spur, ThreadedRodeo};

#[derive(Debug, Clone, PartialEq)]
pub struct Modifier {
    pub name: Spur,
    pub categories: Vec<Spur>,
}

impl Modifier {
    pub fn new(name: Spur, categories: Vec<Spur>) -> Self {
        Self { name, categories }
    }
}

pub fn parse_modifier_log(log_content: &str, interner: &ThreadedRodeo) -> Vec<Modifier> {
    let mut modifiers = Vec::new();
    let mut found_definitions = false;

    for line in log_content.lines() {
        let line = line.trim();

        // Look for the start of modifier definitions
        if line.starts_with("Printing Modifier Definitions:") {
            found_definitions = true;
            continue;
        }

        // Skip lines until we find the definitions section
        if !found_definitions {
            continue;
        }

        // Parse modifier entries that start with "- "
        if line.starts_with("- ") {
            if let Some(modifier) = parse_modifier_line(line, &interner) {
                modifiers.push(modifier);
            }
        }
    }

    modifiers
}

fn parse_modifier_line(line: &str, interner: &ThreadedRodeo) -> Option<Modifier> {
    // Remove the "- " prefix
    let line = line.strip_prefix("- ")?;

    // Split by ", Category: " to separate name and categories
    let parts: Vec<&str> = line.split(", Category: ").collect();

    if parts.len() == 2 {
        let name = interner.get_or_intern(parts[0].trim());
        let categories_str = parts[1].trim();

        // Split categories by comma and trim whitespace
        let categories: Vec<Spur> = categories_str
            .split(',')
            .map(|cat| interner.get_or_intern(cat.trim()))
            .collect();

        Some(Modifier::new(name, categories))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_modifier_log() {
        let interner = ThreadedRodeo::new();
        let log_content = r#"[13:52:17][modifier.cpp:1999]: 
 == MODIFIER DOCUMENTATION ==
Note on Modifier Categories:
-- These are used internally as a tag to suggest where they are intended to be used, but exact behavior depends upon individual implementation.
-- Pop scope modifiers will generally work on e.g. planets and countries as well as pops, in which case they will apply to all pops belonging to that object.

Printing Modifier Definitions:
- blank_modifier, Category: Pops
- pop_political_power, Category: Pops
- pop_happiness, Category: Pops
- pop_housing_usage_base, Category: Pops
- pop_amenities_usage_mult, Category: Pops
- pop_environment_tolerance, Category: Habitability
- ship_hull_add, Category: Orbital Stations, Space Stations, Military Ships
"#;

        let modifiers = parse_modifier_log(log_content, &interner);

        assert_eq!(modifiers.len(), 7);
        assert_eq!(
            modifiers[0],
            Modifier::new(
                interner.get_or_intern("blank_modifier"),
                vec![interner.get_or_intern("Pops")]
            )
        );
        assert_eq!(
            modifiers[1],
            Modifier::new(
                interner.get_or_intern("pop_political_power"),
                vec![interner.get_or_intern("Pops")]
            )
        );
        assert_eq!(
            modifiers[5],
            Modifier::new(
                interner.get_or_intern("pop_environment_tolerance"),
                vec![interner.get_or_intern("Habitability")]
            )
        );
        assert_eq!(
            modifiers[6],
            Modifier::new(
                interner.get_or_intern("ship_hull_add"),
                vec![
                    interner.get_or_intern("Orbital Stations"),
                    interner.get_or_intern("Space Stations"),
                    interner.get_or_intern("Military Ships")
                ]
            )
        );
    }

    #[test]
    fn test_parse_modifier_line() {
        let interner = ThreadedRodeo::new();
        let line = "- pop_happiness, Category: Pops";
        let modifier = parse_modifier_line(line, &interner).unwrap();

        assert_eq!(modifier.name, interner.get_or_intern("pop_happiness"));
        assert_eq!(modifier.categories, vec![interner.get_or_intern("Pops")]);
    }

    #[test]
    fn test_parse_multiple_categories() {
        let interner = ThreadedRodeo::new();
        let line = "- ship_hull_add, Category: Orbital Stations, Space Stations, Military Ships, Civilian Ships, Science Ships, Transport Ships, Ship Design Stats";
        let modifier = parse_modifier_line(line, &interner).unwrap();

        assert_eq!(modifier.name, interner.get_or_intern("ship_hull_add"));
        assert_eq!(
            modifier.categories,
            vec![
                interner.get_or_intern("Orbital Stations"),
                interner.get_or_intern("Space Stations"),
                interner.get_or_intern("Military Ships"),
                interner.get_or_intern("Civilian Ships"),
                interner.get_or_intern("Science Ships"),
                interner.get_or_intern("Transport Ships"),
                interner.get_or_intern("Ship Design Stats")
            ]
        );
    }

    #[test]
    fn test_parse_invalid_line() {
        let interner = ThreadedRodeo::new();
        let line = "- invalid_line_format";
        let result = parse_modifier_line(line, &interner);

        assert!(result.is_none());
    }
}
