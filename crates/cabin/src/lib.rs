use std::{collections::VecDeque, path::Path};

use api::io::Io;

// Re-exports
pub use crate::api::{context::Context, diagnostics::Error, project::Project, span::*, *};
use crate::{
	api::diagnostics::Diagnostics,
	ast::misc::{module::Module, program::Program},
	comptime::CompileTime as _,
	interpreter::Runtime as _,
	lexer::Token,
	parser::Parse as _,
};

/// The `ast` module, short for "abstract syntax tree". This module holds each of the specific
/// types of AST nodes, each of which is coupled with how it's parsed, evaluated, etc.
pub(crate) mod ast;
pub mod comptime;
pub mod interpreter;

/// The `lexer` module. This module handles tokenization of source code, which is the first step of
/// compilation. The raw source code is first split into "tokens" in this module, before being sent
/// off to the `ast` module for parsing.
pub(crate) mod lexer;
pub(crate) mod parser;
pub(crate) mod transpiler;
pub(crate) mod typechecker;

/// The `api` module. This module holds a bunch of utilities and abstractions within the compiler.
pub(crate) mod api;

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
pub fn parse_library_file<P: AsRef<Path>, System: Io>(path: P, context: &mut Context<System>) -> Result<Module, std::io::Error> {
	let code = std::fs::read_to_string(path.as_ref().join("src/library.cabin"))?;
	Ok(parse_library(&code, context))
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
pub fn parse_library<System: Io>(code: &str, context: &mut Context<System>) -> Module {
	let mut tokens = lexer::tokenize(code, context);
	Module::parse(&mut tokens, context)
}

pub fn parse_program<System: Io>(code: &str, context: &mut Context<System>) -> Program {
	let mut tokens = lexer::tokenize(code, context);
	Program::parse(&mut tokens, context)
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

/// Runs Cabin in "interpreter mode" on the given code. This will not compile the code to a native
/// binary; Instead, it'll be interpreted line-by-line. This is mainly used for the `cabin
/// interactive` command, which runs a cabin REPL, as well as the Cabin online playground. Any
/// errors (or other information) that occur will be stored on the context's `diagnostics`.
///
/// # Parameters
///
/// - `code` - The Cabin source code as a string
/// - `context` - The program's context; This contains global data about the program. If you're
pub fn interpret<System: Io>(code: &str, context: &mut Context<System>) {
	let mut tokens = lexer::tokenize(code, context);
	let program = Program::parse(&mut tokens, context);
	let evaluated = program.evaluate_at_compile_time(context);
	let _ = evaluated.evaluate_at_runtime(context);
}

pub fn interpret_with_logs<System: Io>(code: &str, context: &mut Context<System>) {
	context.println("Running...");
	context.println("\tChecking syntax and types...");
	let mut tokens = lexer::tokenize(code, context);
	let program = Program::parse(&mut tokens, context);
	context.println("\tRunning compile-time code...");
	let evaluated = program.evaluate_at_compile_time(context);
	context.println("\tRunning runtime code...");
	let _ = evaluated.evaluate_at_runtime(context);
}
