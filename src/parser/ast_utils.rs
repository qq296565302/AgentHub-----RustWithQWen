use tree_sitter::Node;

pub fn get_node_text<'a>(node: Node<'a>, source: &'a str) -> &'a str {
    node.utf8_text(source.as_bytes()).unwrap_or("")
}

pub fn get_node_range_text<'a>(node: Node<'a>, source: &'a str) -> &'a str {
    let range = node.range();
    &source[range.start_byte..range.end_byte]
}

pub fn find_child_by_type<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    node.children(&mut node.walk())
        .find(|child| child.kind() == kind)
}

pub fn find_children_by_type<'a>(node: Node<'a>, kind: &str) -> Vec<Node<'a>> {
    node.children(&mut node.walk())
        .filter(|child| child.kind() == kind)
        .collect()
}

pub fn get_function_name(node: tree_sitter::Node, source: &str) -> Option<String> {
    find_child_by_type(node, "identifier")
        .map(|n| get_node_text(n, source).to_string())
}
