// #[cfg(test)]
// mod tests {
//     use super::super::super::*;
//     use nom_supreme::error::ErrorTree;

//     #[test]
//     fn module_with_entities() {
//         let input = r#"
//         entity1 = { prop1 = value1 }
//         entity2 = { prop2 = value2 }
//     "#;
//         let (_, result) = module::<ErrorTree<_>>(input, "my_module").unwrap();
//         assert_eq!(
//             result,
//             ParsedModule {
//                 input: input,
//                 filename: "my_module",
//                 values: vec![],
//                 properties: vec![
//                     (
//                         "entity1",
//                         ParsedEntity::new()
//                             .with_property("prop1", ParsedValue::String("value1"))
//                             .into(),
//                     ),
//                     (
//                         "entity2",
//                         ParsedEntity::new()
//                             .with_property("prop2", ParsedValue::String("value2"))
//                             .into(),
//                     ),
//                 ]
//                 .into_iter()
//                 .collect(),
//                 defines: HashMap::new(),
//             }
//         );
//     }

//     #[test]
//     fn module_with_defines() {
//         let input = r#"
//         @MY_DEFINE = 123
//         @ANOTHER_DEFINE = "hello"
//     "#;
//         let (_, result) = module::<ErrorTree<_>>(input, "my_module").unwrap();
//         assert_eq!(
//             result,
//             ParsedModule {
//                 input: input,
//                 filename: "my_module",
//                 values: vec![],
//                 defines: vec![
//                     ("@MY_DEFINE", ParsedValue::Number("123")),
//                     ("@ANOTHER_DEFINE", ParsedValue::String("hello")),
//                 ]
//                 .into_iter()
//                 .collect(),
//                 properties: HashMap::new(),
//             }
//         );
//     }

//     #[test]
//     fn module_with_properties() {
//         let input = r#"
//         @MY_DEFINE = 123
//         my_var1 = value1
//         entity = {
//             prop1 = value1
//         }
//     "#;
//         let (_, result) = module::<ErrorTree<_>>(input, "my_module").unwrap();
//         assert_eq!(
//             result,
//             ParsedModule {
//                 input: input,
//                 filename: "my_module",
//                 values: vec![],
//                 defines: vec![("@MY_DEFINE", ParsedValue::Number("123"))]
//                     .into_iter()
//                     .collect(),
//                 properties: vec![
//                     (
//                         "entity",
//                         ParsedEntity::new()
//                             .with_property("prop1", ParsedValue::String("value1"))
//                             .into()
//                     ),
//                     ("my_var1", ParsedValue::String("value1").into()),
//                 ]
//                 .into_iter()
//                 .collect::<HashMap<&str, ParsedPropertyInfoList>>(),
//             }
//         );
//     }

//     #[test]
//     fn module_with_dynamic_scripting() {
//         let input = r#"
//         revolt_situation_low_stability_factor = { # 0.2 for each point below 25
//             base = @stabilitylevel2
//             subtract = trigger:planet_stability
//             [[ALTERED_STABILITY]
//                 subtract = $ALTERED_STABILITY$
//             ]
//             mult = 0.2
//         }
//     "#;

//         let (_, result) = module::<ErrorTree<_>>(input, "my_module").unwrap();

//         assert_eq!(
//             result,
//             ParsedModule {
//                 namespace: input: input,
//                 filename: "my_module",
//                 values: vec![],
//                 defines: HashMap::new(),
//                 properties: vec![(
//                     "revolt_situation_low_stability_factor",
//                     ParsedEntity::new()
//                         .with_property("base", ParsedValue::Define("@stabilitylevel2"))
//                         .with_property("subtract", ParsedValue::String("trigger:planet_stability"))
//                         .with_property("mult", ParsedValue::Number("0.2"))
//                         .with_conditional(ParsedConditionalBlock {
//                             items: vec![],
//                             key: (false, "ALTERED_STABILITY"),
//                             properties: vec![(
//                                 "subtract",
//                                 ParsedPropertyInfoList::new().with_property(
//                                     Operator::Equals,
//                                     ParsedValue::String("$ALTERED_STABILITY$")
//                                 )
//                             )]
//                             .into_iter()
//                             .collect(),
//                         })
//                         .into()
//                 ),]
//                 .into_iter()
//                 .collect(),
//             }
//         )
//     }

//     #[test]
//     fn module_with_value_list() {
//         let input = r#"
//             weapon_type_energy
//             weapon_type_kinetic
//             weapon_type_explosive
//             weapon_type_strike_craft
//         "#;

//         let (_, result) = module::<ErrorTree<_>>(input, "my_module").unwrap();

//         assert_eq!(
//             result,
//             ParsedModule {
//                 namespace: input: input,
//                 filename: "my_module",
//                 values: vec![
//                     ParsedValue::String("weapon_type_energy"),
//                     ParsedValue::String("weapon_type_kinetic"),
//                     ParsedValue::String("weapon_type_explosive"),
//                     ParsedValue::String("weapon_type_strike_craft"),
//                 ],
//                 defines: HashMap::new(),
//                 properties: HashMap::new(),
//             }
//         );
//     }

//     #[test]
//     fn empty_module() {
//         let input = r#""#;

//         let (_, result) = module::<ErrorTree<_>>(input, "my_module").unwrap();

//         assert_eq!(
//             result,
//             ParsedModule {
//                 namespace: input: input,
//                 filename: "my_module",
//                 values: vec![],
//                 defines: HashMap::new(),
//                 properties: HashMap::new(),
//             }
//         );
//     }

//     #[test]
//     fn commented_out_module() {
//         let input = r#"
//             # @foo = 1
//         "#;

//         let (_, result) = module::<ErrorTree<_>>(input, "my_module").unwrap();

//         assert_eq!(
//             result,
//             ParsedModule {
//                 namespace: input: input,
//                 filename: "my_module",
//                 values: vec![],
//                 defines: HashMap::new(),
//                 properties: HashMap::new(),
//             }
//         );
//     }

//     #[test]
//     fn commented_out_module_2() {
//         let input = r#"# @foo = 1"#;

//         let (_, result) = module::<ErrorTree<_>>(input, "my_module").unwrap();

//         assert_eq!(
//             result,
//             ParsedModule {
//                 namespace: input: input,
//                 filename: "my_module",
//                 values: vec![],
//                 defines: HashMap::new(),
//                 properties: HashMap::new(),
//             }
//         );
//     }

//     #[test]
//     fn handle_bom() {
//         let input = "\u{feff}# Comment";

//         let (_, result) = module::<ErrorTree<_>>(input, "my_module").unwrap();

//         assert_eq!(
//             result,
//             ParsedModule {
//                 namespace: input: input,
//                 filename: "my_module",
//                 values: vec![],
//                 defines: HashMap::new(),
//                 properties: HashMap::new(),
//             }
//         );
//     }

//     #[test]
//     fn handle_readme() {
//         let input = r#"
//             Special variables for Edicts (Country and Empire):
//             # cost, base cost as in resource(s) and amount for activating the edict.
//         "#;

//         let (_, result) = module::<ErrorTree<_>>(input, "99_README_ETC").unwrap();

//         assert_eq!(
//             result,
//             ParsedModule {
//                 namespace: input: input,
//                 filename: "99_README_ETC",
//                 values: vec![],
//                 defines: HashMap::new(),
//                 properties: HashMap::new(),
//             }
//         );
//     }
// }
