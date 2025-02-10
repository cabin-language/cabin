use std::collections::VecDeque;

use api::{context::context, diagnostics::Diagnostics};
use comptime::CompileTime;
use lexer::Token;
use parser::{expressions::Expression, Module, Parse as _, Program};

pub mod api;
pub mod cli;
pub mod compiler;
pub mod comptime;
pub mod lexer;
pub mod parser;
pub mod transpiler;

// Re-exports

pub use api::{diagnostics, diagnostics::Error};

pub use crate::{lexer::Span, parser::expressions::Spanned};

/// Checks a Cabin program for errors.
///
/// # Returns
///
/// The errors that occurred. If no errors occurred, then `errors.is_empty() == true`.
pub fn check_module(code: &str) -> Diagnostics {
	context().reset();
	let mut tokens = lexer::tokenize_without_prelude(code);
	let module = Module::parse(&mut tokens);
	let _ = module.evaluate_at_compile_time();
	context().diagnostics().to_owned()
}

pub fn parse_module(code: &str) -> (Module, Diagnostics) {
	context().reset();
	let mut tokens = lexer::tokenize_without_prelude(code);
	(Module::parse(&mut tokens), context().diagnostics().to_owned())
}

pub fn check_program(code: &str) -> Diagnostics {
	let stdlib = parse_module(STDLIB).0.to_pointer();
	context().scope_data.declare_new_variable("builtin", Expression::Pointer(stdlib));

	let mut tokens = lexer::tokenize_without_prelude(code);
	let program = Program::parse(&mut tokens);
	let _ = program.evaluate_at_compile_time();
	context().diagnostics().to_owned()
}

pub fn tokenize(code: &str) -> (VecDeque<Token>, Diagnostics) {
	context().reset();
	(lexer::tokenize_without_prelude(code), context().diagnostics().to_owned())
}

/// The Cabin standard library. This is a Cabin file that's automatically imported into every Cabin
/// project or file. It contains definitions for all of the built-in types and objects, such as
/// `Text`, `Number`, `terminal`, etc. See `/std/stdlib.cabin` for its contents.
pub const STDLIB: &str = include_str!("../std/stdlib.cabin");

/// The Cabin prelude. This is a Cabin file that's automatically prepended to all Cabin files
/// written by the user. It just brings some useful items into scope from the standard library.
/// See `/std/prelude.cabin` for its contents.
pub const PRELUDE: &str = include_str!("../std/prelude.cabin");
