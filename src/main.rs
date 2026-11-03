//! lolcompiler: compiles a LOLCODE Markdown (.lol) file into HTML.
//!
//! Usage: `lolcompiler input.lol`
//!
//! Steps:
//! 1. Check there is one argument and that it ends in `.lol`.
//! 2. Read the file and run the compiler (lexer -> parser -> scope
//!    check -> HTML). Any error is printed and the program exits
//!    before any file is written.
//! 3. Write `input.html` next to the input file and open it in the
//!    default browser.

mod ast;
mod lexer;
mod parser;
mod semantic;
mod traits;

use parser::LolCompiler;
use std::env;
use std::fs;
use std::path::Path;
use std::process::{self, Command};
use traits::Compiler;

/// Print how to use the program and exit.
fn usage_error(msg: &str) -> ! {
    eprintln!("Usage error: {}", msg);
    eprintln!("Usage: lolcompiler <input.lol>");
    process::exit(1);
}

/// Open the HTML file in the default browser for this OS.
fn open_in_browser(path: &Path) {
    let p = path.to_string_lossy().to_string();
    let result = if cfg!(target_os = "macos") {
        Command::new("open").arg(&p).spawn()
    } else if cfg!(target_os = "windows") {
        // "start" is part of cmd, the empty "" is the window title
        Command::new("cmd").args(["/C", "start", "", &p]).spawn()
    } else {
        Command::new("xdg-open").arg(&p).spawn()
    };
    if result.is_err() {
        eprintln!("Note: could not open a browser. Open {} by hand.", p);
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        usage_error("expected exactly one input file.");
    }

    let input = Path::new(&args[1]);
    let is_lol = match input.extension() {
        Some(ext) => ext.to_string_lossy().eq_ignore_ascii_case("lol"),
        None => false,
    };
    if !is_lol {
        usage_error(&format!("'{}' is not a .lol file.", args[1]));
    }

    let source = match fs::read_to_string(input) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error: could not read '{}': {}", args[1], e);
            process::exit(1);
        }
    };

    // compile() exits on any lexical, syntax or semantic error
    let mut compiler = LolCompiler::new("");
    compiler.compile(&source);

    let output = input.with_extension("html");
    if let Err(e) = fs::write(&output, &compiler.html) {
        eprintln!("Error: could not write '{}': {}", output.display(), e);
        process::exit(1);
    }
    println!("Compiled successfully: {}", output.display());
    open_in_browser(&output);
}
