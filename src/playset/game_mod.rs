use std::path::PathBuf;

use crate::cw_model::Module;

use super::mod_definition::ModDefinition;
use anyhow::Result;
use tokio::task;
use walkdir::WalkDir;

pub struct GameMod {
    pub definition: ModDefinition,
    pub modules: Vec<Module>,
}

impl GameMod {
    pub async fn load(definition: ModDefinition) -> Result<Self> {
        let mut modules = Vec::new();
        let mut tasks = Vec::new();

        let dir = PathBuf::from(definition.path.as_ref().unwrap());

        for entry in WalkDir::new(&dir).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() && entry.path().extension().unwrap_or_default() == "txt"
            {
                let path = entry.path().to_string_lossy().to_string();
                let task = task::spawn(load_module(path));
                tasks.push(task);
            }
        }

        for task in tasks {
            let module = task.await??;
            modules.push(module);
        }

        Ok(Self {
            definition,
            modules,
        })
    }
}

async fn load_module(path: String) -> Result<Module> {
    todo!()
    // let module = Module::parse_from_file(&path).await?;
    // Ok(module)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_game_mod_load() {
        // Create a ModDefinition that matches the expected_output
        let mod_definition = ModDefinition {
            version: Some(String::from("2.8")),
            tags: vec![
                String::from("Technologies"),
                String::from("Economy"),
                String::from("Buildings"),
            ],
            name: String::from("EUTAB - Ethos Unique Techs and Buildings"),
            picture: Some(String::from("eutab.png")),
            supported_version: Some(String::from("3.0.*")),
            path: Some(String::from(
                "D:/SteamLibrary/steamapps/workshop/content/281990/804732593",
            )),
            remote_file_id: Some(String::from("804732593")),
            archive: None,
            dependencies: Vec::new(),
        };

        let game_mod = GameMod::load(mod_definition).await.unwrap();
        assert!(game_mod.modules.len() > 0);

        assert!(game_mod.modules[0].type_path.len() > 0);
        // dbg!(game_mod.modules);
    }
}
