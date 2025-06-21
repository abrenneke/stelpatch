use indent_write::indentable::Indentable;
use lasso::{Spur, ThreadedRodeo};

use cw_parser::model::{
    ConditionalBlock, Entity, EntityMergeMode, Module, Namespace, Operator, Properties,
    PropertyInfo, PropertyInfoList, ToStringWithInterner, Value,
};
use std::collections::HashSet;
use std::fmt::Debug;
use std::{collections::HashMap, hash::Hash};

use super::game_mod::GameMod;
use super::jaccard::JaccardIndex;

#[derive(Debug, PartialEq, Clone)]
pub struct Changed<T> {
    pub old: T,
    pub new: T,
}

#[derive(Debug, PartialEq, Clone)]
pub struct ModuleDiff {
    pub filename: Option<Changed<Spur>>,
    pub namespace: Option<Changed<Spur>>,
    pub properties: PropertiesDiff,
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
    pub properties: PropertiesDiff,
    pub conditional_blocks: HashMapDiff<Spur, ConditionalBlock, ConditionalBlockDiff>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct PropertiesDiff {
    pub kv: HashMapDiff<Spur, PropertyInfoList, PropertyInfoListDiff>,
    pub is_module: bool,
}

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
    pub properties: PropertiesDiff,
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
    pub properties: PropertiesDiff,
    pub values: VecDiff<Value, ValueDiff>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct ModDiff {
    pub namespaces: NamespaceMap,
}

#[derive(Debug, PartialEq, Clone)]
pub struct NamespaceMap(pub HashMap<Spur, NamespaceDiff>);

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
                    // let ns_spur = interner.resolve(&namespace.namespace).to_owned();
                    // let fake_ns = Namespace::new(&ns_spur, None, interner);
                    // namespaces.insert(
                    //     namespace.namespace,
                    //     namespace.diff_to(&fake_ns, fake_ns.merge_mode, interner),
                    // );
                }
            }
        }

        ModDiff {
            namespaces: NamespaceMap(namespaces),
        }
    }
}

impl NamespaceDiff {
    fn merge_properties_in(
        &mut self,
        properties: HashMapDiff<Spur, PropertyInfoList, PropertyInfoListDiff>,
        _merge_mode: EntityMergeMode,
        interner: &ThreadedRodeo,
    ) -> &mut Self {
        let properties_without_variables =
            |properties: &HashMap<Spur, Diff<PropertyInfoList, PropertyInfoListDiff>>| {
                properties
                    .iter()
                    .filter(|(name, _)| !interner.resolve(name).starts_with("@"))
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect::<HashMap<_, _>>()
            };

        if let HashMapDiff::Modified(properties) = properties {
            match self.properties.kv {
                HashMapDiff::Unchanged => {
                    self.properties = PropertiesDiff {
                        kv: HashMapDiff::Modified(properties_without_variables(&properties)),
                        is_module: true,
                    }
                }
                HashMapDiff::Modified(ref mut self_properties) => {
                    for (name, diff) in properties_without_variables(&properties) {
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
            properties: PropertiesDiff {
                kv: HashMapDiff::Unchanged,
                is_module: true,
            },
            values: VecDiff::Unchanged,
        };

        for module_a in self.modules.values() {
            let module_b = other.modules.get(&module_a.filename);
            // If there's a module in B with the same name as that module in A, it overwrites the module in A,
            // so diff them to get some of the changes (including removals), the merge all those changes into the namespace's changes.
            if let Some(module_b) = module_b {
                let diff = module_a.diff_to(module_b, merge_mode, interner);
                namespace_diff.merge_properties_in(diff.properties.kv, merge_mode, interner);
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
        let properties_diff = self
            .properties
            .diff_to(&other.properties, merge_mode, interner);

        namespace_diff.merge_properties_in(properties_diff.kv, merge_mode, interner);

        namespace_diff
    }
}

impl ApplyPatch<ModDiff> for GameMod {
    fn apply_patch(&self, diff: &ModDiff) -> Self {
        let mut namespaces = HashMap::new();

        for namespace in self.namespaces.values() {
            let mut namespace = namespace.clone();
            if let Some(namespace_diff) = diff.namespaces.0.get(&namespace.namespace) {
                namespace = namespace.apply_patch(namespace_diff);
            }
            namespaces.insert(namespace.namespace.clone(), namespace);
        }

        let mut game_mod = self.clone();
        game_mod.namespaces = namespaces;

        game_mod
    }
}

impl ApplyPatch<NamespaceDiff> for Namespace {
    fn apply_patch(&self, diff: &NamespaceDiff) -> Self {
        let properties = self.properties.apply_patch(&diff.properties);
        let values = self.values.apply_patch(&diff.values);

        Namespace {
            namespace: self.namespace.clone(),
            modules: HashMap::new(), // An applied patch doesn't have modules, it's all in the namespace only
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

        let properties = self
            .properties
            .diff_to(&other.properties, merge_mode, interner);
        let values = self.values.diff_to(&other.values, merge_mode, interner);

        ModuleDiff {
            filename,
            namespace,
            properties,
            values,
        }
    }
}

impl Diffable<PropertiesDiff> for Properties {
    fn diff_to(
        &self,
        other: &Self,
        merge_mode: EntityMergeMode,
        interner: &ThreadedRodeo,
    ) -> PropertiesDiff {
        let mut modified = HashMap::new();

        // If these properties are for a top-level entity and we're on MergeShallow, then any next entities will be LIOS
        let next_merge_mode =
            if merge_mode == EntityMergeMode::MergeShallow && self.is_module == false {
                EntityMergeMode::LIOS
            } else {
                merge_mode
            };

        let append_only = |modified: &mut HashMap<_, _>, key: &Spur, value: &PropertyInfoList| {
            let diff = PropertyInfoListDiff(VecDiff::Changed(
                value.0.iter().map(|x| Diff::Added(x.clone())).collect(),
            ));
            modified.insert(key.clone(), Diff::Modified(diff));
        };

        for (key, value_a) in &self.kv {
            match other.kv.get(key) {
                Some(value_b) if value_a != value_b => {
                    if merge_mode == EntityMergeMode::FIOS {
                        // In a FIOS merge mode, anything that exists must stay as-is.
                    } else if let EntityMergeMode::FIOSKeyed(_) = merge_mode {
                        if self.is_module {
                            // For the purposes of a *diff* we treat this like "duplicates" mode, and append-only
                            append_only(&mut modified, key, value_b);
                        }
                    } else if merge_mode == EntityMergeMode::Duplicate {
                        append_only(&mut modified, key, value_b);
                    } else {
                        let diff = value_a.diff_to(value_b, next_merge_mode, interner);
                        modified.insert(key.clone(), Diff::Modified(diff));
                    }
                }
                None => {
                    if merge_mode != EntityMergeMode::MergeShallow
                        && next_merge_mode != EntityMergeMode::Merge
                        && !self.is_module
                    {
                        // In a merge type merge mode, we don't want to remove anything that's missing,
                        // only add new key/value pairs and modify existing ones
                        modified.insert(key.clone(), Diff::Removed(value_a.clone()));
                    }
                }
                _ => {}
            }
        }

        for (key, value_b) in &other.kv {
            if !self.kv.contains_key(key) {
                modified.insert(key.clone(), Diff::Added(value_b.clone()));
            }
        }

        if modified.is_empty() {
            PropertiesDiff {
                kv: HashMapDiff::Unchanged,
                is_module: self.is_module,
            }
        } else {
            PropertiesDiff {
                kv: HashMapDiff::Modified(modified),
                is_module: self.is_module,
            }
        }
    }
}

impl Diffable<HashMapDiff<Spur, ConditionalBlock, ConditionalBlockDiff>>
    for HashMap<Spur, ConditionalBlock>
{
    fn diff_to(
        &self,
        other: &Self,
        merge_mode: EntityMergeMode,
        interner: &ThreadedRodeo,
    ) -> HashMapDiff<Spur, ConditionalBlock, ConditionalBlockDiff> {
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
                        let similarity = if value_a == value_b {
                            1.0
                        } else {
                            value_a.jaccard_index(value_b, interner)
                        };
                        if similarity > threshold {
                            if let Some((_, max_jaccard_index, _)) = max_found {
                                if similarity > max_jaccard_index {
                                    max_found = Some((i, similarity, value_b));
                                }
                            } else {
                                max_found = Some((i, similarity, value_b));
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
        } else if merge_mode == EntityMergeMode::Merge || merge_mode == EntityMergeMode::Duplicate {
            // In a merge type mod for a list, we can only append to the list. No such thing as matching (can only match on keys)

            for value_b in other {
                diffs.push(Diff::Added(value_b.clone()));
            }

            if diffs.is_empty() {
                VecDiff::Unchanged
            } else {
                VecDiff::Changed(diffs)
            }
        } else if merge_mode == EntityMergeMode::MergeShallow {
            if self.len() > 1 || other.len() > 1 {
                panic!(
                    "Merge shallow mode can only be used on lists of length 1. base: {}, other: {}",
                    self.len(),
                    other.len()
                );
            }

            if other.len() == 0 {
                return VecDiff::Unchanged;
            }

            if self.len() == 0 {
                panic!(
                    "Merge shallow mode can only be used on lists of length 1. base: {}, other: {}",
                    self.len(),
                    other.len()
                );
            }

            let val_a = &self[0];
            let val_b = &other[0];

            if val_a == val_b {
                return VecDiff::Unchanged;
            }

            let diff = self[0].diff_to(&other[0], merge_mode, interner);

            VecDiff::Changed(vec![Diff::Modified(diff)])
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

        // Lengths are the same in LIOS mode, so we can diff the entries individually
        if self.len() == other.len()
            && (merge_mode == EntityMergeMode::LIOS
                || merge_mode == EntityMergeMode::Unknown
                || merge_mode == EntityMergeMode::Merge)
        {
            // No duplicate entries under one key, most common case, no diff needed
            if self.len() == 1 {
                let self_first = self.clone().into_vec()[0].clone();
                let other_first = other.clone().into_vec()[0].clone();
                return PropertyInfoListDiff(VecDiff::Changed(vec![Diff::Modified(
                    self_first.diff_to(&other_first, merge_mode, interner),
                )]));
            }

            let mut diff = Vec::new();

            for (a, b) in self.0.iter().zip(other.0.iter()) {
                if a.value.jaccard_index(&b.value, interner) > 0.8 {
                    diff.push(Diff::Modified(a.diff_to(&b, merge_mode, interner)));
                } else {
                    diff.push(Diff::Removed(a.to_owned()));
                    diff.push(Diff::Added(b.to_owned()));
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

impl ToStringWithInterner for ModDiff {
    fn to_string_with_interner(&self, interner: &ThreadedRodeo) -> String {
        let mut buf = String::new();
        buf.push_str(&format!(
            "{}",
            self.namespaces.to_string_with_interner(interner)
        ));
        buf
    }
}

impl ToStringWithInterner for NamespaceMap {
    fn to_string_with_interner(&self, interner: &ThreadedRodeo) -> String {
        let mut buf = String::new();

        for (key, value) in &self.0 {
            buf.push_str(&format!(
                "[{}]\n{}",
                interner.resolve(key),
                value.to_string_with_interner(interner)
            ));
        }

        buf
    }
}

impl ToStringWithInterner for NamespaceDiff {
    fn to_string_with_interner(&self, interner: &ThreadedRodeo) -> String {
        let mut buf = String::new();
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

                buf
            }
        }
    }
}

impl ToStringWithInterner for PropertiesDiff {
    fn to_string_with_interner(&self, interner: &ThreadedRodeo) -> String {
        match &self.kv {
            HashMapDiff::Unchanged => String::new(),
            HashMapDiff::Modified(pairs) => {
                let mut buf = String::new();
                for (key, diff) in pairs {
                    buf.push_str(&format!(
                        "{}: {}\n",
                        interner.resolve(key),
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

impl ToStringWithInterner for HashMapDiff<Spur, ConditionalBlock, ConditionalBlockDiff> {
    fn to_string_with_interner(&self, interner: &ThreadedRodeo) -> String {
        match self {
            HashMapDiff::Unchanged => String::new(),
            HashMapDiff::Modified(pairs) => {
                let mut buf = String::new();
                for (key, diff) in pairs {
                    buf.push_str(&format!(
                        "{:?}: {}\n",
                        interner.resolve(key),
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
        let properties = self.properties.apply_patch(&diff.properties);
        let values = self.values.apply_patch(&diff.values);

        Module {
            filename,
            namespace,
            properties,
            values,
        }
    }
}

impl ApplyPatch<PropertiesDiff> for Properties {
    fn apply_patch(&self, diff: &PropertiesDiff) -> Self {
        match &diff.kv {
            HashMapDiff::Unchanged => self.clone(),
            HashMapDiff::Modified(modified_pairs) => {
                let mut result = self.clone();
                for (key, change) in modified_pairs {
                    match change {
                        Diff::Added(value) => {
                            result.kv.insert(key.clone(), value.clone());
                        }
                        Diff::Removed(_) => {
                            result.kv.remove(key);
                        }
                        Diff::Modified(modified_value) => {
                            if let Some(existing_value) = result.kv.get_mut(key) {
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

impl ApplyPatch<HashMapDiff<Spur, ConditionalBlock, ConditionalBlockDiff>>
    for HashMap<Spur, ConditionalBlock>
{
    fn apply_patch(
        &self,
        diff: &HashMapDiff<Spur, ConditionalBlock, ConditionalBlockDiff>,
    ) -> Self {
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
    use regex::Regex;

    use crate::{
        cw_parser::model::{Module, ToStringWithInterner},
        playset::diff::{Diffable, EntityMergeMode},
    };

    fn check_diff(module_a_def: &str, module_b_def: &str, expected_diff: &str) {
        let mut interner = ThreadedRodeo::new();
        let module_a = Module::parse(module_a_def, "type/path", "a", &mut interner).unwrap();
        let module_b = Module::parse(module_b_def, "type/path", "b", &mut interner).unwrap();
        let diff = module_a.diff_to(&module_b, EntityMergeMode::LIOS, &interner);

        let replacer = Regex::new(r#"(\s+)"#).unwrap();

        let diff_str_raw = diff.to_string_with_interner(&interner);
        let diff_str = replacer.replace_all(&diff_str_raw, " ");
        let expected_diff_str = replacer.replace_all(expected_diff, " ");

        if diff_str != expected_diff_str {
            println!(
                "{}",
                colored_diff::PrettyDifference {
                    expected: &expected_diff_str,
                    actual: &diff_str
                }
            );
            panic!();
        }
    }

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

        let module_b_def = r#"
            @define1 = 1
            @define2 = 3

            val_1 = "CHANGED" # Changed
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
                # added
                entity_2_property_5 = "string_5"
            }

            entity_unchanged = {}
            
            # added
            entity_3 = {
                entity_3_property_1 = "string_1"
            }"#;

        check_diff(
            module_a_def,
            module_b_def,
            "entity_2: { entity_2_property_5: [Added] = string_5 } val_1: \"string_1\" -> \"CHANGED\"",
        );
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
