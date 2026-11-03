//! The traits given in the project description.
//! These traits are used by the different parts of our compiler.
//! They make sure the lexer, parser, and compiler all follow the
//! same structure, so we can keep the code organized.

/// Trait for a simple lolcompiler front-end.
/// Errors should cause immediate exit inside the implementation.
pub trait Compiler {
    /// This is where the whole compilation process starts.
    fn compile(&mut self, source: &str);

    /// Get the next token from the lexer so the parser can work with it.
    fn next_token(&mut self) -> String;

    /// Start parsing the input based on our grammar.
    fn parse(&mut self);

    /// Check which token we're currently working with.
    fn current_token(&self) -> String;

    /// Update the current token as we move through the input.
    fn set_current_token(&mut self, tok: String);
}

/// Trait for a simple lexical analyzer.
/// Implements a character-by-character analysis
/// from a state machine design.
pub trait LexicalAnalyzer {
    /// Read the next character from the input.
    /// The lexer uses this to move through the source one character at a time.
    /// If there is nothing left to read, the program should stop.
    fn get_char(&mut self) -> char;

    /// Add the character we're reading to the token we're building.
    fn add_char(&mut self, c: char);

    /// Check whether the token we have built is actually valid.
    /// This helps the lexer tell the difference between valid tokens and invalid input.
    fn lookup(&self, s: &str) -> bool;
}

// We went with OPTION 2 for the syntax analyzer.
// Each parsing function returns a Result, so if something goes wrong,
// the error can travel back to the main parser instead of exiting from every function.
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
