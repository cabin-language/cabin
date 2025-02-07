use std::{collections::VecDeque, fmt::Display};

use api::context::context;
use comptime::CompileTime;
use lexer::{Span, Token, TokenizeError};
use parser::{expressions::Spanned, Module, ParseError};

pub mod api;
pub mod cli;
pub mod compiler;
pub mod comptime;
pub mod lexer;
pub mod parser;
pub mod transpiler;

#[derive(Clone, thiserror::Error, Debug)]
pub enum ErrorInfo {
	#[error("Tokenization error: {0}")]
	Tokenize(TokenizeError),

	#[error("Parse error: {0}")]
	Parse(ParseError),
}

#[derive(Clone, thiserror::Error, Debug)]
pub struct Error {
	span: Span,
	error: ErrorInfo,
}

impl Spanned for Error {
	fn span(&self) -> Span {
		self.span
	}
}

#[derive(Clone, Debug, thiserror::Error)]
pub struct Errors(Vec<Error>);

impl Display for Errors {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"Multiple errors encountered:\n\n{}",
			self.0.iter().map(|error| format!("{error}")).collect::<Vec<_>>().join("\n")
		)
	}
}

impl Errors {
	pub fn none() -> Self {
		Self(Vec::new())
	}

	pub fn new(errors: Vec<Error>) -> Self {
		Self(errors)
	}

	pub fn push(&mut self, error: Error) {
		self.0.push(error);
	}

	pub fn one(span: Span, error: ErrorInfo) -> Self {
		Errors(vec![Error { span, error }])
	}

	pub fn is_empty(&self) -> bool {
		self.0.is_empty()
	}
}

impl Iterator for Errors {
	type Item = Error;

	fn next(&mut self) -> Option<Self::Item> {
		self.0.pop()
	}
}

impl Display for Error {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.error.fmt(f)
	}
}

/// Checks a Cabin program for errors.
///
/// # Returns
///
/// The errors that occurred. If no errors occurred, then `errors.is_empty() == true`.
pub fn check(code: &str) -> Errors {
	context().reset();
	let mut tokens = lexer::tokenize_without_prelude(code);
	let program = parser::parse(&mut tokens).unwrap();
	let _module = program.evaluate_at_compile_time().unwrap();
	context().errors().to_owned()
}

pub fn parse(code: &str) -> (Result<Module, Error>, Errors) {
	context().reset();
	let mut tokens = lexer::tokenize_without_prelude(code);
	(parser::parse(&mut tokens), context().errors().to_owned())
}

pub fn tokenize(code: &str) -> (VecDeque<Token>, Errors) {
	context().reset();
	(lexer::tokenize_without_prelude(code), context().errors().to_owned())
}

/// The Cabin standard library. This is a Cabin file that's automatically imported into every Cabin
/// project or file. It contains definitions for all of the built-in types and objects, such as
/// `Text`, `Number`, `terminal`, etc. See `/std/stdlib.cabin` for its contents.
pub const STDLIB: &str = include_str!("../std/stdlib.cabin");

/// The Cabin prelude. This is a Cabin file that's automatically prepended to all Cabin files
/// written by the user. It just brings some useful items into scope from the standard library.
/// See `/std/prelude.cabin` for its contents.
pub const PRELUDE: &str = include_str!("../std/prelude.cabin");
