#[cfg(test)]
mod tests {
    use crate::cw_model::Entity;

    use super::super::super::*;
    use nom_supreme::error::ErrorTree;

    #[test]
    fn module_with_entities() {
        let input = r#"
        entity1 = { prop1 = value1 }
        entity2 = { prop2 = value2 }
    "#;
        let (_, result) = module::<ErrorTree<_>>(input, "my/type/path", "my_module").unwrap();
        assert_eq!(
            result,
            cw_model::Module {
                namespace: "my/type/path".to_string(),
                filename: "my_module".to_string(),
                values: vec![],
                properties: vec![
                    (
                        "entity1".to_string(),
                        Entity::new()
                            .with_property("prop1", cw_model::Value::String("value1".to_string()))
                            .into(),
                    ),
                    (
                        "entity2".to_string(),
                        Entity::new()
                            .with_property("prop2", cw_model::Value::String("value2".to_string()))
                            .into(),
                    ),
                ]
                .into_iter()
                .collect(),
                defines: HashMap::new(),
            }
        );
    }

    #[test]
    fn module_with_defines() {
        let input = r#"
        @MY_DEFINE = 123
        @ANOTHER_DEFINE = "hello"
    "#;
        let (_, result) = module::<ErrorTree<_>>(input, "my/type/path", "my_module").unwrap();
        assert_eq!(
            result,
            cw_model::Module {
                namespace: "my/type/path".to_string(),
                filename: "my_module".to_string(),
                values: vec![],
                defines: vec![
                    (
                        "@MY_DEFINE".to_string(),
                        cw_model::Value::Number("123".to_owned())
                    ),
                    (
                        "@ANOTHER_DEFINE".to_string(),
                        cw_model::Value::String("hello".to_string())
                    ),
                ]
                .into_iter()
                .collect(),
                properties: HashMap::new(),
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
        let (_, result) = module::<ErrorTree<_>>(input, "my/type/path", "my_module").unwrap();
        assert_eq!(
            result,
            cw_model::Module {
                namespace: "my/type/path".to_string(),
                filename: "my_module".to_string(),
                values: vec![],
                defines: vec![(
                    "@MY_DEFINE".to_string(),
                    cw_model::Value::Number("123".to_owned())
                )]
                .into_iter()
                .collect(),
                properties: vec![
                    (
                        "entity".to_string(),
                        Entity::new()
                            .with_property("prop1", cw_model::Value::String("value1".to_string()))
                            .into()
                    ),
                    (
                        "my_var1".to_string(),
                        cw_model::Value::String("value1".to_string()).into()
                    ),
                ]
                .into_iter()
                .collect::<HashMap<String, PropertyInfoList>>(),
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

        let (_, result) = module::<ErrorTree<_>>(input, "my/type/path", "my_module").unwrap();

        assert_eq!(
            result,
            cw_model::Module {
                namespace: "my/type/path".to_string(),
                filename: "my_module".to_string(),
                values: vec![],
                defines: HashMap::new(),
                properties: vec![(
                    "revolt_situation_low_stability_factor".to_string(),
                    Entity::new()
                        .with_property(
                            "base",
                            cw_model::Value::Define("@stabilitylevel2".to_string())
                        )
                        .with_property(
                            "subtract",
                            cw_model::Value::String("trigger:planet_stability".to_string())
                        )
                        .with_property("mult", cw_model::Value::Number("0.2".to_owned()))
                        .with_conditional(cw_model::ConditionalBlock {
                            items: vec![],
                            key: (false, "ALTERED_STABILITY".to_string()),
                            properties: vec![(
                                "subtract".to_string(),
                                PropertyInfoList::new().with_property(
                                    cw_model::Operator::Equals,
                                    cw_model::Value::String("$ALTERED_STABILITY$".to_string())
                                )
                            )]
                            .into_iter()
                            .collect(),
                        })
                        .into()
                ),]
                .into_iter()
                .collect(),
            }
        )
    }

    #[test]
    fn module_with_value_list() {
        let input = r#"
            weapon_type_energy
            weapon_type_kinetic
            weapon_type_explosive
            weapon_type_strike_craft
        "#;

        let (_, result) = module::<ErrorTree<_>>(input, "my/type/path", "my_module").unwrap();

        assert_eq!(
            result,
            cw_model::Module {
                namespace: "my/type/path".to_string(),
                filename: "my_module".to_string(),
                values: vec![
                    cw_model::Value::String("weapon_type_energy".to_string()),
                    cw_model::Value::String("weapon_type_kinetic".to_string()),
                    cw_model::Value::String("weapon_type_explosive".to_string()),
                    cw_model::Value::String("weapon_type_strike_craft".to_string()),
                ],
                defines: HashMap::new(),
                properties: HashMap::new(),
            }
        );
    }

    #[test]
    fn empty_module() {
        let input = r#""#;

        let (_, result) = module::<ErrorTree<_>>(input, "my/type/path", "my_module").unwrap();

        assert_eq!(
            result,
            cw_model::Module {
                namespace: "my/type/path".to_string(),
                filename: "my_module".to_string(),
                values: vec![],
                defines: HashMap::new(),
                properties: HashMap::new(),
            }
        );
    }

    #[test]
    fn commented_out_module() {
        let input = r#"
            # @foo = 1
        "#;

        let (_, result) = module::<ErrorTree<_>>(input, "my/type/path", "my_module").unwrap();

        assert_eq!(
            result,
            cw_model::Module {
                namespace: "my/type/path".to_string(),
                filename: "my_module".to_string(),
                values: vec![],
                defines: HashMap::new(),
                properties: HashMap::new(),
            }
        );
    }

    #[test]
    fn commented_out_module_2() {
        let input = r#"# @foo = 1"#;

        let (_, result) = module::<ErrorTree<_>>(input, "my/type/path", "my_module").unwrap();

        assert_eq!(
            result,
            cw_model::Module {
                namespace: "my/type/path".to_string(),
                filename: "my_module".to_string(),
                values: vec![],
                defines: HashMap::new(),
                properties: HashMap::new(),
            }
        );
    }

    #[test]
    fn handle_bom() {
        let input = "\u{feff}# Comment";

        let (_, result) = module::<ErrorTree<_>>(input, "my/type/path", "my_module").unwrap();

        assert_eq!(
            result,
            cw_model::Module {
                namespace: "my/type/path".to_string(),
                filename: "my_module".to_string(),
                values: vec![],
                defines: HashMap::new(),
                properties: HashMap::new(),
            }
        );
    }

    #[test]
    fn handle_readme() {
        let input = r#"
            Special variables for Edicts (Country and Empire):
            # cost, base cost as in resource(s) and amount for activating the edict.
        "#;

        let (_, result) = module::<ErrorTree<_>>(input, "my/type/path", "99_README_ETC").unwrap();

        assert_eq!(
            result,
            cw_model::Module {
                namespace: "my/type/path".to_string(),
                filename: "99_README_ETC".to_string(),
                values: vec![],
                defines: HashMap::new(),
                properties: HashMap::new(),
            }
        );
    }
}
