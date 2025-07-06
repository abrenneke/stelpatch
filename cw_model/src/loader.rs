use std::{fs::File, io::Read, path::Path};

use anyhow::{Context, Result, anyhow};
use serde::Deserialize;

use crate::ModDefinition;

#[derive(Debug, Deserialize)]
pub struct EnabledMod {
    pub path: String,
}

#[derive(Debug, Deserialize)]
struct EnabledModsWrapper {
    enabled_mods: Vec<String>,
}

impl EnabledMod {
    pub fn load_definition(&self, dir_path: &Path) -> Result<ModDefinition> {
        let mut mod_definition_path = dir_path.to_path_buf();
        mod_definition_path.push(&self.path);

        let mut mod_definition_file = File::open(&mod_definition_path).with_context(|| {
            format!(
                "Failed to open mod definition file at {}",
                mod_definition_path.display()
            )
        })?;

        let mut contents = String::new();
        mod_definition_file
            .read_to_string(&mut contents)
            .with_context(|| {
                format!(
                    "Failed to read mod definition file at {}",
                    mod_definition_path.display()
                )
            })?;

        let mod_definition = contents.parse::<ModDefinition>().with_context(|| {
            format!(
                "Failed to parse mod definition file at {}",
                mod_definition_path.display()
            )
        })?;

        Ok(mod_definition)
    }
}

pub fn load_playset(dir_path: &Path) -> Result<Vec<EnabledMod>> {
    let mut json_file_path = dir_path.to_path_buf();
    json_file_path.push("dlc_load.json");

    let mut json_file = File::open(&json_file_path)
        .with_context(|| format!("Failed to open JSON file at {}", json_file_path.display()))?;

    let mut contents = String::new();
    json_file
        .read_to_string(&mut contents)
        .with_context(|| format!("Failed to read JSON file at {}", json_file_path.display()))?;

    let wrapper: EnabledModsWrapper = serde_json::from_str(&contents).map_err(|e| {
        anyhow!(
            "Failed to parse JSON file at {}: {}",
            json_file_path.display(),
            e
        )
    })?;

    let enabled_mods = wrapper
        .enabled_mods
        .into_iter()
        .map(|path| EnabledMod { path })
        .collect();

    Ok(enabled_mods)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    /// Gets /Users/Username/Documents/Paradox Interactive/Stellaris
    pub fn stellaris_documents_dir() -> Result<PathBuf, anyhow::Error> {
        let home_dir =
            dirs::document_dir().ok_or_else(|| anyhow!("Could not find Documents directory"))?;
        let path = vec![
            home_dir.to_str().ok_or_else(|| {
                anyhow!(
                    "Could not convert Documents directory to string: {}",
                    home_dir.display()
                )
            })?,
            "Paradox Interactive",
            "Stellaris",
        ]
        .iter()
        .collect::<PathBuf>();
        Ok(path.into())
    }

    #[test]
    fn test_load_playset() {
        let enabled_mods = load_playset(&stellaris_documents_dir().unwrap()).unwrap();
        assert!(enabled_mods.len() > 0);

        for enabled_mod in enabled_mods {
            assert!(enabled_mod.path.contains("/"));
            assert!(enabled_mod.path.ends_with(".mod"));
        }
    }

    #[test]
    fn test_load_all_mods() {
        let enabled_mods = load_playset(&stellaris_documents_dir().unwrap()).unwrap();
        for enabled_mod in enabled_mods {
            let mod_definition = enabled_mod
                .load_definition(&stellaris_documents_dir().unwrap())
                .unwrap();
            assert!(mod_definition.name.len() > 0);

            if let Some(version) = mod_definition.version {
                assert!(version.len() > 0);
            }

            if let Some(supported_version) = mod_definition.supported_version {
                assert!(supported_version.len() > 0);
            }

            if let Some(path) = mod_definition.path {
                assert!(path.exists());
            }

            if let Some(remote_file_id) = mod_definition.remote_file_id {
                assert!(remote_file_id.len() > 0);
            }
        }
    }
}
