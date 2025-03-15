use std::{collections::VecDeque, path::Path};

use api::{
	context::StandardContext,
	io::{IoReader, IoWriter},
};

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

pub mod ast;
pub mod comptime;
pub mod interpreter;
pub mod lexer;
pub mod parser;
pub mod transpiler;
pub mod typechecker;

pub(crate) mod api;

/// The Cabin standard library. This is a Cabin file that's automatically imported into every Cabin
/// project or file. It contains definitions for all of the built-in types and objects, such as
/// `Text`, `Number`, `terminal`, etc. See `/std/stdlib.cabin` for its contents.
pub const STDLIB: &str = include_str!("../std/stdlib.cabin");

pub fn parse_module<Input: IoReader, Output: IoWriter, Error: IoWriter>(code: &str, context: &mut Context<Input, Output, Error>) -> (Module, Diagnostics) {
	let mut tokens = lexer::tokenize(code, context);
	(Module::parse(&mut tokens, context), context.diagnostics().to_owned())
}

pub fn parse_library_file<P: AsRef<Path>, Input: IoReader, Output: IoWriter, Error: IoWriter>(
	path: P,
	context: &mut Context<Input, Output, Error>,
) -> Result<Module, std::io::Error> {
	let code = std::fs::read_to_string(path.as_ref().join("src/library.cabin"))?;
	Ok(parse_library(&code, context))
}

pub fn parse_library<Input: IoReader, Output: IoWriter, Error: IoWriter>(code: &str, context: &mut Context<Input, Output, Error>) -> Module {
	let mut tokens = lexer::tokenize(code, context);
	Module::parse(&mut tokens, context)
}

pub fn parse_program<Input: IoReader, Output: IoWriter, Error: IoWriter>(code: &str, context: &mut Context<Input, Output, Error>) -> Program {
	let mut tokens = lexer::tokenize(code, context);
	Program::parse(&mut tokens, context)
}

pub fn tokenize(code: &str) -> (VecDeque<Token>, Diagnostics) {
	let mut context = Context::default();
	(lexer::tokenize(code, &mut context), context.diagnostics().to_owned())
}

pub fn interpret(code: &str, context: &mut StandardContext) {
	let mut tokens = lexer::tokenize(code, context);
	let program = Program::parse(&mut tokens, context);
	let evaluated = program.evaluate_at_compile_time(context);
	let _ = evaluated.evaluate_at_runtime(context);
}
