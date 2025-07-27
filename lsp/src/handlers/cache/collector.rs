use std::collections::HashSet;

mod complex_enums;
mod scripted_effect_arguments;
mod value_sets;

use cw_model::SpurMap;
use lasso::Spur;

use crate::handlers::cache::{
    collector::{
        complex_enums::ComplexEnumCollector,
        scripted_effect_arguments::ScriptedEffectArgumentCollector, value_sets::ValueSetCollector,
    },
    resolver::TypeResolver,
};

pub struct DataCollector<'resolver> {
    value_sets: SpurMap<HashSet<Spur>>,
    complex_enums: SpurMap<HashSet<Spur>>,
    scripted_effect_arguments: SpurMap<HashSet<Spur>>, // Also scripted triggers for convenience... might be wrong because clashes
    type_resolver: &'resolver TypeResolver,
}

impl<'resolver> DataCollector<'resolver> {
    pub fn new(type_resolver: &'resolver TypeResolver) -> Self {
        Self {
            value_sets: SpurMap::new(),
            complex_enums: SpurMap::new(),
            scripted_effect_arguments: SpurMap::new(),
            type_resolver,
        }
    }

    pub fn value_sets(&self) -> &SpurMap<HashSet<Spur>> {
        &self.value_sets
    }

    pub fn complex_enums(&self) -> &SpurMap<HashSet<Spur>> {
        &self.complex_enums
    }

    pub fn scripted_effect_arguments(&self) -> &SpurMap<HashSet<Spur>> {
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
