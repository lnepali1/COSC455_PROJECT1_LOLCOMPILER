//! The compiler front-end and recursive descent parser (Task 2).
//!
//! `LolCompiler` owns the lexer and pulls one token at a time from it
//! (one-token lookahead, like Lab 5). Every nonterminal in the BNF has a
//! `parse_...` function. Each one checks the current token, uses it, and
//! calls the functions for the nonterminals inside it.
//!
//! Building the tree: the parser keeps a stack of node lists. Starting a
//! paragraph, list or list item pushes a new empty list on the stack.
//! Nodes parsed inside go into the list on top. At the closing token the
//! list is popped and wrapped in its parent node.

use crate::ast::Node;
use crate::lexer::LolLexer;
use crate::semantic;
use crate::traits::{Compiler, SyntaxAnalyzer};
use std::process;

/// The LOLCODE Markdown compiler.
pub struct LolCompiler {
    /// The lexical analyzer that gives us tokens.
    lexer: LolLexer,
    /// The token we are looking at right now (the lookahead).
    current: String,
    /// Line where the current token starts.
    line: usize,
    /// Stack of child lists while the tree is being built.
    stack: Vec<Vec<Node>>,
    /// The finished tree (top-level nodes of the program).
    pub ast: Vec<Node>,
    /// The HTML made by the semantic analyzer.
    pub html: String,
}

impl LolCompiler {
    /// Make a compiler for the given source text.
    pub fn new(source: &str) -> LolCompiler {
        LolCompiler {
            lexer: LolLexer::new(source),
            current: String::new(),
            line: 1,
            stack: vec![Vec::new()],
            ast: Vec::new(),
            html: String::new(),
        }
    }

    // ---------- small helper functions ----------

    /// True if a token is plain text (not an annotation, not end of file).
    fn is_text(tok: &str) -> bool {
        !tok.is_empty() && !tok.starts_with('#')
    }

    /// How to show a token in an error message.
    fn describe(tok: &str) -> String {
        if tok.is_empty() {
            "end of file".to_string()
        } else {
            format!("'{}'", tok)
        }
    }

    /// Build a syntax error message for the current token.
    fn error(&self, expected: &str) -> String {
        format!(
            "Syntax error at line {}: expected {} but found {}.",
            self.line,
            expected,
            LolCompiler::describe(&self.current_token())
        )
    }

    /// Move to the next token.
    fn advance(&mut self) {
        let tok = self.next_token();
        self.set_current_token(tok);
    }

    /// The current token must be `tok`. Use it up or return an error.
    fn expect(&mut self, tok: &str) -> Result<(), String> {
        if self.current == tok {
            self.advance();
            Ok(())
        } else {
            Err(self.error(&format!("'{}'", tok)))
        }
    }

    /// The current token must be plain text. Use it up and return it.
    fn expect_text(&mut self, what: &str) -> Result<String, String> {
        if LolCompiler::is_text(&self.current) {
            let text = self.current.clone();
            self.advance();
            Ok(text)
        } else {
            Err(self.error(what))
        }
    }

    /// The current token must be a variable name: one word made of
    /// letters, digits or underscores.
    fn expect_varname(&mut self) -> Result<String, String> {
        let name = self.current.clone();
        let ok = LolCompiler::is_text(&name)
            && name.chars().all(|c| c.is_alphanumeric() || c == '_');
        if ok {
            self.advance();
            Ok(name)
        } else {
            Err(self.error("a variable name (one word)"))
        }
    }

    /// Add a finished node to the list on top of the stack.
    fn push_node(&mut self, node: Node) {
        let top = self.stack.len() - 1;
        self.stack[top].push(node);
    }

    /// Start collecting children for a new block.
    fn open_block(&mut self) {
        self.stack.push(Vec::new());
    }

    /// Stop collecting children and return them.
    fn close_block(&mut self) -> Vec<Node> {
        self.stack.pop().unwrap_or_default()
    }

    /// Can this token start <inner-text> (things allowed in a paragraph)?
    fn starts_inner_text(tok: &str) -> bool {
        LolCompiler::is_text(tok)
            || tok == "#GIMMEH VAR"
            || tok == "#OBTW"
            || tok == "#GIMMEH BOLD"
            || tok == "#MAEK LIST"
            || tok == "#GIMMEH LINX"
            || tok == "#NEWLINE"
    }

    /// Can this token start <inner-body> (paragraphs plus everything above)?
    fn starts_inner_body(tok: &str) -> bool {
        tok == "#MAEK PARAGRAF" || LolCompiler::starts_inner_text(tok)
    }

    /// Can this token start <list-content> (no lists inside list items)?
    fn starts_list_content(tok: &str) -> bool {
        tok != "#MAEK LIST" && LolCompiler::starts_inner_text(tok)
    }

    /// <optional-variable-define> ::= <variable-define> | ε
    fn parse_optional_variable_define(&mut self) -> Result<(), String> {
        if self.current == "#IHAZ" {
            self.parse_variable_define()?;
        }
        Ok(())
    }

    /// <list-item> ::= GIMMEH ITEM <optional-variable-define> <inner-list> OIC
    fn parse_list_item(&mut self) -> Result<(), String> {
        self.expect("#GIMMEH ITEM")?;
        self.open_block();
        self.parse_optional_variable_define()?;
        self.parse_inner_list()?;
        self.expect("#OIC")?;
        let children = self.close_block();
        self.push_node(Node::ListItem(children));
        Ok(())
    }

    /// <list-content> ::= <bold> | <comment> | <link> | <newline> | <variable-use> | TEXT
    fn parse_list_content(&mut self) -> Result<(), String> {
        match self.current.as_str() {
            "#GIMMEH BOLD" => self.parse_bold(),
            "#OBTW" => self.parse_comment(),
            "#GIMMEH LINX" => self.parse_link(),
            "#NEWLINE" => self.parse_newline(),
            "#GIMMEH VAR" => self.parse_variable_use(),
            _ => self.parse_text(),
        }
    }
}

impl Compiler for LolCompiler {
    /// Run the whole compiler: parse (lexing happens as tokens are
    /// needed), then check variables and make HTML. Any error is printed
    /// and the program exits, so no HTML is made.
    fn compile(&mut self, source: &str) {
        *self = LolCompiler::new(source);
        self.parse();
        match semantic::generate_html(&self.ast) {
            Ok(html) => self.html = html,
            Err(msg) => {
                eprintln!("{}", msg);
                process::exit(1);
            }
        }
    }

    /// Ask the lexer for the next token and remember its line.
    fn next_token(&mut self) -> String {
        let tok = self.lexer.next_token();
        self.line = self.lexer.token_line;
        tok
    }

    /// Read the first token and parse from <lolcode>.
    /// A syntax error is printed and the program exits.
    fn parse(&mut self) {
        self.advance();
        if let Err(msg) = self.parse_lolcode() {
            eprintln!("{}", msg);
            process::exit(1);
        }
        self.ast = self.close_block();
    }

    fn current_token(&self) -> String {
        self.current.clone()
    }

    fn set_current_token(&mut self, tok: String) {
        self.current = tok;
    }
}

impl SyntaxAnalyzer for LolCompiler {
    /// <lolcode> ::= HAI <comments> <head> <optional-variable-define> <body> KBYE
    fn parse_lolcode(&mut self) -> Result<(), String> {
        self.expect("#HAI")?;
        // <comments> ::= <comment> <comments> | ε
        while self.current == "#OBTW" {
            self.parse_comment()?;
        }
        self.parse_head()?;
        self.parse_optional_variable_define()?;
        self.parse_body()?;
        self.expect("#KBYE")?;
        if !self.current.is_empty() {
            return Err(self.error("end of file after '#KBYE'"));
        }
        Ok(())
    }

    /// <head> ::= MAEK HEAD <title> MKAY | ε
    fn parse_head(&mut self) -> Result<(), String> {
        if self.current == "#MAEK HEAD" {
            self.advance();
            self.parse_title()?;
            self.expect("#MKAY")?;
        }
        Ok(())
    }

    /// <title> ::= GIMMEH TITLE TEXT OIC
    fn parse_title(&mut self) -> Result<(), String> {
        self.expect("#GIMMEH TITLE")?;
        let title = self.expect_text("title text")?;
        self.expect("#OIC")?;
        self.push_node(Node::Head(title));
        Ok(())
    }

    /// <comment> ::= OBTW TEXT TLDR
    fn parse_comment(&mut self) -> Result<(), String> {
        self.expect("#OBTW")?;
        let text = self.expect_text("comment text")?;
        self.expect("#TLDR")?;
        self.push_node(Node::Comment(text));
        Ok(())
    }

    /// <body> ::= <inner-body> <body> | ε
    /// The loop runs while the token can start an <inner-body>.
    fn parse_body(&mut self) -> Result<(), String> {
        while LolCompiler::starts_inner_body(&self.current) {
            if self.current == "#MAEK PARAGRAF" {
                self.parse_paragraph()?;
            } else {
                self.parse_inner_text()?;
            }
        }
        Ok(())
    }

    /// <paragraph> ::= MAEK PARAGRAF <optional-variable-define> <inner-paragraph> MKAY
    fn parse_paragraph(&mut self) -> Result<(), String> {
        self.expect("#MAEK PARAGRAF")?;
        self.open_block();
        self.parse_optional_variable_define()?;
        self.parse_inner_paragraph()?;
        self.expect("#MKAY")?;
        let children = self.close_block();
        self.push_node(Node::Paragraph(children));
        Ok(())
    }

    /// <inner-paragraph> ::= <inner-text> <inner-paragraph> | ε
    fn parse_inner_paragraph(&mut self) -> Result<(), String> {
        while LolCompiler::starts_inner_text(&self.current) {
            self.parse_inner_text()?;
        }
        Ok(())
    }

    /// <inner-text> ::= <variable-use> | <comment> | <bold> | <list> | <link> | <newline> | TEXT
    fn parse_inner_text(&mut self) -> Result<(), String> {
        match self.current.as_str() {
            "#GIMMEH VAR" => self.parse_variable_use(),
            "#OBTW" => self.parse_comment(),
            "#GIMMEH BOLD" => self.parse_bold(),
            "#MAEK LIST" => self.parse_list(),
            "#GIMMEH LINX" => self.parse_link(),
            "#NEWLINE" => self.parse_newline(),
            _ => self.parse_text(),
        }
    }

    /// <variable-define> ::= IHAZ VARNAME ITIZ VARVALUE MKAY
    fn parse_variable_define(&mut self) -> Result<(), String> {
        let line = self.line;
        self.expect("#IHAZ")?;
        let name = self.expect_varname()?;
        self.expect("#ITIZ")?;
        let value = self.expect_text("a variable value")?;
        self.expect("#MKAY")?;
        self.push_node(Node::VarDefine { name, value, line });
        Ok(())
    }

    /// <variable-use> ::= GIMMEH VAR VARNAME OIC
    fn parse_variable_use(&mut self) -> Result<(), String> {
        let line = self.line;
        self.expect("#GIMMEH VAR")?;
        let name = self.expect_varname()?;
        self.expect("#OIC")?;
        self.push_node(Node::VarUse { name, line });
        Ok(())
    }

    /// <bold> ::= GIMMEH BOLD TEXT OIC
    fn parse_bold(&mut self) -> Result<(), String> {
        self.expect("#GIMMEH BOLD")?;
        let text = self.expect_text("bold text")?;
        self.expect("#OIC")?;
        self.push_node(Node::Bold(text));
        Ok(())
    }

    /// <list> ::= MAEK LIST <list-item> <list-items> MKAY
    fn parse_list(&mut self) -> Result<(), String> {
        self.expect("#MAEK LIST")?;
        self.open_block();
        // a list needs at least one item
        self.parse_list_item()?;
        self.parse_list_items()?;
        self.expect("#MKAY")?;
        let items = self.close_block();
        self.push_node(Node::List(items));
        Ok(())
    }

    /// <list-items> ::= <list-item> <list-items> | ε
    fn parse_list_items(&mut self) -> Result<(), String> {
        while self.current == "#GIMMEH ITEM" {
            self.parse_list_item()?;
        }
        Ok(())
    }

    /// <inner-list> ::= <list-content> <inner-list> | ε
    fn parse_inner_list(&mut self) -> Result<(), String> {
        while LolCompiler::starts_list_content(&self.current) {
            self.parse_list_content()?;
        }
        Ok(())
    }

    /// <link> ::= GIMMEH LINX ADDRESS OIC
    fn parse_link(&mut self) -> Result<(), String> {
        self.expect("#GIMMEH LINX")?;
        let address = self.expect_text("a link address")?;
        self.expect("#OIC")?;
        self.push_node(Node::Link(address));
        Ok(())
    }

    /// <newline> ::= NEWLINE
    fn parse_newline(&mut self) -> Result<(), String> {
        self.expect("#NEWLINE")?;
        self.push_node(Node::Newline);
        Ok(())
    }

    /// TEXT: one plain-text token.
    fn parse_text(&mut self) -> Result<(), String> {
        let text = self.expect_text("text")?;
        self.push_node(Node::Text(text));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Parse a source string and return the result of parse_lolcode.
    fn try_parse(src: &str) -> Result<Vec<Node>, String> {
        let mut c = LolCompiler::new(src);
        c.advance();
        c.parse_lolcode()?;
        Ok(c.close_block())
    }

    #[test]
    fn current_token_get_and_set() {
        let mut c = LolCompiler::new("#HAI #KBYE");
        assert_eq!(c.next_token(), "#HAI");
        c.set_current_token("#KBYE".to_string());
        assert_eq!(c.current_token(), "#KBYE");
    }

    #[test]
    fn parses_minimal_program() {
        assert_eq!(try_parse("#HAI #KBYE"), Ok(vec![]));
    }

    #[test]
    fn parses_paragraph_with_variable() {
        let ast = try_parse("#HAI #MAEK PARAGRAF #IHAZ x #ITIZ 5 #MKAY hi #GIMMEH VAR x #OIC #MKAY #KBYE").unwrap();
        assert_eq!(ast.len(), 1);
        match &ast[0] {
            Node::Paragraph(kids) => assert_eq!(kids.len(), 3),
            _ => panic!("expected a paragraph"),
        }
    }

    #[test]
    fn parses_list() {
        let ast = try_parse("#HAI #MAEK LIST #GIMMEH ITEM one #OIC #GIMMEH ITEM two #OIC #MKAY #KBYE").unwrap();
        match &ast[0] {
            Node::List(items) => assert_eq!(items.len(), 2),
            _ => panic!("expected a list"),
        }
    }

    #[test]
    fn rejects_extra_mkay() {
        assert!(try_parse("#HAI #MKAY #KBYE").is_err());
    }

    #[test]
    fn rejects_empty_list() {
        assert!(try_parse("#HAI #MAEK LIST #MKAY #KBYE").is_err());
    }

    #[test]
    fn rejects_missing_kbye() {
        assert!(try_parse("#HAI hello").is_err());
    }
}
