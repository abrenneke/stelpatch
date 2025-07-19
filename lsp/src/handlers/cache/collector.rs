use std::collections::{HashMap, HashSet};

mod complex_enums;
mod scripted_effect_arguments;
mod value_sets;

use crate::handlers::cache::{
    collector::{
        complex_enums::ComplexEnumCollector,
        scripted_effect_arguments::ScriptedEffectArgumentCollector, value_sets::ValueSetCollector,
    },
    resolver::TypeResolver,
};

pub struct DataCollector<'resolver> {
    value_sets: HashMap<String, HashSet<String>>,
    complex_enums: HashMap<String, HashSet<String>>,
    scripted_effect_arguments: HashMap<String, HashSet<String>>, // Also scripted triggers for convenience... might be wrong because clashes
    type_resolver: &'resolver TypeResolver,
}

impl<'resolver> DataCollector<'resolver> {
    pub fn new(type_resolver: &'resolver TypeResolver) -> Self {
        Self {
            value_sets: HashMap::new(),
            complex_enums: HashMap::new(),
            scripted_effect_arguments: HashMap::new(),
            type_resolver,
        }
    }

    pub fn value_sets(&self) -> &HashMap<String, HashSet<String>> {
        &self.value_sets
    }

    pub fn complex_enums(&self) -> &HashMap<String, HashSet<String>> {
        &self.complex_enums
    }

    pub fn scripted_effect_arguments(&self) -> &HashMap<String, HashSet<String>> {
        &self.scripted_effect_arguments
    }

    pub fn collect_all(&mut self) {
        let value_set_collector = ValueSetCollector::new(self.type_resolver);
        self.value_sets = value_set_collector.collect();

        let complex_enum_collector = ComplexEnumCollector::new(self.type_resolver);
        self.complex_enums = complex_enum_collector.collect();

        let scripted_effect_argument_collector = ScriptedEffectArgumentCollector::new();
        self.scripted_effect_arguments = scripted_effect_argument_collector.collect();
    }
}
