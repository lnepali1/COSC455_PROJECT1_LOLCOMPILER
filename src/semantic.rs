//! Static scope checking (Task 3) and HTML generation (Task 4).
//!
//! The tree is walked once. A stack of scopes keeps the variables:
//! the bottom scope is the global one (variables defined right after the
//! head), and every paragraph or list item pushes its own scope. A
//! variable use is looked up from the innermost scope out to the global
//! one, so an inner definition hides an outer one with the same name.
//! If no scope has the name, that is a static semantic error.

use crate::ast::Node;

/// One scope: a list of (name, value) pairs.
type Scope = Vec<(String, String)>;

/// Walks the tree, keeps the scopes, and builds the HTML text.
struct Generator {
    scopes: Vec<Scope>,
}

impl Generator {
    /// Find a variable, starting from the innermost scope.
    fn lookup(&self, name: &str) -> Option<String> {
        for scope in self.scopes.iter().rev() {
            for (n, v) in scope.iter().rev() {
                if n == name {
                    return Some(v.clone());
                }
            }
        }
        None
    }

    /// Add a variable to the innermost scope.
    fn define(&mut self, name: &str, value: &str) {
        let top = self.scopes.len() - 1;
        self.scopes[top].push((name.to_string(), value.to_string()));
    }

    /// Turn a list of nodes into HTML pieces. Pieces are joined with a
    /// space so text and tags do not run together.
    fn gen_nodes(&mut self, nodes: &[Node]) -> Result<String, String> {
        let mut parts: Vec<String> = Vec::new();
        for node in nodes {
            let piece = self.gen_node(node)?;
            if !piece.is_empty() {
                parts.push(piece);
            }
        }
        Ok(parts.join(" "))
    }

    /// Turn one node into HTML.
    fn gen_node(&mut self, node: &Node) -> Result<String, String> {
        match node {
            Node::Comment(text) => Ok(format!("<!-- {} -->", text)),
            // the head is written separately in generate_html
            Node::Head(_) => Ok(String::new()),
            Node::Text(text) => Ok(text.clone()),
            Node::Bold(text) => Ok(format!("<b>{}</b>", text)),
            Node::Link(addr) => Ok(format!("<a href=\"{}\">{}</a>", addr, addr)),
            Node::Newline => Ok("<br>".to_string()),
            Node::VarDefine { name, value, .. } => {
                self.define(name, value);
                Ok(String::new())
            }
            Node::VarUse { name, line } => match self.lookup(name) {
                Some(value) => Ok(value),
                None => Err(format!(
                    "Static semantic error at line {}: variable '{}' is used but has not been defined in this scope.",
                    line, name
                )),
            },
            Node::Paragraph(children) => {
                let inner = self.in_new_scope(children)?;
                Ok(format!("<p>{}</p>", inner))
            }
            Node::List(items) => {
                let inner = self.gen_nodes(items)?;
                Ok(format!("<ul>{}</ul>", inner))
            }
            Node::ListItem(children) => {
                let inner = self.in_new_scope(children)?;
                Ok(format!("<li>{}</li>", inner))
            }
        }
    }

    /// Generate the children inside a fresh scope, then drop the scope.
    fn in_new_scope(&mut self, children: &[Node]) -> Result<String, String> {
        self.scopes.push(Vec::new());
        let result = self.gen_nodes(children);
        self.scopes.pop();
        result
    }
}

/// Check the variables in the tree and build the HTML page.
/// Returns an error message for the first undefined variable.
pub fn generate_html(ast: &[Node]) -> Result<String, String> {
    let mut generator = Generator { scopes: vec![Vec::new()] };

    // the title comes from the Head node, if there is one
    let mut head = String::new();
    for node in ast {
        if let Node::Head(title) = node {
            head = format!("<head>\n<title>{}</title>\n</head>\n", title);
        }
    }

    // every other top-level node goes on its own line in the body
    let mut body = String::new();
    for node in ast {
        let piece = generator.gen_node(node)?;
        if !piece.is_empty() {
            body.push_str(&piece);
            body.push('\n');
        }
    }

    Ok(format!("<html>\n{}<body>\n{}</body>\n</html>\n", head, body))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inner_scope_hides_outer() {
        let ast = vec![
            Node::VarDefine { name: "a".into(), value: "outer".into(), line: 1 },
            Node::Paragraph(vec![
                Node::VarDefine { name: "a".into(), value: "inner".into(), line: 2 },
                Node::VarUse { name: "a".into(), line: 2 },
            ]),
            Node::Paragraph(vec![Node::VarUse { name: "a".into(), line: 3 }]),
        ];
        let html = generate_html(&ast).unwrap();
        assert!(html.contains("<p>inner</p>"));
        assert!(html.contains("<p>outer</p>"));
    }

    #[test]
    fn undefined_variable_is_error() {
        let ast = vec![Node::Paragraph(vec![Node::VarUse { name: "x".into(), line: 4 }])];
        assert!(generate_html(&ast).is_err());
    }

    #[test]
    fn variable_does_not_leak_out_of_paragraph() {
        let ast = vec![
            Node::Paragraph(vec![Node::VarDefine { name: "x".into(), value: "1".into(), line: 1 }]),
            Node::Paragraph(vec![Node::VarUse { name: "x".into(), line: 2 }]),
        ];
        assert!(generate_html(&ast).is_err());
    }
}
