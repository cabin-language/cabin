use std::{collections::VecDeque, fmt::Display};

use api::context::context;
use comptime::{CompileTime, CompileTimeError};
use convert_case::{Case, Casing as _};
use lexer::{Span, Token, TokenizeError};
use parser::{expressions::Spanned, Module, ParseError};

pub mod api;
pub mod cli;
pub mod compiler;
pub mod comptime;
pub mod lexer;
pub mod parser;
pub mod transpiler;

#[derive(Clone, Debug, thiserror::Error)]
pub enum Warning {
	#[error("{type_name} names should be in PascalCase: Change \"{original_name}\" to \"{}\"", .original_name.to_case(Case::Pascal))]
	NonPascalCaseGroup { type_name: String, original_name: String },

	#[error("Variable names should be in snake_case: Change \"{original_name}\" to \"{}\"", .original_name.to_case(Case::Snake))]
	NonSnakeCaseName { original_name: String },

	#[error("This either has no variants, meaning it can never be instantiated")]
	EmptyEither,
}

#[derive(Clone, thiserror::Error, Debug)]
pub enum Error {
	#[error("Tokenization error: {0}")]
	Tokenize(TokenizeError),

	#[error("Parse error: {0}")]
	Parse(ParseError),

	#[error("Evaluation error: {0}")]
	CompileTime(CompileTimeError),
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum DiagnosticInfo {
	#[error("{0}")]
	Error(Error),

	#[error("Warning: {0}")]
	Warning(Warning),
}

#[derive(Clone, Debug, thiserror::Error)]
pub struct Diagnostic {
	span: Span,
	error: DiagnosticInfo,
}

impl Diagnostic {
	pub fn info(&self) -> &DiagnosticInfo {
		&self.error
	}
}

impl Spanned for Diagnostic {
	fn span(&self) -> Span {
		self.span
	}
}

#[derive(Clone, Debug, thiserror::Error)]
pub struct Diagnostics(Vec<Diagnostic>);

impl Display for Diagnostics {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"Multiple errors encountered:\n\n{}",
			self.0.iter().map(|error| format!("{error}")).collect::<Vec<_>>().join("\n")
		)
	}
}

impl Diagnostics {
	pub fn none() -> Self {
		Self(Vec::new())
	}

	pub fn new(errors: Vec<Diagnostic>) -> Self {
		Self(errors)
	}

	pub fn push(&mut self, error: Diagnostic) {
		self.0.push(error);
	}

	pub fn is_empty(&self) -> bool {
		self.0.is_empty()
	}
}

impl Iterator for Diagnostics {
	type Item = Diagnostic;

	fn next(&mut self) -> Option<Self::Item> {
		self.0.pop()
	}
}

impl Display for Diagnostic {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.error.fmt(f)
	}
}

/// Checks a Cabin program for errors.
///
/// # Returns
///
/// The errors that occurred. If no errors occurred, then `errors.is_empty() == true`.
pub fn check(code: &str) -> Diagnostics {
	context().reset();
	let mut tokens = lexer::tokenize_without_prelude(code);
	let program = parser::parse(&mut tokens);
	let _ = program.evaluate_at_compile_time();
	context().diagnostics().to_owned()
}

pub fn parse(code: &str) -> (Module, Diagnostics) {
	context().reset();
	let mut tokens = lexer::tokenize_without_prelude(code);
	(parser::parse(&mut tokens), context().diagnostics().to_owned())
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
