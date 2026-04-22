pub mod language_registry;
pub mod ast_utils;
pub mod rust_parser;
pub mod python_parser;
pub mod js_parser;

use crate::error::Result;

pub trait CodeParser: Send + Sync {
    fn parse(&self, source: &str) -> Result<tree_sitter::Tree>;

    fn language(&self) -> &str;

    fn get_function_nodes(&self, tree: &tree_sitter::Tree, source: &str) -> Result<Vec<FunctionInfo>>;

    fn get_class_nodes(&self, tree: &tree_sitter::Tree, source: &str) -> Result<Vec<ClassInfo>>;
}

#[derive(Debug, Clone)]
pub struct FunctionInfo {
    pub name: String,
    pub start_line: usize,
    pub end_line: usize,
    pub body: String,
}

#[derive(Debug, Clone)]
pub struct ClassInfo {
    pub name: String,
    pub start_line: usize,
    pub end_line: usize,
    pub methods: Vec<FunctionInfo>,
}
