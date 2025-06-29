#![cfg(test)]
use pretty_assertions::assert_eq;
use winnow::{LocatingSlice, Parser};

use super::super::super::*;

#[test]
fn empty_entity() {
    let input = LocatingSlice::new("{}");
    let result = entity.parse(input).unwrap();
    assert_eq!(result, AstEntity::new().into());
}

#[test]
fn entity_with_property() {
    let input = LocatingSlice::new("{ my_var = value }");
    let result = entity.parse(input).unwrap();
    assert_eq!(
        result,
        AstEntity::new()
            .with_property(
                AstString::new("my_var", false, 2..8),
                AstOperator::equals(9..10),
                AstValue::new_string("value", false, 11..16)
            )
            .into()
    );
}

#[test]
fn entity_with_many_properties() {
    let input = LocatingSlice::new("{ my_var1 = value1\nmy_var2 = value2 my_var3 = value3 }");
    let result = entity.parse(input).unwrap();
    assert_eq!(
        result,
        AstEntity::new()
            .with_property(
                AstString::new("my_var1", false, 2..9),
                AstOperator::equals(10..11),
                AstValue::new_string("value1", false, 12..18)
            )
            .with_property(
                AstString::new("my_var2", false, 19..26),
                AstOperator::equals(27..28),
                AstValue::new_string("value2", false, 29..35)
            )
            .with_property(
                AstString::new("my_var3", false, 36..43),
                AstOperator::equals(44..45),
                AstValue::new_string("value3", false, 46..52)
            )
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
        AstEntity::new()
            .with_property(
                AstString::new("float_val", false, 14..23),
                AstOperator::equals(24..25),
                AstValue::new_number("123.4", 26..31)
            )
            .with_property(
                AstString::new("int_val", false, 44..51),
                AstOperator::equals(52..53),
                AstValue::new_number("12", 54..56)
            )
            .with_property(
                AstString::new("str_val1", false, 57..65),
                AstOperator::equals(66..67),
                AstValue::new_string("value3", false, 68..74)
            )
            .with_property(
                AstString::new("str_val2", false, 95..103),
                AstOperator::equals(104..105),
                AstValue::new_string("value4", true, 106..114)
            )
            .with_property(
                AstString::new("color_val", false, 127..136),
                AstOperator::equals(137..138),
                AstValue::new_color(
                    "rgb",
                    139..142,
                    "1",
                    145..146,
                    "2",
                    147..148,
                    "3",
                    149..150,
                    None,
                    None,
                    139..152
                )
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
        AstEntity::new()
            .with_item(AstValue::new_string("value", false, 2..7))
            .into()
    );
}

#[test]
fn entity_with_many_items() {
    let input = LocatingSlice::new("{ value1 value2 value3 }");
    let result = entity.parse(input).unwrap();
    assert_eq!(
        result,
        AstEntity::new()
            .with_item(AstValue::new_string("value1", false, 2..8))
            .with_item(AstValue::new_string("value2", false, 9..15))
            .with_item(AstValue::new_string("value3", false, 16..22))
            .into()
    );
}

#[test]
fn entity_with_color_item() {
    let input = LocatingSlice::new("{ rgb { 1 2 3 } }");
    let result = entity.parse(input).unwrap();
    assert_eq!(
        result,
        AstEntity::new()
            .with_item(AstValue::new_color(
                "rgb",
                2..5,
                "1",
                8..9,
                "2",
                10..11,
                "3",
                12..13,
                None,
                None,
                2..15
            ))
            .into()
    );
}

#[test]
fn entity_with_color_items() {
    let input = LocatingSlice::new("{ rgb { 1 2 3 } rgb { 4 5 6 } }");
    let result = entity.parse(input).unwrap();
    assert_eq!(
        result,
        AstEntity::new()
            .with_item(AstValue::new_color(
                "rgb",
                2..5,
                "1",
                8..9,
                "2",
                10..11,
                "3",
                12..13,
                None,
                None,
                2..15
            ))
            .with_item(AstValue::new_color(
                "rgb",
                16..19,
                "4",
                22..23,
                "5",
                24..25,
                "6",
                26..27,
                None,
                None,
                16..29
            ))
            .into()
    );
}

#[test]
fn entity_with_mixed_items() {
    let input = LocatingSlice::new("{ value1 rgb { 1 2 3 } value2 }");
    let result = entity.parse(input).unwrap();
    assert_eq!(
        result,
        AstEntity::new()
            .with_item(AstValue::new_string("value1", false, 2..8))
            .with_item(AstValue::new_color(
                "rgb",
                9..12,
                "1",
                15..16,
                "2",
                17..18,
                "3",
                19..20,
                None,
                None,
                9..22
            ))
            .with_item(AstValue::new_string("value2", false, 23..29))
            .into()
    );
}

#[test]
fn entity_with_values_and_properties() {
    let input = LocatingSlice::new("{ value1 my_var = value2 }");
    let result = entity.parse(input).unwrap();
    assert_eq!(
        result,
        AstEntity::new()
            .with_item(AstValue::new_string("value1", false, 2..8))
            .with_property(
                AstString::new("my_var", false, 9..15),
                AstOperator::equals(16..17),
                AstValue::new_string("value2", false, 18..24)
            )
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
        AstEntity::new()
            .with_item(AstValue::new_string("value1", false, 14..20))
            .with_property(
                AstString::new("my_var1", false, 33..40),
                AstOperator::equals(41..42),
                AstValue::new_string("value2", false, 43..49)
            )
            .with_item(AstValue::new_string("value3", false, 62..68))
            .with_property(
                AstString::new("my_var2", false, 91..98),
                AstOperator::equals(99..100),
                AstValue::new_string("value4", false, 101..107)
            )
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
        AstEntity::new()
            .with_property(
                AstString::new("my_var1", false, 14..21),
                AstOperator::equals(22..23),
                AstValue::Entity(
                    AstEntity::new()
                        .with_item(AstValue::new_string("value1", false, 26..32))
                        .into()
                )
            )
            .with_property(
                AstString::new("my_var2", false, 47..54),
                AstOperator::equals(55..56),
                AstValue::Entity(
                    AstEntity::new()
                        .with_item(AstValue::new_string("value2", false, 59..65))
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
        AstEntity::new()
            .with_item(AstValue::Entity(
                AstEntity::new()
                    .with_item(AstValue::new_string("value1", false, 16..22))
                    .into()
            ))
            .with_item(AstValue::Entity(
                AstEntity::new()
                    .with_item(AstValue::new_string("value2", false, 39..45))
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
        AstEntity::new()
            .with_property(
                AstString::new("color1", false, 10..16),
                AstOperator::equals(17..18),
                AstValue::new_color(
                    "rgb",
                    19..22,
                    "255",
                    25..28,
                    "0",
                    29..30,
                    "0",
                    31..32,
                    None,
                    None,
                    19..34
                )
            )
            .with_property(
                AstString::new("color2", false, 43..49),
                AstOperator::equals(50..51),
                AstValue::new_color(
                    "rgb",
                    52..55,
                    "0",
                    58..59,
                    "255",
                    60..63,
                    "0",
                    64..65,
                    Some("0.5"),
                    Some(66..69),
                    52..71
                )
            )
            .with_property(
                AstString::new("color3", false, 80..86),
                AstOperator::equals(87..88),
                AstValue::new_color(
                    "hsv",
                    89..92,
                    "120",
                    95..98,
                    "50",
                    99..101,
                    "100",
                    102..105,
                    None,
                    None,
                    89..107
                )
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
        AstEntity::new()
            .with_property(
                AstString::new("my_var1", false, 35..42),
                AstOperator::equals(43..44),
                AstValue::new_string("value1", false, 45..51)
            )
            .with_property(
                AstString::new("my_var2", false, 90..97),
                AstOperator::equals(98..99),
                AstValue::new_string("value2", false, 100..106)
            )
            .with_property(
                AstString::new("my_var3", false, 115..122),
                AstOperator::equals(123..124),
                AstValue::new_string("value3", false, 125..131)
            )
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
        AstEntity::new()
            // .with_property(
            //     "my_var1",
            //     AstValue::String("value1 with space and \"special\" chars")
            // )
            .with_property(
                AstString::new("my_var2", false, 72..79),
                AstOperator::equals(80..81),
                AstValue::Entity(
                    AstEntity::new()
                        .with_property(
                            AstString::new("nested_var", false, 84..94),
                            AstOperator::equals(95..96),
                            AstValue::Entity(
                                AstEntity::new()
                                    .with_property(
                                        AstString::new("deep_var", false, 99..107),
                                        AstOperator::equals(108..109),
                                        AstValue::new_string("deep_value", true, 110..122)
                                    )
                                    .into()
                            )
                        )
                        .into()
                )
            )
            .with_property(
                AstString::new("my_var3", false, 135..142),
                AstOperator::equals(143..144),
                AstValue::new_color(
                    "rgb",
                    145..148,
                    "255",
                    151..154,
                    "0",
                    155..156,
                    "0",
                    157..158,
                    None,
                    None,
                    145..160
                )
            )
            // .with_property(
            //     "my_var4",
            //     AstValue::String("value_with_escape_characters\nand\ttabs")
            // )
            .with_property(
                AstString::new("my_var5", false, 229..236),
                AstOperator::equals(237..238),
                AstValue::Entity(
                    AstEntity::new()
                        .with_item(AstValue::new_string("nested_entity1", false, 241..255))
                        .with_item(AstValue::new_string("nested_entity2", false, 256..270))
                )
            )
            .with_property(
                AstString::new("my_var6", false, 281..288),
                AstOperator::equals(289..290),
                AstValue::new_string("value6", false, 291..297)
            )
            .with_property(
                AstString::new("my_var7", false, 306..313),
                AstOperator::equals(314..315),
                AstValue::new_string("value7", false, 316..322)
            )
            .with_property(
                AstString::new("my_var8", false, 331..338),
                AstOperator::equals(339..340),
                AstValue::Entity(
                    AstEntity::new()
                        .with_property(
                            AstString::new("nested_var", false, 343..353),
                            AstOperator::equals(354..355),
                            AstValue::new_string("value8", false, 356..362)
                        )
                        .into()
                )
            )
            .with_property(
                AstString::new("my_var9", false, 373..380),
                AstOperator::equals(381..382),
                AstValue::new_string("value9", false, 383..389)
            )
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
        AstEntity::new()
            .with_property(
                AstString::new("my_var", false, 10..16),
                AstOperator::equals(17..18),
                AstValue::new_string("value1", false, 19..25)
            )
            .with_property(
                AstString::new("my_var", false, 34..40),
                AstOperator::equals(41..42),
                AstValue::new_string("value2", false, 43..49)
            )
            .with_property(
                AstString::new("my_var", false, 58..64),
                AstOperator::equals(65..66),
                AstValue::new_string("value3", false, 67..73)
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
        AstEntity::new()
            .with_property(
                AstString::new("base", false, 44..48),
                AstOperator::equals(49..50),
                AstValue::new_string("@stabilitylevel2", false, 51..67)
            )
            .with_property(
                AstString::new("subtract", false, 80..88),
                AstOperator::equals(89..90),
                AstValue::new_string("trigger:planet_stability", false, 91..115)
            )
            .with_conditional_block(AstConditionalBlock::new(
                false,
                AstString::new("ALTERED_STABILITY", false, 130..147),
                vec![AstEntityItem::Property(AstProperty::new(
                    AstString::new("subtract", false, 165..173),
                    AstOperator::equals(174..175),
                    AstValue::new_string("$ALTERED_STABILITY$", false, 176..195)
                ))]
            ))
            .with_property(
                AstString::new("mult", false, 222..226),
                AstOperator::equals(227..228),
                AstValue::new_number("0.2", 229..232)
            )
            .into()
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
        AstEntity::new()
            .with_property(
                AstString::new("val", false, 14..17),
                AstOperator::equals(18..19),
                AstValue::new_string("@stabilitylevel2", false, 20..36)
            )
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
        AstEntity::new()
            .with_property(
                AstString::new("mesh", false, 14..18),
                AstOperator::equals(18..19),
                AstValue::new_string("asteroid_01_mesh", true, 19..37)
            )
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
        AstEntity::new()
            .with_property(
                AstString::new("trigger", false, 14..21),
                AstOperator::equals(22..23),
                AstValue::new_string("free_housing", false, 24..36)
            )
            .with_property(
                AstString::new("-9", false, 49..51),
                AstOperator::new("<", 52..53).unwrap(),
                AstValue::Entity(
                    AstEntity::new()
                        .with_property(
                            AstString::new("nine", false, 56..60),
                            AstOperator::equals(61..62),
                            AstValue::new_string("yes", false, 63..66)
                        )
                        .into()
                )
            )
            .with_property(
                AstString::new("-8", false, 86..88),
                AstOperator::new("<", 89..90).unwrap(),
                AstValue::Entity(
                    AstEntity::new()
                        .with_property(
                            AstString::new("eight", false, 93..98),
                            AstOperator::equals(99..100),
                            AstValue::new_string("yes", false, 101..104)
                        )
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
        AstEntity::new()
            .with_property(
                AstString::new("planet_stability", false, 5..21),
                AstOperator::new("<", 22..23).unwrap(),
                AstValue::Maths(AstMaths::new("@[ stabilitylevel2 + 10 ]", 24..49))
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
        AstEntity::new()
            .with_property(
                AstString::new("planet_stability", false, 5..21),
                AstOperator::new("<", 22..23).unwrap(),
                AstValue::Maths(AstMaths::new("@\\[ stabilitylevel2 + 10 ]", 24..50))
            )
            .into()
    );
}
