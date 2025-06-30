use std::path::{Path, PathBuf};

use cw_parser::AstModuleCell;
use path_slash::PathBufExt;

pub struct Module {
    pub namespace: String,
    pub module_name: String,
    pub ast: AstModuleCell,
}

impl Module {
    pub fn from_file(file_path: &Path) -> Result<Self, anyhow::Error> {
        let (namespace, module_name) = Self::get_module_info(file_path);

        let file_content = std::fs::read_to_string(file_path)?;

        let ast = AstModuleCell::from_input(file_content);

        Ok(Self {
            namespace,
            module_name,
            ast,
        })
    }

    pub fn get_module_info(file_path: &Path) -> (String, String) {
        let path = PathBuf::from(file_path);
        let mut namespace = String::new();
        let mut cur_path = path.clone();

        while let Some(common_index) = cur_path
            .components()
            .position(|c| c.as_os_str() == "common")
        {
            if let Some(common_prefix) = cur_path
                .components()
                .take(common_index + 1)
                .collect::<PathBuf>()
                .to_str()
            {
                namespace = cur_path
                    .strip_prefix(common_prefix)
                    .unwrap()
                    .parent()
                    .unwrap()
                    .to_string_lossy()
                    .to_string();
                cur_path = cur_path.strip_prefix(common_prefix).unwrap().to_path_buf();
            }
        }

        namespace = ["common", &namespace]
            .iter()
            .collect::<PathBuf>()
            .to_slash_lossy()
            .to_string();

        let module_name = path.file_stem().unwrap().to_str().unwrap();

        (namespace, module_name.to_string())
    }
}
