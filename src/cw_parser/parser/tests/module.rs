#[cfg(test)]
mod tests {
    use super::super::super::*;
    use nom_supreme::error::ErrorTree;

    #[test]
    fn module_with_entities() {
        let input = r#"
        entity1 = { prop1 = value1 }
        entity2 = { prop2 = value2 }
    "#;
        let (_, (properties, values)) = module::<ErrorTree<_>>(input, "my_module").unwrap();

        assert_eq!(values, vec![]);
        assert_eq!(
            properties,
            ParsedProperties {
                is_module: true,
                kv: vec![
                    (
                        "entity1",
                        ParsedEntity::new()
                            .with_property("prop1", ParsedValue::String("value1"))
                            .into(),
                    ),
                    (
                        "entity2",
                        ParsedEntity::new()
                            .with_property("prop2", ParsedValue::String("value2"))
                            .into(),
                    ),
                ]
                .into_iter()
                .collect()
            }
        );
    }

    #[test]
    fn module_with_defines() {
        let input = r#"
        @MY_DEFINE = 123
        @ANOTHER_DEFINE = "hello"
    "#;
        let (_, (properties, values)) = module::<ErrorTree<_>>(input, "my_module").unwrap();
        assert_eq!(values, vec![]);
        assert_eq!(
            properties,
            ParsedProperties {
                is_module: true,
                kv: vec![
                    ("@MY_DEFINE", ParsedValue::Number("123")),
                    ("@ANOTHER_DEFINE", ParsedValue::String("hello")),
                ]
                .into_iter()
                .collect()
            }
        );
    }

    #[test]
    fn module_with_properties() {
        let input = r#"
        @MY_DEFINE = 123
        my_var1 = value1
        entity = {
            prop1 = value1
        }
    "#;
        let (_, (properties, values)) = module::<ErrorTree<_>>(input, "my_module").unwrap();
        assert_eq!(values, vec![]);
        assert_eq!(
            properties,
            ParsedProperties {
                is_module: true,
                kv: vec![
                    ("@MY_DEFINE", ParsedValue::Number("123")),
                    ("my_var1", ParsedValue::String("value1")),
                    (
                        "entity",
                        ParsedEntity::new()
                            .with_property("prop1", ParsedValue::String("value1"))
                            .into()
                    ),
                ]
                .into_iter()
                .collect()
            }
        );
    }

    #[test]
    fn module_with_dynamic_scripting() {
        let input = r#"
        revolt_situation_low_stability_factor = { # 0.2 for each point below 25
            base = @stabilitylevel2
            subtract = trigger:planet_stability
            [[ALTERED_STABILITY]
                subtract = $ALTERED_STABILITY$
            ]
            mult = 0.2
        }
    "#;

        let (_, (properties, values)) = module::<ErrorTree<_>>(input, "my_module").unwrap();

        assert_eq!(values, vec![]);
        assert_eq!(
            properties,
            ParsedProperties {
                is_module: true,
                kv: vec![(
                    "revolt_situation_low_stability_factor",
                    ParsedEntity::new()
                        .with_property("base", ParsedValue::Define("@stabilitylevel2"))
                        .with_property("subtract", ParsedValue::String("trigger:planet_stability"))
                        .with_property("mult", ParsedValue::Number("0.2"))
                        .with_conditional(ParsedConditionalBlock {
                            items: vec![],
                            key: (false, "ALTERED_STABILITY"),
                            properties: vec![(
                                "subtract",
                                ParsedPropertyInfoList::new().with_property(
                                    Operator::Equals,
                                    ParsedValue::String("$ALTERED_STABILITY$")
                                )
                            )]
                            .into_iter()
                            .collect(),
                        })
                        .into()
                ),]
                .into_iter()
                .collect()
            }
        );
    }

    #[test]
    fn module_with_value_list() {
        let input = r#"
            weapon_type_energy
            weapon_type_kinetic
            weapon_type_explosive
            weapon_type_strike_craft
        "#;

        let (_, (properties, values)) = module::<ErrorTree<_>>(input, "my_module").unwrap();

        assert_eq!(properties, HashMap::new());

        assert_eq!(
            values,
            vec![
                ParsedValue::String("weapon_type_energy"),
                ParsedValue::String("weapon_type_kinetic"),
                ParsedValue::String("weapon_type_explosive"),
                ParsedValue::String("weapon_type_strike_craft"),
            ]
        );
    }

    #[test]
    fn empty_module() {
        let input = r#""#;

        let (_, (properties, values)) = module::<ErrorTree<_>>(input, "my_module").unwrap();

        assert_eq!(properties, HashMap::new());

        assert_eq!(values, vec![]);
    }

    #[test]
    fn commented_out_module() {
        let input = r#"
            # @foo = 1
        "#;

        let (_, (properties, values)) = module::<ErrorTree<_>>(input, "my_module").unwrap();

        assert_eq!(properties, HashMap::new());

        assert_eq!(values, vec![]);
    }

    #[test]
    fn commented_out_module_2() {
        let input = r#"# @foo = 1"#;

        let (_, (properties, values)) = module::<ErrorTree<_>>(input, "my_module").unwrap();

        assert_eq!(properties, HashMap::new());

        assert_eq!(values, vec![]);
    }

    #[test]
    fn handle_bom() {
        let input = "\u{feff}# Comment";

        let (_, (properties, values)) = module::<ErrorTree<_>>(input, "my_module").unwrap();

        assert_eq!(properties, HashMap::new());

        assert_eq!(values, vec![]);
    }

    #[test]
    fn handle_readme() {
        let input = r#"
            Special variables for Edicts (Country and Empire):
            # cost, base cost as in resource(s) and amount for activating the edict.
        "#;

        let (_, (properties, values)) = module::<ErrorTree<_>>(input, "99_README_ETC").unwrap();

        assert_eq!(properties, HashMap::new());

        assert_eq!(values, vec![]);
    }
}
