use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use cw_model::{CwtType, Entity, ReferenceType};
use rayon::prelude::*;

use crate::handlers::{
    cache::{
        Namespace, game_data::GameDataCache, get_namespace_entity_type, resolver::TypeResolver,
    },
    scoped_type::{CwtTypeOrSpecial, PropertyNavigationResult, ScopedType},
};

pub struct ValueSetCollector<'game_data, 'resolver> {
    value_sets: HashMap<String, HashSet<String>>,
    game_data: &'game_data GameDataCache,
    type_resolver: &'resolver TypeResolver,
}

impl<'game_data, 'resolver> ValueSetCollector<'game_data, 'resolver> {
    pub fn new(
        game_data: &'game_data GameDataCache,
        type_resolver: &'resolver TypeResolver,
    ) -> Self {
        Self {
            value_sets: HashMap::new(),
            game_data,
            type_resolver,
        }
    }

    pub fn value_sets(&self) -> &HashMap<String, HashSet<String>> {
        &self.value_sets
    }

    pub fn collect_from_game_data(&mut self) {
        // Collect results from parallel processing
        let results: Vec<HashMap<String, HashSet<String>>> = self
            .game_data
            .get_namespaces()
            .par_iter()
            .filter_map(|(namespace, namespace_data)| {
                get_namespace_entity_type(namespace)
                    .and_then(|namespace_type| namespace_type.scoped_type)
                    .map(|scoped_type| self.collect_from_namespace(namespace_data, scoped_type))
            })
            .collect();

        // Merge all results into the main value_sets HashMap
        for result in results {
            for (key, values) in result {
                self.value_sets.entry(key).or_default().extend(values);
            }
        }
    }

    fn collect_from_namespace(
        &self,
        namespace_data: &Namespace,
        scoped_type: Arc<ScopedType>,
    ) -> HashMap<String, HashSet<String>> {
        // Process entities in parallel within the namespace
        let results: Vec<HashMap<String, HashSet<String>>> = namespace_data
            .entities
            .par_iter()
            .map(|(_entity_name, entity)| self.collect_from_entity(entity, scoped_type.clone()))
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

    fn collect_from_entity(
        &self,
        entity: &Entity,
        scoped_type: Arc<ScopedType>,
    ) -> HashMap<String, HashSet<String>> {
        let mut entity_value_sets = HashMap::new();

        for (property_name, property_value) in entity.properties.kv.iter() {
            let property_type = self
                .type_resolver
                .navigate_to_property(scoped_type.clone(), property_name);

            if let PropertyNavigationResult::Success(property_type) = property_type {
                match property_type.cwt_type() {
                    CwtTypeOrSpecial::CwtType(CwtType::Reference(ReferenceType::ValueSet {
                        key,
                    })) => {
                        let mut values = HashSet::new();
                        for value in property_value.0.iter() {
                            if let Some(value) = value.value.as_string() {
                                values.insert(value.clone());
                            }
                        }
                        if !values.is_empty() {
                            entity_value_sets.insert(key.clone(), values);
                        }
                    }
                    CwtTypeOrSpecial::CwtType(CwtType::Block(_)) => {
                        for value in property_value.0.iter() {
                            if let Some(value) = value.value.as_entity() {
                                let nested_results =
                                    self.collect_from_entity(value, property_type.clone());
                                for (key, values) in nested_results {
                                    entity_value_sets.entry(key).or_default().extend(values);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        entity_value_sets
    }
}
