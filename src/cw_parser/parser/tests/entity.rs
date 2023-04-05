#[cfg(test)]
mod tests {
    use crate::cw_model::Entity;

    use super::super::super::*;
    use nom_supreme::error::ErrorTree;

    #[test]
    fn empty_entity() {
        let input = "{}";
        let (_, result) = entity::<ErrorTree<_>>(input).unwrap();
        assert_eq!(result, Entity::new().into());
    }

    #[test]
    fn entity_with_property() {
        let input = "{ my_var = value }";
        let (_, result) = entity::<ErrorTree<_>>(input).unwrap();
        assert_eq!(
            result,
            Entity::new()
                .with_property("my_var", cw_model::Value::String("value".to_string()))
                .into()
        );
    }

    #[test]
    fn entity_with_many_properties() {
        let input = "{ my_var1 = value1\nmy_var2 = value2 my_var3 = value3 }";
        let (_, result) = entity::<ErrorTree<_>>(input).unwrap();
        assert_eq!(
            result,
            Entity::new()
                .with_property("my_var1", cw_model::Value::String("value1".to_string()))
                .with_property("my_var2", cw_model::Value::String("value2".to_string()))
                .with_property("my_var3", cw_model::Value::String("value3".to_string()))
                .into()
        );
    }

    #[test]
    fn entity_with_mixed_properties() {
        let input = r#"{
            float_val = 123.4
            int_val = 12 str_val1 = value3#comment
            str_val2 = "value4"
            color_val = rgb { 1 2 3 }
        }"#;
        let (_, result) = entity::<ErrorTree<_>>(input).unwrap();
        assert_eq!(
            result,
            Entity::new()
                .with_property("float_val", cw_model::Value::Number("123.4".to_owned()))
                .with_property("int_val", cw_model::Value::Number("12.0".to_owned()))
                .with_property("str_val1", cw_model::Value::String("value3".to_string()))
                .with_property("str_val2", cw_model::Value::String("value4".to_string()))
                .with_property(
                    "color_val",
                    cw_model::Value::Color((
                        "rgb".to_string(),
                        "1.0".to_owned(),
                        "2.0".to_owned(),
                        "3.0".to_owned(),
                        None
                    ))
                )
                .into()
        );
    }

    #[test]
    fn entity_with_item() {
        let input = "{ value }";
        let (_, result) = entity::<ErrorTree<_>>(input).unwrap();
        assert_eq!(
            result,
            Entity::new()
                .with_item(cw_model::Value::String("value".to_string()))
                .into()
        );
    }

    #[test]
    fn entity_with_many_items() {
        let input = "{ value1 value2 value3 }";
        let (_, result) = entity::<ErrorTree<_>>(input).unwrap();
        assert_eq!(
            result,
            Entity::new()
                .with_item(cw_model::Value::String("value1".to_string()))
                .with_item(cw_model::Value::String("value2".to_string()))
                .with_item(cw_model::Value::String("value3".to_string()))
                .into()
        );
    }

    #[test]
    fn entity_with_color_item() {
        let input = "{ rgb { 1 2 3 } }";
        let (_, result) = entity::<ErrorTree<_>>(input).unwrap();
        assert_eq!(
            result,
            Entity::new()
                .with_item(cw_model::Value::Color((
                    "rgb".to_string(),
                    "1.0".to_owned(),
                    "2.0".to_owned(),
                    "3.0".to_owned(),
                    None
                )))
                .into()
        );
    }

    #[test]
    fn entity_with_color_items() {
        let input = "{ rgb { 1 2 3 } rgb { 4 5 6 } }";
        let (_, result) = entity::<ErrorTree<_>>(input).unwrap();
        assert_eq!(
            result,
            Entity::new()
                .with_item(cw_model::Value::Color((
                    "rgb".to_string(),
                    "1.0".to_owned(),
                    "2.0".to_owned(),
                    "3.0".to_owned(),
                    None
                )))
                .with_item(cw_model::Value::Color((
                    "rgb".to_string(),
                    "4.0".to_owned(),
                    "5.0".to_owned(),
                    "6.0".to_owned(),
                    None
                )))
                .into()
        );
    }

    #[test]
    fn entity_with_mixed_items() {
        let input = "{ value1 rgb { 1 2 3 } value2 }";
        let (_, result) = entity::<ErrorTree<_>>(input).unwrap();
        assert_eq!(
            result,
            Entity::new()
                .with_item(cw_model::Value::String("value1".to_string()))
                .with_item(cw_model::Value::Color((
                    "rgb".to_string(),
                    "1.0".to_owned(),
                    "2.0".to_owned(),
                    "3.0".to_owned(),
                    None
                )))
                .with_item(cw_model::Value::String("value2".to_string()))
                .into()
        );
    }

    #[test]
    fn entity_with_values_and_properties() {
        let input = "{ value1 my_var = value2 }";
        let (_, result) = entity::<ErrorTree<_>>(input).unwrap();
        assert_eq!(
            result,
            Entity::new()
                .with_item(cw_model::Value::String("value1".to_string()))
                .with_property("my_var", cw_model::Value::String("value2".to_string()))
                .into()
        );
    }

    #[test]
    fn entity_with_many_values_and_properties() {
        let input = r#"{
            value1
            my_var1 = value2
            value3 # comment
            my_var2 = value4
        }"#;
        let (_, result) = entity::<ErrorTree<_>>(input).unwrap();
        assert_eq!(
            result,
            Entity::new()
                .with_item(cw_model::Value::String("value1".to_string()))
                .with_property("my_var1", cw_model::Value::String("value2".to_string()))
                .with_item(cw_model::Value::String("value3".to_string()))
                .with_property("my_var2", cw_model::Value::String("value4".to_string()))
                .into()
        );
    }

    #[test]
    fn entity_with_entity_properties() {
        let input = r#"{
            my_var1 = { value1 }
            my_var2 = { value2 }
        }"#;
        let (_, result) = entity::<ErrorTree<_>>(input).unwrap();
        assert_eq!(
            result,
            Entity::new()
                .with_property(
                    "my_var1",
                    cw_model::Value::Entity(
                        Entity::new()
                            .with_item(cw_model::Value::String("value1".to_string()))
                            .into()
                    )
                )
                .with_property(
                    "my_var2",
                    cw_model::Value::Entity(
                        Entity::new()
                            .with_item(cw_model::Value::String("value2".to_string()))
                            .into()
                    )
                )
                .into()
        );
    }

    #[test]
    fn entity_with_entity_values() {
        let input = r#"{
            { value1 }
            { value2 }
        }"#;
        let (_, result) = entity::<ErrorTree<_>>(input).unwrap();
        assert_eq!(
            result,
            Entity::new()
                .with_item(cw_model::Value::Entity(
                    Entity::new()
                        .with_item(cw_model::Value::String("value1".to_string()))
                        .into()
                ))
                .with_item(cw_model::Value::Entity(
                    Entity::new()
                        .with_item(cw_model::Value::String("value2".to_string()))
                        .into()
                ))
                .into()
        );
    }

    #[test]
    fn invalid_entity_missing_opening_bracket() {
        let input = " my_var = value }";
        assert!(entity::<ErrorTree<_>>(input).is_err());
    }

    #[test]
    fn invalid_entity_missing_closing_bracket() {
        let input = "{ my_var = value ";
        assert!(entity::<ErrorTree<_>>(input).is_err());
    }

    #[test]
    fn entity_with_mixed_color_values() {
        let input = r#"{
        color1 = rgb { 255 0 0 }
        color2 = rgb { 0 255 0 0.5 }
        color3 = hsv { 120 50 100 }
    }"#;
        let (_, result) = entity::<ErrorTree<_>>(input).unwrap();
        assert_eq!(
            result,
            Entity::new()
                .with_property(
                    "color1",
                    cw_model::Value::Color((
                        "rgb".to_string(),
                        "255.0".to_owned(),
                        "0.0".to_owned(),
                        "0.0".to_owned(),
                        None
                    ))
                )
                .with_property(
                    "color2",
                    cw_model::Value::Color((
                        "rgb".to_string(),
                        "0.0".to_owned(),
                        "255.0".to_owned(),
                        "0.0".to_owned(),
                        Some("0.5".to_owned())
                    ))
                )
                .with_property(
                    "color3",
                    cw_model::Value::Color((
                        "hsv".to_string(),
                        "120.0".to_owned(),
                        "50.0".to_owned(),
                        "100.0".to_owned(),
                        None
                    ))
                )
                .into()
        );
    }

    #[test]
    fn entity_with_comments() {
        let input = r#"{#comment
        #comment
        my_var1 = value1 # comment1
        # comment2
        my_var2 = value2
        my_var3 = value3 # comment3
    }"#;
        let (_, result) = entity::<ErrorTree<_>>(input).unwrap();
        assert_eq!(
            result,
            Entity::new()
                .with_property("my_var1", cw_model::Value::String("value1".to_string()))
                .with_property("my_var2", cw_model::Value::String("value2".to_string()))
                .with_property("my_var3", cw_model::Value::String("value3".to_string()))
                .into()
        );
    }

    #[test]
    fn entity_with_complex_input() {
        let input = r#"{
        # my_var1 = "value1 with space and \"special\" chars"
        my_var2 = { nested_var = { deep_var = "deep_value" } }
        my_var3 = rgb { 255 0 0 }
        # my_var4 = value_with_escape_characters\nand\ttabs
        my_var5 = { nested_entity1 nested_entity2 }
        my_var6 = value6
        my_var7 = value7
        my_var8 = { nested_var = value8 }
        my_var9 = value9
    }"#;
        let (_, result) = entity::<ErrorTree<_>>(input).unwrap();
        assert_eq!(
            result,
            Entity::new()
                // .with_property(
                //     "my_var1",
                //     cw_model::Value::String("value1 with space and \"special\" chars".to_string())
                // )
                .with_property(
                    "my_var2",
                    cw_model::Value::Entity(
                        Entity::new()
                            .with_property(
                                "nested_var",
                                cw_model::Value::Entity(
                                    Entity::new()
                                        .with_property(
                                            "deep_var",
                                            cw_model::Value::String("deep_value".to_string())
                                        )
                                        .into()
                                )
                            )
                            .into()
                    )
                )
                .with_property(
                    "my_var3",
                    cw_model::Value::Color((
                        "rgb".to_string(),
                        "255.0".to_owned(),
                        "0.0".to_owned(),
                        "0.0".to_owned(),
                        None
                    ))
                )
                // .with_property(
                //     "my_var4",
                //     cw_model::Value::String("value_with_escape_characters\nand\ttabs".to_string())
                // )
                .with_property(
                    "my_var5",
                    cw_model::Value::Entity(
                        Entity::new()
                            .with_item(cw_model::Value::String("nested_entity1".to_string()))
                            .with_item(cw_model::Value::String("nested_entity2".to_string()))
                    )
                )
                .with_property("my_var6", cw_model::Value::String("value6".to_string()))
                .with_property("my_var7", cw_model::Value::String("value7".to_string()))
                .with_property(
                    "my_var8",
                    cw_model::Value::Entity(
                        Entity::new()
                            .with_property(
                                "nested_var",
                                cw_model::Value::String("value8".to_string())
                            )
                            .into()
                    )
                )
                .with_property("my_var9", cw_model::Value::String("value9".to_string()))
                .into()
        );
    }

    #[test]
    fn entity_with_duplicate_properties_adds_to_array_at_key() {
        let input = r#"{
        my_var = value1
        my_var = value2
        my_var = value3
    }"#;
        let (_, result) = entity::<ErrorTree<_>>(input).unwrap();
        assert_eq!(
            result,
            Entity::new()
                .with_property_values(
                    "my_var",
                    vec![
                        cw_model::Value::String("value1".to_string()),
                        cw_model::Value::String("value2".to_string()),
                        cw_model::Value::String("value3".to_string()),
                    ]
                )
                .into()
        );
    }

    #[test]
    fn entity_with_dynamic_scripting() {
        let input = r#"{ # 0.2 for each point below 25
            base = @stabilitylevel2
            subtract = trigger:planet_stability
            [[ALTERED_STABILITY]
                subtract = $ALTERED_STABILITY$
            ]
            mult = 0.2
        }"#;

        let (_, result) = entity::<ErrorTree<_>>(input).unwrap();

        assert_eq!(
            result,
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
        )
    }

    #[test]
    fn entity_with_define_value() {
        let input = r#"{
            val = @stabilitylevel2
        }"#;

        let (_, result) = entity::<ErrorTree<_>>(input).unwrap();

        assert_eq!(
            result,
            Entity::new()
                .with_property(
                    "val",
                    cw_model::Value::Define("@stabilitylevel2".to_string())
                )
                .into()
        );
    }

    #[test]
    fn compact_equality() {
        let input = r#"{
            mesh="asteroid_01_mesh"
        }"#;

        let (_, result) = entity::<ErrorTree<_>>(input).unwrap();

        assert_eq!(
            result,
            Entity::new()
                .with_property(
                    "mesh",
                    cw_model::Value::String("asteroid_01_mesh".to_string())
                )
                .into()
        );
    }

    #[test]
    fn switch_statement() {
        let input = r#"{
            trigger = free_housing
            -9 < { nine = yes } # 10
            -8 < { eight = yes } # 9
		}"#;

        let (_, result) = entity::<ErrorTree<_>>(input).unwrap();

        assert_eq!(
            result,
            Entity::new()
                .with_property(
                    "trigger",
                    cw_model::Value::String("free_housing".to_string())
                )
                .with_property_with_operator(
                    "-9",
                    cw_model::Operator::LessThan,
                    cw_model::Value::Entity(
                        Entity::new()
                            .with_property("nine", cw_model::Value::String("yes".to_string()))
                            .into()
                    )
                )
                .with_property_with_operator(
                    "-8",
                    cw_model::Operator::LessThan,
                    cw_model::Value::Entity(
                        Entity::new()
                            .with_property("eight", cw_model::Value::String("yes".to_string()))
                            .into()
                    )
                )
                .into()
        );
    }

    #[test]
    fn inline_maths() {
        let input = r#"{
			planet_stability < @[ stabilitylevel2 + 10 ]
        }"#;

        let (_, result) = entity::<ErrorTree<_>>(input).unwrap();

        assert_eq!(
            result,
            Entity::new()
                .with_property_with_operator(
                    "planet_stability",
                    cw_model::Operator::LessThan,
                    cw_model::Value::Maths("@[ stabilitylevel2 + 10 ]".to_string())
                )
                .into()
        );
    }

    #[test]
    fn inline_maths_alt() {
        let input = r#"{
			planet_stability < @\[ stabilitylevel2 + 10 ]
        }"#;

        let (_, result) = entity::<ErrorTree<_>>(input).unwrap();

        assert_eq!(
            result,
            Entity::new()
                .with_property_with_operator(
                    "planet_stability",
                    cw_model::Operator::LessThan,
                    cw_model::Value::Maths("@\\[ stabilitylevel2 + 10 ]".to_string())
                )
                .into()
        );
    }
}
