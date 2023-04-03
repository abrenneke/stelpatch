#[cfg(test)]
mod tests {
    use crate::cw_model::Entity;

    use super::super::super::*;
    use nom_supreme::error::ErrorTree;

    #[test]
    fn parse_module_with_entities() {
        let input = r#"
        entity1 = { prop1 = value1 }
        entity2 = { prop2 = value2 }
    "#;
        let (_, result) = parse_module::<ErrorTree<_>>(input, "my/type/path", "my_module").unwrap();
        assert_eq!(
            result,
            cw_model::Module {
                type_path: "my/type/path".to_string(),
                filename: "my_module".to_string(),
                entities: vec![
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
                properties: HashMap::new(),
            }
        );
    }

    #[test]
    fn parse_module_with_defines() {
        let input = r#"
        @MY_DEFINE = 123
        @ANOTHER_DEFINE = "hello"
    "#;
        let (_, result) = parse_module::<ErrorTree<_>>(input, "my/type/path", "my_module").unwrap();
        assert_eq!(
            result,
            cw_model::Module {
                type_path: "my/type/path".to_string(),
                filename: "my_module".to_string(),
                entities: HashMap::new(),
                defines: vec![
                    ("@MY_DEFINE".to_string(), cw_model::Value::Number(123.0)),
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
    fn parse_module_with_properties() {
        let input = r#"
        @MY_DEFINE = 123
        my_var1 = value1
        entity = {
            prop1 = value1
        }
    "#;
        let (_, result) = parse_module::<ErrorTree<_>>(input, "my/type/path", "my_module").unwrap();
        assert_eq!(
            result,
            cw_model::Module {
                type_path: "my/type/path".to_string(),
                filename: "my_module".to_string(),
                entities: vec![(
                    "entity".to_string(),
                    Entity::new()
                        .with_property("prop1", cw_model::Value::String("value1".to_string()))
                        .into(),
                )]
                .into_iter()
                .collect(),
                defines: vec![("@MY_DEFINE".to_string(), cw_model::Value::Number(123.0))]
                    .into_iter()
                    .collect(),
                properties: vec![(
                    "my_var1".to_string(),
                    PropertyInfoList::new().with_property(
                        cw_model::Operator::Equals,
                        cw_model::Value::String("value1".to_string())
                    )
                ),]
                .into_iter()
                .collect(),
            }
        );
    }
}
