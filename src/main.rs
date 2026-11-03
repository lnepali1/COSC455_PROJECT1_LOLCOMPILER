//! lolcompiler: compiles a LOLCODE Markdown (.lol) file into HTML.
//! Usage: `lolcompiler input.lol`

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

/// Show the user how to run the program if something is wrong with the input.
fn usage_error(msg: &str) -> ! {
    eprintln!("Usage error: {}", msg);
    eprintln!("Usage: lolcompiler <input.lol>");
    process::exit(1);
}

/// Open the HTML file in the default browser once we're done compiling.
fn open_in_browser(path: &Path) {
    let p = path.to_string_lossy().to_string();

    // Different operating systems use different commands to open files.
    // We check which OS we're running on so the HTML opens in the right browser.
    let result = if cfg!(target_os = "macos") {
        Command::new("open").arg(&p).spawn()
    } else if cfg!(target_os = "windows") {
        // "start" is part of cmd, the empty "" is the window title
        Command::new("cmd").args(["/C", "start", "", &p]).spawn()
    } else {
        Command::new("xdg-open").arg(&p).spawn()
    };

    // If the browser doesn't open, the user can still find the HTML file themselves.
    if result.is_err() {
        eprintln!("Note: could not open a browser. Open {} by hand.", p);
    }
}

fn main() {
    // Get the command-line arguments. We only expect the program name and one file.
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        usage_error("expected exactly one input file.");
    }

    // Make sure the file has a .lol extension before trying to read it.
    let input = Path::new(&args[1]);
    let is_lol = match input.extension() {
        Some(ext) => ext.to_string_lossy().eq_ignore_ascii_case("lol"),
        None => false,
    };
    if !is_lol {
        usage_error(&format!("'{}' is not a .lol file.", args[1]));
    }

    // Read the source code from the input file.
    // If the file can't be opened or read, there's no point continuing.
    let source = match fs::read_to_string(input) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error: could not read '{}': {}", args[1], e);
            process::exit(1);
        }
    };

    // Run the compiler on the source. This handles the lexer, parser,
    // and semantic checks before we generate the final HTML.
    let mut compiler = LolCompiler::new("");
    compiler.compile(&source);

    // Use the same file name but change the extension to .html.
    let output = input.with_extension("html");
    if let Err(e) = fs::write(&output, &compiler.html) {
        eprintln!("Error: could not write '{}': {}", output.display(), e);
        process::exit(1);
    }

    println!("Compiled successfully: {}", output.display());

    // Open the finished HTML file so the user can see the result right away.
    open_in_browser(&output);
}
