#[derive(Debug, Clone, PartialEq)]
pub struct Modifier {
    pub name: String,
    pub categories: Vec<String>,
}

impl Modifier {
    pub fn new(name: String, categories: Vec<String>) -> Self {
        Self { name, categories }
    }
}

pub fn parse_modifier_log(log_content: &str) -> Vec<Modifier> {
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
            if let Some(modifier) = parse_modifier_line(line) {
                modifiers.push(modifier);
            }
        }
    }

    modifiers
}

fn parse_modifier_line(line: &str) -> Option<Modifier> {
    // Remove the "- " prefix
    let line = line.strip_prefix("- ")?;

    // Split by ", Category: " to separate name and categories
    let parts: Vec<&str> = line.split(", Category: ").collect();

    if parts.len() == 2 {
        let name = parts[0].trim().to_string();
        let categories_str = parts[1].trim();

        // Split categories by comma and trim whitespace
        let categories: Vec<String> = categories_str
            .split(',')
            .map(|cat| cat.trim().to_string())
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

        let modifiers = parse_modifier_log(log_content);

        assert_eq!(modifiers.len(), 7);
        assert_eq!(
            modifiers[0],
            Modifier::new("blank_modifier".to_string(), vec!["Pops".to_string()])
        );
        assert_eq!(
            modifiers[1],
            Modifier::new("pop_political_power".to_string(), vec!["Pops".to_string()])
        );
        assert_eq!(
            modifiers[5],
            Modifier::new(
                "pop_environment_tolerance".to_string(),
                vec!["Habitability".to_string()]
            )
        );
        assert_eq!(
            modifiers[6],
            Modifier::new(
                "ship_hull_add".to_string(),
                vec![
                    "Orbital Stations".to_string(),
                    "Space Stations".to_string(),
                    "Military Ships".to_string()
                ]
            )
        );
    }

    #[test]
    fn test_parse_modifier_line() {
        let line = "- pop_happiness, Category: Pops";
        let modifier = parse_modifier_line(line).unwrap();

        assert_eq!(modifier.name, "pop_happiness");
        assert_eq!(modifier.categories, vec!["Pops".to_string()]);
    }

    #[test]
    fn test_parse_multiple_categories() {
        let line = "- ship_hull_add, Category: Orbital Stations, Space Stations, Military Ships, Civilian Ships, Science Ships, Transport Ships, Ship Design Stats";
        let modifier = parse_modifier_line(line).unwrap();

        assert_eq!(modifier.name, "ship_hull_add");
        assert_eq!(
            modifier.categories,
            vec![
                "Orbital Stations".to_string(),
                "Space Stations".to_string(),
                "Military Ships".to_string(),
                "Civilian Ships".to_string(),
                "Science Ships".to_string(),
                "Transport Ships".to_string(),
                "Ship Design Stats".to_string()
            ]
        );
    }

    #[test]
    fn test_parse_invalid_line() {
        let line = "- invalid_line_format";
        let result = parse_modifier_line(line);

        assert!(result.is_none());
    }
}
