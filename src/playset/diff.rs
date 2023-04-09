use indent_write::indentable::Indentable;
use lasso::{Spur, ThreadedRodeo};

use crate::cw_model::{
    ConditionalBlock, Entity, Module, Namespace, Operator, PropertyInfo, PropertyInfoList,
    ToStringWithInterner, Value,
};
use std::collections::HashSet;
use std::fmt::Debug;
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

    /// Like LIOS, but for the properties of the entities instead of the entities themselves.
    MergeShallow,

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
    pub filename: Option<Changed<Spur>>,
    pub namespace: Option<Changed<Spur>>,
    pub defines: HashMapDiff<Spur, Value, ValueDiff>,
    pub properties: HashMapDiff<Spur, PropertyInfoList, PropertyInfoListDiff>,
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
    pub properties: HashMapDiff<Spur, PropertyInfoList, PropertyInfoListDiff>,
    pub conditional_blocks: HashMapDiff<Spur, ConditionalBlock, ConditionalBlockDiff>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct NamedEntityDiff(pub EntityDiff, pub Option<(Spur, Spur)>);

#[derive(Debug, PartialEq, Clone)]
pub struct PropertyInfoDiff {
    pub operator: Option<(Operator, Operator)>,
    pub value: ValueDiff,
}

#[derive(Debug, PartialEq, Clone)]
pub struct PropertyInfoListDiff(pub VecDiff<PropertyInfo, PropertyInfoDiff>);

#[derive(Debug, PartialEq, Clone)]
pub struct ConditionalBlockDiff {
    pub key: Option<((bool, Spur), (bool, Spur))>,
    pub items: VecDiff<Value, ValueDiff>,
    pub properties: HashMapDiff<Spur, PropertyInfoList, PropertyInfoListDiff>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum OperatorDiff {
    Unchanged,
    Modified(Operator, Operator),
}

#[derive(Debug, PartialEq, Clone)]
pub enum ValueDiff {
    String(Option<(Spur, Spur)>),
    Number(Option<(Spur, Spur)>),
    Boolean(Option<(bool, bool)>),
    Entity(EntityDiff),
    Define(Option<(Spur, Spur)>),
    Color(
        Option<(
            (Spur, Spur, Spur, Spur, Option<Spur>),
            (Spur, Spur, Spur, Spur, Option<Spur>),
        )>,
    ),
    Maths(Option<(Spur, Spur)>),
    TypeChanged(Value, Value),
}

#[derive(Debug, PartialEq, Clone)]
pub struct NamespaceDiff {
    // TODO to support Duplicate, an entity name actually should be working like a PropertyInfoList,
    // should probably just remove entities and use properties
    // pub entities: HashMapDiff<String, PropertyInfoList, PropertyInfoListDiff>,
    // pub entities: HashMapDiff<String, Value, ValueDiff>,
    pub defines: HashMapDiff<Spur, Value, ValueDiff>,
    pub properties: HashMapDiff<Spur, PropertyInfoList, PropertyInfoListDiff>,
    pub values: VecDiff<Value, ValueDiff>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct ModDiff {
    pub namespaces: HashMap<Spur, NamespaceDiff>,
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
    fn diff_to(&self, other: &Self, merge_mode: EntityMergeMode, interner: &ThreadedRodeo) -> T;
}

impl Diffable<ValueDiff> for Value {
    fn diff_to(
        &self,
        other: &Self,
        merge_mode: EntityMergeMode,
        interner: &ThreadedRodeo,
    ) -> ValueDiff {
        match (self, other) {
            (Value::String(a), Value::String(b)) => ValueDiff::String(Some((*a, *b))),
            (Value::Number(a), Value::Number(b)) => ValueDiff::Number(Some((*a, *b))),
            (Value::Boolean(a), Value::Boolean(b)) => ValueDiff::Boolean(Some((*a, *b))),
            (Value::Entity(a), Value::Entity(b)) => {
                ValueDiff::Entity(a.diff_to(&b, merge_mode, interner))
            }
            (Value::Define(a), Value::Define(b)) => ValueDiff::Define(Some((*a, *b))),
            (Value::Color(a), Value::Color(b)) => ValueDiff::Color(Some((a.clone(), b.clone()))),
            (Value::Maths(a), Value::Maths(b)) => ValueDiff::Maths(Some((*a, *b))),
            (a, b) => ValueDiff::TypeChanged(a.clone(), b.clone()),
        }
    }
}

impl Diffable<ModDiff> for GameMod {
    fn diff_to(
        &self,
        other: &Self,
        _merge_mode: EntityMergeMode,
        interner: &ThreadedRodeo,
    ) -> ModDiff {
        let mut namespaces = HashMap::new();

        // For each namespace in a (base), find the corresponding namespace in b, if exists, and get patch from that
        // In most cases, mod_a will be the base game and mod_b will be the mod.
        for namespace in self.namespaces.values() {
            let b_namespace = other.namespaces.get(&namespace.namespace);
            match b_namespace {
                Some(b_namespace) => {
                    let diff =
                        namespace.diff_to(b_namespace, namespace.merge_mode.clone(), interner);
                    namespaces.insert(namespace.namespace, diff);
                }
                None => {
                    let ns_spur = interner.resolve(&namespace.namespace).to_owned();
                    let fake_ns = Namespace::new(&ns_spur, None, interner);
                    namespaces.insert(
                        namespace.namespace,
                        namespace.diff_to(&fake_ns, fake_ns.merge_mode, interner),
                    );
                }
            }
        }

        ModDiff { namespaces }
    }
}

impl NamespaceDiff {
    fn merge_defines_in(&mut self, defines: HashMapDiff<Spur, Value, ValueDiff>) -> &mut Self {
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
        properties: HashMapDiff<Spur, PropertyInfoList, PropertyInfoListDiff>,
        _merge_mode: EntityMergeMode,
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
    fn diff_to(
        &self,
        other: &Namespace,
        merge_mode: EntityMergeMode,
        interner: &ThreadedRodeo,
    ) -> NamespaceDiff {
        let mut namespace_diff = NamespaceDiff {
            // entities: HashMapDiff::Unchanged,
            defines: HashMapDiff::Unchanged,
            properties: HashMapDiff::Unchanged,
            values: VecDiff::Unchanged,
        };

        for module_a in self.modules.values() {
            let module_b = other.modules.get(&module_a.filename);
            // If there's a module in B with the same name as that module in A, it overwrites the module in A,
            // so diff them to get some of the changes (including removals), the merge all those changes into the namespace's changes.
            if let Some(module_b) = module_b {
                let diff = module_a.diff_to(module_b, merge_mode, interner);
                // namespace_diff.merge_entities_in(diff.entities);
                namespace_diff.merge_defines_in(diff.defines);
                namespace_diff.merge_properties_in(diff.properties, merge_mode);
                namespace_diff.merge_values_in(diff.values);
            }
        }

        // For the rest of the entities in namespace B, if they don't exist in namespace A, they're new entities.
        // If they do exist in namespace A, it depends on the merge mode:
        // - Merge: diff them to get the changes, then merge those changes into the namespace's changes
        // - Duplicate: Add the value to the property list of the namespace as a new value
        // - LIOS: The entity overwrites the entity in A
        // - No / FIOS: Do nothing (TODO nuance)
        //diff them to get the changes. TODO: Will do some of them twice
        let mut properties_changed: HashMap<Spur, Diff<PropertyInfoList, PropertyInfoListDiff>> =
            HashMap::new();
        for (property_name, property_b) in &other.properties {
            let property_a = self.properties.get(property_name);
            if let Some(property_a) = property_a {
                if property_a == property_b {
                    continue;
                }

                // The entity exists in both A and B, so diff them to get the changes
                // then merge those changes into the namespace's changes
                let diff = property_a.diff_to(property_b, merge_mode, interner);
                match merge_mode {
                    EntityMergeMode::No => {}
                    _ => {
                        // Merge is handled in the diff, so we can just set the changes here as well
                        properties_changed.insert(property_name.to_owned(), Diff::Modified(diff));
                    }
                }
            } else {
                // It's new in B, so take the values of B and insert that for the property name
                properties_changed
                    .insert(property_name.clone(), Diff::Added(property_b.to_owned()));
            }
        }

        namespace_diff.merge_properties_in(HashMapDiff::Modified(properties_changed), merge_mode);

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
        // let entities = self.entities.apply_patch(&diff.entities);
        let defines = self.defines.apply_patch(&diff.defines);
        let properties = self.properties.apply_patch(&diff.properties);
        let values = self.values.apply_patch(&diff.values);

        Namespace {
            namespace: self.namespace.clone(),
            modules: HashMap::new(), // An applied patch doesn't have modules, it's all in the namespace only
            // entities,
            defines,
            properties,
            values,
            merge_mode: self.merge_mode,
        }
    }
}

impl Diffable<ModuleDiff> for Module {
    fn diff_to(
        &self,
        other: &Module,
        merge_mode: EntityMergeMode,
        interner: &ThreadedRodeo,
    ) -> ModuleDiff {
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

        // let entities = self.entities.diff_to(&other.entities, merge_mode);
        let defines = self.defines.diff_to(&other.defines, merge_mode, interner);
        let properties = self
            .properties
            .diff_to(&other.properties, merge_mode, interner);
        let values = self.values.diff_to(&other.values, merge_mode, interner);

        ModuleDiff {
            filename,
            namespace,
            // entities,
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
        interner: &ThreadedRodeo,
    ) -> HashMapDiff<K, V, VModified> {
        let mut modified = HashMap::new();

        let next_merge_mode = if merge_mode == EntityMergeMode::MergeShallow {
            EntityMergeMode::LIOS
        } else {
            merge_mode
        };

        for (key, value_a) in self {
            match other.get(key) {
                Some(value_b) if value_a != value_b => {
                    modified.insert(
                        key.clone(),
                        Diff::Modified(value_a.diff_to(value_b, next_merge_mode, interner)),
                    );
                }
                None => {
                    if merge_mode != EntityMergeMode::MergeShallow
                        && next_merge_mode != EntityMergeMode::Merge
                    {
                        // In a merge type merge mode, we don't want to remove anything that's missing,
                        // only add new key/value pairs and modify existing ones
                        modified.insert(key.clone(), Diff::Removed(value_a.clone()));
                    }
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

impl<T: PartialEq + Clone + Debug + JaccardIndex + Diffable<VModified>, VModified: Debug>
    Diffable<VecDiff<T, VModified>> for Vec<T>
{
    fn diff_to(
        &self,
        other: &Self,
        merge_mode: EntityMergeMode,
        interner: &ThreadedRodeo,
    ) -> VecDiff<T, VModified> {
        let next_merge_mode = if merge_mode == EntityMergeMode::MergeShallow {
            EntityMergeMode::LIOS
        } else {
            merge_mode
        };

        let mut diffs = Vec::new();

        // Keep track of claimed items so that find_map doesn't return the same item twice
        let mut claimed = HashSet::new();

        // List merging depends highly on the merge mode, in LIOS mode we want to merge the lists as much as possible
        // and in merge mode we want to only append to the list.
        if merge_mode == EntityMergeMode::LIOS
            || merge_mode == EntityMergeMode::FIOS
            || merge_mode == EntityMergeMode::Unknown
        {
            // We're in a "replace" merge mode, so find the best matches for each item in the other vec in order
            // to generate a small diff
            for value_a in self {
                let threshold = 0.4;
                let mut max_found: Option<(usize, f64, &T)> = None;

                for (i, value_b) in other.iter().enumerate() {
                    if claimed.contains(&i) == false {
                        let jaccard_index = value_a.jaccard_index(value_b, interner);
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
                        diffs.push(Diff::Modified(value_a.diff_to(
                            &value_b,
                            next_merge_mode,
                            interner,
                        )));
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
        } else if merge_mode == EntityMergeMode::MergeShallow
            || merge_mode == EntityMergeMode::Merge
            || merge_mode == EntityMergeMode::Duplicate
        {
            // In a merge type mod for a list, we can only append to the list. No such thing as matching (can only match on keys)
            // for _ in self {
            //     diffs.push(Diff::Unchanged);
            // }

            for value_b in other {
                diffs.push(Diff::Added(value_b.clone()));
            }

            if diffs.is_empty() {
                VecDiff::Unchanged
            } else {
                VecDiff::Changed(diffs)
            }
        } else {
            // We're in No merge mode, so can't merge the lists at all
            VecDiff::Unchanged
        }
    }
}

impl Diffable<EntityDiff> for Entity {
    fn diff_to(
        &self,
        other: &Self,
        merge_mode: EntityMergeMode,
        interner: &ThreadedRodeo,
    ) -> EntityDiff {
        let items = self.items.diff_to(&other.items, merge_mode, interner);
        let properties = self
            .properties
            .diff_to(&other.properties, merge_mode, interner);
        let conditional_blocks =
            self.conditional_blocks
                .diff_to(&other.conditional_blocks, merge_mode, interner);

        EntityDiff {
            items,
            properties,
            conditional_blocks,
        }
    }
}

impl Diffable<PropertyInfoListDiff> for PropertyInfoList {
    fn diff_to(
        &self,
        other: &Self,
        merge_mode: EntityMergeMode,
        interner: &ThreadedRodeo,
    ) -> PropertyInfoListDiff {
        if self == other {
            return PropertyInfoListDiff(VecDiff::Unchanged);
        }

        if self.len() == other.len() {
            if self.len() == 1 {
                let self_first = self.clone().into_vec()[0].clone();
                let other_first = other.clone().into_vec()[0].clone();
                return PropertyInfoListDiff(VecDiff::Changed(vec![Diff::Modified(
                    self_first.diff_to(&other_first, merge_mode, interner),
                )]));
            }

            let mut diff = Vec::new();

            for (a, b) in self
                .clone()
                .into_vec()
                .into_iter()
                .zip(other.clone().into_vec().into_iter())
            {
                if a.value.jaccard_index(&b.value, interner) > 0.8 {
                    diff.push(Diff::Modified(a.diff_to(&b, merge_mode, interner)));
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
            .diff_to(&other.clone().into_vec(), merge_mode, interner);

        PropertyInfoListDiff(diff)
    }
}

impl Diffable<PropertyInfoDiff> for PropertyInfo {
    fn diff_to(
        &self,
        other: &PropertyInfo,
        merge_mode: EntityMergeMode,
        interner: &ThreadedRodeo,
    ) -> PropertyInfoDiff {
        let operator = if self.operator == other.operator {
            None
        } else {
            Some((self.operator, other.operator))
        };

        let value = self.value.diff_to(&other.value, merge_mode, interner);

        PropertyInfoDiff { operator, value }
    }
}

impl Diffable<ConditionalBlockDiff> for ConditionalBlock {
    fn diff_to(
        &self,
        other: &Self,
        merge_mode: EntityMergeMode,
        interner: &ThreadedRodeo,
    ) -> ConditionalBlockDiff {
        let (a_is_not, a_key) = &self.key;
        let (b_is_not, b_key) = &other.key;

        let key = if a_is_not == b_is_not && a_key == b_key {
            None
        } else {
            Some(((*a_is_not, *a_key), (*b_is_not, *b_key)))
        };

        let items = self.items.diff_to(&other.items, merge_mode, interner);
        let properties = self
            .properties
            .diff_to(&other.properties, merge_mode, interner);

        ConditionalBlockDiff {
            items,
            key,
            properties,
        }
    }
}

impl ToStringWithInterner for NamespaceDiff {
    fn to_string_with_interner(&self, interner: &ThreadedRodeo) -> String {
        let mut buf = String::new();
        buf.push_str(&format!(
            "{}",
            self.defines.to_string_with_interner(interner)
        ));
        buf.push_str(&format!(
            "{}",
            self.properties.to_string_with_interner(interner)
        ));
        buf.push_str(&format!(
            "{}",
            self.values.to_string_with_interner(interner)
        ));
        buf
    }
}

impl ToStringWithInterner for ModuleDiff {
    fn to_string_with_interner(&self, interner: &ThreadedRodeo) -> String {
        let mut buf = String::new();
        buf.push_str(&format!(
            "{}",
            self.defines.to_string_with_interner(interner)
        ));
        buf.push_str(&format!(
            "{}",
            self.properties.to_string_with_interner(interner)
        ));
        buf.push_str(&format!(
            "{}",
            self.values.to_string_with_interner(interner)
        ));
        buf
    }
}

impl<V: ToStringWithInterner, VModified: ToStringWithInterner> ToStringWithInterner
    for Diff<V, VModified>
{
    fn to_string_with_interner(&self, interner: &ThreadedRodeo) -> String {
        match self {
            Diff::Added(added) => format!("[Added] {}", added.to_string_with_interner(interner)),
            Diff::Removed(removed) => {
                format!("[Removed] {}", removed.to_string_with_interner(interner))
            }
            Diff::Modified(modified) => {
                format!("{}", modified.to_string_with_interner(interner))
            }
            Diff::Unchanged => String::new(),
        }
    }
}

impl ToStringWithInterner for PropertyInfoListDiff {
    fn to_string_with_interner(&self, interner: &ThreadedRodeo) -> String {
        match &self.0 {
            VecDiff::Unchanged => String::new(),
            VecDiff::Changed(items) => {
                let mut buf = String::new();
                for item in items {
                    match item {
                        Diff::Added(item) => {
                            buf.push_str(&format!(
                                "[Added] {}",
                                item.to_string_with_interner(interner)
                            ));
                        }
                        Diff::Removed(item) => {
                            buf.push_str(&format!(
                                "[Removed] {}",
                                item.to_string_with_interner(interner)
                            ));
                        }
                        Diff::Modified(item) => {
                            buf.push_str(&format!("{}", item.to_string_with_interner(interner)));
                        }
                        Diff::Unchanged => {}
                    }
                }
                buf
            }
        }
    }
}

impl ToStringWithInterner for PropertyInfoDiff {
    fn to_string_with_interner(&self, interner: &ThreadedRodeo) -> String {
        let mut buf = String::new();
        if let Some((old, new)) = &self.operator {
            buf.push_str(&format!("({} -> {}) ", old, new));
        }

        buf.push_str(&format!(
            "{} ",
            self.value.to_string_with_interner(interner)
        ));
        buf
    }
}

impl<V, VModified> ToStringWithInterner for VecDiff<V, VModified>
where
    V: ToStringWithInterner,
    VModified: ToStringWithInterner,
{
    fn to_string_with_interner(&self, interner: &ThreadedRodeo) -> String {
        match self {
            VecDiff::Unchanged => String::new(),
            VecDiff::Changed(items) => {
                let mut buf = String::new();
                for item in items {
                    match item {
                        Diff::Added(item) => {
                            buf.push_str(&format!(
                                "[Added] {}",
                                item.to_string_with_interner(interner)
                            ));
                        }
                        Diff::Removed(item) => {
                            buf.push_str(&format!(
                                "[Removed] {}",
                                item.to_string_with_interner(interner)
                            ));
                        }
                        Diff::Modified(item) => {
                            buf.push_str(&format!("{}", item.to_string_with_interner(interner)))
                        }
                        Diff::Unchanged => {}
                    }
                }

                String::new()
            }
        }
    }
}

impl<K: Debug + Eq + Hash, V: ToStringWithInterner, VModified: ToStringWithInterner>
    ToStringWithInterner for HashMapDiff<K, V, VModified>
{
    fn to_string_with_interner(&self, interner: &ThreadedRodeo) -> String {
        match self {
            HashMapDiff::Unchanged => String::new(),
            HashMapDiff::Modified(pairs) => {
                let mut buf = String::new();
                for (key, diff) in pairs {
                    buf.push_str(&format!(
                        "{:?}: {}\n",
                        key,
                        diff.clone()
                            .to_string_with_interner(interner)
                            .indented_skip_initial("    ")
                    ));
                }
                buf
            }
        }
    }
}

impl ToStringWithInterner for EntityDiff {
    fn to_string_with_interner(&self, interner: &ThreadedRodeo) -> String {
        let mut buf = String::from("{\n");
        buf.push_str(&format!("{}", self.items.to_string_with_interner(interner)));
        buf.push_str(&format!(
            "{}",
            self.properties.to_string_with_interner(interner)
        ));
        buf.push_str(&format!(
            "{}",
            self.conditional_blocks.to_string_with_interner(interner)
        ));
        buf.push_str("}");
        buf
    }
}

impl ToStringWithInterner for ConditionalBlockDiff {
    fn to_string_with_interner(&self, interner: &ThreadedRodeo) -> String {
        let mut buf = String::from("[[");
        if let Some(((is_not_old, key_old), (is_not_new, key_new))) = &self.key {
            let key_old = interner.resolve(key_old);
            let old = if *is_not_old {
                "!".to_owned() + key_old
            } else {
                key_old.to_string()
            };
            let key_new = interner.resolve(key_new);
            let new = if *is_not_new {
                "!".to_owned() + key_new
            } else {
                key_new.to_string()
            };
            buf.push_str(&format!("{} -> {}", old, new));
        }
        buf.push_str(&format!("]"));
        buf.push_str(&format!("{}", self.items.to_string_with_interner(interner)));
        buf.push_str(&format!("]"));
        buf
    }
}

impl ToStringWithInterner for ValueDiff {
    fn to_string_with_interner(&self, interner: &ThreadedRodeo) -> String {
        match self {
            ValueDiff::String(Some((old, new))) => {
                format!(
                    "\"{}\" -> \"{}\"",
                    interner.resolve(old),
                    interner.resolve(new)
                )
            }
            ValueDiff::String(None) => format!("Unchanged"),
            ValueDiff::Number(Some((old, new))) => {
                format!("{} -> {}", interner.resolve(old), interner.resolve(new))
            }
            ValueDiff::Number(None) => format!("Unchanged"),
            ValueDiff::Boolean(Some((old, new))) => {
                format!("{} -> {}", old, new)
            }
            ValueDiff::Boolean(None) => format!("Unchanged"),
            ValueDiff::Define(Some((old, new))) => {
                format!("{} -> {}", interner.resolve(old), interner.resolve(new))
            }
            ValueDiff::Define(None) => format!("Unchanged"),
            ValueDiff::Color(Some(((type1, a1, b1, c1, d1), (type2, a2, b2, c2, d2)))) => {
                let (type1, a1, b1, c1, d1) = (
                    interner.resolve(type1),
                    interner.resolve(a1),
                    interner.resolve(b1),
                    interner.resolve(c1),
                    d1.as_ref().map(|d1| interner.resolve(d1)),
                );
                let (type2, a2, b2, c2, d2) = (
                    interner.resolve(type2),
                    interner.resolve(a2),
                    interner.resolve(b2),
                    interner.resolve(c2),
                    d2.as_ref().map(|d2| interner.resolve(d2)),
                );
                let d1 = match d1 {
                    Some(d1) => format!("{} ", d1),
                    None => "".to_string(),
                };
                let d2 = match d2 {
                    Some(d2) => format!("{} ", d2),
                    None => "".to_string(),
                };
                format!(
                    "{} {{ {} {} {} {} }} -> {} {{ {} {} {} {} }}",
                    type1, a1, b1, c1, d1, type2, a2, b2, c2, d2
                )
            }
            ValueDiff::Maths(Some((old, new))) => {
                format!("{} -> {}", interner.resolve(old), interner.resolve(new))
            }
            ValueDiff::Maths(None) => format!("Unchanged"),
            ValueDiff::TypeChanged(old, new) => {
                format!(
                    "{} -> {}",
                    old.to_string_with_interner(interner),
                    new.to_string_with_interner(interner)
                )
            }
            ValueDiff::Entity(entity_diff) => {
                format!("{}", entity_diff.to_string_with_interner(interner))
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
        // let entities = self.entities.apply_patch(&diff.entities);
        let defines = self.defines.apply_patch(&diff.defines);
        let properties = self.properties.apply_patch(&diff.properties);
        let values = self.values.apply_patch(&diff.values);

        Module {
            filename,
            namespace,
            // entities,
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
                    Value::String(*new)
                } else {
                    self.clone()
                }
            }
            ValueDiff::Number(option) => {
                if let Some((_, new)) = option {
                    Value::Number(*new)
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
                    Value::Define(*new)
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
                    Value::Maths(*new)
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
    use lasso::ThreadedRodeo;

    use crate::{
        cw_model::{Module, ToStringWithInterner},
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
        let interner = &ThreadedRodeo::default();

        let module_a = Module::parse(module_a_def, "type/path/", "a", interner).unwrap();
        let module_b = Module::parse(module_b_dev, "type/path/", "b", interner).unwrap();

        let diff = module_a.diff_to(&module_b, EntityMergeMode::LIOS, interner);

        print!("{}", diff.to_string_with_interner(interner));
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
        let interner = &ThreadedRodeo::default();

        let module_a = Module::parse(module_a_def, "type/path/", "a", interner).unwrap();
        let module_b = Module::parse(module_b_dev, "type/path/", "b", interner).unwrap();

        let diff = module_a.diff_to(&module_b, EntityMergeMode::LIOS, interner);

        assert_eq!(
            diff.to_string_with_interner(interner)
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
        let interner = &ThreadedRodeo::default();

        let module_a = Module::parse(module_a_def, "type/path/", "a", interner).unwrap();
        let module_b = Module::parse(module_b_dev, "type/path/", "b", interner).unwrap();

        let diff = module_a.diff_to(&module_b, EntityMergeMode::LIOS, interner);

        assert_eq!(
            diff.to_string_with_interner(interner)
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
