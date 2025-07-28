use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use cw_parser::{AstModuleCell, AstValue, AstVisitor};
use path_slash::PathBufExt;

use crate::{
    CaseInsensitiveInterner, Properties, PropertyInfo, PropertyInfoList, PropertyVisitor, Value,
};

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
    pub values: Vec<Arc<Value>>,
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

    pub fn from_file(
        file_path: &Path,
        interner: &CaseInsensitiveInterner,
    ) -> Result<Self, anyhow::Error> {
        let (namespace, module_name) = Self::get_module_info(file_path);

        if module_name.starts_with("99_README") {
            return Ok(Self::new(namespace, module_name));
        }

        let file_content = std::fs::read_to_string(file_path)?;

        let ast = AstModuleCell::from_input(file_content);

        let mut module = Self::new(namespace, module_name);
        let mut module_visitor = ModuleVisitor::new(&mut module, interner);

        match ast.borrow_dependent() {
            Ok(ast) => module_visitor.visit_module(ast),
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "Failed to parse module at {}: {}",
                    file_path.display(),
                    e
                ));
            }
        }

        module.ast = Some(ast);

        Ok(module)
    }

    pub fn get_module_info(file_path: &Path) -> (String, String) {
        let path = PathBuf::from(file_path);

        // Define the base directories we support
        let base_dirs = vec!["common", "interface", "events", "gfx", "localisation"];

        let mut namespace = String::new();
        let mut found_base = false;

        // Find the rightmost occurrence of ANY base directory
        let components: Vec<_> = path.components().collect();
        let mut rightmost_base_index = None;
        let mut rightmost_base_dir = None;

        for base_dir in base_dirs {
            if let Some(base_index) = components.iter().rposition(|c| c.as_os_str() == base_dir) {
                if rightmost_base_index.is_none() || base_index > rightmost_base_index.unwrap() {
                    rightmost_base_index = Some(base_index);
                    rightmost_base_dir = Some(base_dir);
                }
            }
        }

        if let (Some(base_index), Some(base_dir)) = (rightmost_base_index, rightmost_base_dir) {
            if let Some(base_prefix) = components
                .iter()
                .take(base_index + 1)
                .collect::<PathBuf>()
                .to_str()
            {
                // Get the subdirectory path after the base directory
                let remaining_path = path.strip_prefix(base_prefix).unwrap();

                if let Some(parent_dir) = remaining_path.parent() {
                    if parent_dir.as_os_str().is_empty() {
                        // File is directly in the base directory
                        namespace = base_dir.to_string();
                    } else {
                        // File is in a subdirectory, include the subdirectory in namespace
                        namespace = [base_dir, &parent_dir.to_string_lossy()]
                            .iter()
                            .collect::<PathBuf>()
                            .to_slash_lossy()
                            .to_string();
                    }
                } else {
                    namespace = base_dir.to_string();
                }
                found_base = true;
            }
        }

        // Fallback if no base directory found (shouldn't happen with proper glob patterns)
        if !found_base {
            namespace = "unknown".to_string();
        }

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
            let value = format!("{:?} = {}\n", key, value.to_string());
            buf.push_str(&value);
        }
        buf
    }
}

pub(crate) struct ModuleVisitor<'a, 'interner> {
    module: &'a mut Module,
    interner: &'interner CaseInsensitiveInterner,
}

impl<'a, 'interner> ModuleVisitor<'a, 'interner> {
    pub fn new(module: &'a mut Module, interner: &'interner CaseInsensitiveInterner) -> Self {
        Self { module, interner }
    }
}

impl<'a, 'b, 'ast, 'interner> cw_parser::AstVisitor<'b, 'ast> for ModuleVisitor<'a, 'interner>
where
    'b: 'ast,
{
    fn visit_expression(&mut self, node: &cw_parser::AstExpression<'b>) -> () {
        let mut property = PropertyInfo::default();
        let mut property_visitor = PropertyVisitor::new(&mut property, self.interner);
        property_visitor.visit_expression(node);
        let key = self.interner.get_or_intern(node.key.raw_value());

        let list = self
            .module
            .properties
            .kv
            .entry(key)
            .or_insert_with(|| Arc::new(PropertyInfoList::new()));
        let list = Arc::make_mut(list);
        list.push(property);
    }

    fn visit_value(&mut self, node: &cw_parser::AstValue<'b>) -> () {
        match node {
            AstValue::String(string) => {
                self.module.values.push(Arc::new(Value::String(
                    self.interner.get_or_intern(string.raw_value()),
                )));
            }
            _ => {}
        }
    }
}
