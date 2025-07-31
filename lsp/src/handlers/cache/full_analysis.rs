use std::{collections::HashSet, sync::RwLock};

use cw_model::SpurMap;
use lasso::Spur;

use crate::handlers::cache::{DataCollector, TypeCache};

pub struct FullAnalysis {
    type_cache: &'static TypeCache,
}

#[derive(Clone)]
pub struct FullAnalysisResult {
    pub dynamic_value_sets: SpurMap<HashSet<Spur>>,
    pub complex_enums: SpurMap<HashSet<Spur>>,
    pub scripted_effect_arguments: SpurMap<HashSet<Spur>>,
}

static FULL_ANALYSIS: RwLock<Option<FullAnalysisResult>> = RwLock::new(None);

impl FullAnalysis {
    pub fn new(type_cache: &'static TypeCache) -> Self {
        Self { type_cache }
    }

    pub fn get() -> Option<FullAnalysisResult> {
        FULL_ANALYSIS.read().unwrap().clone()
    }

    /// Check if the full analysis has been initialized
    pub fn is_initialized() -> bool {
        FULL_ANALYSIS.read().unwrap().is_some()
    }

    /// Reset the full analysis cache, forcing re-initialization on next access
    pub fn reset() {
        eprintln!("Resetting FullAnalysis cache");
        let mut cache = FULL_ANALYSIS.write().unwrap();
        *cache = None;
    }

    pub fn load_global_blocking() {
        let full_analysis = FullAnalysis::new(TypeCache::get().unwrap());
        full_analysis.load();
    }

    pub fn load(&self) {
        // Check if already initialized
        if Self::is_initialized() {
            return;
        }

        // Compute the result without holding the lock
        let start = std::time::Instant::now();

        let mut collector = DataCollector::new(self.type_cache.get_resolver());
        collector.collect_all();

        let duration = start.elapsed();
        eprintln!("Full analysis loaded in {:?}", duration);

        let result = FullAnalysisResult {
            dynamic_value_sets: collector.value_sets().clone(),
            complex_enums: collector.complex_enums().clone(),
            scripted_effect_arguments: collector.scripted_effect_arguments().clone(),
        };

        // Now acquire the lock only to store the result
        let mut cache = FULL_ANALYSIS.write().unwrap();

        // Double-check after acquiring write lock
        if cache.is_none() {
            *cache = Some(result);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reset_functionality() {
        // Test that reset clears the cache and allows reinitialization

        // First, simulate that the cache is already initialized
        {
            let mut cache = FULL_ANALYSIS.write().unwrap();
            *cache = Some(FullAnalysisResult {
                dynamic_value_sets: SpurMap::new(),
                complex_enums: SpurMap::new(),
                scripted_effect_arguments: SpurMap::new(),
            });
        }

        // Verify it's initialized
        assert!(FullAnalysis::is_initialized());

        // Reset the cache
        FullAnalysis::reset();

        // Verify it's no longer initialized
        assert!(!FullAnalysis::is_initialized());

        // Verify get() returns None after reset
        assert!(FullAnalysis::get().is_none());
    }

    #[test]
    fn test_reset_method_exists() {
        // Simple test to verify that the reset method exists and can be called
        // This ensures the trigger functionality is available
        FullAnalysis::reset();

        // After reset, should not be initialized
        assert!(!FullAnalysis::is_initialized());
    }
}
