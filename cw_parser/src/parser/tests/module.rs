#![cfg(test)]
use pretty_assertions::assert_eq;
use winnow::{LocatingSlice, Parser};

use super::super::super::*;

#[test]
fn module_with_entities() {
    let input = LocatingSlice::new(
        r#"
        entity1 = { prop1 = value1 }
        entity2 = { prop2 = value2 }
    "#,
    );
    let module = module.parse(input).unwrap();

    assert_eq!(module.span, 0..79);

    assert_eq!(
        module.items,
        vec![
            AstEntityItem::Expression(Box::new(AstExpression::new(
                AstString::new("entity1", false, 9..16),
                AstOperator::new("=", 17..18).unwrap(),
                AstValue::Entity(AstEntity::new(19..37).with_property(
                    AstString::new("prop1", false, 21..26),
                    AstOperator::new("=", 27..28).unwrap(),
                    AstValue::new_string("value1", false, 29..35)
                ))
            ))),
            AstEntityItem::Expression(Box::new(AstExpression::new(
                AstString::new("entity2", false, 46..53),
                AstOperator::new("=", 54..55).unwrap(),
                AstValue::Entity(AstEntity::new(56..74).with_property(
                    AstString::new("prop2", false, 58..63),
                    AstOperator::new("=", 64..65).unwrap(),
                    AstValue::new_string("value2", false, 66..72)
                ))
            ))),
        ]
    );
}

#[test]
fn module_with_defines() {
    let input = LocatingSlice::new(
        r#"
        @MY_DEFINE = 123
        @ANOTHER_DEFINE = "hello"
    "#,
    );
    let module = module.parse(input).unwrap();

    assert_eq!(module.span, 0..64);

    assert_eq!(
        module.items,
        vec![
            AstEntityItem::Expression(Box::new(AstExpression::new(
                AstString::new("@MY_DEFINE", false, 9..19),
                AstOperator::new("=", 20..21).unwrap(),
                AstValue::Number(AstNumber::new("123", 22..25))
            ))),
            AstEntityItem::Expression(Box::new(AstExpression::new(
                AstString::new("@ANOTHER_DEFINE", false, 34..49),
                AstOperator::new("=", 50..51).unwrap(),
                AstValue::String(AstString::new("hello", true, 52..59))
            ))),
        ]
    );
}

#[test]
fn module_with_properties() {
    let input = LocatingSlice::new(
        r#"
        @MY_DEFINE = 123
        my_var1 = value1
        entity = {
            prop1 = value1
        }
    "#,
    );
    let module = module.parse(input).unwrap();

    assert_eq!(module.span, 0..111);

    assert_eq!(
        module.items,
        vec![
            AstEntityItem::Expression(Box::new(AstExpression::new(
                AstString::new("@MY_DEFINE", false, 9..19),
                AstOperator::new("=", 20..21).unwrap(),
                AstValue::Number(AstNumber::new("123", 22..25))
            ))),
            AstEntityItem::Expression(Box::new(AstExpression::new(
                AstString::new("my_var1", false, 34..41),
                AstOperator::new("=", 42..43).unwrap(),
                AstValue::String(AstString::new("value1", false, 44..50))
            ))),
            AstEntityItem::Expression(Box::new(AstExpression::new(
                AstString::new("entity", false, 59..65),
                AstOperator::new("=", 66..67).unwrap(),
                AstValue::Entity(AstEntity::new(68..106).with_property(
                    AstString::new("prop1", false, 82..87),
                    AstOperator::new("=", 88..89).unwrap(),
                    AstValue::new_string("value1", false, 90..96)
                ))
            ))),
        ]
    );
}

#[test]
fn module_with_dynamic_scripting() {
    let input = LocatingSlice::new(
        r#"
        revolt_situation_low_stability_factor = { # 0.2 for each point below 25
            base = @stabilitylevel2
            subtract = trigger:planet_stability
            [[ALTERED_STABILITY]
                subtract = $ALTERED_STABILITY$
            ]
            mult = 0.2
        }
    "#,
    );

    assert!(module.parse(input).is_ok());
}

#[test]
fn module_with_value_list() {
    let input = LocatingSlice::new(
        r#"
            weapon_type_energy
            weapon_type_kinetic
            weapon_type_explosive
            weapon_type_strike_craft
        "#,
    );

    let module = module.parse(input).unwrap();

    assert_eq!(module.span, 0..143);

    assert_eq!(
        module.items,
        vec![
            AstEntityItem::Item(Box::new(AstValue::String(AstString::new(
                "weapon_type_energy",
                false,
                13..31
            )))),
            AstEntityItem::Item(Box::new(AstValue::String(AstString::new(
                "weapon_type_kinetic",
                false,
                44..63
            )))),
            AstEntityItem::Item(Box::new(AstValue::String(AstString::new(
                "weapon_type_explosive",
                false,
                76..97
            )))),
            AstEntityItem::Item(Box::new(AstValue::String(AstString::new(
                "weapon_type_strike_craft",
                false,
                110..134
            )))),
        ]
    );
}

#[test]
fn empty_module() {
    let input = LocatingSlice::new(r#""#);

    let module = module.parse(input).unwrap();

    assert_eq!(module.span, 0..0);

    assert_eq!(module.items, vec![]);
}

#[test]
fn commented_out_module() {
    let input = LocatingSlice::new(
        r#"
            # @foo = 1
        "#,
    );

    let module = module.parse(input).unwrap();

    assert_eq!(module.span, 0..32);

    assert_eq!(module.items, vec![]);
}

#[test]
fn commented_out_module_2() {
    let input = LocatingSlice::new(r#"# @foo = 1"#);

    let module = module.parse(input).unwrap();

    assert_eq!(module.span, 0..10);

    assert_eq!(module.items, vec![]);
}

#[test]
fn handle_bom() {
    let input = LocatingSlice::new("\u{feff}# Comment");

    let module = module.parse(input).unwrap();

    assert_eq!(module.span, 0..12);

    assert_eq!(module.items, vec![]);
}

#[test]
fn item_with_comments() {
    let input = LocatingSlice::new(
        r#"
        # comment1
        my_var # comment2
        # comment3
        # comment4
        "#,
    );

    let module = module.parse(input).unwrap();

    assert_eq!(
        module,
        AstModule {
            items: vec![AstEntityItem::Item(Box::new(AstValue::String(AstString {
                value: AstToken::new("my_var", 28..34),
                is_quoted: false,
                leading_newlines: 0,
                leading_comments: vec![],
                trailing_comment: Some(AstComment::new(" comment2", 35..45)),
            })))],
            span: 0..92,
            leading_comments: vec![AstComment::new(" comment1", 9..19)],
            trailing_comments: vec![
                AstComment::new(" comment3", 54..64),
                AstComment::new(" comment4", 73..83),
            ],
        }
    );
}
