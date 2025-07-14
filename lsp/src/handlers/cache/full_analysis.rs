use std::{
    collections::{HashMap, HashSet},
    sync::OnceLock,
};

use crate::handlers::cache::{DataCollector, GameDataCache, TypeCache};

pub struct FullAnalysis {
    game_data: &'static GameDataCache,
    type_cache: &'static TypeCache,
}

pub struct FullAnalysisResult {
    pub dynamic_value_sets: HashMap<String, HashSet<String>>,
    pub complex_enums: HashMap<String, HashSet<String>>,
    pub scripted_effect_arguments: HashMap<String, HashSet<String>>,
}

static FULL_ANALYSIS: OnceLock<FullAnalysisResult> = OnceLock::new();

impl FullAnalysis {
    pub fn new(game_data: &'static GameDataCache, type_cache: &'static TypeCache) -> Self {
        Self {
            game_data,
            type_cache,
        }
    }

    pub fn get() -> Option<&'static FullAnalysisResult> {
        FULL_ANALYSIS.get()
    }

    pub fn load(&self) {
        FULL_ANALYSIS.get_or_init(|| {
            let start = std::time::Instant::now();

            let mut collector = DataCollector::new(self.game_data, self.type_cache.get_resolver());
            collector.collect_all();

            let duration = start.elapsed();
            eprintln!("Full analysis loaded in {:?}", duration);

            FullAnalysisResult {
                dynamic_value_sets: collector.value_sets().clone(),
                complex_enums: collector.complex_enums().clone(),
                scripted_effect_arguments: collector.scripted_effect_arguments().clone(),
            }
        });
    }
}
