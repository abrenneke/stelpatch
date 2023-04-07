use std::collections::{HashMap, HashSet};

use crate::cw_model::{Entity, Module, PropertyInfo, PropertyInfoList, Value};

trait DeepKeys {
    fn deep_keys(&self) -> Vec<String>;
}

impl<T: DeepKeys> DeepKeys for HashMap<String, T> {
    fn deep_keys(&self) -> Vec<String> {
        let mut keys = vec![];
        for (key, value) in self {
            keys.push(key.clone());
            keys.extend(value.deep_keys().iter().map(|k| format!("{}/{}", key, k)));
        }
        keys
    }
}

impl DeepKeys for Value {
    fn deep_keys(&self) -> Vec<String> {
        match self {
            Value::Entity(entity) => entity.deep_keys(),
            _ => vec![],
        }
    }
}

impl DeepKeys for Entity {
    fn deep_keys(&self) -> Vec<String> {
        let mut keys = vec![];
        keys.extend(self.properties.deep_keys());
        keys.extend(self.items.deep_keys());
        keys
    }
}

impl DeepKeys for Module {
    fn deep_keys(&self) -> Vec<String> {
        let mut keys = vec![];
        keys.extend(self.values.deep_keys());
        keys.extend(self.entities.deep_keys());
        keys.extend(self.defines.deep_keys());
        keys.extend(self.properties.deep_keys());
        keys
    }
}

impl<T: DeepKeys> DeepKeys for Vec<T> {
    fn deep_keys(&self) -> Vec<String> {
        let mut keys = vec![];
        for (i, value) in self.iter().enumerate() {
            keys.push(i.to_string());
            keys.extend(value.deep_keys().iter().map(|k| format!("{}/{}", i, k)));
        }
        keys
    }
}

impl DeepKeys for PropertyInfoList {
    fn deep_keys(&self) -> Vec<String> {
        self.0.deep_keys()
    }
}

impl DeepKeys for PropertyInfo {
    fn deep_keys(&self) -> Vec<String> {
        self.value.deep_keys()
    }
}

pub trait JaccardIndex {
    fn jaccard_index(&self, other: &Self) -> f64;
}

impl JaccardIndex for Entity {
    fn jaccard_index(self: &Entity, other: &Entity) -> f64 {
        let self_keys = self.deep_keys();
        let other_keys = other.deep_keys();

        let self_set: HashSet<String> = self_keys.into_iter().collect();
        let other_set: HashSet<String> = other_keys.into_iter().collect();

        let intersection = self_set.intersection(&other_set).count();
        let union = self_set.union(&other_set).count();

        intersection as f64 / union as f64
    }
}

impl JaccardIndex for Value {
    fn jaccard_index(self: &Value, other: &Value) -> f64 {
        match self {
            Value::Entity(self_entity) => match other {
                Value::Entity(other_entity) => self_entity.jaccard_index(other_entity),
                _ => 0.0,
            },
            _ => 0.0,
        }
    }
}

impl JaccardIndex for PropertyInfo {
    fn jaccard_index(self: &PropertyInfo, other: &PropertyInfo) -> f64 {
        self.value.jaccard_index(&other.value)
    }
}

impl JaccardIndex for PropertyInfoList {
    fn jaccard_index(self: &PropertyInfoList, other: &PropertyInfoList) -> f64 {
        let self_keys = self.deep_keys();
        let other_keys = other.deep_keys();

        let self_set: HashSet<String> = self_keys.into_iter().collect();
        let other_set: HashSet<String> = other_keys.into_iter().collect();

        let intersection = self_set.intersection(&other_set).count();
        let union = self_set.union(&other_set).count();

        intersection as f64 / union as f64
    }
}

#[cfg(test)]
mod tests {
    use crate::{cw_model::Module, playset::jaccard::*};

    #[test]
    fn deep_keys_test_1() {
        let input = r#"
        @define1 = 1
        @define2 = 2

        val_1 = "string_1"
        val_2 = 2
        val_3 = { 1 2 3 }
        val_4 = "string_2"
        val_5 = string_3

        entity_1 = {
            entity_1_property_1 = "string_1"
            entity_1_property_2 = 2
            entity_1_property_3 = { 1 2 3 }
            entity_1_property_4 = {
                nested_property_1 = "string_1"
                nested_property_2 = 2
                nested_property_3 = { 1 2 3 }
            }
        }

        entity_2 = {
            entity_2_property_1 = "string_1"
            entity_2_property_2 = 2
            entity_2_property_3 = { 1 2 3 }
            entity_2_property_4 = {
                nested_property_1 = "string_1"
                nested_property_2 = 2
                nested_property_3 = { 1 2 3 }
            }
        }
        
        entity_unchanged = {}
    "#;

        let module = Module::parse(input.to_string(), "", "").unwrap();

        let keys = module.deep_keys();
        assert_eq!(
            keys,
            vec![
                "entity_1",
                "entity_1/entity_1_property_1",
                "entity_1/entity_1_property_2",
                "entity_1/entity_1_property_3",
                "entity_1/entity_1_property_4",
                "entity_1/entity_1_property_4/nested_property_1",
                "entity_1/entity_1_property_4/nested_property_2",
                "entity_1/entity_1_property_4/nested_property_3",
                "entity_2",
                "entity_2/entity_2_property_1",
                "entity_2/entity_2_property_2",
                "entity_2/entity_2_property_3",
                "entity_2/entity_2_property_4",
                "entity_2/entity_2_property_4/nested_property_1",
                "entity_2/entity_2_property_4/nested_property_2",
                "entity_2/entity_2_property_4/nested_property_3",
                "entity_unchanged",
                "val_1",
                "val_2",
                "val_3",
                "val_4",
                "val_5",
            ]
        );
    }

    #[test]
    fn jaccard_index() {
        let input = r#"
        entity_1 = {
            entity_1_property_1 = "string_1"
            entity_1_property_2 = 2
            entity_1_property_3 = { 1 2 3 }
            entity_1_property_4 = {
                nested_property_1 = "string_1"
                nested_property_2 = 2
                nested_property_3 = { 1 2 3 }
            }
        }
    "#;

        let input2 = r#"
        entity_1 = {
            entity_1_property_1 = "string_1"
            entity_1_property_2 = 2
            entity_1_property_3 = { 1 2 }
            entity_1_property_4 = {
                nested_property_1 = "string_1"
                nested_property_2 = 2
                nested_property_3 = 3
            }
        }
    "#;

        let module = Module::parse(input.to_string(), "", "").unwrap();
        let module2 = Module::parse(input2.to_string(), "", "").unwrap();

        let entity1 = module.entities.get("entity_1").unwrap().entity();
        let entity2 = module2.entities.get("entity_1").unwrap().entity();

        let index = entity1.jaccard_index(&entity2);

        assert_eq!(index, 0.8);
    }
}
