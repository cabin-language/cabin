use std::{collections::VecDeque, path::Path};

use api::io::Io;

// Re-exports
pub use crate::api::{context::Context, project::Project, span::*, *};
use crate::{
	api::diagnostics::{Diagnostic, Diagnostics},
	ast::misc::{module::Module, program::Program},
	comptime::Evaluate as _,
	lexer::Token,
	parser::{Parse as _, TryParse as _},
};

/// The `ast` module, short for "abstract syntax tree". This module holds each of the specific
/// types of AST nodes, each of which is coupled with how it's parsed, evaluated, etc.
pub mod ast;
pub mod comptime;
pub mod interpreter;

/// The `lexer` module. This module handles tokenization of source code, which is the first step of
/// compilation. The raw source code is first split into "tokens" in this module, before being sent
/// off to the `ast` module for parsing.
pub mod lexer;
pub mod parser;
pub mod transpiler;
pub mod util;

/// The `api` module. This module holds a bunch of utilities and abstractions within the compiler.
pub mod api;

/// The Cabin standard library. This is a Cabin file that's automatically imported into every Cabin
/// project or file. It contains definitions for all of the built-in types and objects, such as
/// `Text`, `Number`, `terminal`, etc. See `/std/stdlib.cabin` for its contents.
pub const STDLIB: &str = include_str!("../std/stdlib.cabin");

/// Parses a Cabin library file. A library file is like a program file, except it may only contain
/// compile-time declarations, instead of arbitrary runtime code and statements. This function
/// reads a file and parses it as a library.
///
/// To parse a library from its source code directly, use `parse_library`.
///
/// # Parameters
///
/// - `path` - The path to the library file
/// - `context` - The parsing context; This contains global data about the program. If you're
/// unsure what to use here, consider using `Context::default()`.
///
/// # Returns
///
/// The parsed `Module` object
///
/// # Errors
///
/// If the file cannot be read (for any reason, like the file not existing or not having the
/// correct permissions), an error is returned. If the file is read successfully, this will return
/// `Ok`, even if the program isn't formatted correctly or parseable correctly. To check if the
/// program was syntactically correct, check `context.diagnostics()` on your `context` object.
pub fn parse_library_file<P: AsRef<Path>, System: Io>(path: P, context: &mut Context) -> anyhow::Result<Module> {
	let code = std::fs::read_to_string(path.as_ref().join("src/library.cabin"))?;
	Ok(parse_library(&code, context)?)
}

/// Parses a Cabin library from its source code. A library is like a program, except it may only contain
/// compile-time declarations, instead of arbitrary runtime code and statements. This function
/// parses a string of Cabin source code under the assumption that it's a library.
///
/// To parse a library from a file, use `parse_library_file`.
///
/// # Parameters
///
/// - `code` - The library source code
/// - `context` - The parsing context; This contains global data about the program. If you're
/// unsure what to use here, consider using `Context::default()`.
///
/// # Returns
///
/// The parsed `Module` object
///
/// # Errors
///
/// If the file cannot be read (for any reason, like the file not existing or not having the
/// correct permissions), an error is returned. If the file is read successfully, this will return
/// `Ok`, even if the program isn't formatted correctly or parseable correctly. To check if the
/// program was syntactically correct, check `context.diagnostics()` on your `context` object.
pub fn parse_library(code: &str, context: &mut Context) -> Result<Module, Diagnostic> {
	let mut tokens = lexer::tokenize(code, context);
	Module::try_parse(&mut tokens, context)
}

pub fn parse_program(code: &str, context: &mut Context) -> Result<Program, Diagnostic> {
	let mut tokens = lexer::tokenize(code, context);
	Program::try_parse(&mut tokens, context)
}

/// Tokenizes a string of Cabin code, returning the tokens and any diagnostics (such as errors)
/// that occurred during tokenization.
///
/// # Parameters
///
/// - `code` - The Cabin source code to tokenize
///
/// # Returns
///
/// The tokens that were tokenized, which may contain `Unknown` tokens, and any diagnostics that
/// occurred during tokenization, such as tokenization errors.
pub fn tokenize(code: &str) -> (VecDeque<Token>, Diagnostics) {
	let mut context = Context::default();
	(lexer::tokenize(code, &mut context), context.diagnostics().to_owned())
}
