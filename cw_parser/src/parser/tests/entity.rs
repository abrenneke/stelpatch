#[cfg(test)]
mod tests {
    use super::super::super::*;

    #[test]
    fn empty_entity() {
        let input = LocatingSlice::new("{}");
        let result = entity.parse(input).unwrap();
        assert_eq!(result, ParsedEntity::new().into());
    }

    #[test]
    fn entity_with_property() {
        let input = LocatingSlice::new("{ my_var = value }");
        let result = entity.parse(input).unwrap();
        assert_eq!(
            result,
            ParsedEntity::new()
                .with_property("my_var", ParsedValue::String("value"))
                .into()
        );
    }

    #[test]
    fn entity_with_many_properties() {
        let input = LocatingSlice::new("{ my_var1 = value1\nmy_var2 = value2 my_var3 = value3 }");
        let result = entity.parse(input).unwrap();
        assert_eq!(
            result,
            ParsedEntity::new()
                .with_property("my_var1", ParsedValue::String("value1"))
                .with_property("my_var2", ParsedValue::String("value2"))
                .with_property("my_var3", ParsedValue::String("value3"))
                .into()
        );
    }

    #[test]
    fn entity_with_mixed_properties() {
        let input = LocatingSlice::new(
            r#"{
            float_val = 123.4
            int_val = 12 str_val1 = value3#comment
            str_val2 = "value4"
            color_val = rgb { 1 2 3 }
        }"#,
        );
        let result = entity.parse(input).unwrap();
        assert_eq!(
            result,
            ParsedEntity::new()
                .with_property("float_val", ParsedValue::Number("123.4"))
                .with_property("int_val", ParsedValue::Number("12"))
                .with_property("str_val1", ParsedValue::String("value3"))
                .with_property("str_val2", ParsedValue::String("value4"))
                .with_property(
                    "color_val",
                    ParsedValue::Color(("rgb", "1", "2", "3", None))
                )
                .into()
        );
    }

    #[test]
    fn entity_with_item() {
        let input = LocatingSlice::new("{ value }");
        let result = entity.parse(input).unwrap();
        assert_eq!(
            result,
            ParsedEntity::new()
                .with_item(ParsedValue::String("value"))
                .into()
        );
    }

    #[test]
    fn entity_with_many_items() {
        let input = LocatingSlice::new("{ value1 value2 value3 }");
        let result = entity.parse(input).unwrap();
        assert_eq!(
            result,
            ParsedEntity::new()
                .with_item(ParsedValue::String("value1"))
                .with_item(ParsedValue::String("value2"))
                .with_item(ParsedValue::String("value3"))
                .into()
        );
    }

    #[test]
    fn entity_with_color_item() {
        let input = LocatingSlice::new("{ rgb { 1 2 3 } }");
        let result = entity.parse(input).unwrap();
        assert_eq!(
            result,
            ParsedEntity::new()
                .with_item(ParsedValue::Color(("rgb", "1", "2", "3", None)))
                .into()
        );
    }

    #[test]
    fn entity_with_color_items() {
        let input = LocatingSlice::new("{ rgb { 1 2 3 } rgb { 4 5 6 } }");
        let result = entity.parse(input).unwrap();
        assert_eq!(
            result,
            ParsedEntity::new()
                .with_item(ParsedValue::Color(("rgb", "1", "2", "3", None)))
                .with_item(ParsedValue::Color(("rgb", "4", "5", "6", None)))
                .into()
        );
    }

    #[test]
    fn entity_with_mixed_items() {
        let input = LocatingSlice::new("{ value1 rgb { 1 2 3 } value2 }");
        let result = entity.parse(input).unwrap();
        assert_eq!(
            result,
            ParsedEntity::new()
                .with_item(ParsedValue::String("value1"))
                .with_item(ParsedValue::Color(("rgb", "1", "2", "3", None)))
                .with_item(ParsedValue::String("value2"))
                .into()
        );
    }

    #[test]
    fn entity_with_values_and_properties() {
        let input = LocatingSlice::new("{ value1 my_var = value2 }");
        let result = entity.parse(input).unwrap();
        assert_eq!(
            result,
            ParsedEntity::new()
                .with_item(ParsedValue::String("value1"))
                .with_property("my_var", ParsedValue::String("value2"))
                .into()
        );
    }

    #[test]
    fn entity_with_many_values_and_properties() {
        let input = LocatingSlice::new(
            r#"{
            value1
            my_var1 = value2
            value3 # comment
            my_var2 = value4
        }"#,
        );
        let result = entity.parse(input).unwrap();
        assert_eq!(
            result,
            ParsedEntity::new()
                .with_item(ParsedValue::String("value1"))
                .with_property("my_var1", ParsedValue::String("value2"))
                .with_item(ParsedValue::String("value3"))
                .with_property("my_var2", ParsedValue::String("value4"))
                .into()
        );
    }

    #[test]
    fn entity_with_entity_properties() {
        let input = LocatingSlice::new(
            r#"{
            my_var1 = { value1 }
            my_var2 = { value2 }
        }"#,
        );
        let result = entity.parse(input).unwrap();
        assert_eq!(
            result,
            ParsedEntity::new()
                .with_property(
                    "my_var1",
                    ParsedValue::Entity(
                        ParsedEntity::new()
                            .with_item(ParsedValue::String("value1"))
                            .into()
                    )
                )
                .with_property(
                    "my_var2",
                    ParsedValue::Entity(
                        ParsedEntity::new()
                            .with_item(ParsedValue::String("value2"))
                            .into()
                    )
                )
                .into()
        );
    }

    #[test]
    fn entity_with_entity_values() {
        let input = LocatingSlice::new(
            r#"{
            { value1 }
            { value2 }
        }"#,
        );
        let result = entity.parse(input).unwrap();
        assert_eq!(
            result,
            ParsedEntity::new()
                .with_item(ParsedValue::Entity(
                    ParsedEntity::new()
                        .with_item(ParsedValue::String("value1"))
                        .into()
                ))
                .with_item(ParsedValue::Entity(
                    ParsedEntity::new()
                        .with_item(ParsedValue::String("value2"))
                        .into()
                ))
                .into()
        );
    }

    #[test]
    fn invalid_entity_missing_opening_bracket() {
        let input = LocatingSlice::new(" my_var = value }");
        assert!(entity.parse(input).is_err());
    }

    #[test]
    fn invalid_entity_missing_closing_bracket() {
        let input = LocatingSlice::new("{ my_var = value ");
        assert!(entity.parse(input).is_err());
    }

    #[test]
    fn entity_with_mixed_color_values() {
        let input = LocatingSlice::new(
            r#"{
        color1 = rgb { 255 0 0 }
        color2 = rgb { 0 255 0 0.5 }
        color3 = hsv { 120 50 100 }
    }"#,
        );
        let result = entity.parse(input).unwrap();
        assert_eq!(
            result,
            ParsedEntity::new()
                .with_property("color1", ParsedValue::Color(("rgb", "255", "0", "0", None)))
                .with_property(
                    "color2",
                    ParsedValue::Color(("rgb", "0", "255", "0", Some("0.5")))
                )
                .with_property(
                    "color3",
                    ParsedValue::Color(("hsv", "120", "50", "100", None))
                )
                .into()
        );
    }

    #[test]
    fn entity_with_comments() {
        let input = LocatingSlice::new(
            r#"{#comment
        #comment
        my_var1 = value1 # comment1
        # comment2
        my_var2 = value2
        my_var3 = value3 # comment3
    }"#,
        );
        let result = entity.parse(input).unwrap();
        assert_eq!(
            result,
            ParsedEntity::new()
                .with_property("my_var1", ParsedValue::String("value1"))
                .with_property("my_var2", ParsedValue::String("value2"))
                .with_property("my_var3", ParsedValue::String("value3"))
                .into()
        );
    }

    #[test]
    fn entity_with_complex_input() {
        let input = LocatingSlice::new(
            r#"{
        # my_var1 = "value1 with space and \"special\" chars"
        my_var2 = { nested_var = { deep_var = "deep_value" } }
        my_var3 = rgb { 255 0 0 }
        # my_var4 = value_with_escape_characters\nand\ttabs
        my_var5 = { nested_entity1 nested_entity2 }
        my_var6 = value6
        my_var7 = value7
        my_var8 = { nested_var = value8 }
        my_var9 = value9
    }"#,
        );
        let result = entity.parse(input).unwrap();
        assert_eq!(
            result,
            ParsedEntity::new()
                // .with_property(
                //     "my_var1",
                //     ParsedValue::String("value1 with space and \"special\" chars")
                // )
                .with_property(
                    "my_var2",
                    ParsedValue::Entity(
                        ParsedEntity::new()
                            .with_property(
                                "nested_var",
                                ParsedValue::Entity(
                                    ParsedEntity::new()
                                        .with_property(
                                            "deep_var",
                                            ParsedValue::String("deep_value")
                                        )
                                        .into()
                                )
                            )
                            .into()
                    )
                )
                .with_property(
                    "my_var3",
                    ParsedValue::Color(("rgb", "255", "0", "0", None))
                )
                // .with_property(
                //     "my_var4",
                //     ParsedValue::String("value_with_escape_characters\nand\ttabs")
                // )
                .with_property(
                    "my_var5",
                    ParsedValue::Entity(
                        ParsedEntity::new()
                            .with_item(ParsedValue::String("nested_entity1"))
                            .with_item(ParsedValue::String("nested_entity2"))
                    )
                )
                .with_property("my_var6", ParsedValue::String("value6"))
                .with_property("my_var7", ParsedValue::String("value7"))
                .with_property(
                    "my_var8",
                    ParsedValue::Entity(
                        ParsedEntity::new()
                            .with_property("nested_var", ParsedValue::String("value8"))
                            .into()
                    )
                )
                .with_property("my_var9", ParsedValue::String("value9"))
                .into()
        );
    }

    #[test]
    fn entity_with_duplicate_properties_adds_to_array_at_key() {
        let input = LocatingSlice::new(
            r#"{
        my_var = value1
        my_var = value2
        my_var = value3
    }"#,
        );
        let result = entity.parse(input).unwrap();
        assert_eq!(
            result,
            ParsedEntity::new()
                .with_property_values(
                    "my_var",
                    vec![
                        ParsedValue::String("value1"),
                        ParsedValue::String("value2"),
                        ParsedValue::String("value3"),
                    ]
                )
                .into()
        );
    }

    #[test]
    fn entity_with_dynamic_scripting() {
        let input = LocatingSlice::new(
            r#"{ # 0.2 for each point below 25
            base = @stabilitylevel2
            subtract = trigger:planet_stability
            [[ALTERED_STABILITY]
                subtract = $ALTERED_STABILITY$
            ]
            mult = 0.2
        }"#,
        );

        let result = entity.parse(input).unwrap();

        assert_eq!(
            result,
            ParsedEntity::new()
                .with_property("base", ParsedValue::String("@stabilitylevel2"))
                .with_property("subtract", ParsedValue::String("trigger:planet_stability"))
                .with_property("mult", ParsedValue::Number("0.2"))
                .with_conditional(ParsedConditionalBlock {
                    items: vec![],
                    key: (false, "ALTERED_STABILITY"),
                    properties: ParsedProperties {
                        kv: vec![(
                            "subtract",
                            ParsedPropertyInfoList::new().with_property(
                                Operator::Equals,
                                ParsedValue::String("$ALTERED_STABILITY$")
                            )
                        )]
                        .into_iter()
                        .collect(),
                        is_module: false
                    },
                })
        )
    }

    #[test]
    fn entity_with_define_value() {
        let input = LocatingSlice::new(
            r#"{
            val = @stabilitylevel2
        }"#,
        );

        let result = entity.parse(input).unwrap();

        assert_eq!(
            result,
            ParsedEntity::new()
                .with_property("val", ParsedValue::String("@stabilitylevel2"))
                .into()
        );
    }

    #[test]
    fn compact_equality() {
        let input = LocatingSlice::new(
            r#"{
            mesh="asteroid_01_mesh"
        }"#,
        );

        let result = entity.parse(input).unwrap();

        assert_eq!(
            result,
            ParsedEntity::new()
                .with_property("mesh", ParsedValue::String("asteroid_01_mesh"))
                .into()
        );
    }

    #[test]
    fn switch_statement() {
        let input = LocatingSlice::new(
            r#"{
            trigger = free_housing
            -9 < { nine = yes } # 10
            -8 < { eight = yes } # 9
		}"#,
        );

        let result = entity.parse(input).unwrap();

        assert_eq!(
            result,
            ParsedEntity::new()
                .with_property("trigger", ParsedValue::String("free_housing"))
                .with_property_with_operator(
                    "-9",
                    Operator::LessThan,
                    ParsedValue::Entity(
                        ParsedEntity::new()
                            .with_property("nine", ParsedValue::String("yes"))
                            .into()
                    )
                )
                .with_property_with_operator(
                    "-8",
                    Operator::LessThan,
                    ParsedValue::Entity(
                        ParsedEntity::new()
                            .with_property("eight", ParsedValue::String("yes"))
                            .into()
                    )
                )
                .into()
        );
    }

    #[test]
    fn inline_maths() {
        let input = LocatingSlice::new(
            r#"{
			planet_stability < @[ stabilitylevel2 + 10 ]
        }"#,
        );

        let result = entity.parse(input).unwrap();

        assert_eq!(
            result,
            ParsedEntity::new()
                .with_property_with_operator(
                    "planet_stability",
                    Operator::LessThan,
                    ParsedValue::Maths("@[ stabilitylevel2 + 10 ]")
                )
                .into()
        );
    }

    #[test]
    fn inline_maths_alt() {
        let input = LocatingSlice::new(
            r#"{
			planet_stability < @\[ stabilitylevel2 + 10 ]
        }"#,
        );

        let result = entity.parse(input).unwrap();

        assert_eq!(
            result,
            ParsedEntity::new()
                .with_property_with_operator(
                    "planet_stability",
                    Operator::LessThan,
                    ParsedValue::Maths("@\\[ stabilitylevel2 + 10 ]")
                )
                .into()
        );
    }
}
