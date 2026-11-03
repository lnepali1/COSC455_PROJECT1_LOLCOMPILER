//! Static scope checking (Task 3) and HTML generation (Task 4).

use crate::ast::Node;

// A scope holds all the variables defined in one place.
// We use a stack of these so that paragraphs and list items can have
// their own variables without messing with the variables outside them.
type Scope = Vec<(String, String)>;

struct Generator {
    scopes: Vec<Scope>,
}

impl Generator {
    // When we use a variable, we need to check if it was already defined.
    // We start with the current scope and then check the ones outside it.
    // This also makes sure an inner variable takes priority over an outer one.
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

    // We always add new variables to the scope we're currently working in.
    // This way, a variable defined inside a paragraph stays inside that paragraph.
    fn define(&mut self, name: &str, value: &str) {
        let top = self.scopes.len() - 1;
        self.scopes[top].push((name.to_string(), value.to_string()));
    }

    // Go through all the nodes and generate their HTML.
    // We put the results into a list first so we can join them together at the end.
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

    // Each node gets handled differently depending on what kind of content it has.
    // Some nodes create HTML tags, while others are used for checking variables.
    fn gen_node(&mut self, node: &Node) -> Result<String, String> {
        match node {
            Node::Comment(text) => Ok(format!("<!-- {} -->", text)),
            Node::Head(_) => Ok(String::new()),
            Node::Text(text) => Ok(text.clone()),
            Node::Bold(text) => Ok(format!("<b>{}</b>", text)),
            Node::Link(addr) => Ok(format!("<a href=\"{}\">{}</a>", addr, addr)),
            Node::Newline => Ok("<br>".to_string()),

            // Variable definitions don't need to show up in the HTML.
            // We just need to keep their values so we can use them later.
            Node::VarDefine { name, value, .. } => {
                self.define(name, value);
                Ok(String::new())
            }

            // Before putting a variable into the HTML, we check if it exists.
            // If it doesn't, we return an error instead of letting an undefined variable through.
            Node::VarUse { name, line } => match self.lookup(name) {
                Some(value) => Ok(value),
                None => Err(format!(
                    "Static semantic error at line {}: variable '{}' is used but has not been defined in this scope.",
                    line, name
                )),
            },

            // A paragraph gets its own scope because variables defined inside it
            // should not be available in other paragraphs.
            Node::Paragraph(children) => {
                let inner = self.in_new_scope(children)?;
                Ok(format!("<p>{}</p>", inner))
            }

            Node::List(items) => {
                let inner = self.gen_nodes(items)?;
                Ok(format!("<ul>{}</ul>", inner))
            }

            // We do the same thing with list items so each one has its own variables.
            Node::ListItem(children) => {
                let inner = self.in_new_scope(children)?;
                Ok(format!("<li>{}</li>", inner))
            }
        }
    }

    // Push a new scope before going through these children.
    // Once we're done, we remove it so those variables can't be used outside this block.
    fn in_new_scope(&mut self, children: &[Node]) -> Result<String, String> {
        self.scopes.push(Vec::new());
        let result = self.gen_nodes(children);
        self.scopes.pop();
        result
    }
}

/// Check the variables in the tree and build the HTML page.
pub fn generate_html(ast: &[Node]) -> Result<String, String> {
    // Start with one empty scope. This is where the global variables will go.
    let mut generator = Generator { scopes: vec![Vec::new()] };

    // First, look through the tree for the head so we can get the page title.
    let mut head = String::new();
    for node in ast {
        if let Node::Head(title) = node {
            head = format!("<head>\n<title>{}</title>\n</head>\n", title);
        }
    }

    // Now go through the rest of the nodes and put their HTML inside the body.
    // We skip anything that doesn't produce HTML, like variable definitions.
    let mut body = String::new();
    for node in ast {
        let piece = generator.gen_node(node)?;
        if !piece.is_empty() {
            body.push_str(&piece);
            body.push('\n');
        }
    }

    // Put everything together to get the final HTML page.
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
