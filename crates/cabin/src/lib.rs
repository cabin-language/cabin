use std::collections::VecDeque;

use api::{context::Context, diagnostics::Diagnostics};
use comptime::CompileTime;
use lexer::Token;
use parser::{expressions::Expression, Module, Parse as _, Program};

pub(crate) mod api;
pub mod cli;
pub mod compiler;
pub mod comptime;
pub mod lexer;
pub mod parser;

// Re-exports

pub use api::{diagnostics::Error, *};

pub use crate::{lexer::Span, parser::expressions::Spanned};

/// Checks a Cabin program for errors.
///
/// # Returns
///
/// The errors that occurred. If no errors occurred, then `errors.is_empty() == true`.
pub fn check_module(code: &str) -> Context {
	let mut context = Context::default();
	let mut tokens = lexer::tokenize(code, &mut context);
	let module = Module::parse(&mut tokens, &mut context);
	let _ = module.evaluate_at_compile_time(&mut context);
	context
}

pub fn parse_module(code: &str, context: &mut Context) -> (Module, Diagnostics) {
	let mut tokens = lexer::tokenize(code, context);
	(Module::parse(&mut tokens, context), context.diagnostics().to_owned())
}

pub fn check_program(code: &str) -> Context {
	let mut context = Context::default();
	let stdlib = parse_module(STDLIB, &mut context).0.to_pointer(&mut context);
	context.scope_data.declare_new_variable("builtin", Expression::Pointer(stdlib)).unwrap();

	let mut tokens = lexer::tokenize(code, &mut context);
	let program = Program::parse(&mut tokens, &mut context);
	let _ = program.evaluate_at_compile_time(&mut context);

	context
}

pub fn tokenize(code: &str) -> (VecDeque<Token>, Diagnostics) {
	let mut context = Context::default();
	(lexer::tokenize(code, &mut context), context.diagnostics().to_owned())
}

pub fn try_tokenize(code: &str) -> Result<VecDeque<Token>, Error> {
	let mut context = Context::default();
	let tokens = lexer::tokenize(code, &mut context);
	if let Some(error) = context.diagnostics().errors().first().map(|error| error.0) {
		Err(error.to_owned())
	} else {
		Ok(tokens)
	}
}

/// The Cabin standard library. This is a Cabin file that's automatically imported into every Cabin
/// project or file. It contains definitions for all of the built-in types and objects, such as
/// `Text`, `Number`, `terminal`, etc. See `/std/stdlib.cabin` for its contents.
pub const STDLIB: &str = include_str!("../std/stdlib.cabin");

/// The Cabin prelude. This is a Cabin file that's automatically prepended to all Cabin files
/// written by the user. It just brings some useful items into scope from the standard library.
/// See `/std/prelude.cabin` for its contents.
pub const PRELUDE: &str = include_str!("../std/prelude.cabin");
