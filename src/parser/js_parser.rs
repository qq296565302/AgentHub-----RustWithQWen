use crate::error::Result;
use crate::parser::{ClassInfo, CodeParser, FunctionInfo};

#[cfg(feature = "core-languages")]
use tree_sitter_javascript::language;

pub struct JavaScriptParser {
    #[cfg(feature = "core-languages")]
    language: tree_sitter::Language,
}

impl JavaScriptParser {
    pub fn new() -> Self {
        #[cfg(feature = "core-languages")]
        {
            Self {
                language: language(),
            }
        }
        #[cfg(not(feature = "core-languages"))]
        {
            Self {}
        }
    }
}

impl CodeParser for JavaScriptParser {
    fn parse(&self, source: &str) -> Result<tree_sitter::Tree> {
        #[cfg(feature = "core-languages")]
        {
            let mut parser = tree_sitter::Parser::new();
            parser.set_language(&self.language)?;
            parser
                .parse(source, None)
                .ok_or_else(|| crate::error::AgentHubError::ParseError {
                    message: "Failed to parse JavaScript source".to_string(),
                })
        }
        #[cfg(not(feature = "core-languages"))]
        {
            Err(crate::error::AgentHubError::UnsupportedLanguage("javascript".to_string()))
        }
    }

    fn language(&self) -> &str {
        "javascript"
    }

    fn get_function_nodes(&self, tree: &tree_sitter::Tree, source: &str) -> Result<Vec<FunctionInfo>> {
        let mut functions = Vec::new();
        let root = tree.root_node();
        let mut cursor = root.walk();

        for child in root.children(&mut cursor) {
            if child.kind() == "function_declaration" || child.kind() == "arrow_function" {
                let name_node = child
                    .children(&mut child.walk())
                    .find(|n| n.kind() == "identifier");
                if let Some(name_node) = name_node {
                    let name = name_node
                        .utf8_text(source.as_bytes())
                        .unwrap_or("anonymous")
                        .to_string();
                    let start_line = child.start_position().row + 1;
                    let end_line = child.end_position().row + 1;
                    let body = child
                        .utf8_text(source.as_bytes())
                        .unwrap_or("")
                        .to_string();
                    functions.push(FunctionInfo {
                        name,
                        start_line,
                        end_line,
                        body,
                    });
                }
            }
        }

        Ok(functions)
    }

    fn get_class_nodes(&self, tree: &tree_sitter::Tree, source: &str) -> Result<Vec<ClassInfo>> {
        let mut classes = Vec::new();
        let root = tree.root_node();
        let mut cursor = root.walk();

        for child in root.children(&mut cursor) {
            if child.kind() == "class_declaration" {
                let name_node = child
                    .children(&mut child.walk())
                    .find(|n| n.kind() == "identifier");
                if let Some(name_node) = name_node {
                    let name = name_node
                        .utf8_text(source.as_bytes())
                        .unwrap_or("unknown")
                        .to_string();
                    let start_line = child.start_position().row + 1;
                    let end_line = child.end_position().row + 1;
                    classes.push(ClassInfo {
                        name,
                        start_line,
                        end_line,
                        methods: Vec::new(),
                    });
                }
            }
        }

        Ok(classes)
    }
}
