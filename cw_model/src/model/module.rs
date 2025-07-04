use std::path::{Path, PathBuf};

use cw_parser::{AstModuleCell, AstVisitor};
use path_slash::PathBufExt;

use crate::{Properties, PropertyInfo, PropertyInfoList, PropertyVisitor, Value};

/// A Module is a single file inside of a Namespace. Another module in the same namespace with the same name will overwrite
/// the previous module in the game's load order. Entities in a module are unique in a namespace. An entity defined in one module
/// and defined in another module with a different name will be overwritten by the second module in the game's load order. If two
/// modules at the same point in the load order define the same entity, the entity will be overwritten by the second module's name alphabetically.
/// This is why some modules start with 00_, 01_, etc. to ensure they are loaded first and get overridden first.
#[derive(Debug, PartialEq, Clone)]
pub struct Module {
    pub filename: String,
    pub namespace: String,
    pub properties: Properties,
    pub values: Vec<Value>,
    pub ast: Option<AstModuleCell>,
}

impl Module {
    pub fn new(namespace: String, module_name: String) -> Self {
        Self {
            filename: module_name,
            namespace,
            properties: Properties::new_module(),
            values: Vec::new(),
            ast: None,
        }
    }

    pub fn from_file(file_path: &Path) -> Result<Self, anyhow::Error> {
        let (namespace, module_name) = Self::get_module_info(file_path);

        let file_content = std::fs::read_to_string(file_path)?;

        let ast = AstModuleCell::from_input(file_content);

        let mut module = Self::new(namespace, module_name);
        let mut module_visitor = ModuleVisitor::new(&mut module);

        if let Ok(ast) = ast.borrow_dependent() {
            module_visitor.visit_module(ast);
        } else {
            return Err(anyhow::anyhow!(
                "Failed to parse module at {}",
                file_path.display()
            ));
        }

        module.ast = Some(ast);

        Ok(module)
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

impl ToString for Module {
    fn to_string(&self) -> String {
        let mut buf = String::from("");
        for value in &self.values {
            let value = format!("{}\n", value.to_string());
            buf.push_str(&value);
        }
        for (key, value) in &self.properties.kv {
            let value = format!("{} = {}\n", key, value.to_string());
            buf.push_str(&value);
        }
        buf
    }
}

pub(crate) struct ModuleVisitor<'a> {
    module: &'a mut Module,
}

impl<'a> ModuleVisitor<'a> {
    pub fn new(module: &'a mut Module) -> Self {
        Self { module }
    }
}

impl<'a, 'b> cw_parser::AstVisitor<'b> for ModuleVisitor<'a> {
    type Result = ();

    fn visit_expression(&mut self, node: &cw_parser::AstExpression<'b>) -> Self::Result {
        let mut property = PropertyInfo::default();
        let mut property_visitor = PropertyVisitor::new(&mut property);
        property_visitor.visit_expression(node);
        self.module
            .properties
            .kv
            .entry(node.key.value.to_string())
            .or_insert_with(PropertyInfoList::new)
            .0
            .push(property);
    }
}
