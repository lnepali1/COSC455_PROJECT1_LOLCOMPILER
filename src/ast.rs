//! The abstract syntax tree built by the parser.
//!
//! Each `Node` is one piece of the document. Paragraphs, lists and list
//! items hold their children in a `Vec`, so the tree is just nested vectors.

/// One node of the syntax tree.
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    /// `#OBTW text #TLDR`
    Comment(String),
    /// `#MAEK HEAD #GIMMEH TITLE text #OIC #MKAY` (only the title text is kept)
    Head(String),
    /// Plain text
    Text(String),
    /// `#GIMMEH BOLD text #OIC`
    Bold(String),
    /// `#GIMMEH LINX address #OIC`
    Link(String),
    /// `#NEWLINE`
    Newline,
    /// `#IHAZ name #ITIZ value #MKAY` (line is kept for error messages)
    VarDefine { name: String, value: String, line: usize },
    /// `#GIMMEH VAR name #OIC`
    VarUse { name: String, line: usize },
    /// `#MAEK PARAGRAF ... #MKAY`
    Paragraph(Vec<Node>),
    /// `#MAEK LIST ... #MKAY` (children are all `ListItem`s)
    List(Vec<Node>),
    /// `#GIMMEH ITEM ... #OIC`
    ListItem(Vec<Node>),
}
