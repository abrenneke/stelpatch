use std::{path::Path, sync::Arc};

use cw_parser::{AstModuleCell, AstValue, AstVisitor};
use path_slash::PathExt;

use crate::{
    CaseInsensitiveInterner, Properties, PropertyInfo, PropertyInfoList, PropertyVisitor, Value,
};

/// A Module is a single file inside of a Namespace. The module name includes the file extension to prevent collisions
/// between files with the same base name but different extensions (e.g., archaeology_view.gfx vs archaeology_view.gui).
/// Another module in the same namespace with the exact same filename will overwrite the previous module in the game's load order.
/// Entities in a module are unique in a namespace. An entity defined in one module and defined in another module with a
/// different name will be overwritten by the second module in the game's load order. If two modules at the same point in
/// the load order define the same entity, the entity will be overwritten by the second module's name alphabetically.
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
        root_dir: &Path,
        interner: &CaseInsensitiveInterner,
    ) -> Result<Self, anyhow::Error> {
        let (namespace, module_name) = Self::get_module_info(file_path, root_dir);

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

    pub fn get_module_info(file_path: &Path, root_dir: &Path) -> (String, String) {
        let module_name = file_path.file_name().unwrap().to_str().unwrap();

        // Calculate the relative path from root_dir to the file
        let relative_path = match file_path.strip_prefix(root_dir) {
            Ok(rel_path) => rel_path,
            Err(_) => {
                // If stripping fails, use a fallback namespace
                return ("unknown".to_string(), module_name.to_string());
            }
        };

        // Get the parent directory of the file as the namespace, prefixed with "game"
        let namespace = if let Some(parent_dir) = relative_path.parent() {
            if parent_dir.as_os_str().is_empty() {
                // File is directly in the root directory
                "game".to_string()
            } else {
                // Prepend "game/" to the parent directory path, converting to forward slashes
                format!("game/{}", parent_dir.to_slash_lossy())
            }
        } else {
            // This shouldn't happen, but provide a fallback
            "game".to_string()
        };

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
