use std::collections::{HashMap, HashSet};

use cw_model::{CwtType, Entity, ReferenceType};

use crate::handlers::{
    cache::{game_data::GameDataCache, get_namespace_entity_type, resolver::TypeResolver},
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
        for (namespace, namespace_data) in self.game_data.get_namespaces().iter() {
            if let Some(namespace_type) = get_namespace_entity_type(namespace) {
                if let Some(scoped_type) = namespace_type.scoped_type {
                    for (_entity_name, entity) in namespace_data.entities.iter() {
                        self.visit_entity(entity, &scoped_type);
                    }
                }
            }
        }
    }

    fn visit_entity(&mut self, entity: &Entity, scoped_type: &ScopedType) {
        for (property_name, property_value) in entity.properties.kv.iter() {
            let property_type = self
                .type_resolver
                .navigate_to_property(scoped_type, property_name);

            if let PropertyNavigationResult::Success(property_type) = property_type {
                match property_type.cwt_type() {
                    CwtTypeOrSpecial::CwtType(CwtType::Reference(ReferenceType::ValueSet {
                        key,
                    })) => {
                        for value in property_value.0.iter() {
                            if let Some(value) = value.value.as_string() {
                                self.value_sets
                                    .entry(key.clone())
                                    .or_default()
                                    .insert(value.clone());
                            }
                        }
                    }
                    CwtTypeOrSpecial::CwtType(CwtType::Block(_)) => {
                        for value in property_value.0.iter() {
                            if let Some(value) = value.value.as_entity() {
                                self.visit_entity(value, &property_type);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
