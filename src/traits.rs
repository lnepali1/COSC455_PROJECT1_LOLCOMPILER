//! The traits given in the project description.
//!
//! These are copied from the assignment so the rest of the compiler
//! can implement them. I picked OPTION 2 for the syntax analyzer
//! (each parse function returns a `Result`) because it lets the
//! error message travel back up to one place that prints it and exits,
//! and it makes the parse functions easy to unit test.

/// Trait for a simple lolcompiler front-end.
/// Errors should cause immediate exit inside the implementation.
pub trait Compiler {
    /// Begin the compilation process (entry point).
    fn compile(&mut self, source: &str);
    /// Get the next token from the lexical analyzer.
    fn next_token(&mut self) -> String;
    /// Run the syntax analyzer starting from <lolcode>.
    fn parse(&mut self);
    /// Get the current token being processed.
    fn current_token(&self) -> String;
    /// Set the current token (typically used internally).
    fn set_current_token(&mut self, tok: String);
}

/// Trait for a simple lexical analyzer.
/// Implements a character-by-character analysis
/// from a state machine design.
pub trait LexicalAnalyzer {
    /// Return the next character from the input.
    /// If input is exhausted, should terminate the program.
    fn get_char(&mut self) -> char;
    /// Add a character to the current potential token.
    fn add_char(&mut self, c: char);
    /// Lookup a potential token to determine if it is valid.
    /// Returns true if a valid token/lexeme, false otherwise.
    fn lookup(&self, s: &str) -> bool;
}

/// OPTION 2 - Trait for a recursive descent Syntax Analyzer.
/// Each function parses a nonterminal in the grammar.
/// On error: return Err(message), on success: Ok(()).
pub trait SyntaxAnalyzer {
    fn parse_lolcode(&mut self) -> Result<(), String>;
    fn parse_head(&mut self) -> Result<(), String>;
    fn parse_title(&mut self) -> Result<(), String>;
    fn parse_comment(&mut self) -> Result<(), String>;
    fn parse_body(&mut self) -> Result<(), String>;
    fn parse_paragraph(&mut self) -> Result<(), String>;
    fn parse_inner_paragraph(&mut self) -> Result<(), String>;
    fn parse_inner_text(&mut self) -> Result<(), String>;
    fn parse_variable_define(&mut self) -> Result<(), String>;
    fn parse_variable_use(&mut self) -> Result<(), String>;
    fn parse_bold(&mut self) -> Result<(), String>;
    fn parse_list(&mut self) -> Result<(), String>;
    fn parse_list_items(&mut self) -> Result<(), String>;
    fn parse_inner_list(&mut self) -> Result<(), String>;
    fn parse_link(&mut self) -> Result<(), String>;
    fn parse_newline(&mut self) -> Result<(), String>;
    fn parse_text(&mut self) -> Result<(), String>;
}
