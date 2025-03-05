use std::{fmt::Display, path::PathBuf};

use convert_case::{Case, Casing as _};
use indexmap::IndexSet;

use crate::{comptime::CompileTimeError, lexer::TokenizeError, parser::ParseError, Context, Span, Spanned, STDLIB};

#[derive(Clone, Debug, thiserror::Error, Hash, PartialEq, Eq)]
pub enum Warning {
	#[error("{type_name} names should be in PascalCase: Change \"{original_name}\" to \"{}\"", .original_name.to_case(Case::Pascal))]
	NonPascalCaseGroup { type_name: String, original_name: String },

	#[error("Variable names should be in snake_case: Change \"{original_name}\" to \"{}\"", .original_name.to_case(Case::Snake))]
	NonSnakeCaseName { original_name: String },

	#[error("Empty either: This either has no variants, meaning it can never be instantiated")]
	EmptyEither,

	#[error("Empty extension: This extension is empty, so it does nothing")]
	EmptyExtension,
}

impl From<Warning> for DiagnosticInfo {
	fn from(value: Warning) -> Self {
		DiagnosticInfo::Warning(value)
	}
}

#[derive(Clone, thiserror::Error, Debug, Hash, PartialEq, Eq)]
pub enum Error {
	#[error("{0}")]
	Tokenize(TokenizeError),

	#[error("{0}")]
	Parse(ParseError),

	#[error("{0}")]
	CompileTime(CompileTimeError),
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq, Hash)]
pub enum DiagnosticInfo {
	#[error("{0}")]
	Error(Error),

	#[error("{0}")]
	Warning(Warning),

	#[error("{0}")]
	Info(String),
}

#[derive(Clone, Debug, thiserror::Error, PartialEq, Eq, Hash)]
pub struct Diagnostic {
	pub span: Span,
	pub info: DiagnosticInfo,
	pub file: PathBuf,
}

impl Diagnostic {
	pub fn info(&self) -> &DiagnosticInfo {
		&self.info
	}

	pub fn start_line_column(&self) -> (usize, usize) {
		self.span.start_line_column(&std::fs::read_to_string(&self.file).unwrap_or(STDLIB.to_owned())).unwrap()
	}
}

impl Spanned for Diagnostic {
	fn span(&self, _context: &Context) -> Span {
		self.span
	}
}

#[derive(Clone, Debug, thiserror::Error)]
pub struct Diagnostics(IndexSet<Diagnostic>);

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
		Self(IndexSet::new())
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

	pub fn take_errors(&self) -> Vec<(Error, Span)> {
		// Vec::<(Diagnostic, Span)>::new()
		// 	.extract_if(|(diagnostic, span)| matches!(diagnostic.info, DiagnosticInfo::Error(_)))
		// 	.map(|(diagnostic, span)| {
		// 		let DiagnosticInfo::Error(error) = diagnostic.info else { unreachable!() };
		// 		(error, span)
		// 	})
		// 	.collect()
		unimplemented!()
	}

	pub fn push(&mut self, error: Diagnostic) {
		let _ = self.0.insert(error);
	}

	pub fn is_empty(&self) -> bool {
		self.0.is_empty()
	}
}

impl IntoIterator for Diagnostics {
	type IntoIter = <IndexSet<Diagnostic> as IntoIterator>::IntoIter;
	type Item = Diagnostic;

	fn into_iter(self) -> Self::IntoIter {
		self.0.into_iter()
	}
}

pub struct DiagnosticsIterator<'a> {
	diagnostics: &'a Diagnostics,
	index: usize,
}

impl<'a> Iterator for DiagnosticsIterator<'a> {
	type Item = &'a Diagnostic;

	fn next(&mut self) -> Option<Self::Item> {
		self.index += 1;
		self.diagnostics.0.get_index(self.index - 1)
	}
}

impl<'a> IntoIterator for &'a Diagnostics {
	type IntoIter = DiagnosticsIterator<'a>;
	type Item = &'a Diagnostic;

	fn into_iter(self) -> Self::IntoIter {
		DiagnosticsIterator { diagnostics: self, index: 0 }
	}
}

impl Display for Diagnostic {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.info.fmt(f)
	}
}
