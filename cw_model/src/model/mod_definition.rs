use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

use cw_parser::mod_definition::{AstExpression, AstModDefinitionCell, ModDefinitionAstVisitor};
use walkdir::WalkDir;

// Define the ModDefinition struct to hold the parsed values
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ModDefinition {
    pub ast: Option<AstModDefinitionCell>,
    pub version: Option<String>,
    pub tags: Vec<String>,
    pub name: String,
    pub picture: Option<String>,
    pub supported_version: Option<String>,
    pub path: Option<PathBuf>,
    pub remote_file_id: Option<String>,
    pub dependencies: Vec<String>,
    pub archive: Option<String>,
    pub definition_dir: Option<PathBuf>,
}

/// A set of mod definitions (likely loaded from Documents)
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ModDefinitionList {
    /// The mods that were parsed and in the definition list
    pub mods: Vec<ModDefinition>,

    /// A list of files that failed to parse
    pub failed_parse_files: Vec<String>,
}

impl ModDefinitionList {
    pub fn new() -> Self {
        ModDefinitionList {
            mods: Vec::new(),
            failed_parse_files: Vec::new(),
        }
    }

    /// Loads all mod definitions from the Documents folder
    pub fn load_from_my_documents(dir_path: &Path) -> Result<Self, anyhow::Error> {
        let mut mod_dir = dir_path.to_path_buf();
        mod_dir.push("mod");

        let dot_mod_files = WalkDir::new(mod_dir)
            .max_depth(1)
            .sort_by_file_name()
            .into_iter()
            .filter_map(|e| {
                if let Ok(e) = e {
                    if e.file_type().is_file() && e.path().extension().unwrap_or_default() == "mod"
                    {
                        return Some(e);
                    }
                }
                None
            });

        let mut definitions = ModDefinitionList::new();

        for item in dot_mod_files {
            let mod_definition = ModDefinition::load_from_file(&item.path());
            definitions.push(mod_definition);
        }

        Ok(definitions)
    }

    pub fn push(&mut self, definition: Result<ModDefinition, anyhow::Error>) -> &Self {
        match definition {
            Ok(definition) => self.mods.push(definition),
            Err(e) => self.failed_parse_files.push(e.to_string()),
        }
        self
    }

    pub fn get_by_name(&self, name: &str) -> Option<&ModDefinition> {
        for mod_definition in &self.mods {
            if mod_definition.name == name {
                return Some(mod_definition);
            }
        }

        None
    }

    pub fn get_by_id(&self, id: &str) -> Option<&ModDefinition> {
        for mod_definition in &self.mods {
            if mod_definition.remote_file_id.as_deref() == Some(id) {
                return Some(mod_definition);
            }
        }

        None
    }

    pub fn search(&self, search: &str) -> Vec<&ModDefinition> {
        let mut results = Vec::new();

        for mod_definition in &self.mods {
            if mod_definition.name == search {
                results.push(mod_definition);
            }
        }

        results
    }

    pub fn search_first(&self, search: &str) -> Result<&ModDefinition, String> {
        for mod_definition in &self.mods {
            let pattern = search.to_lowercase();
            if mod_definition.name.to_lowercase().contains(&pattern) {
                return Ok(mod_definition);
            }
        }

        Err(format!("Could not find mod with name {}", search))
    }
}

impl ModDefinition {
    pub fn new() -> Self {
        ModDefinition {
            ast: None,
            version: None,
            tags: Vec::new(),
            name: String::new(),
            picture: None,
            supported_version: None,
            path: None,
            remote_file_id: None,
            dependencies: Vec::new(),
            archive: None,
            definition_dir: None,
        }
    }

    pub fn load(input: &str, definition_path: Option<&Path>) -> Result<Self, anyhow::Error> {
        let ast = AstModDefinitionCell::from_input(input.to_string());
        let mut mod_definition = ModDefinition::new();
        mod_definition.definition_dir = definition_path.map(|p| p.to_path_buf());
        let mut visitor = ModDefinitionLoaderVisitor {
            mod_definition: &mut mod_definition,
        };

        match ast.borrow_dependent() {
            Ok(ast) => visitor.visit_mod_definition(ast),
            Err(e) => return Err(anyhow::anyhow!(e.to_string())),
        }

        Ok(mod_definition)
    }

    pub fn load_from_file(path: &Path) -> Result<Self, anyhow::Error> {
        let contents = std::fs::read_to_string(path).map_err(|e| anyhow::anyhow!(e.to_string()))?;
        Self::load(&contents, Some(path.parent().unwrap()))
    }

    pub fn populate_from_ast(&mut self, ast: AstModDefinitionCell) -> &Self {
        self.ast = Some(ast);
        self
    }
}

impl Default for ModDefinition {
    fn default() -> Self {
        Self::new()
    }
}

impl FromStr for ModDefinition {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::load(s, None)
    }
}

struct ModDefinitionLoaderVisitor<'a> {
    mod_definition: &'a mut ModDefinition,
}

impl<'a, 'def> ModDefinitionAstVisitor<'a> for ModDefinitionLoaderVisitor<'def> {
    fn walk_expression(&mut self, node: &AstExpression<'a>) -> () {
        match node.key.value {
            "version" => {
                let version = node.value.as_string();
                if let Some(version) = version {
                    self.mod_definition.version = Some(version.to_string());
                }
            }
            "tags" => {
                let tags = node.value.as_array();
                if let Some(tags) = tags {
                    self.mod_definition.tags =
                        tags.iter().map(|s| s.raw_value().to_string()).collect();
                }
            }
            "name" => {
                let name = node.value.as_string();
                if let Some(name) = name {
                    self.mod_definition.name = name.to_string();
                }
            }
            "picture" => {
                let picture = node.value.as_string();
                if let Some(picture) = picture {
                    self.mod_definition.picture = Some(picture.to_string());
                }
            }
            "supported_version" => {
                let supported_version = node.value.as_string();
                if let Some(supported_version) = supported_version {
                    self.mod_definition.supported_version = Some(supported_version.to_string());
                }
            }
            "path" => {
                let path = node.value.as_string();
                if let Some(path) = path {
                    self.mod_definition.path = Some(PathBuf::from(path));
                }
            }
            "remote_file_id" => {
                let remote_file_id = node.value.as_string();
                if let Some(remote_file_id) = remote_file_id {
                    self.mod_definition.remote_file_id = Some(remote_file_id.to_string());
                }
            }
            "dependencies" => {
                let dependencies = node.value.as_array();
                if let Some(dependencies) = dependencies {
                    self.mod_definition.dependencies = dependencies
                        .iter()
                        .map(|s| s.raw_value().to_string())
                        .collect();
                }
            }
            "archive" => {
                let archive = node.value.as_string();
                if let Some(archive) = archive {
                    self.mod_definition.archive = Some(archive.to_string());
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_mod_definition() {
        let input = r#"version="2.8"
        tags={
            "Technologies"
            "Economy"
            "Buildings"
        }
        name="EUTAB - Ethos Unique Techs and Buildings"
        picture="eutab.png"
        supported_version="3.0.*"
        path="D:/SteamLibrary/steamapps/workshop/content/281990/804732593"
        remote_file_id="804732593""#;
        let expected_output = ModDefinition {
            ast: None,
            version: Some(String::from("2.8")),
            tags: vec![
                String::from("Technologies"),
                String::from("Economy"),
                String::from("Buildings"),
            ],
            name: String::from("EUTAB - Ethos Unique Techs and Buildings"),
            picture: Some(String::from("eutab.png")),
            supported_version: Some(String::from("3.0.*")),
            path: Some(PathBuf::from(
                "D:/SteamLibrary/steamapps/workshop/content/281990/804732593",
            )),
            remote_file_id: Some(String::from("804732593")),
            archive: None,
            dependencies: Vec::new(),
            definition_dir: None,
        };

        let mut parsed = ModDefinition::load(input, None).unwrap();
        parsed.ast = None; // Ignore for comparison, ast testing done elsewhere

        assert_eq!(parsed, expected_output);
    }
}
