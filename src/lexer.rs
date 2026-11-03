
use crate::traits::LexicalAnalyzer;
use std::process;

// These are the annotations that can be used by themselves.
const ANNOTATIONS: [&str; 9] = [
    "#HAI", "#KBYE", "#OBTW", "#TLDR", "#MKAY", "#OIC", "#IHAZ", "#ITIZ", "#NEWLINE",
];

// These ones need a second word after them to make a complete annotation.
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

// This is the lexer for reading the LOLCODE Markdown file one character at a time.
pub struct LolLexer {
    // Keep all the characters from the input here.
    chars: Vec<char>,

    // This tells us which character we are currently on.
    pos: usize,

    // Holds whatever token we are working on.
    current: String,

    // Keep track of the current line and column for error messages.
    line: usize,
    col: usize,

    // We need this to know which line the token started on.
    pub token_line: usize,
}

impl LolLexer {
    // Set up the lexer before we start reading the input.
    pub fn new(source: &str) -> LolLexer {
        LolLexer {
            // Turn the input into individual characters so we can read them one by one.
            chars: source.chars().collect(),
            pos: 0,
            current: String::new(),

            // Start counting from line 1 and column 1.
            line: 1,
            col: 1,
            token_line: 1,
        }
    }

    // Look at the next character without actually moving to it yet.
    fn peek_char(&self) -> Option<char> {
        if self.pos < self.chars.len() {
            Some(self.chars[self.pos])
        } else {
            // If we reach the end, there is nothing left to read.
            None
        }
    }

    // Get past any spaces or newlines before we start reading the next token.
    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek_char() {
            if c.is_whitespace() {
                self.get_char();
            } else {
                // Stop once we find an actual character.
                break;
            }
        }
    }

    // Show where the problem happened if we find an annotation that isn't supported.
    fn lexical_error(&self, lexeme: &str, line: usize, col: usize) -> ! {
        eprintln!(
            "Lexical error at line {}, column {}: '{}' is not a valid LOLCODE Markdown annotation.",
            line, col, lexeme
        );

        // Stop the program because we cannot continue with an invalid annotation.
        process::exit(1);
    }

    // Read the rest of the word until we hit a space, another #, or the end.
    fn read_word(&mut self) {
        while let Some(c) = self.peek_char() {
            if c.is_whitespace() || c == '#' {
                // We don't want to include these in the word.
                break;
            }

            // Read the character and make it uppercase.
            let c = self.get_char();
            self.add_char(c.to_ascii_uppercase());
        }
    }

    // This gets called whenever we need the next token from the input.
    pub fn next_token(&mut self) -> String {
        // Clear out the previous token so we can start reading a new one.
        self.current.clear();

        // Get past spaces and newlines first.
        self.skip_whitespace();

        // Keep the starting position in case we need it for an error.
        let start_line = self.line;
        let start_col = self.col;
        self.token_line = start_line;

        // Check what the next character is before deciding what to do.
        let first = match self.peek_char() {
            None => return String::new(), // Nothing left in the file.
            Some(c) => c,
        };

        if first == '#' {
            // ANNOTATION STATE:
            // If we see #, we know that we are starting an annotation.

            // Read the # first and add it to the current token.
            let c = self.get_char();
            self.add_char(c);

            // Get the annotation name after the #.
            self.read_word();

            // KEYWORD STATE:
            // #MAEK and #GIMMEH are different because they need another word.
            if self.current == "#MAEK" || self.current == "#GIMMEH" {

                // Get past the spaces between the two words.
                self.skip_whitespace();

                // Add a space so both words stay together as one annotation.
                self.add_char(' ');

                // Now read the second word too.
                self.read_word();
            }

            // Make sure the annotation we just read is actually supported.
            if !self.lookup(&self.current) {
                let bad = self.current.clone();

                // Show the invalid annotation and where we found it.
                self.lexical_error(&bad, start_line, start_col);
            }
        } else {
            // TEXT STATE:
            // If the next character isn't #, we treat it as regular text.

            // Keep reading until we reach another annotation.
            while let Some(c) = self.peek_char() {
                if c == '#' {
                    // Stop here because the next annotation is starting.
                    break;
                }

                let c = self.get_char();
                self.add_char(c);
            }

            // Get rid of extra spaces and newlines at the end of the text.
            let trimmed = self.current.trim_end().to_string();
            self.current = trimmed;
        }

        // Return the token we just finished reading.
        self.current.clone()
    }
}

impl LexicalAnalyzer for LolLexer {
    // Read the current character and move forward to the next one.
    fn get_char(&mut self) -> char {
        if self.pos >= self.chars.len() {
            // Make sure we don't try to read past the end of the input.
            eprintln!("Lexical error: unexpected end of input.");
            process::exit(1);
        }

        // Get the character at the current position.
        let c = self.chars[self.pos];

        // Move forward by one character.
        self.pos += 1;

        // Update the line and column depending on what character we just read.
        if c == '\n' {
            // We are on a new line now, so start the column count over.
            self.line += 1;
            self.col = 1;
        } else {
            // Otherwise just move one column forward.
            self.col += 1;
        }

        // Give back the character we just read.
        c
    }

    // Add the character to whatever token we are currently building.
    fn add_char(&mut self, c: char) {
        self.current.push(c);
    }

    // Check if the annotation is in our list of allowed annotations.
    fn lookup(&self, s: &str) -> bool {

        // If it doesn't start with #, we can treat it as regular text.
        if !s.starts_with('#') {
            return true;
        }

        // Make it uppercase so lowercase annotations work too.
        let upper = s.to_ascii_uppercase();

        // Check both lists to make sure we cover single and two-word annotations.
        ANNOTATIONS.contains(&upper.as_str()) || COMPOUND_ANNOTATIONS.contains(&upper.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // This helper keeps getting tokens until there is nothing left to read.
    fn all_tokens(src: &str) -> Vec<String> {
        let mut lex = LolLexer::new(src);
        let mut out = Vec::new();

        loop {
            // Ask the lexer for the next token.
            let t = lex.next_token();

            // An empty string means we have reached the end.
            if t.is_empty() {
                break;
            }

            // Put the token in the output list.
            out.push(t);
        }

        out
    }

    #[test]
    fn lookup_accepts_valid_annotations() {
        let lex = LolLexer::new("");

        // Make sure valid single-word annotations are accepted.
        assert_eq!(lex.lookup("#HAI"), true);
        assert_eq!(lex.lookup("#kbye"), true);

        // These should work even when the annotation is lowercase.
        assert_eq!(lex.lookup("#MAEK PARAGRAF"), true);
        assert_eq!(lex.lookup("#gimmeh bold"), true);

        // Regular text should be allowed too.
        assert_eq!(lex.lookup("hello world"), true);
    }

    #[test]
    fn lookup_rejects_bad_annotations() {
        let lex = LolLexer::new("");

        // Make sure annotations that aren't in our lists get rejected.
        assert_eq!(lex.lookup("#HEY"), false);
        assert_eq!(lex.lookup("#HEAD"), false);
        assert_eq!(lex.lookup("#MAEK FOO"), false);
    }

    #[test]
    fn get_and_add_char() {
        let mut lex = LolLexer::new("ab");

        // Check that we get the first character correctly.
        assert_eq!(lex.get_char(), 'a');

        // Add another character and make sure it goes into the current token.
        lex.add_char('x');
        assert_eq!(lex.current, "x");

        // We should now be at the second character.
        assert_eq!(lex.get_char(), 'b');
    }

    #[test]
    fn splits_simple_program() {
        // Try an input with different annotations and some regular text.
        let toks = all_tokens("#hai\n#MAEK   head #GIMMEH TITLE Hi there #OIC #MKAY\nsome text\n#KBYE");

        // Make sure everything gets split into the tokens we expect.
        assert_eq!(
            toks,
            vec!["#HAI", "#MAEK HEAD", "#GIMMEH TITLE", "Hi there", "#OIC", "#MKAY", "some text", "#KBYE"]
        );
    }
}
