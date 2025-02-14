use std::fmt::Display;

use convert_case::{Case, Casing as _};

use crate::{comptime::CompileTimeError, lexer::TokenizeError, parser::ParseError, Context, Span, Spanned};

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
	#[error("{0}")]
	Tokenize(TokenizeError),

	#[error("{0}")]
	Parse(ParseError),

	#[error("{0}")]
	CompileTime(CompileTimeError),
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum DiagnosticInfo {
	#[error("{0}")]
	Error(Error),

	#[error("{0}")]
	Warning(Warning),
}

#[derive(Clone, Debug, thiserror::Error)]
pub struct Diagnostic {
	pub span: Span,
	pub info: DiagnosticInfo,
}

impl Diagnostic {
	pub fn info(&self) -> &DiagnosticInfo {
		&self.info
	}
}

impl Spanned for Diagnostic {
	fn span(&self, _context: &Context) -> Span {
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
	pub fn empty() -> Self {
		Self(Vec::new())
	}

	pub fn warnings(&self) -> Vec<(&Warning, Span)> {
		self.0
			.iter()
			.filter_map(|diagnostic| {
				if let DiagnosticInfo::Warning(warning) = &diagnostic.info {
					Some((warning, diagnostic.span))
				} else {
					None
				}
			})
			.collect()
	}

	pub fn errors(&self) -> Vec<(&Error, Span)> {
		self.0
			.iter()
			.filter_map(|diagnostic| {
				if let DiagnosticInfo::Error(error) = &diagnostic.info {
					Some((error, diagnostic.span))
				} else {
					None
				}
			})
			.collect()
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
		self.info.fmt(f)
	}
}
