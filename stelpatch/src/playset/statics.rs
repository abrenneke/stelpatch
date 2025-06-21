use std::collections::HashMap;

use cw_parser::model::EntityMergeMode;
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
                ("common/special_projects", EntityMergeMode::FIOSKeyed("key")),
                ("common/component_sets", EntityMergeMode::FIOS),
                ("common/component_templates", EntityMergeMode::FIOS),
                ("common/event_chains", EntityMergeMode::FIOS),
                ("common/global_ship_designs", EntityMergeMode::FIOS),
                ("common/observation_station_missions", EntityMergeMode::LIOS), // DUPL/LIOS Entire override ONLY ???
                ("common/opinion_modifiers", EntityMergeMode::Duplicate),
                ("common/planet_classes", EntityMergeMode::Duplicate),
                ("common/section_templates", EntityMergeMode::No),
                ("common/ship_behaviors", EntityMergeMode::FIOS),
                ("common/special_projects", EntityMergeMode::FIOS),
                ("common/start_screen_messages", EntityMergeMode::FIOS),
                ("common/static_modifiers", EntityMergeMode::FIOS),
                ("common/strategic_resources", EntityMergeMode::FIOS),
                ("common/terraform", EntityMergeMode::Duplicate),
                ("common/traits", EntityMergeMode::No)
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
