use std::{collections::VecDeque, path::Path};

use crate::{
	api::diagnostics::Diagnostics,
	ast::{
		expressions::Expression,
		misc::{module::Module, program::Program},
	},
	comptime::{memory::VirtualPointer, CompileTime},
	lexer::Token,
	parser::Parse as _,
};

pub(crate) mod api;
pub mod ast;
pub mod cli;
pub mod compiler;
pub mod comptime;
pub mod lexer;
pub mod parser;
pub mod transpiler;

// Re-exports
pub use crate::{
	api::{context::Context, diagnostics::Error, project::Project, *},
	ast::expressions::Spanned,
	lexer::Span,
};

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

pub fn parse_library_file<P: AsRef<Path>>(path: P, context: &mut Context) -> Result<Module, std::io::Error> {
	let code = std::fs::read_to_string(path.as_ref().join("src/library.cabin"))?;

	let mut tokens = lexer::tokenize(&code, context);
	let module = Module::parse(&mut tokens, context);

	Ok(module)
}

pub fn parse_library(code: &str, context: &mut Context) -> Module {
	let mut tokens = lexer::tokenize(code, context);
	let module = Module::parse(&mut tokens, context);
	module
}

/// Performs compile-time evaluation on the program. The returned program may contain
/// errors, check `context.diagnostics().errors()` to check if any errors occurred
/// during or before compile-time evaluating the program.
///
/// Note that this isn't entirely pure; It may have system side effects, such as the
/// user printing information at compile-time. This may pose security risks if running
/// unsanitized Cabin code.
///
/// # Parameters
///
/// - `code` - A string of Cabin source code
/// - `context` - Global data about the program, such as scope data. if this function is being
/// called with no prior code having been parsed or evaluated in the same context, use
/// `Context::default()`.
///
/// # Returns
///
/// The program after compile-time evaluation, as well as the context used to evaluate
/// the program. The context contains global data about the program, such as diagnostics
/// for errors or warnings that occurred, as well as data about the program's scopes,
/// etc.
pub fn compile_time_evaluate_program(code: &str, context: &mut Context) -> Program {
	let stdlib = parse_module(STDLIB, context).0.to_pointer(context);
	context.scope_tree.declare_new_variable("builtin", Expression::Pointer(stdlib)).unwrap();

	let mut tokens = lexer::tokenize(code, context);
	let program = Program::parse(&mut tokens, context);
	let evaluated_program = program.evaluate_at_compile_time(context);

	evaluated_program
}

pub fn evaluate_library(context: &mut Context, name: &str) -> VirtualPointer {
	let library = parse_module(STDLIB, context).0.to_pointer(context);
	context.scope_tree.declare_new_variable(name, Expression::Pointer(library)).unwrap();
	library
}

pub fn parse_program(code: &str, context: &mut Context) -> Program {
	let mut tokens = lexer::tokenize(code, context);
	let program = Program::parse(&mut tokens, context);
	program
}

pub fn tokenize(code: &str) -> (VecDeque<Token>, Diagnostics) {
	let mut context = Context::default();
	(lexer::tokenize(code, &mut context), context.diagnostics().to_owned())
}

/// The Cabin standard library. This is a Cabin file that's automatically imported into every Cabin
/// project or file. It contains definitions for all of the built-in types and objects, such as
/// `Text`, `Number`, `terminal`, etc. See `/std/stdlib.cabin` for its contents.
pub const STDLIB: &str = include_str!("../std/stdlib.cabin");

/// The Cabin prelude. This is a Cabin file that's automatically prepended to all Cabin files
/// written by the user. It just brings some useful items into scope from the standard library.
/// See `/std/prelude.cabin` for its contents.
pub const PRELUDE: &str = include_str!("../std/prelude.cabin");
