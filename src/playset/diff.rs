use indent_write::indentable::Indentable;

use crate::cw_model::{
    ConditionalBlock, Entity, Module, Namespace, Operator, PropertyInfo, PropertyInfoList, Value,
};
use std::collections::HashSet;
use std::fmt::{self, Debug, Display};
use std::{collections::HashMap, hash::Hash};

use super::game_mod::GameMod;
use super::jaccard::JaccardIndex;

/// Different namespaces in stellaris have different merge mechanics when it comes to entities with the same name
/// in different files. This defines the merge mode to use for entities with the same name.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum EntityMergeMode {
    /// Last-in-only-served - the last entity in the list will be the one that is used
    LIOS,

    /// First-in-only-served - the first entity in the list will be the one that is used
    FIOS,

    /// Entities with the same name will be merged
    Merge,

    /// Entities with the same name act like a PropertyInfoList, and there are multiple for the one key
    Duplicate,

    /// Entities cannot be target overridden at all, have to only overwrite at the module level
    No,

    /// Who knows!
    Unknown,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Changed<T> {
    pub old: T,
    pub new: T,
}

#[derive(Debug, PartialEq, Clone)]
pub struct ModuleDiff {
    pub filename: Option<Changed<String>>,
    pub namespace: Option<Changed<String>>,
    pub entities: HashMapDiff<String, Value, ValueDiff>,
    pub defines: HashMapDiff<String, Value, ValueDiff>,
    pub properties: HashMapDiff<String, PropertyInfoList, PropertyInfoListDiff>,
    pub values: VecDiff<Value, ValueDiff>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum HashMapDiff<K: Eq + Hash, V, VModified> {
    Unchanged,
    Modified(HashMap<K, Diff<V, VModified>>),
}

#[derive(Debug, PartialEq, Clone)]
pub enum VecDiff<T, TModified> {
    Unchanged,
    Changed(Vec<Diff<T, TModified>>),
}

#[derive(Debug, PartialEq, Clone)]
pub enum Diff<T, TModified> {
    Unchanged,
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
pub struct PropertyInfoListDiff(pub VecDiff<PropertyInfo, PropertyInfoDiff>);

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

#[derive(Debug, PartialEq, Clone)]
pub struct NamespaceDiff {
    pub entities: HashMapDiff<String, Value, ValueDiff>,
    pub defines: HashMapDiff<String, Value, ValueDiff>,
    pub properties: HashMapDiff<String, PropertyInfoList, PropertyInfoListDiff>,
    pub values: VecDiff<Value, ValueDiff>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct ModDiff {
    pub namespaces: HashMap<String, NamespaceDiff>,
}

impl<T> Changed<T> {
    pub fn new(old: T, new: T) -> Self {
        Changed { old, new }
    }

    pub fn from(old: &T, new: &T) -> Self
    where
        T: Clone,
    {
        Changed {
            old: old.clone(),
            new: new.clone(),
        }
    }
}

pub trait Diffable<T> {
    fn diff_to(&self, other: &Self, merge_mode: EntityMergeMode) -> T;
}

impl Diffable<ValueDiff> for Value {
    fn diff_to(&self, other: &Self, merge_mode: EntityMergeMode) -> ValueDiff {
        match (self, other) {
            (Value::String(a), Value::String(b)) => {
                ValueDiff::String(Some((a.to_string(), b.to_string())))
            }
            (Value::Number(a), Value::Number(b)) => {
                ValueDiff::Number(Some((a.to_string(), b.to_string())))
            }
            (Value::Boolean(a), Value::Boolean(b)) => ValueDiff::Boolean(Some((*a, *b))),
            (Value::Entity(a), Value::Entity(b)) => ValueDiff::Entity(a.diff_to(&b, merge_mode)),
            (Value::Define(a), Value::Define(b)) => {
                ValueDiff::Define(Some((a.to_string(), b.to_string())))
            }
            (Value::Color(a), Value::Color(b)) => ValueDiff::Color(Some((a.clone(), b.clone()))),
            (Value::Maths(a), Value::Maths(b)) => {
                ValueDiff::Maths(Some((a.to_string(), b.to_string())))
            }
            (a, b) => ValueDiff::TypeChanged(a.clone(), b.clone()),
        }
    }
}

impl Diffable<ModDiff> for GameMod {
    fn diff_to(&self, other: &Self, merge_mode: EntityMergeMode) -> ModDiff {
        let mut namespaces = HashMap::new();

        // For each namespace in a (base), find the corresponding namespace in b, if exists, and get patch from that
        // In most cases, mod_a will be the base game and mod_b will be the mod.
        for namespace in self.namespaces.values() {
            let b_namespace = other.namespaces.get(&namespace.namespace);
            match b_namespace {
                Some(b_namespace) => {
                    let diff = namespace.diff_to(b_namespace, merge_mode.clone());
                    namespaces.insert(namespace.namespace.to_string(), diff);
                }
                None => {
                    namespaces.insert(
                        namespace.namespace.to_string(),
                        namespace
                            .diff_to(&Namespace::new(namespace.namespace.to_owned()), merge_mode),
                    );
                }
            }
        }

        ModDiff { namespaces }
    }
}

impl NamespaceDiff {
    fn merge_entities_in(&mut self, entities: HashMapDiff<String, Value, ValueDiff>) -> &mut Self {
        if let HashMapDiff::Modified(entities) = entities {
            match self.entities {
                HashMapDiff::Unchanged => self.entities = HashMapDiff::Modified(entities),
                HashMapDiff::Modified(ref mut self_entities) => {
                    for (name, diff) in &entities {
                        self_entities.insert(name.clone(), diff.clone());
                    }
                }
            }
        }
        self
    }

    fn merge_defines_in(&mut self, defines: HashMapDiff<String, Value, ValueDiff>) -> &mut Self {
        if let HashMapDiff::Modified(defines) = defines {
            match self.defines {
                HashMapDiff::Unchanged => self.defines = HashMapDiff::Modified(defines),
                HashMapDiff::Modified(ref mut self_defines) => {
                    for (name, diff) in &defines {
                        self_defines.insert(name.clone(), diff.clone());
                    }
                }
            }
        }
        self
    }

    fn merge_properties_in(
        &mut self,
        properties: HashMapDiff<String, PropertyInfoList, PropertyInfoListDiff>,
    ) -> &mut Self {
        if let HashMapDiff::Modified(properties) = properties {
            match self.properties {
                HashMapDiff::Unchanged => self.properties = HashMapDiff::Modified(properties),
                HashMapDiff::Modified(ref mut self_properties) => {
                    for (name, diff) in &properties {
                        self_properties.insert(name.clone(), diff.clone());
                    }
                }
            }
        }
        self
    }

    fn merge_values_in(&mut self, values: VecDiff<Value, ValueDiff>) -> &mut Self {
        if let VecDiff::Changed(values) = values {
            match self.values {
                VecDiff::Unchanged => self.values = VecDiff::Changed(values),
                VecDiff::Changed(ref mut self_values) => {
                    for value in values {
                        self_values.push(value);
                    }
                }
            }
        }
        self
    }
}

impl Diffable<NamespaceDiff> for Namespace {
    fn diff_to(&self, other: &Namespace, merge_mode: EntityMergeMode) -> NamespaceDiff {
        let mut namespace_diff = NamespaceDiff {
            entities: HashMapDiff::Unchanged,
            defines: HashMapDiff::Unchanged,
            properties: HashMapDiff::Unchanged,
            values: VecDiff::Unchanged,
        };

        for module_a in self.modules.values() {
            let module_b = other.modules.get(&module_a.filename);
            // If there's a module in B with the same name as that module in A, it overwrites the module in A,
            // so diff them to get some of the changes (including removals), the merge all those changes into the namespace's changes.
            if let Some(module_b) = module_b {
                let diff = module_a.diff_to(module_b, merge_mode);
                namespace_diff.merge_entities_in(diff.entities);
                namespace_diff.merge_defines_in(diff.defines);
                namespace_diff.merge_properties_in(diff.properties);
                namespace_diff.merge_values_in(diff.values);
            }
        }

        // For the rest of the entities in namespace B, if they don't exist in namespace A, they're new entities.
        // If they do exist in namespace A, diff them to get the changes. TODO: Will do some of them twice
        let mut entities_changed: HashMap<String, Diff<Value, ValueDiff>> = HashMap::new();
        for (entity_name, entity_b) in &other.entities {
            let entity_a = self.entities.get(entity_name);
            if let Some(entity_a) = entity_a {
                if entity_a == entity_b {
                    continue;
                }

                // The entity exists in both A and B, so diff them to get the changes
                // then merge those changes into the namespace's changes
                let diff = entity_a.diff_to(entity_b, merge_mode);
                match diff {
                    ValueDiff::Entity(_) => {
                        entities_changed.insert(entity_name.clone(), Diff::Modified(diff));
                    }
                    _ => {}
                }
            } else {
                // It's new in B, so add it to the list of changed entities
                let new_value: Diff<Value, ValueDiff> = Diff::Added(entity_b.to_owned());
                entities_changed.insert(entity_name.clone(), new_value);
            }
        }
        namespace_diff.merge_entities_in(HashMapDiff::Modified(entities_changed));

        namespace_diff
    }
}

impl ApplyPatch<ModDiff> for GameMod {
    fn apply_patch(&self, diff: &ModDiff) -> Self {
        let mut namespaces = HashMap::new();

        for namespace in self.namespaces.values() {
            let mut namespace = namespace.clone();
            if let Some(namespace_diff) = diff.namespaces.get(&namespace.namespace) {
                namespace = namespace.apply_patch(namespace_diff);
            }
            namespaces.insert(namespace.namespace.clone(), namespace);
        }

        let mut game_mod = self.clone();
        game_mod.namespaces = namespaces;
        game_mod.modules = vec![]; // An applied patch doesn't have modules, it's all in the namespace only

        game_mod
    }
}

impl ApplyPatch<NamespaceDiff> for Namespace {
    fn apply_patch(&self, diff: &NamespaceDiff) -> Self {
        let entities = self.entities.apply_patch(&diff.entities);
        let defines = self.defines.apply_patch(&diff.defines);
        let properties = self.properties.apply_patch(&diff.properties);
        let values = self.values.apply_patch(&diff.values);

        Namespace {
            namespace: self.namespace.clone(),
            modules: HashMap::new(), // An applied patch doesn't have modules, it's all in the namespace only
            entities,
            defines,
            properties,
            values,
        }
    }
}

impl Diffable<ModuleDiff> for Module {
    fn diff_to(&self, other: &Module, merge_mode: EntityMergeMode) -> ModuleDiff {
        let filename = if self.filename != other.filename {
            Some(Changed::from(&self.filename, &other.filename))
        } else {
            None
        };

        let namespace = if self.namespace != other.namespace {
            Some(Changed::from(&self.namespace, &other.namespace))
        } else {
            None
        };

        let entities = self.entities.diff_to(&other.entities, merge_mode);
        let defines = self.defines.diff_to(&other.defines, merge_mode);
        let properties = self.properties.diff_to(&other.properties, merge_mode);
        let values = self.values.diff_to(&other.values, merge_mode);

        ModuleDiff {
            filename,
            namespace,
            entities,
            defines,
            properties,
            values,
        }
    }
}

impl<K: Eq + Hash + Clone, V: PartialEq + Eq + Clone + Diffable<VModified>, VModified>
    Diffable<HashMapDiff<K, V, VModified>> for HashMap<K, V>
{
    fn diff_to(
        &self,
        other: &HashMap<K, V>,
        merge_mode: EntityMergeMode,
    ) -> HashMapDiff<K, V, VModified> {
        let mut modified = HashMap::new();

        for (key, value_a) in self {
            match other.get(key) {
                Some(value_b) if value_a != value_b => {
                    modified.insert(
                        key.clone(),
                        Diff::Modified(value_a.diff_to(value_b, merge_mode)),
                    );
                }
                None => {
                    modified.insert(key.clone(), Diff::Removed(value_a.clone()));
                }
                _ => {}
            }
        }

        for (key, value_b) in other {
            if !self.contains_key(key) {
                modified.insert(key.clone(), Diff::Added(value_b.clone()));
            }
        }

        if modified.is_empty() {
            HashMapDiff::Unchanged
        } else {
            HashMapDiff::Modified(modified)
        }
    }
}

impl<T: PartialEq + Clone + Debug + JaccardIndex + Diffable<VModified>, VModified>
    Diffable<VecDiff<T, VModified>> for Vec<T>
{
    fn diff_to(&self, other: &Self, merge_mode: EntityMergeMode) -> VecDiff<T, VModified> {
        let mut diffs = Vec::new();

        // Keep track of claimed items so that find_map doesn't return the same item twice
        let mut claimed = HashSet::new();

        for value_a in self {
            let threshold = 0.4;
            let mut max_found: Option<(usize, f64, &T)> = None;

            for (i, value_b) in other.iter().enumerate() {
                if claimed.contains(&i) == false {
                    let jaccard_index = value_a.jaccard_index(value_b);
                    if jaccard_index > threshold {
                        if let Some((_, max_jaccard_index, _)) = max_found {
                            if jaccard_index > max_jaccard_index {
                                max_found = Some((i, jaccard_index, value_b));
                            }
                        } else {
                            max_found = Some((i, jaccard_index, value_b));
                        }
                    }
                }
            }

            if let Some(max_found) = max_found {
                let (i, _, value_b) = max_found;
                claimed.insert(i);
                if *value_a == *value_b {
                    diffs.push(Diff::Unchanged);
                } else {
                    diffs.push(Diff::Modified(value_a.diff_to(&value_b, merge_mode)));
                }
            } else {
                diffs.push(Diff::Removed(value_a.clone()));
            }
        }

        for value_b in other {
            if !self.contains(value_b) {
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

impl Diffable<EntityDiff> for Entity {
    fn diff_to(&self, other: &Self, merge_mode: EntityMergeMode) -> EntityDiff {
        let items = self.items.diff_to(&other.items, merge_mode);
        let properties = self.properties.diff_to(&other.properties, merge_mode);
        let conditional_blocks = self
            .conditional_blocks
            .diff_to(&other.conditional_blocks, merge_mode);

        EntityDiff {
            items,
            properties,
            conditional_blocks,
        }
    }
}

impl Diffable<PropertyInfoListDiff> for PropertyInfoList {
    fn diff_to(&self, other: &Self, merge_mode: EntityMergeMode) -> PropertyInfoListDiff {
        if self == other {
            return PropertyInfoListDiff(VecDiff::Unchanged);
        }

        if self.len() == other.len() {
            if self.len() == 1 {
                let self_first = self.clone().into_vec()[0].clone();
                let other_first = other.clone().into_vec()[0].clone();
                return PropertyInfoListDiff(VecDiff::Changed(vec![Diff::Modified(
                    self_first.diff_to(&other_first, merge_mode),
                )]));
            }

            let mut diff = Vec::new();

            for (a, b) in self
                .clone()
                .into_vec()
                .into_iter()
                .zip(other.clone().into_vec().into_iter())
            {
                if a.value.jaccard_index(&b.value) > 0.8 {
                    diff.push(Diff::Modified(a.diff_to(&b, merge_mode)));
                } else {
                    diff.push(Diff::Removed(a));
                    diff.push(Diff::Added(b));
                }
            }

            return PropertyInfoListDiff(VecDiff::Changed(diff));
        }

        let diff = self
            .clone()
            .into_vec()
            .diff_to(&other.clone().into_vec(), merge_mode);

        PropertyInfoListDiff(diff)
    }
}

impl Diffable<PropertyInfoDiff> for PropertyInfo {
    fn diff_to(&self, other: &PropertyInfo, merge_mode: EntityMergeMode) -> PropertyInfoDiff {
        let operator = if self.operator == other.operator {
            None
        } else {
            Some((self.operator, other.operator))
        };

        let value = self.value.diff_to(&other.value, merge_mode);

        PropertyInfoDiff { operator, value }
    }
}

impl Diffable<ConditionalBlockDiff> for ConditionalBlock {
    fn diff_to(&self, other: &Self, merge_mode: EntityMergeMode) -> ConditionalBlockDiff {
        let (a_is_not, a_key) = &self.key;
        let (b_is_not, b_key) = &other.key;

        let key = if a_is_not == b_is_not && a_key == b_key {
            None
        } else {
            Some((
                (a_is_not.clone(), a_key.clone()),
                (b_is_not.clone(), b_key.clone()),
            ))
        };

        let items = self.items.diff_to(&other.items, merge_mode);
        let properties = self.properties.diff_to(&other.properties, merge_mode);

        ConditionalBlockDiff {
            items,
            key: key,
            properties,
        }
    }
}

impl Display for NamespaceDiff {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.defines)?;
        write!(f, "{}", self.properties)?;
        write!(f, "{}", &self.entities)?;
        write!(f, "{}", self.values)?;
        Ok(())
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
            Diff::Unchanged => Ok(()),
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
                        Diff::Unchanged => {}
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
                        Diff::Unchanged => {}
                    }
                }

                Ok(())
            }
        }
    }
}

impl<K: Display + Eq + Hash, V: Display, VModified: Display> Display
    for HashMapDiff<K, V, VModified>
{
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
            .map(|c| c.new.clone())
            .unwrap_or_else(|| self.filename.clone());
        let namespace = diff
            .namespace
            .as_ref()
            .map(|c| c.new.clone())
            .unwrap_or_else(|| self.namespace.clone());
        let entities = self.entities.apply_patch(&diff.entities);
        let defines = self.defines.apply_patch(&diff.defines);
        let properties = self.properties.apply_patch(&diff.properties);
        let values = self.values.apply_patch(&diff.values);

        Module {
            filename,
            namespace,
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
                        Diff::Unchanged => {}
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
                        Diff::Unchanged => {}
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
                        Diff::Unchanged => {}
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
        playset::diff::{Diffable, EntityMergeMode},
    };

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

        let diff = module_a.diff_to(&module_b, EntityMergeMode::LIOS);

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

        let diff = module_a.diff_to(&module_b, EntityMergeMode::LIOS);

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

        let diff = module_a.diff_to(&module_b, EntityMergeMode::LIOS);

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
