//! The abstract syntax tree built by the parser.
//! Each Node represents one part of our document. We use different node
//! types for things like text, paragraphs, lists and variables. Some nodes
//! can also contain other nodes, which lets us keep the document organized
//! like a tree instead of having everything in one long list.

/// One node of the syntax tree.
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    // Keep the comment text so we can generate it later.
    /// `#OBTW text #TLDR`
    Comment(String),

    // We only need the title text from the head section.
    /// `#MAEK HEAD #GIMMEH TITLE text #OIC #MKAY`
    Head(String),

    // Regular text that doesn't have any special formatting.
    /// Plain text
    Text(String),

    // This will be wrapped in a bold tag when we generate the HTML.
    /// `#GIMMEH BOLD text #OIC`
    Bold(String),

    // Keep the address so we can turn it into a clickable link.
    /// `#GIMMEH LINX address #OIC`
    Link(String),

    // We use this whenever the parser comes across a newline.
    /// `#NEWLINE`
    Newline,

    // We keep the variable name and value so we can use it later.
    // The line number also helps us show where an error happened
    // if someone tries to use a variable that hasn't been defined.
    /// `#IHAZ name #ITIZ value #MKAY`
    VarDefine { name: String, value: String, line: usize },

    // This represents a variable being used somewhere in the document.
    // We keep the line number here too in case the variable isn't found.
    /// `#GIMMEH VAR name #OIC`
    VarUse { name: String, line: usize },

    // A paragraph can contain different kinds of nodes, including text and variables.
    // Using Vec<Node> lets us keep all of its content together.
    /// `#MAEK PARAGRAF ... #MKAY`
    Paragraph(Vec<Node>),

    // A list holds multiple list items, so its children will be ListItem nodes.
    /// `#MAEK LIST ... #MKAY`
    List(Vec<Node>),

    // Each item can have its own content, just like a paragraph.
    /// `#GIMMEH ITEM ... #OIC`
    ListItem(Vec<Node>),
}
