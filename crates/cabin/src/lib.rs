use std::collections::VecDeque;

use api::context::{context, Context, CONTEXT};
use lexer::{Span, Token, TokenizeError};
use parser::{expressions::Spanned, Module, ParseError};

pub mod api;
pub mod cli;
pub mod compiler;
pub mod comptime;
pub mod lexer;
pub mod parser;
pub mod transpiler;

#[derive(thiserror::Error, Debug)]
pub enum ErrorInfo {
	#[error("Tokenization error: {0}")]
	Tokenize(TokenizeError),

	#[error("Parse error: {0}")]
	Parse(ParseError),
}

#[derive(thiserror::Error, Debug)]
pub struct Error {
	span: Span,
	error: ErrorInfo,
}

impl Spanned for Error {
	fn span(&self) -> Span {
		self.span
	}
}

pub struct Errors(Vec<Error>);

macro_rules! errors {
	($tokens: tt) => {
		$crate::Errors::new(vec![tt])
	};
}

impl Errors {
	pub fn new(errors: Vec<Error>) -> Self {
		Self(errors)
	}

	pub fn one(span: Span, error: ErrorInfo) -> Self {
		Errors(vec![Error { span, error }])
	}
}

impl std::fmt::Display for Error {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.error.fmt(f)
	}
}

pub fn parse(code: &str) -> Result<Module, Error> {
	context().reset();
	let mut tokens = lexer::tokenize_without_prelude(code)?;
	parser::parse(&mut tokens)
}

pub fn tokenize(code: &str) -> Result<VecDeque<Token>, Error> {
	context().reset();
	lexer::tokenize_without_prelude(code)
}

/// The Cabin standard library. This is a Cabin file that's automatically imported into every Cabin
/// project or file. It contains definitions for all of the built-in types and objects, such as
/// `Text`, `Number`, `terminal`, etc. See `/std/stdlib.cabin` for its contents.
pub const STDLIB: &str = include_str!("../std/stdlib.cabin");

/// The Cabin prelude. This is a Cabin file that's automatically prepended to all Cabin files
/// written by the user. It just brings some useful items into scope from the standard library.
/// See `/std/prelude.cabin` for its contents.
pub const PRELUDE: &str = include_str!("../std/prelude.cabin");
