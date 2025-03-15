use std::{fmt::Display, path::PathBuf};

use convert_case::{Case, Casing as _};
use indexmap::IndexSet;

use super::io::{IoReader, IoWriter};
use crate::{comptime::CompileTimeError, lexer::TokenizeError, parser::ParseError, Context, Span, Spanned, STDLIB};

#[derive(Clone, Debug, thiserror::Error, Hash, PartialEq, Eq)]
pub enum Warning {
	/// The warning that occurs when a name that should be in `PascalCase` isn't in `PascalCase`.
	#[error("{type_name} names should be in PascalCase: Change \"{original_name}\" to \"{}\"", .original_name.to_case(Case::Pascal))]
	NonPascalCaseGroup {
		/// The type of the binding that should be in `PascalCase`, such as "group", "extension",
		/// or "either". See `Literal::kind_name()` for more information.
		type_name: String,

		/// The original name used that's not in `PascalCase`.
		original_name: String,
	},

	/// The warning that occurs when a name that should be in `snake_case` isn't in `snake_case`.
	#[error("Variable names should be in snake_case: Change \"{original_name}\" to \"{}\"", .original_name.to_case(Case::Snake))]
	NonSnakeCaseName {
		/// The original name used that's not in `snake_case`.
		original_name: String,
	},

	/// The warning that occurs when an empty `either` is created. Empty `either`s can never be
	/// instantited, so there's no reason to create one.
	#[error("Empty either: This either has no variants, meaning it can never be instantiated")]
	EmptyEither,

	/// The warning that occurs when an empty extension is created. Empty extensions do nothing, so
	/// there's no reason to create one.
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

/// Information about a diagnostic. This holds the specific severity of the diagnostic, the type of
/// the diagnostic, and any specific data associated with diagnostics of that type.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq, Hash)]
pub enum DiagnosticInfo {
	/// An error diagnostic. These are fatal diagnostics that prevent compilation and indicate that
	/// the developer has written code that is fundamentally incorrect.
	#[error("{0}")]
	Error(Error),

	/// A warning diagnostic. These are non-fatal diagnostics that suggest that the developer is
	/// doing something that's probably wrong or inefficient.
	#[error("{0}")]
	Warning(Warning),

	/// An info diagnostic. These are non-fatal diagnostics that just provide general information to
	/// the user.
	#[error("{0}")]
	Info(String),
}

/// A compiler diagnostic. Diagnostics are information about source code, such as errors, warnings,
/// information, hints, etc. For the primary diagnostic information, see `diagnostic.info`.
#[derive(Clone, Debug, thiserror::Error, PartialEq, Eq, Hash)]
pub struct Diagnostic {
	/// The span of the diagnostic.
	pub span: Span,

	/// The information of the diagnostic. This includes the severity, the type, and any data
	/// associated with that specific diagnostic type.
	pub info: DiagnosticInfo,

	/// The file that the diagnostic occurred in.
	pub file: PathBuf,
}

impl Diagnostic {
	pub const fn info(&self) -> &DiagnosticInfo {
		&self.info
	}

	pub fn start_line_column(&self) -> (usize, usize) {
		self.span
			.start_line_column(&std::fs::read_to_string(&self.file).unwrap_or_else(|_| STDLIB.to_owned()))
			.unwrap()
	}
}

impl Spanned for Diagnostic {
	fn span<Input: IoReader, Output: IoWriter, Error: IoWriter>(&self, _context: &Context<Input, Output, Error>) -> Span {
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
	/// Creates an empty diagnostic set.
	pub fn empty() -> Self {
		Self(IndexSet::new())
	}

	/// Returns the diagnostics that are warnings, as well as their spans.
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

	/// Returns the diagnostics that are errors, as well as their spans.
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

	/// Removes and returns owned errors from this diagnostic set without cloning or copying.
	///
	/// # Tracking
	///
	/// This feature is not yet implementedl; Waiting on <https://github.com/indexmap-rs/indexmap/issues/242>.
	pub fn take_errors(&self) -> ! {
		// self.0
		// 	.extract_if(|(diagnostic, span)| matches!(diagnostic.info, DiagnosticInfo::Error(_)))
		// 	.map(|(diagnostic, span)| {
		// 		let DiagnosticInfo::Error(error) = diagnostic.info else { unreachable!() };
		// 		(error, span)
		// 	})
		// 	.collect()
		unimplemented!()
	}

	/// Adds a new diagnostic to this diagnostic set. If there is a completely identical diagnostic
	/// already present, it will be replaced with the new diagnostic.
	///
	/// # Parameters
	///
	/// - `diagnostic` - The diagnostic to add
	pub fn push(&mut self, diagnostic: Diagnostic) {
		let _ = self.0.insert(diagnostic);
	}

	/// Returns whether this diagnostic set has no diagnostics in it.
	///
	/// # Returns
	///
	/// Whether there are zero diagnostics in this diagnostic set.
	pub fn is_empty(&self) -> bool {
		self.0.is_empty()
	}

	pub fn clear(&mut self) {
		self.0.clear();
	}
}

impl IntoIterator for Diagnostics {
	type IntoIter = <IndexSet<Diagnostic> as IntoIterator>::IntoIter;
	type Item = Diagnostic;

	fn into_iter(self) -> Self::IntoIter {
		self.0.into_iter()
	}
}

/// An iterator over diagnostics returned from a call to `Diagnostics::iter()`.
pub struct DiagnosticsIterator<'diagnostics> {
	diagnostics: &'diagnostics Diagnostics,
	index: usize,
}

impl<'diagnostics> Iterator for DiagnosticsIterator<'diagnostics> {
	type Item = &'diagnostics Diagnostic;

	fn next(&mut self) -> Option<Self::Item> {
		self.index += 1;
		self.diagnostics.0.get_index(self.index - 1)
	}
}

impl<'diagnostics> IntoIterator for &'diagnostics Diagnostics {
	type IntoIter = DiagnosticsIterator<'diagnostics>;
	type Item = &'diagnostics Diagnostic;

	fn into_iter(self) -> Self::IntoIter {
		DiagnosticsIterator { diagnostics: self, index: 0 }
	}
}

impl Display for Diagnostic {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.info.fmt(f)
	}
}
