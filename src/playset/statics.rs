use std::collections::HashMap;

use super::diff::EntityMergeMode;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref ENTITY_MERGE_MODES: HashMap<String, EntityMergeMode> = {
        let map: HashMap<String, EntityMergeMode> = vec![
            ("achievements", EntityMergeMode::Unknown),
            ("agendas", EntityMergeMode::LIOS),
            ("agreement_presets", EntityMergeMode::Unknown),
            ("agreement_resources", EntityMergeMode::Unknown),
            ("agreement_term_values", EntityMergeMode::FIOS),
            ("agreement_terms", EntityMergeMode::Unknown),
            ("ai_budget", EntityMergeMode::Unknown),
            ("ai_espionage/spynetworks", EntityMergeMode::Unknown),
            ("ai_espionage/operations", EntityMergeMode::Unknown),
            ("ai_espionage", EntityMergeMode::Unknown),
            ("ambient_objects", EntityMergeMode::Unknown),
            ("anomalies", EntityMergeMode::LIOS),
            ("archaeological_site_types", EntityMergeMode::Unknown),
            ("armies", EntityMergeMode::LIOS),
            ("artifact_actions", EntityMergeMode::LIOS),
            ("ascension_perks", EntityMergeMode::LIOS),
            ("asteroid_belts", EntityMergeMode::Unknown),
            ("attitudes", EntityMergeMode::LIOS),
            ("bombardment_stances", EntityMergeMode::LIOS),
            // TODO the rest of them D:
        ].iter().map(|(k, v)| (k.to_string(), *v)).collect();
        map
    };
}

pub fn get_merge_mode_for_namespace(namespace: &str) -> EntityMergeMode {
    ENTITY_MERGE_MODES
        .get(namespace)
        .unwrap_or(&EntityMergeMode::Unknown)
        .clone()
}
