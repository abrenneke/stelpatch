use std::collections::HashMap;

use super::diff::EntityMergeMode;
use lasso::ThreadedRodeo;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref ENTITY_MERGE_MODES: HashMap<String, EntityMergeMode> = {
            let map: HashMap<String, EntityMergeMode> = vec![
                ("common/achievements", EntityMergeMode::Unknown),
                ("common/agendas", EntityMergeMode::LIOS),
                ("common/agreement_presets", EntityMergeMode::Unknown),
                ("common/agreement_resources", EntityMergeMode::Unknown),
                ("common/agreement_term_values", EntityMergeMode::FIOS),
                ("common/agreement_terms", EntityMergeMode::Unknown),
                ("common/ai_budget", EntityMergeMode::Unknown),
                ("common/ai_espionage/spynetworks", EntityMergeMode::Unknown),
                ("common/ai_espionage/operations", EntityMergeMode::Unknown),
                ("common/ai_espionage", EntityMergeMode::Unknown),
                ("common/ambient_objects", EntityMergeMode::Unknown),
                ("common/anomalies", EntityMergeMode::LIOS),
                ("common/archaeological_site_types", EntityMergeMode::Unknown),
                ("common/armies", EntityMergeMode::LIOS),
                ("common/artifact_actions", EntityMergeMode::LIOS),
                ("common/ascension_perks", EntityMergeMode::LIOS),
                ("common/asteroid_belts", EntityMergeMode::Unknown),
                ("common/attitudes", EntityMergeMode::LIOS),
                ("common/bombardment_stances", EntityMergeMode::LIOS),
                ("common/defines", EntityMergeMode::MergeShallow),
                ("common/on_actions", EntityMergeMode::Merge),
                // TODO the rest of them D:
            ].iter().map(|(k, v)| (k.to_string(), *v)).collect();
            map
        };
}

pub fn get_merge_mode_for_namespace(namespace: &str) -> EntityMergeMode {
    let merge_mode = ENTITY_MERGE_MODES
        .get(namespace.trim())
        .unwrap_or(&EntityMergeMode::Unknown)
        .clone();

    merge_mode
}
