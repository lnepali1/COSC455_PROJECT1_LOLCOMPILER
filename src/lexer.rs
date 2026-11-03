//! Lexical analyzer (Task 1).
//!
//! The lexer reads the source one character at a time and follows the
//! state machine from my Phase 1 design:
//!
//! * START: skip whitespace. A `#` moves to the ANNOTATION state, any
//!   other character moves to the TEXT state.
//! * ANNOTATION: keep adding letters until whitespace, another `#`, or
//!   the end of the file. If the word is `#MAEK` or `#GIMMEH`, go to the
//!   KEYWORD state to read the word that must follow it.
//! * KEYWORD: skip spaces, then read the next word (HEAD, PARAGRAF, ...)
//!   and glue it on, so the token becomes e.g. `#MAEK HEAD`.
//! * TEXT: keep adding characters until a `#` or the end of the file.
//!
//! Annotations are case-insensitive, so they are stored in upper case.
//! Plain text keeps its original case.
//!
//! Why `#MAEK HEAD` is one token: in the BNF, `#MAEK` starts a head, a
//! paragraph and a list, and `#GIMMEH` starts a title, bold, item, link
//! and variable use. With only one token of lookahead the parser could
//! not choose between them. Joining the two words fixes that and keeps
//! the grammar LL(1).

use crate::traits::LexicalAnalyzer;
use std::process;

/// Single-word annotations that are valid on their own.
const ANNOTATIONS: [&str; 9] = [
    "#HAI", "#KBYE", "#OBTW", "#TLDR", "#MKAY", "#OIC", "#IHAZ", "#ITIZ", "#NEWLINE",
];

/// Two-word annotations. The first word is `#MAEK` or `#GIMMEH`.
const COMPOUND_ANNOTATIONS: [&str; 8] = [
    "#MAEK HEAD",
    "#MAEK PARAGRAF",
    "#MAEK LIST",
    "#GIMMEH TITLE",
    "#GIMMEH BOLD",
    "#GIMMEH ITEM",
    "#GIMMEH LINX",
    "#GIMMEH VAR",
];

/// Character-by-character lexer for LOLCODE Markdown.
pub struct LolLexer {
    /// The whole source file as characters.
    chars: Vec<char>,
    /// Index of the next character to read.
    pos: usize,
    /// The token being built right now.
    current: String,
    /// Line and column of the next character (for error messages).
    line: usize,
    col: usize,
    /// Line where the most recent token started.
    pub token_line: usize,
}

impl LolLexer {
    /// Make a lexer for the given source text.
    pub fn new(source: &str) -> LolLexer {
        LolLexer {
            chars: source.chars().collect(),
            pos: 0,
            current: String::new(),
            line: 1,
            col: 1,
            token_line: 1,
        }
    }

    /// Look at the next character without using it up.
    /// Returns None at the end of the file.
    fn peek_char(&self) -> Option<char> {
        if self.pos < self.chars.len() {
            Some(self.chars[self.pos])
        } else {
            None
        }
    }

    /// Skip spaces, tabs and newlines.
    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek_char() {
            if c.is_whitespace() {
                self.get_char();
            } else {
                break;
            }
        }
    }

    /// Print a lexical error with its position and stop the program.
    fn lexical_error(&self, lexeme: &str, line: usize, col: usize) -> ! {
        eprintln!(
            "Lexical error at line {}, column {}: '{}' is not a valid LOLCODE Markdown annotation.",
            line, col, lexeme
        );
        process::exit(1);
    }

    /// Read the rest of a word (stops at whitespace, `#`, or end of file).
    fn read_word(&mut self) {
        while let Some(c) = self.peek_char() {
            if c.is_whitespace() || c == '#' {
                break;
            }
            let c = self.get_char();
            self.add_char(c.to_ascii_uppercase());
        }
    }

    /// Return the next token, or an empty string at the end of the file.
    /// A bad annotation is reported and the program exits.
    pub fn next_token(&mut self) -> String {
        self.current.clear();
        self.skip_whitespace();

        let start_line = self.line;
        let start_col = self.col;
        self.token_line = start_line;

        let first = match self.peek_char() {
            None => return String::new(), // end of file
            Some(c) => c,
        };

        if first == '#' {
            // ANNOTATION state
            let c = self.get_char();
            self.add_char(c);
            self.read_word();

            // KEYWORD state: #MAEK and #GIMMEH need a second word
            if self.current == "#MAEK" || self.current == "#GIMMEH" {
                self.skip_whitespace();
                self.add_char(' ');
                self.read_word();
            }

            if !self.lookup(&self.current) {
                let bad = self.current.clone();
                self.lexical_error(&bad, start_line, start_col);
            }
        } else {
            // TEXT state: everything up to the next '#'
            while let Some(c) = self.peek_char() {
                if c == '#' {
                    break;
                }
                let c = self.get_char();
                self.add_char(c);
            }
            // drop the whitespace at the end of the text
            let trimmed = self.current.trim_end().to_string();
            self.current = trimmed;
        }

        self.current.clone()
    }
}

impl LexicalAnalyzer for LolLexer {
    /// Read one character and move forward, keeping track of line and column.
    /// The lexer always checks `peek_char` first, so running out here
    /// would be a bug, and the program stops.
    fn get_char(&mut self) -> char {
        if self.pos >= self.chars.len() {
            eprintln!("Lexical error: unexpected end of input.");
            process::exit(1);
        }
        let c = self.chars[self.pos];
        self.pos += 1;
        if c == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        c
    }

    /// Add a character to the token being built.
    fn add_char(&mut self, c: char) {
        self.current.push(c);
    }

    /// Check if a lexeme is a valid annotation. Plain text (anything not
    /// starting with `#`) is always valid. The check ignores case.
    fn lookup(&self, s: &str) -> bool {
        if !s.starts_with('#') {
            return true;
        }
        let upper = s.to_ascii_uppercase();
        ANNOTATIONS.contains(&upper.as_str()) || COMPOUND_ANNOTATIONS.contains(&upper.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Run the lexer to the end and collect every token.
    fn all_tokens(src: &str) -> Vec<String> {
        let mut lex = LolLexer::new(src);
        let mut out = Vec::new();
        loop {
            let t = lex.next_token();
            if t.is_empty() {
                break;
            }
            out.push(t);
        }
        out
    }

    #[test]
    fn lookup_accepts_valid_annotations() {
        let lex = LolLexer::new("");
        assert_eq!(lex.lookup("#HAI"), true);
        assert_eq!(lex.lookup("#kbye"), true);
        assert_eq!(lex.lookup("#MAEK PARAGRAF"), true);
        assert_eq!(lex.lookup("#gimmeh bold"), true);
        assert_eq!(lex.lookup("hello world"), true);
    }

    #[test]
    fn lookup_rejects_bad_annotations() {
        let lex = LolLexer::new("");
        assert_eq!(lex.lookup("#HEY"), false);
        assert_eq!(lex.lookup("#HEAD"), false);
        assert_eq!(lex.lookup("#MAEK FOO"), false);
    }

    #[test]
    fn get_and_add_char() {
        let mut lex = LolLexer::new("ab");
        assert_eq!(lex.get_char(), 'a');
        lex.add_char('x');
        assert_eq!(lex.current, "x");
        assert_eq!(lex.get_char(), 'b');
    }

    #[test]
    fn splits_simple_program() {
        let toks = all_tokens("#hai\n#MAEK   head #GIMMEH TITLE Hi there #OIC #MKAY\nsome text\n#KBYE");
        assert_eq!(
            toks,
            vec!["#HAI", "#MAEK HEAD", "#GIMMEH TITLE", "Hi there", "#OIC", "#MKAY", "some text", "#KBYE"]
        );
    }
}
