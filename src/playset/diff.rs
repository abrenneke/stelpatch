use indent_write::indentable::Indentable;

use crate::cw_model::{
    ConditionalBlock, Entity, Module, Operator, PropertyInfo, PropertyInfoList, Value,
};
use std::collections::HashSet;
use std::fmt::{self, Debug, Display};
use std::{collections::HashMap, hash::Hash};

#[derive(Debug, PartialEq, Clone)]
pub struct ModuleDiff {
    pub filename: Option<(String, String)>,
    pub type_path: Option<(String, String)>,
    pub entities: HashMapDiff<String, Value, ValueDiff>,
    pub defines: HashMapDiff<String, Value, ValueDiff>,
    pub properties: HashMapDiff<String, PropertyInfoList, PropertyInfoListDiff>,
    pub values: VecDiff<Value, ValueDiff>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum HashMapDiff<K, V, VModified> {
    Unchanged,
    Modified(Vec<(K, Diff<V, VModified>)>),
}

#[derive(Debug, PartialEq, Clone)]
pub enum VecDiff<T, TModified> {
    Unchanged,
    Changed(Vec<Diff<T, TModified>>),
}

#[derive(Debug, PartialEq, Clone)]
pub enum Diff<T, TModified> {
    Added(T),
    Removed(T),
    Modified(TModified),
}

#[derive(Debug, PartialEq, Clone)]
pub struct EntityDiff {
    pub items: VecDiff<Value, ValueDiff>,
    pub properties: HashMapDiff<String, PropertyInfoList, PropertyInfoListDiff>,
    pub conditional_blocks: HashMapDiff<String, ConditionalBlock, ConditionalBlockDiff>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct NamedEntityDiff(pub EntityDiff, pub Option<(String, String)>);

#[derive(Debug, PartialEq, Clone)]
pub struct PropertyInfoDiff {
    pub operator: Option<(Operator, Operator)>,
    pub value: ValueDiff,
}

#[derive(Debug, PartialEq, Clone)]
pub struct PropertyInfoListDiff(VecDiff<PropertyInfo, PropertyInfoDiff>);

#[derive(Debug, PartialEq, Clone)]
pub struct ConditionalBlockDiff {
    pub key: Option<((bool, String), (bool, String))>,
    pub items: VecDiff<Value, ValueDiff>,
    pub properties: HashMapDiff<String, PropertyInfoList, PropertyInfoListDiff>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum OperatorDiff {
    Unchanged,
    Modified(Operator, Operator),
}

#[derive(Debug, PartialEq, Clone)]
pub enum ValueDiff {
    String(Option<(String, String)>),
    Number(Option<(String, String)>),
    Boolean(Option<(bool, bool)>),
    Entity(EntityDiff),
    Define(Option<(String, String)>),
    Color(
        Option<(
            (String, String, String, String, Option<String>),
            (String, String, String, String, Option<String>),
        )>,
    ),
    Maths(Option<(String, String)>),
    TypeChanged(Value, Value),
}

impl ModuleDiff {
    pub fn from_modules(module_a: &Module, module_b: &Module) -> Self {
        let filename = if module_a.filename != module_b.filename {
            Some((module_a.filename.clone(), module_b.filename.clone()))
        } else {
            None
        };

        let type_path = if module_a.type_path != module_b.type_path {
            Some((module_a.type_path.clone(), module_b.type_path.clone()))
        } else {
            None
        };

        let entities = HashMapDiff::from_hashmaps(&module_a.entities, &module_b.entities);
        let defines = HashMapDiff::from_hashmaps(&module_a.defines, &module_b.defines);
        let properties = HashMapDiff::from_hashmaps(&module_a.properties, &module_b.properties);
        let values = VecDiff::from_vecs(&module_a.values, &module_b.values, |a, b| {
            if a.jaccard_index(b) > 0.8 {
                Some(ValueDiff::from((a.clone(), b.clone())))
            } else {
                None
            }
        });

        ModuleDiff {
            filename,
            type_path,
            entities,
            defines,
            properties,
            values,
        }
    }
}

impl Module {
    pub fn diff(&self, other: &Module) -> ModuleDiff {
        ModuleDiff::from_modules(self, other)
    }
}

impl<K: Eq + Hash + Clone, V: PartialEq + Eq + Clone, VModified> HashMapDiff<K, V, VModified> {
    pub fn from_hashmaps(
        hashmap_a: &HashMap<K, V>,
        hashmap_b: &HashMap<K, V>,
    ) -> HashMapDiff<K, V, VModified>
    where
        VModified: From<(V, V)>,
    {
        let mut modified = Vec::new();

        for (key, value_a) in hashmap_a {
            match hashmap_b.get(key) {
                Some(value_b) if value_a != value_b => {
                    modified.push((
                        key.clone(),
                        Diff::Modified(VModified::from((value_a.clone(), value_b.clone()))),
                    ));
                }
                None => {
                    modified.push((key.clone(), Diff::Removed(value_a.clone())));
                }
                _ => {}
            }
        }

        for (key, value_b) in hashmap_b {
            if !hashmap_a.contains_key(key) {
                modified.push((key.clone(), Diff::Added(value_b.clone())));
            }
        }

        if modified.is_empty() {
            HashMapDiff::Unchanged
        } else {
            HashMapDiff::Modified(modified)
        }
    }
}

impl<T: PartialEq + Clone + Debug, VModified> VecDiff<T, VModified> {
    pub fn from_vecs<F>(vec_a: &[T], vec_b: &[T], modifier: F) -> VecDiff<T, VModified>
    where
        F: Fn(&T, &T) -> Option<VModified>,
    {
        let mut diffs = Vec::new();

        // Keep track of claimed items so that find_map doesn't return the same item twice
        let mut claimed = HashSet::new();

        for value_a in vec_a {
            if let Some(modified) = vec_b.iter().enumerate().find_map(|(i, value_b)| {
                if claimed.contains(&i) == false {
                    if let Some(modified) = modifier(value_a, value_b) {
                        claimed.insert(i);
                        Some(modified)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }) {
                diffs.push(Diff::Modified(modified));
            } else {
                diffs.push(Diff::Removed(value_a.clone()));
            }
        }

        for value_b in vec_b {
            if !vec_a.contains(value_b) {
                diffs.push(Diff::Added(value_b.clone()));
            }
        }

        if diffs.is_empty() {
            VecDiff::Unchanged
        } else {
            VecDiff::Changed(diffs)
        }
    }
}

impl EntityDiff {
    pub fn from_entities(entity_a: &Entity, entity_b: &Entity) -> Self {
        let items = VecDiff::from_vecs(&entity_a.items, &entity_b.items, |a, b| {
            if a.jaccard_index(b) > 0.8 {
                Some(ValueDiff::from((a.clone(), b.clone())))
            } else {
                None
            }
        });
        let properties = HashMapDiff::from_hashmaps(&entity_a.properties, &entity_b.properties);
        let conditional_blocks =
            HashMapDiff::from_hashmaps(&entity_a.conditional_blocks, &entity_b.conditional_blocks);

        EntityDiff {
            items,
            properties,
            conditional_blocks,
        }
    }
}

impl From<(Value, Value)> for ValueDiff {
    fn from(values: (Value, Value)) -> Self {
        match (values.0, values.1) {
            (Value::String(a), Value::String(b)) => ValueDiff::String(Some((a, b))),
            (Value::Number(a), Value::Number(b)) => ValueDiff::Number(Some((a, b))),
            (Value::Boolean(a), Value::Boolean(b)) => ValueDiff::Boolean(Some((a, b))),
            (Value::Entity(a), Value::Entity(b)) => {
                ValueDiff::Entity(EntityDiff::from_entities(&a, &b))
            }
            (Value::Define(a), Value::Define(b)) => ValueDiff::Define(Some((a, b))),
            (Value::Color(a), Value::Color(b)) => ValueDiff::Color(Some((a, b))),
            (Value::Maths(a), Value::Maths(b)) => ValueDiff::Maths(Some((a, b))),
            (a, b) => ValueDiff::TypeChanged(a, b),
        }
    }
}

impl From<(PropertyInfoList, PropertyInfoList)> for PropertyInfoListDiff {
    fn from((a, b): (PropertyInfoList, PropertyInfoList)) -> Self {
        if a == b {
            return PropertyInfoListDiff(VecDiff::Unchanged);
        }

        if a.len() == b.len() {
            if a.len() == 1 {
                return PropertyInfoListDiff(VecDiff::Changed(vec![Diff::Modified(
                    PropertyInfoDiff::from((a.into_vec()[0].clone(), b.into_vec()[0].clone())),
                )]));
            }

            let mut diff = Vec::new();

            for (a, b) in a.into_vec().into_iter().zip(b.into_vec().into_iter()) {
                if a.value.jaccard_index(&b.value) > 0.8 {
                    diff.push(Diff::Modified(PropertyInfoDiff::from((a, b))));
                } else {
                    diff.push(Diff::Removed(a));
                    diff.push(Diff::Added(b));
                }
            }

            return PropertyInfoListDiff(VecDiff::Changed(diff));
        }

        let diff = VecDiff::from_vecs(&a.into_vec(), &b.into_vec(), |a, b| {
            if a.value.jaccard_index(&b.value) > 0.8 {
                Some(PropertyInfoDiff::from((a.clone(), b.clone())))
            } else {
                None
            }
        });

        PropertyInfoListDiff(diff)
    }
}

impl From<(PropertyInfo, PropertyInfo)> for PropertyInfoDiff {
    fn from(values: (PropertyInfo, PropertyInfo)) -> Self {
        let operator = if values.0.operator == values.1.operator {
            None
        } else {
            Some((values.0.operator, values.1.operator))
        };

        let value = ValueDiff::from((values.0.value, values.1.value));

        PropertyInfoDiff { operator, value }
    }
}

impl From<(ConditionalBlock, ConditionalBlock)> for ConditionalBlockDiff {
    fn from((a, b): (ConditionalBlock, ConditionalBlock)) -> Self {
        let (a_is_not, a_key) = a.key;
        let (b_is_not, b_key) = b.key;

        let key = if a_is_not == b_is_not && a_key == b_key {
            None
        } else {
            Some(((a_is_not, a_key), (b_is_not, b_key)))
        };

        let items = VecDiff::from_vecs(&a.items, &b.items, |a, b| {
            if a.jaccard_index(b) > 0.8 {
                Some(ValueDiff::from((a.clone(), b.clone())))
            } else {
                None
            }
        });
        let properties = HashMapDiff::from_hashmaps(&a.properties, &b.properties);

        ConditionalBlockDiff {
            items,
            key,
            properties,
        }
    }
}

impl Display for ModuleDiff {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.defines)?;
        write!(f, "{}", self.properties)?;
        write!(f, "{}", &self.entities)?;
        write!(f, "{}", self.values)?;
        Ok(())
    }
}

impl<V: Display, VModified: Display> Display for Diff<V, VModified> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Diff::Added(added) => write!(f, "[Added] {}", added),
            Diff::Removed(removed) => write!(f, "[Removed] {}", removed),
            Diff::Modified(modified) => {
                write!(f, "{}", modified)
            }
        }
    }
}

impl Display for PropertyInfoListDiff {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.0 {
            VecDiff::Unchanged => Ok(()),
            VecDiff::Changed(items) => {
                for item in items {
                    match item {
                        Diff::Added(item) => {
                            write!(f, "[Added] {}", item)?;
                        }
                        Diff::Removed(item) => {
                            write!(f, "[Removed] {}", item)?;
                        }
                        Diff::Modified(item) => {
                            write!(f, "{}", item)?;
                        }
                    }
                }

                Ok(())
            }
        }
    }
}

impl Display for PropertyInfoDiff {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some((old, new)) = &self.operator {
            write!(f, "({} -> {}) ", old, new)?;
        }

        write!(f, "{} ", self.value)
    }
}

impl<V, VModified> Display for VecDiff<V, VModified>
where
    V: Display,
    VModified: Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            VecDiff::Unchanged => Ok(()),
            VecDiff::Changed(items) => {
                for item in items {
                    match item {
                        Diff::Added(item) => {
                            write!(f, "[Added] {}", item)?;
                        }
                        Diff::Removed(item) => {
                            write!(f, "[Removed] {}", item)?;
                        }
                        Diff::Modified(item) => write!(f, "{}", item)?,
                    }
                }

                Ok(())
            }
        }
    }
}

impl<K: Display, V: Display, VModified: Display> Display for HashMapDiff<K, V, VModified> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            HashMapDiff::Unchanged => Ok(()),
            HashMapDiff::Modified(pairs) => {
                for (key, diff) in pairs {
                    writeln!(f, "{}: {}", key, diff.clone().indented_skip_initial("    "))?;
                }
                Ok(())
            }
        }
    }
}

// ... similar code for other enums ...

impl Display for EntityDiff {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{{ ")?;
        write!(f, "{}", self.items)?;
        write!(f, "{}", self.properties)?;
        write!(f, "{}", self.conditional_blocks)?;
        write!(f, "}} ")
    }
}

impl Display for ConditionalBlockDiff {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[[")?;
        if let Some(((is_not_old, key_old), (is_not_new, key_new))) = &self.key {
            let old = if *is_not_old {
                "!".to_owned() + key_old
            } else {
                key_old.to_string()
            };
            let new = if *is_not_new {
                "!".to_owned() + key_new
            } else {
                key_new.to_string()
            };
            write!(f, "{} -> {}", old, new)?;
        }
        write!(f, "]")?;
        write!(f, "{}", self.items)?;
        write!(f, "]")
    }
}

impl Display for ValueDiff {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ValueDiff::String(Some((old, new))) => {
                write!(f, "\"{}\" -> \"{}\"", old, new)
            }
            ValueDiff::String(None) => write!(f, "Unchanged"),
            ValueDiff::Number(Some((old, new))) => {
                write!(f, "{} -> {}", old, new)
            }
            ValueDiff::Number(None) => write!(f, "Unchanged"),
            ValueDiff::Boolean(Some((old, new))) => {
                write!(f, "{} -> {}", old, new)
            }
            ValueDiff::Boolean(None) => write!(f, "Unchanged"),
            ValueDiff::Define(Some((old, new))) => {
                write!(f, "{} -> {}", old, new)
            }
            ValueDiff::Define(None) => write!(f, "Unchanged"),
            ValueDiff::Color(Some(((type1, a1, b1, c1, d1), (type2, a2, b2, c2, d2)))) => {
                let d1 = match d1 {
                    Some(d1) => format!("{} ", d1),
                    None => "".to_string(),
                };
                let d2 = match d2 {
                    Some(d2) => format!("{} ", d2),
                    None => "".to_string(),
                };
                write!(
                    f,
                    "{} {{ {} {} {} {} }} -> {} {{ {} {} {} {} }}",
                    type1, a1, b1, c1, d1, type2, a2, b2, c2, d2
                )
            }
            ValueDiff::Maths(Some((old, new))) => {
                write!(f, "{} -> {}", old, new)
            }
            ValueDiff::Maths(None) => write!(f, "Unchanged"),
            ValueDiff::TypeChanged(old, new) => {
                write!(f, "{} -> {}", old, new)
            }
            ValueDiff::Entity(entity_diff) => {
                write!(f, "{}", entity_diff)
            }
            _ => todo!(),
        }
    }
}

trait ApplyPatch<TDiff> {
    fn apply_patch(&self, diff: &TDiff) -> Self;
}

impl ApplyPatch<ModuleDiff> for Module {
    fn apply_patch(&self, diff: &ModuleDiff) -> Module {
        let filename = diff
            .filename
            .as_ref()
            .map(|(_, new)| new.clone())
            .unwrap_or_else(|| self.filename.clone());
        let type_path = diff
            .type_path
            .as_ref()
            .map(|(_, new)| new.clone())
            .unwrap_or_else(|| self.type_path.clone());
        let entities = self.entities.apply_patch(&diff.entities);
        let defines = self.defines.apply_patch(&diff.defines);
        let properties = self.properties.apply_patch(&diff.properties);
        let values = self.values.apply_patch(&diff.values);

        Module {
            filename,
            type_path,
            entities,
            defines,
            properties,
            values,
        }
    }
}

impl<K: Hash + Eq + Clone, V: ApplyPatch<VChanged> + Clone, VChanged>
    ApplyPatch<HashMapDiff<K, V, VChanged>> for HashMap<K, V>
{
    fn apply_patch(&self, diff: &HashMapDiff<K, V, VChanged>) -> HashMap<K, V> {
        match diff {
            HashMapDiff::Unchanged => self.clone(),
            HashMapDiff::Modified(modified_pairs) => {
                let mut result = self.clone();
                for (key, change) in modified_pairs {
                    match change {
                        Diff::Added(value) => {
                            result.insert(key.clone(), value.clone());
                        }
                        Diff::Removed(_) => {
                            result.remove(key);
                        }
                        Diff::Modified(modified_value) => {
                            if let Some(existing_value) = result.get_mut(key) {
                                *existing_value = existing_value.apply_patch(modified_value);
                            }
                        }
                    }
                }
                result
            }
        }
    }
}

impl<T, VModified> ApplyPatch<VecDiff<T, VModified>> for Vec<T>
where
    T: Clone + Eq + ApplyPatch<VModified>,
{
    fn apply_patch(&self, diff: &VecDiff<T, VModified>) -> Vec<T> {
        match diff {
            VecDiff::Unchanged => self.clone(),
            VecDiff::Changed(items) => {
                let mut result = self.clone();
                for (index, change) in items.iter().enumerate() {
                    match change {
                        Diff::Added(value) => {
                            result.insert(index, value.clone());
                        }
                        Diff::Removed(_) => {
                            result.remove(index);
                        }
                        Diff::Modified(patch) => {
                            let existing_value = result.get_mut(index).unwrap();
                            existing_value.apply_patch(patch);
                        }
                    }
                }
                result
            }
        }
    }
}

impl ApplyPatch<ValueDiff> for Value {
    fn apply_patch(&self, diff: &ValueDiff) -> Self {
        match diff {
            ValueDiff::String(option) => {
                if let Some((_, new)) = option {
                    Value::String(new.clone())
                } else {
                    self.clone()
                }
            }
            ValueDiff::Number(option) => {
                if let Some((_, new)) = option {
                    Value::Number(new.parse().unwrap())
                } else {
                    self.clone()
                }
            }
            ValueDiff::Boolean(option) => {
                if let Some((_, new)) = option {
                    Value::Boolean(*new)
                } else {
                    self.clone()
                }
            }
            ValueDiff::Entity(diff) => {
                if let Value::Entity(entity) = self {
                    Value::Entity(entity.apply_patch(diff))
                } else {
                    self.clone()
                }
            }
            ValueDiff::Define(option) => {
                if let Some((_, new)) = option {
                    Value::Define(new.clone())
                } else {
                    self.clone()
                }
            }
            ValueDiff::Color(option) => {
                if let Some((_, new)) = option {
                    Value::Color(new.clone())
                } else {
                    self.clone()
                }
            }
            ValueDiff::Maths(option) => {
                if let Some((_, new)) = option {
                    Value::Maths(new.clone())
                } else {
                    self.clone()
                }
            }
            ValueDiff::TypeChanged(_, new) => new.clone(),
        }
    }
}

impl ApplyPatch<PropertyInfoDiff> for PropertyInfo {
    fn apply_patch(&self, diff: &PropertyInfoDiff) -> Self {
        PropertyInfo {
            operator: diff.operator.map_or(self.operator, |(_, new)| new),
            value: self.value.apply_patch(&diff.value),
        }
    }
}

impl ApplyPatch<PropertyInfoListDiff> for PropertyInfoList {
    fn apply_patch(&self, diff: &PropertyInfoListDiff) -> Self {
        match &diff.0 {
            VecDiff::Unchanged => self.clone(),
            VecDiff::Changed(items) => {
                let mut result = self.clone();
                for (index, change) in items.iter().enumerate() {
                    match change {
                        Diff::Added(value) => {
                            result.0.insert(index, value.clone());
                        }
                        Diff::Removed(_) => {
                            result.0.remove(index);
                        }
                        Diff::Modified(patch) => {
                            let existing_value = result.0.get_mut(index).unwrap();
                            existing_value.apply_patch(patch);
                        }
                    }
                }
                result
            }
        }
    }
}

impl ApplyPatch<EntityDiff> for Entity {
    fn apply_patch(&self, diff: &EntityDiff) -> Self {
        Entity {
            items: self.items.apply_patch(&diff.items),
            properties: self.properties.apply_patch(&diff.properties),
            conditional_blocks: self
                .conditional_blocks
                .apply_patch(&diff.conditional_blocks),
        }
    }
}

impl ApplyPatch<ConditionalBlockDiff> for ConditionalBlock {
    fn apply_patch(&self, diff: &ConditionalBlockDiff) -> Self {
        let (is_not, key) = match &diff.key {
            Some((_, (new_is_not, new_key))) => (*new_is_not, new_key.clone()),
            None => (self.key.0, self.key.1.clone()),
        };

        ConditionalBlock {
            key: (is_not, key),
            items: self.items.apply_patch(&diff.items),
            properties: self.properties.apply_patch(&diff.properties),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        cw_model::Module,
        playset::diff::{Diff, HashMapDiff, ValueDiff},
    };

    use super::ModuleDiff;

    #[test]
    fn compare_modules_1() {
        let module_a_def = r#"
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
            
            entity_unchanged = {}"#;

        let module_b_dev = r#"
            @define1 = 1
            @define2 = 3

            val_1 = "CHANGED"
            val_2 = 3
            val_3 = { 1 4 3 4 }
            val_4 = 4

            entity_2 = {
                entity_2_property_1 = "string_1"
                entity_2_property_2 = 2
                entity_2_property_3 = { 1 2 3 }
                entity_2_property_4 = {
                    nested_property_1 = "string_1"
                    nested_property_2 = 2
                    nested_property_3 = { 1 2 3 }
                }
                entity_2_property_5 = "string_5"
            }

            entity_unchanged = {}
            
            entity_3 = {
                entity_3_property_1 = "string_1"
            }"#;

        let module_a = Module::parse(module_a_def.to_string(), "type/path/", "a").unwrap();
        let module_b = Module::parse(module_b_dev.to_string(), "type/path/", "b").unwrap();

        let diff = ModuleDiff::from_modules(&module_a, &module_b);

        print!("{}", diff);
    }

    #[test]
    fn compare_modules_2() {
        let module_a_def = r#"
        agenda_slave_optimization = {
            weight_modifier = {
                modifier = { factor = 1.5 }
                modifier = { factor = 2 }
                modifier = { factor = 1 }
                modifier = { factor = 0 }
            }
        }"#;

        let module_b_dev = r#"
        agenda_slave_optimization = {
            weight_modifier = {
                modifier = { factor = 1.5 }
                modifier = { factor = 2 }
                modifier = { factor = 0 }
            }
        }"#;

        let module_a = Module::parse(module_a_def.to_string(), "type/path/", "a").unwrap();
        let module_b = Module::parse(module_b_dev.to_string(), "type/path/", "b").unwrap();

        let diff = ModuleDiff::from_modules(&module_a, &module_b);

        if let HashMapDiff::Modified(modified_entities) = &diff.entities {
            let (key, entity) = modified_entities.first().unwrap();

            assert_eq!(key, "agenda_slave_optimization");

            if let Diff::Modified(entity) = entity {
                if let ValueDiff::Entity(_entity) = entity {
                    // dbg!(&entity.properties);
                } else {
                    panic!("Expected entity");
                }
            } else {
                panic!("Expected modified entity");
            }
        } else {
            panic!("Expected modified entities");
        }

        assert_eq!(
            diff.to_string()
                .replace(" ", "")
                .replace("\r", "")
                .replace("\t", "")
                .replace("\n", ""),
            r#"agenda_slave_optimization: { weight_modifier: { modifier: {} {} { factor: 1 -> 0 } [Removed] = { factor = 0 } } }"#
            .replace(" ", "")
            .replace("\r", "")
            .replace("\t", "")
            .replace("\n", "")
        );
    }

    #[test]
    fn compare_modules_3() {
        let module_a_def = r#"
        agenda_slave_optimization = {
            weight_modifier = {
                modifier = { factor = 1.5 }
                modifier = { factor = 2 }
                modifier = { factor = 1 }
                modifier = { factor = 0 }
            }
        }"#;

        let module_b_dev = r#"
        agenda_slave_optimization = {
            weight_modifier = {
                modifier = { factor = 1.5 }
                modifier = { factor = 2 }
                modifier = { factor = 1 }
            }
        }"#;

        let module_a = Module::parse(module_a_def.to_string(), "type/path/", "a").unwrap();
        let module_b = Module::parse(module_b_dev.to_string(), "type/path/", "b").unwrap();

        let diff = ModuleDiff::from_modules(&module_a, &module_b);

        if let HashMapDiff::Modified(modified_entities) = &diff.entities {
            let (key, entity) = modified_entities.first().unwrap();

            assert_eq!(key, "agenda_slave_optimization");

            if let Diff::Modified(entity) = entity {
                if let ValueDiff::Entity(_entity) = entity {
                    // dbg!(&entity.properties);
                } else {
                    panic!("Expected entity");
                }
            } else {
                panic!("Expected modified entity");
            }
        } else {
            panic!("Expected modified entities");
        }

        assert_eq!(
            diff.to_string()
                .replace(" ", "")
                .replace("\r", "")
                .replace("\t", "")
                .replace("\n", ""),
            r#"agenda_slave_optimization: { weight_modifier: { modifier: {} {} {} [Removed] = { factor = 0 } } }"#
            .replace(" ", "")
            .replace("\r", "")
            .replace("\t", "")
            .replace("\n", "")
        );
    }
}
