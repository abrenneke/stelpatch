use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use cw_model::{CwtType, Entity, LowerCaseHashMap, PropertyInfoList, ReferenceType};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::handlers::{
    cache::{
        EntityRestructurer, GameDataCache, TypeCache, get_namespace_entity_type,
        resolver::TypeResolver,
    },
    scoped_type::{CwtTypeOrSpecialRef, PropertyNavigationResult, ScopedType},
};

pub struct ValueSetCollector<'resolver> {
    value_sets: LowerCaseHashMap<HashSet<String>>,
    type_resolver: &'resolver TypeResolver,
}

impl<'resolver> ValueSetCollector<'resolver> {
    pub fn new(type_resolver: &'resolver TypeResolver) -> Self {
        Self {
            value_sets: LowerCaseHashMap::new(),
            type_resolver,
        }
    }

    pub fn collect(mut self) -> LowerCaseHashMap<HashSet<String>> {
        // Get namespaces from GameDataCache, then use EntityRestructurer for entity access
        let namespaces = match GameDataCache::get() {
            Some(game_data) => game_data.get_namespaces(),
            None => return LowerCaseHashMap::new(), // Early return if game data not available
        };

        // Collect value_sets from parallel processing using EntityRestructurer
        let results: Vec<HashMap<String, HashSet<String>>> = namespaces
            .par_iter()
            .filter_map(|(namespace, _namespace_data)| {
                get_namespace_entity_type(namespace, None) // TODO: Add file_path
                    .and_then(|namespace_type| namespace_type.scoped_type)
                    .map(|scoped_type| {
                        self.collect_value_sets_from_namespace(namespace, scoped_type)
                    })
            })
            .collect();

        // Merge all results into the main value_sets HashMap
        for result in results {
            for (key, values) in result {
                self.value_sets.entry(key).or_default().extend(values);
            }
        }

        self.value_sets
    }

    fn collect_value_sets_from_namespace(
        &self,
        namespace: &str,
        scoped_type: Arc<ScopedType>,
    ) -> HashMap<String, HashSet<String>> {
        // Use EntityRestructurer to get entities instead of direct GameDataCache access
        let entities = match EntityRestructurer::get_all_namespace_entities(namespace) {
            Some(entities) => entities,
            None => return HashMap::new(),
        };

        // Process entities in parallel within the namespace
        let results: Vec<HashMap<String, HashSet<String>>> = entities
            .as_inner()
            .par_iter()
            .map(|(entity_name, entity)| {
                // Perform subtype narrowing for this entity, similar to provider.rs
                let narrowed_scoped_type =
                    self.narrow_entity_type(entity_name, entity, scoped_type.clone());
                self.collect_value_sets_from_entity(entity, narrowed_scoped_type)
            })
            .collect();

        // Merge results from this namespace
        let mut namespace_value_sets: HashMap<String, HashSet<String>> = HashMap::new();
        for result in results {
            for (key, values) in result {
                namespace_value_sets.entry(key).or_default().extend(values);
            }
        }

        namespace_value_sets
    }

    fn narrow_entity_type(
        &self,
        _entity_name: &str,
        entity: &Entity,
        scoped_type: Arc<ScopedType>,
    ) -> Arc<ScopedType> {
        // Check if TypeCache is available for subtype narrowing
        let type_cache = match TypeCache::get() {
            Some(cache) => cache,
            None => return scoped_type, // Return original type if TypeCache not available
        };

        let filtered_scoped_type =
            type_cache.filter_union_types_by_properties(scoped_type.clone(), &entity);

        // Perform subtype narrowing with the entity data
        let matching_subtypes = type_cache
            .get_resolver()
            .determine_matching_subtypes(filtered_scoped_type.clone(), &entity);

        if !matching_subtypes.is_empty() {
            Arc::new(filtered_scoped_type.with_subtypes(matching_subtypes))
        } else {
            filtered_scoped_type
        }
    }

    fn collect_value_sets_from_entity(
        &self,
        entity: &Entity,
        scoped_type: Arc<ScopedType>,
    ) -> HashMap<String, HashSet<String>> {
        let mut entity_value_sets: HashMap<String, HashSet<String>> = HashMap::new();

        for (property_name, property_value) in entity.properties.kv.iter() {
            let property_type = self
                .type_resolver
                .navigate_to_property(scoped_type.clone(), property_name);

            if let PropertyNavigationResult::Success(property_type) = property_type {
                let nested_results =
                    self.collect_value_sets_from_property(property_value, property_type);
                for (key, values) in nested_results {
                    entity_value_sets.entry(key).or_default().extend(values);
                }
            }
        }

        // Process items (new behavior for constructs like flags = { value_set[planet_flag] })
        if !entity.items.is_empty() {
            let item_results = self.collect_value_sets_from_items(&entity.items, scoped_type);
            for (key, values) in item_results {
                entity_value_sets.entry(key).or_default().extend(values);
            }
        }

        entity_value_sets
    }

    fn collect_value_sets_from_property(
        &self,
        property_value: &PropertyInfoList,
        property_type: Arc<ScopedType>,
    ) -> HashMap<String, HashSet<String>> {
        let mut property_value_sets: HashMap<String, HashSet<String>> = HashMap::new();

        match property_type.cwt_type_for_matching() {
            CwtTypeOrSpecialRef::Reference(ReferenceType::ValueSet { key }) => {
                let mut values = HashSet::new();
                for value in property_value.0.iter() {
                    if let Some(value) = value.value.as_string() {
                        values.insert(value.clone());
                    }
                }
                if !values.is_empty() {
                    property_value_sets
                        .entry(key.clone())
                        .or_default()
                        .extend(values);
                }
            }
            CwtTypeOrSpecialRef::Block(_) => {
                for value in property_value.0.iter() {
                    if let Some(value) = value.value.as_entity() {
                        let nested_results =
                            self.collect_value_sets_from_entity(value, property_type.clone());
                        for (key, values) in nested_results {
                            property_value_sets.entry(key).or_default().extend(values);
                        }
                    }
                }
            }
            CwtTypeOrSpecialRef::Union(union_types) => {
                // Process all union members by creating scoped types for each
                for union_type in union_types {
                    // Create a scoped type for this union member
                    let union_member_type = Arc::new(ScopedType::new_cwt_with_subtypes(
                        union_type.clone(),
                        property_type.scope_stack().clone(),
                        property_type.subtypes().clone(),
                        property_type.in_scripted_effect_block().cloned(),
                    ));

                    // Recursively process this union member
                    let nested_results =
                        self.collect_value_sets_from_property(property_value, union_member_type);
                    for (key, values) in nested_results {
                        property_value_sets.entry(key).or_default().extend(values);
                    }
                }
            }
            CwtTypeOrSpecialRef::ScopedUnion(scoped_union) => {
                // Process all scoped union members using the same logic
                for scoped_type in scoped_union {
                    let nested_results =
                        self.collect_value_sets_from_property(property_value, scoped_type.clone());
                    for (key, values) in nested_results {
                        property_value_sets.entry(key).or_default().extend(values);
                    }
                }
            }
            _ => {}
        }

        property_value_sets
    }

    fn collect_value_sets_from_items(
        &self,
        items: &[cw_model::Value],
        scoped_type: Arc<ScopedType>,
    ) -> HashMap<String, HashSet<String>> {
        let mut item_value_sets: HashMap<String, HashSet<String>> = HashMap::new();

        // Check if the scoped type has additional flags that are value sets
        if let CwtTypeOrSpecialRef::Block(block_type) = scoped_type.cwt_type_for_matching() {
            for additional_flag in &block_type.additional_flags {
                if let CwtType::Reference(ReferenceType::ValueSet { key }) = &**additional_flag {
                    let mut values = HashSet::new();
                    for item in items {
                        if let Some(string_value) = item.as_string() {
                            values.insert(string_value.clone());
                        }
                    }
                    if !values.is_empty() {
                        item_value_sets.insert(key.clone(), values);
                    }
                }
            }
        }

        item_value_sets
    }
}
