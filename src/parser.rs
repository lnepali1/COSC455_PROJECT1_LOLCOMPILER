
use crate::ast::Node;
use crate::lexer::LolLexer;
use crate::semantic;
use crate::traits::{Compiler, SyntaxAnalyzer};
use std::process;

// This is the compiler and parser for the LOLCODE Markdown program.
// It gets tokens from the lexer one at a time and checks if they follow the grammar.
//
// We only look one token ahead, so the parser always knows what it is checking next.
//
// While parsing, we also build the AST. The stack helps us keep track of
// paragraphs, lists, and list items while we are adding their contents.

/// The LOLCODE Markdown compiler.
pub struct LolCompiler {
    // This is where we get the tokens from.
    lexer: LolLexer,

    // Holds the token we are currently looking at.
    current: String,

    // Keep track of the line so we can show it if something goes wrong.
    line: usize,

    // We use this to keep track of nodes while building the AST.
    stack: Vec<Vec<Node>>,

    // This will hold the finished AST after parsing.
    pub ast: Vec<Node>,

    // The HTML gets stored here after the semantic checks.
    pub html: String,
}

impl LolCompiler {
    // Set up everything we need before starting to parse the program.
    pub fn new(source: &str) -> LolCompiler {
        LolCompiler {
            lexer: LolLexer::new(source),
            current: String::new(),
            line: 1,

            // Start with an empty list because we need somewhere to put the top-level nodes.
            stack: vec![Vec::new()],

            ast: Vec::new(),
            html: String::new(),
        }
    }

    // ---------- small helper functions ----------

    // Make sure this token is regular text and not an annotation or an empty token.
    fn is_text(tok: &str) -> bool {
        !tok.is_empty() && !tok.starts_with('#')
    }

    // This helps make error messages easier to read.
    // If there is no token left, just say end of file.
    fn describe(tok: &str) -> String {
        if tok.is_empty() {
            "end of file".to_string()
        } else {
            format!("'{}'", tok)
        }
    }

    // Put together an error message using the line we are currently on.
    fn error(&self, expected: &str) -> String {
        format!(
            "Syntax error at line {}: expected {} but found {}.",
            self.line,
            expected,
            LolCompiler::describe(&self.current_token())
        )
    }

    // Move forward so we can look at the next token.
    fn advance(&mut self) {
        let tok = self.next_token();
        self.set_current_token(tok);
    }

    // Make sure the current token is what we are expecting.
    // If it matches, move on. Otherwise, return an error.
    fn expect(&mut self, tok: &str) -> Result<(), String> {
        if self.current == tok {
            self.advance();
            Ok(())
        } else {
            Err(self.error(&format!("'{}'", tok)))
        }
    }

    // We use this when we are expecting regular text instead of an annotation.
    fn expect_text(&mut self, what: &str) -> Result<String, String> {
        if LolCompiler::is_text(&self.current) {
            // Keep the text before moving to the next token.
            let text = self.current.clone();
            self.advance();
            Ok(text)
        } else {
            Err(self.error(what))
        }
    }

    // Make sure the variable name is just one word.
    // It can have letters, numbers, and underscores.
    fn expect_varname(&mut self) -> Result<String, String> {
        let name = self.current.clone();

        // Check each character to make sure it follows the variable name rules.
        let ok = LolCompiler::is_text(&name)
            && name.chars().all(|c| c.is_alphanumeric() || c == '_');

        if ok {
            self.advance();
            Ok(name)
        } else {
            Err(self.error("a variable name (one word)"))
        }
    }

    // Add the finished node to the list we are currently working with.
    fn push_node(&mut self, node: Node) {
        let top = self.stack.len() - 1;
        self.stack[top].push(node);
    }

    // Start a new empty list for the contents of a paragraph or list.
    fn open_block(&mut self) {
        self.stack.push(Vec::new());
    }

    // We are done with this block, so take its contents out of the stack.
    fn close_block(&mut self) -> Vec<Node> {
        self.stack.pop().unwrap_or_default()
    }

    // Check if this token is something that can appear inside a paragraph.
    fn starts_inner_text(tok: &str) -> bool {
        LolCompiler::is_text(tok)
            || tok == "#GIMMEH VAR"
            || tok == "#OBTW"
            || tok == "#GIMMEH BOLD"
            || tok == "#MAEK LIST"
            || tok == "#GIMMEH LINX"
            || tok == "#NEWLINE"
    }

    // A paragraph can also start with another paragraph's allowed content.
    fn starts_inner_body(tok: &str) -> bool {
        tok == "#MAEK PARAGRAF" || LolCompiler::starts_inner_text(tok)
    }

    // Lists have their own rules, so we don't allow another list inside a list item.
    fn starts_list_content(tok: &str) -> bool {
        tok != "#MAEK LIST" && LolCompiler::starts_inner_text(tok)
    }

    // This part is optional, so only parse it if we actually find #IHAZ.
    fn parse_optional_variable_define(&mut self) -> Result<(), String> {
        if self.current == "#IHAZ" {
            self.parse_variable_define()?;
        }
        Ok(())
    }

    // A list item starts with #GIMMEH ITEM and ends with #OIC.
    fn parse_list_item(&mut self) -> Result<(), String> {
        self.expect("#GIMMEH ITEM")?;

        // Start collecting everything that belongs inside this list item.
        self.open_block();

        // A variable definition can be included, but it is not required.
        self.parse_optional_variable_define()?;

        // Read the actual contents of the list item.
        self.parse_inner_list()?;

        // Make sure the item is closed properly.
        self.expect("#OIC")?;

        // Get everything we collected and make it one list item node.
        let children = self.close_block();
        self.push_node(Node::ListItem(children));

        Ok(())
    }

    // Check which kind of content we have inside a list item.
    fn parse_list_content(&mut self) -> Result<(), String> {
        match self.current.as_str() {
            "#GIMMEH BOLD" => self.parse_bold(),
            "#OBTW" => self.parse_comment(),
            "#GIMMEH LINX" => self.parse_link(),
            "#NEWLINE" => self.parse_newline(),
            "#GIMMEH VAR" => self.parse_variable_use(),

            // If it isn't one of these annotations, treat it as regular text.
            _ => self.parse_text(),
        }
    }
}

impl Compiler for LolCompiler {
    // Run the whole compiler from beginning to end.
    fn compile(&mut self, source: &str) {
        // Start fresh whenever we compile a new source file.
        *self = LolCompiler::new(source);

        // First check that the program follows the grammar.
        self.parse();

        // If parsing works, check the AST and turn it into HTML.
        match semantic::generate_html(&self.ast) {
            Ok(html) => self.html = html,
            Err(msg) => {
                // Show the semantic error and stop instead of making HTML.
                eprintln!("{}", msg);
                process::exit(1);
            }
        }
    }

    // Get the next token from the lexer and update the line number too.
    fn next_token(&mut self) -> String {
        let tok = self.lexer.next_token();

        // The lexer already keeps track of which line the token starts on.
        self.line = self.lexer.token_line;

        tok
    }

    // Start parsing from the first token.
    fn parse(&mut self) {
        self.advance();

        // Try to parse the full LOLCODE program.
        if let Err(msg) = self.parse_lolcode() {
            eprintln!("{}", msg);
            process::exit(1);
        }

        // Get the finished top-level nodes and put them in the AST.
        self.ast = self.close_block();
    }

    // Return the token we are currently looking at.
    fn current_token(&self) -> String {
        self.current.clone()
    }

    // Change the current token to the one we just got from the lexer.
    fn set_current_token(&mut self, tok: String) {
        self.current = tok;
    }
}

impl SyntaxAnalyzer for LolCompiler {
    // This is the main rule for the whole program.
    // A program starts with HAI and must finish with KBYE.
    fn parse_lolcode(&mut self) -> Result<(), String> {
        self.expect("#HAI")?;

        // Comments can appear after HAI, and there can be more than one.
        while self.current == "#OBTW" {
            self.parse_comment()?;
        }

        // Check the header, optional variable definition, and the main body.
        self.parse_head()?;
        self.parse_optional_variable_define()?;
        self.parse_body()?;

        // Make sure the program ends with KBYE.
        self.expect("#KBYE")?;

        // There shouldn't be anything else after KBYE.
        if !self.current.is_empty() {
            return Err(self.error("end of file after '#KBYE'"));
        }

        Ok(())
    }

    // The header is optional, so only parse it if we find #MAEK HEAD.
    fn parse_head(&mut self) -> Result<(), String> {
        if self.current == "#MAEK HEAD" {
            self.advance();

            // Read the title inside the header.
            self.parse_title()?;

            // Make sure the header ends with MKAY.
            self.expect("#MKAY")?;
        }
        Ok(())
    }

    // Read the title text and make it a header node.
    fn parse_title(&mut self) -> Result<(), String> {
        self.expect("#GIMMEH TITLE")?;
        let title = self.expect_text("title text")?;
        self.expect("#OIC")?;

        // Add the title to the AST.
        self.push_node(Node::Head(title));
        Ok(())
    }

    // Read a comment between OBTW and TLDR.
    fn parse_comment(&mut self) -> Result<(), String> {
        self.expect("#OBTW")?;
        let text = self.expect_text("comment text")?;
        self.expect("#TLDR")?;

        // Keep the comment as its own node.
        self.push_node(Node::Comment(text));
        Ok(())
    }

    // Keep reading the body as long as we find something that belongs in it.
    fn parse_body(&mut self) -> Result<(), String> {
        while LolCompiler::starts_inner_body(&self.current) {

            // Paragraphs have their own parsing function.
            if self.current == "#MAEK PARAGRAF" {
                self.parse_paragraph()?;
            } else {
                // Everything else is handled as regular inner text.
                self.parse_inner_text()?;
            }
        }
        Ok(())
    }

    // A paragraph starts with MAEK PARAGRAF and ends with MKAY.
    fn parse_paragraph(&mut self) -> Result<(), String> {
        self.expect("#MAEK PARAGRAF")?;

        // Start a separate list for everything inside this paragraph.
        self.open_block();

        // Check for a variable definition before reading the paragraph contents.
        self.parse_optional_variable_define()?;
        self.parse_inner_paragraph()?;

        // Make sure we found the closing annotation.
        self.expect("#MKAY")?;

        // Put all the paragraph contents together in one node.
        let children = self.close_block();
        self.push_node(Node::Paragraph(children));

        Ok(())
    }

    // Keep reading paragraph contents until we reach something that doesn't belong.
    fn parse_inner_paragraph(&mut self) -> Result<(), String> {
        while LolCompiler::starts_inner_text(&self.current) {
            self.parse_inner_text()?;
        }
        Ok(())
    }

    // Figure out what kind of content we are looking at inside the paragraph.
    fn parse_inner_text(&mut self) -> Result<(), String> {
        match self.current.as_str() {
            "#GIMMEH VAR" => self.parse_variable_use(),
            "#OBTW" => self.parse_comment(),
            "#GIMMEH BOLD" => self.parse_bold(),
            "#MAEK LIST" => self.parse_list(),
            "#GIMMEH LINX" => self.parse_link(),
            "#NEWLINE" => self.parse_newline(),

            // If none of the annotations match, it must be regular text.
            _ => self.parse_text(),
        }
    }

    // This reads a variable definition.
    // The format needs a name, a value, and the closing MKAY.
    fn parse_variable_define(&mut self) -> Result<(), String> {
        let line = self.line;

        self.expect("#IHAZ")?;
        let name = self.expect_varname()?;
        self.expect("#ITIZ")?;
        let value = self.expect_text("a variable value")?;
        self.expect("#MKAY")?;

        // Add the variable definition to the AST with its line number.
        self.push_node(Node::VarDefine { name, value, line });
        Ok(())
    }

    // Read a variable name that is being used in the program.
    fn parse_variable_use(&mut self) -> Result<(), String> {
        let line = self.line;

        self.expect("#GIMMEH VAR")?;
        let name = self.expect_varname()?;
        self.expect("#OIC")?;

        // Keep the variable name and line number so semantic checking can use them later.
        self.push_node(Node::VarUse { name, line });
        Ok(())
    }

    // Read the text that should appear in bold.
    fn parse_bold(&mut self) -> Result<(), String> {
        self.expect("#GIMMEH BOLD")?;
        let text = self.expect_text("bold text")?;
        self.expect("#OIC")?;

        // Add the bold text to the AST.
        self.push_node(Node::Bold(text));
        Ok(())
    }

    // A list needs at least one list item.
    fn parse_list(&mut self) -> Result<(), String> {
        self.expect("#MAEK LIST")?;

        // Start collecting the items that belong to this list.
        self.open_block();

        // Read the first item because an empty list is not allowed.
        self.parse_list_item()?;

        // Check if there are any more items after the first one.
        self.parse_list_items()?;

        // Make sure the whole list ends with MKAY.
        self.expect("#MKAY")?;

        // Put all the items together and add the list to the AST.
        let items = self.close_block();
        self.push_node(Node::List(items));

        Ok(())
    }

    // Keep reading list items as long as another GIMMEH ITEM appears.
    fn parse_list_items(&mut self) -> Result<(), String> {
        while self.current == "#GIMMEH ITEM" {
            self.parse_list_item()?;
        }
        Ok(())
    }

    // Read the contents inside a list item.
    // We keep going until we reach something that doesn't belong in the item.
    fn parse_inner_list(&mut self) -> Result<(), String> {
        while LolCompiler::starts_list_content(&self.current) {
            self.parse_list_content()?;
        }
        Ok(())
    }

    // Read a link address and make it a link node.
    fn parse_link(&mut self) -> Result<(), String> {
        self.expect("#GIMMEH LINX")?;
        let address = self.expect_text("a link address")?;
        self.expect("#OIC")?;

        self.push_node(Node::Link(address));
        Ok(())
    }

    // Add a newline node whenever we find the NEWLINE annotation.
    fn parse_newline(&mut self) -> Result<(), String> {
        self.expect("#NEWLINE")?;
        self.push_node(Node::Newline);
        Ok(())
    }

    // Read regular text and add it to the AST.
    fn parse_text(&mut self) -> Result<(), String> {
        let text = self.expect_text("text")?;
        self.push_node(Node::Text(text));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // This lets us test the parser without having to run the entire compiler.
    fn try_parse(src: &str) -> Result<Vec<Node>, String> {
        let mut c = LolCompiler::new(src);

        // Get the first token before starting the parser.
        c.advance();

        // Try parsing the program and return an error if something is wrong.
        c.parse_lolcode()?;

        // Get the nodes that were created during parsing.
        Ok(c.close_block())
    }

    #[test]
    fn current_token_get_and_set() {
        let mut c = LolCompiler::new("#HAI #KBYE");

        // Check that we can read the next token.
        assert_eq!(c.next_token(), "#HAI");

        // Change the current token and make sure we get the updated value.
        c.set_current_token("#KBYE".to_string());
        assert_eq!(c.current_token(), "#KBYE");
    }

    #[test]
    fn parses_minimal_program() {
        // A program with just HAI and KBYE should work.
        assert_eq!(try_parse("#HAI #KBYE"), Ok(vec![]));
    }

    #[test]
    fn parses_paragraph_with_variable() {
        // Try a paragraph that has a variable definition and a variable use.
        let ast = try_parse("#HAI #MAEK PARAGRAF #IHAZ x #ITIZ 5 #MKAY hi #GIMMEH VAR x #OIC #MKAY #KBYE").unwrap();

        // There should only be one top-level node.
        assert_eq!(ast.len(), 1);

        // Make sure that node is a paragraph with three children.
        match &ast[0] {
            Node::Paragraph(kids) => assert_eq!(kids.len(), 3),
            _ => panic!("expected a paragraph"),
        }
    }

    #[test]
    fn parses_list() {
        // Try a list with two separate items.
        let ast = try_parse("#HAI #MAEK LIST #GIMMEH ITEM one #OIC #GIMMEH ITEM two #OIC #MKAY #KBYE").unwrap();

        // Make sure both items were added to the list.
        match &ast[0] {
            Node::List(items) => assert_eq!(items.len(), 2),
            _ => panic!("expected a list"),
        }
    }

    #[test]
    fn rejects_extra_mkay() {
        // There shouldn't be an extra MKAY in a program with no paragraph or list.
        assert!(try_parse("#HAI #MKAY #KBYE").is_err());
    }

    #[test]
    fn rejects_empty_list() {
        // Make sure the parser doesn't accept a list without any items.
        assert!(try_parse("#HAI #MAEK LIST #MKAY #KBYE").is_err());
    }

    #[test]
    fn rejects_missing_kbye() {
        // Every program needs to end with KBYE.
        assert!(try_parse("#HAI hello").is_err());
    }
}
