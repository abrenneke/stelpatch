use crate::cw_model::{PropertyInfo, PropertyInfoList, Value};

use super::{
    diff::{
        Diff, EntityDiff, HashMapDiff, ModuleDiff, PropertyInfoDiff, PropertyInfoListDiff,
        ValueDiff, VecDiff,
    },
    to_string_one_line::ToStringOneLine,
};
use std::{collections::HashMap, hash::Hash};

#[derive(Debug, PartialEq, Clone)]
pub struct FlatDiff {
    pub path: String,
    pub operation: FlatDiffOperation,
    pub old_value: Option<FlatDiffLeaf>,
    pub new_value: Option<FlatDiffLeaf>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum FlatDiffLeaf {
    Value(Value),
    PropertyInfo(PropertyInfo),
    PropertyInfoList(PropertyInfoList),
}

#[derive(Debug, PartialEq, Clone)]
pub enum FlatDiffOperation {
    Add,
    Remove,
    Modify,
}

pub trait FlattenDiff {
    fn flatten_diff(&self, path: &str) -> Vec<FlatDiff>;
}

impl FlattenDiff for ModuleDiff {
    fn flatten_diff(&self, path: &str) -> Vec<FlatDiff> {
        let mut changes = Vec::new();

        // changes.extend(self.entities.flatten_diff(&format!("{}/entities", path)));
        changes.extend(self.defines.flatten_diff(&format!("{}", path)));
        changes.extend(self.properties.flatten_diff(&format!("{}", path)));

        changes
    }
}

impl<K: Eq + Hash + ToString, V: ToString + Clone + Into<FlatDiffLeaf>, VModified: FlattenDiff>
    FlattenDiff for HashMapDiff<K, V, VModified>
{
    fn flatten_diff(&self, path: &str) -> Vec<FlatDiff> {
        let mut changes = Vec::new();

        if let HashMapDiff::Modified(map) = self {
            for (key, diff) in map.iter() {
                let new_path = format!("{}/{}", path, key.to_string());
                changes.extend(diff.flatten_diff(&new_path));
            }
        }

        changes
    }
}

// impl FlattenDiff for HashMapDiff<String, PropertyInfo, PropertyInfoDiff> {
//     fn flatten_diff(&self, path: &str) -> Vec<FlatDiff> {
//         let mut changes = Vec::new();

//         if let HashMapDiff::Modified(map) = self {
//             for (key, diff) in map.iter() {
//                 let new_path = format!("{}/{}", path, key);
//                 changes.extend(diff.flatten_diff(&new_path));
//             }
//         }

//         changes
//     }
// }

// impl FlattenDiff for Diff<PropertyInfo, PropertyInfoDiff> {
//     fn flatten_diff(&self, path: &str) -> Vec<FlatDiff> {
//         match self {
//             Diff::Unchanged => Vec::new(),
//             Diff::Added(value) => vec![FlatDiff {
//                 path: path.to_string(),
//                 operation: FlatDiffOperation::Add,
//                 old_value: None,
//                 new_value: Some(FlatDiffLeaf::PropertyInfo(value.to_owned())),
//             }],
//             Diff::Removed(value) => vec![FlatDiff {
//                 path: path.to_string(),
//                 operation: FlatDiffOperation::Remove,
//                 old_value: Some(FlatDiffLeaf::PropertyInfo(value.to_owned())),
//                 new_value: None,
//             }],
//             Diff::Modified(modified) => modified.flatten_diff(path),
//         }
//     }
// }

impl FlattenDiff for PropertyInfoListDiff {
    fn flatten_diff(&self, path: &str) -> Vec<FlatDiff> {
        self.0.flatten_diff(path)
    }
}

// FlattenDiff for Diff
impl<T: ToString + Clone + Into<FlatDiffLeaf>, TModified: FlattenDiff> FlattenDiff
    for Diff<T, TModified>
{
    fn flatten_diff(&self, path: &str) -> Vec<FlatDiff> {
        match self {
            Diff::Unchanged => Vec::new(),
            Diff::Added(value) => vec![FlatDiff {
                path: path.to_string(),
                operation: FlatDiffOperation::Add,
                old_value: None,
                new_value: Some(value.clone().into()),
            }],
            Diff::Removed(value) => vec![FlatDiff {
                path: path.to_string(),
                operation: FlatDiffOperation::Remove,
                old_value: Some(value.clone().into()),
                new_value: None,
            }],
            Diff::Modified(modified) => modified.flatten_diff(path),
        }
    }
}

// FlattenDiff for VecDiff
impl<T: ToString + Clone + Into<FlatDiffLeaf>, TModified: FlattenDiff> FlattenDiff
    for VecDiff<T, TModified>
{
    fn flatten_diff(&self, path: &str) -> Vec<FlatDiff> {
        let mut changes = Vec::new();

        if let VecDiff::Changed(vec) = self {
            for (index, diff) in vec.iter().enumerate() {
                let new_path = match index {
                    0 => format!("{}", path),
                    _ => format!("{}/{}", path, index),
                };
                changes.extend(diff.flatten_diff(&new_path));
            }
        }

        changes
    }
}

impl From<Value> for FlatDiffLeaf {
    fn from(value: Value) -> Self {
        FlatDiffLeaf::Value(value)
    }
}

impl From<PropertyInfo> for FlatDiffLeaf {
    fn from(value: PropertyInfo) -> Self {
        FlatDiffLeaf::PropertyInfo(value)
    }
}

impl From<PropertyInfoList> for FlatDiffLeaf {
    fn from(value: PropertyInfoList) -> Self {
        FlatDiffLeaf::PropertyInfoList(value)
    }
}

// FlattenDiff for EntityDiff
impl FlattenDiff for EntityDiff {
    fn flatten_diff(&self, path: &str) -> Vec<FlatDiff> {
        let mut changes = Vec::new();

        changes.extend(self.items.flatten_diff(&format!("{}", path)));
        changes.extend(self.properties.flatten_diff(&format!("{}", path)));
        // changes.extend(
        //     self.conditional_blocks
        //         .flatten_diff(&format!("{}/conditional_blocks", path)),
        // );

        changes
    }
}

impl FlattenDiff for PropertyInfoDiff {
    fn flatten_diff(&self, path: &str) -> Vec<FlatDiff> {
        let mut changes = Vec::new();

        if let Some((old_operator, new_operator)) = &self.operator {
            changes.push(FlatDiff {
                path: format!("{}/$OP", path),
                operation: FlatDiffOperation::Modify,
                old_value: Some(FlatDiffLeaf::Value(Value::String(
                    old_operator.to_string_one_line(),
                ))),
                new_value: Some(FlatDiffLeaf::Value(Value::String(
                    new_operator.to_string_one_line(),
                ))),
            });
        }

        changes.extend(self.value.flatten_diff(&format!("{}", path)));

        changes
    }
}

// FlattenDiff for ValueDiff
impl FlattenDiff for ValueDiff {
    fn flatten_diff(&self, path: &str) -> Vec<FlatDiff> {
        let mut changes = Vec::new();

        match self {
            ValueDiff::String(option) => {
                if let Some((old, new)) = option {
                    changes.push(FlatDiff {
                        path: path.to_string(),
                        operation: FlatDiffOperation::Modify,
                        old_value: Some(FlatDiffLeaf::Value(Value::String(old.clone()))),
                        new_value: Some(FlatDiffLeaf::Value(Value::String(new.clone()))),
                    });
                }
            }
            ValueDiff::Number(option) => {
                if let Some((old, new)) = option {
                    changes.push(FlatDiff {
                        path: path.to_string(),
                        operation: FlatDiffOperation::Modify,
                        old_value: Some(FlatDiffLeaf::Value(Value::Number(old.clone()))),
                        new_value: Some(FlatDiffLeaf::Value(Value::Number(new.clone()))),
                    });
                }
            }
            ValueDiff::Boolean(option) => {
                if let Some((old, new)) = option {
                    changes.push(FlatDiff {
                        path: path.to_string(),
                        operation: FlatDiffOperation::Modify,
                        old_value: Some(FlatDiffLeaf::Value(Value::Boolean(*old))),
                        new_value: Some(FlatDiffLeaf::Value(Value::Boolean(*new))),
                    });
                }
            }
            ValueDiff::Entity(entity_diff) => {
                changes.extend(entity_diff.flatten_diff(path));
            }
            ValueDiff::Define(option) => {
                if let Some((old, new)) = option {
                    changes.push(FlatDiff {
                        path: path.to_string(),
                        operation: FlatDiffOperation::Modify,
                        old_value: Some(FlatDiffLeaf::Value(Value::Define(old.clone()))),
                        new_value: Some(FlatDiffLeaf::Value(Value::Define(new.clone()))),
                    });
                }
            }
            ValueDiff::Color(option) => {
                if let Some((old, new)) = option {
                    changes.push(FlatDiff {
                        path: path.to_string(),
                        operation: FlatDiffOperation::Modify,
                        old_value: Some(FlatDiffLeaf::Value(Value::Color((
                            old.0.clone(),
                            old.1.clone(),
                            old.2.clone(),
                            old.3.clone(),
                            old.4.clone(),
                        )))),
                        new_value: Some(FlatDiffLeaf::Value(Value::Color((
                            new.0.clone(),
                            new.1.clone(),
                            new.2.clone(),
                            new.3.clone(),
                            new.4.clone(),
                        )))),
                    });
                }
            }
            ValueDiff::Maths(option) => {
                if let Some((old, new)) = option {
                    changes.push(FlatDiff {
                        path: path.to_string(),
                        operation: FlatDiffOperation::Modify,
                        old_value: Some(FlatDiffLeaf::Value(Value::Maths(old.clone()))),
                        new_value: Some(FlatDiffLeaf::Value(Value::Maths(new.clone()))),
                    });
                }
            }
            ValueDiff::TypeChanged(old, new) => {
                changes.push(FlatDiff {
                    path: path.to_string(),
                    operation: FlatDiffOperation::Modify,
                    old_value: Some(FlatDiffLeaf::Value(old.clone())),
                    new_value: Some(FlatDiffLeaf::Value(new.clone())),
                });
            }
        }

        changes
    }
}

impl ToStringOneLine for FlatDiffLeaf {
    fn to_string_one_line(&self) -> String {
        match self {
            FlatDiffLeaf::Value(value) => value.to_string_one_line(),
            FlatDiffLeaf::PropertyInfo(property_info) => property_info.to_string_one_line(),
            FlatDiffLeaf::PropertyInfoList(property_info_list) => {
                property_info_list.to_string_one_line()
            }
        }
    }
}

impl ToStringOneLine for FlatDiff {
    fn to_string_one_line(&self) -> String {
        match &self.operation {
            FlatDiffOperation::Add => {
                format!(
                    "+{}: {}",
                    self.path,
                    self.new_value.as_ref().unwrap().to_string_one_line()
                )
            }
            FlatDiffOperation::Remove => {
                format!("-{}", self.path)
            }
            FlatDiffOperation::Modify => format!(
                "{}: {} -> {}",
                self.path,
                self.old_value.as_ref().unwrap().to_string_one_line(),
                self.new_value.as_ref().unwrap().to_string_one_line()
            ),
        }
    }
}

impl super::diff::ModDiff {
    pub fn short_changes_string(&self) -> String {
        let mut s = String::new();
        for (namespace_name, namespace) in sorted_key_value_iter(&self.namespaces) {
            match &namespace.properties {
                HashMapDiff::Modified(properties) => {
                    if properties.len() > 0 {
                        s.push_str(&format!("{}\n", namespace_name));

                        let mut entries = vec![];
                        for (changed_entity_name, entity_diff) in sorted_key_value_iter(properties)
                        {
                            match entity_diff {
                                Diff::Added(_) => {
                                    entries.push(format!("  +{}", changed_entity_name))
                                }
                                Diff::Removed(_) => {
                                    entries.push(format!("  -{}", changed_entity_name))
                                }
                                Diff::Modified(diff) => {
                                    let flattened = diff.flatten_diff(&changed_entity_name);
                                    for flat_diff in flattened {
                                        entries.push(format!(
                                            "  {}",
                                            super::to_string_one_line::ToStringOneLine::to_string_one_line(&flat_diff)
                                        ))
                                    }
                                }
                                Diff::Unchanged => {}
                            }
                        }
                        entries.sort();
                        s.push_str(entries.join("\n").as_str());
                        s.push_str("\n");
                    }
                }
                HashMapDiff::Unchanged => {}
            }
        }

        s
    }
}

fn sorted_key_value_iter<K, V>(map: &HashMap<K, V>) -> impl Iterator<Item = (K, V)> + '_
where
    K: Ord + Clone + Hash,
    V: Clone,
{
    let mut sorted_keys = map.keys().cloned().collect::<Vec<K>>();
    sorted_keys.sort();

    sorted_keys
        .into_iter()
        .filter_map(move |key| map.get(&key).cloned().map(|value| (key, value)))
}
