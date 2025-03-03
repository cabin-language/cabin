use std::{collections::VecDeque, path::Path};

// Re-exports
pub use crate::api::{context::Context, diagnostics::Error, project::Project, span::*, *};
use crate::{
	api::diagnostics::Diagnostics,
	ast::misc::{module::Module, program::Program},
	lexer::Token,
	parser::Parse as _,
};

pub mod ast;
pub mod compiler;
pub mod comptime;
pub mod lexer;
pub mod parser;
pub mod transpiler;
pub mod typechecker;

pub(crate) mod api;

/// The Cabin standard library. This is a Cabin file that's automatically imported into every Cabin
/// project or file. It contains definitions for all of the built-in types and objects, such as
/// `Text`, `Number`, `terminal`, etc. See `/std/stdlib.cabin` for its contents.
pub const STDLIB: &str = include_str!("../std/stdlib.cabin");

pub fn parse_module(code: &str, context: &mut Context) -> (Module, Diagnostics) {
	let mut tokens = lexer::tokenize(code, context);
	(Module::parse(&mut tokens, context), context.diagnostics().to_owned())
}

pub fn parse_library_file<P: AsRef<Path>>(path: P, context: &mut Context) -> Result<Module, std::io::Error> {
	let code = std::fs::read_to_string(path.as_ref().join("src/library.cabin"))?;
	Ok(parse_library(&code, context))
}

pub fn parse_library(code: &str, context: &mut Context) -> Module {
	let mut tokens = lexer::tokenize(code, context);
	let module = Module::parse(&mut tokens, context);
	module
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
