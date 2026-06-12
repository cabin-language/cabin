use std::{fmt::Display, path::PathBuf};

use convert_case::{Case, Casing as _};
use indexmap::IndexSet;

use crate::{Context, STDLIB, Span, Spanned, ast::statements::Statement, lexer::TokenType, typechecker::Type};

#[derive(Clone, Debug, Hash, PartialEq, Eq, Copy)]
pub enum Severity {
	ProdError,
	ProdWarning,
	ProdInfo,
	ProdHint,

	AlwaysError,
	AlwaysWarn,
	AlwaysInfo,
	AlwaysHint,
}

impl Severity {
	pub fn is_error(self) -> bool {
		self == Severity::ProdError || self == Severity::AlwaysError
	}

	pub fn is_warning(self) -> bool {
		self == Severity::ProdWarning || self == Severity::AlwaysWarn
	}
}

#[derive(Clone, Debug, thiserror::Error, Hash, PartialEq, Eq)]
pub enum DiagnosticInfo {
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

	#[error("Bad compile-time call: This action should only be called at runtime. Reason: {reason}")]
	CallRuntimeAtCompileTime { reason: String },

	/// The warning that occurs when an empty `either` is created. Empty `either`s can never be
	/// instantited, so there's no reason to create one.
	#[error("Empty either: This either has no variants, meaning it can never be instantiated")]
	EmptyEither,

	/// The warning that occurs when an empty extension is created. Empty extensions do nothing, so
	/// there's no reason to create one.
	#[error("Empty extension: This extension is empty, so it does nothing")]
	EmptyExtension,

	#[error("Unrecognized token: {0}")]
	UnrecognizedToken(String),

	#[error("Unresolvable Type: This expression can't be fully evaluated at compile-time; Default values for group fields must be known at compile-time.")]
	GroupValueNotKnownAtCompileTime,

	#[error("Invalid run expression: run has no effect on this type of expression")]
	RunNonFunctionCall,

	#[error("Iterate over non-iterable: This type of value can't be iterated over")]
	IterateOverNonList,

	#[error("Call non-callable: This type of value can't be called")]
	CallNonFunction,

	#[error("Unknown variable: \"{0}\"")]
	UnknownVariable(String),

	#[error("Unresolvable Type: This expression is being used as a type, but it can't be fully evaluated at compile-time")]
	ExpressionUsedAsType,

	#[error("Missing tail statement: This block is being assigned to a value, but it's missing a tail statement")]
	MissingTailStatement,

	#[error("Unreachable code: This code will never be executed")]
	UnreachableCode,

	#[error("Unknown property: No property \"{0}\" exists on this value")]
	NoSuchField(String),

	#[error("Type mismatch: This value cannot be assigned to this type")]
	TypeMismatch(Type, Type),

	#[error("Missing property: Missing property \"{0}\"")]
	MissingField(String),

	#[error("Unknown property: This type has no property called \"{0}\"")]
	ExtraField(String),

	#[error("Invalid extension target: Attempted to extend a type to be a non-group")]
	ExtendToBeNonGroup,

	#[error("Duplicate property: The property \"{0}\" appears multiple times in this group.")]
	DuplicateGroupField(String),

	#[error("Unexpected token: Expected {expected} but found {actual}")]
	UnexpectedTokenExpected { expected: &'static str, actual: TokenType },

	#[error("Invalid top-level statement: Only declarations can appear at the top level of a module.")]
	InvalidTopLevelStatement { statement: Statement },

	#[error("Invalid format string: The string \"{0}\" is not properly formatted.")]
	InvalidFormatString(String),

	#[error("Unexpected token: Expected {expected} but found {actual}")]
	UnexpectedToken { expected: TokenType, actual: TokenType },

	#[error("Unexpected end of file: Expected {expected} but found end of file")]
	UnexpectedEOF { expected: TokenType },

	#[error("Unexpected end of file: Expected more tokens")]
	UnexpectedGenericEOF,

	#[error("Duplicate variable: The variable \"{name}\" was declared twice")]
	DuplicateVariableDeclaration { name: String },

	#[error("{0}")]
	Info(String),
}

impl DiagnosticInfo {
	pub const fn severity(&self) -> Severity {
		match self {
			Self::NonPascalCaseGroup { .. } | Self::NonSnakeCaseName { .. } | Self::CallRuntimeAtCompileTime { .. } | Self::EmptyEither | Self::EmptyExtension => {
				Severity::AlwaysWarn
			},

			Self::UnrecognizedToken(_)
			| Self::ExtendToBeNonGroup
			| Self::ExtraField(_)
			| Self::MissingField(_)
			| Self::TypeMismatch(..)
			| Self::NoSuchField(_)
			| Self::UnreachableCode
			| Self::MissingTailStatement
			| Self::ExpressionUsedAsType
			| Self::UnknownVariable(_)
			| Self::CallNonFunction
			| Self::IterateOverNonList
			| Self::GroupValueNotKnownAtCompileTime
			| Self::RunNonFunctionCall
			| Self::UnexpectedTokenExpected { .. }
			| Self::DuplicateGroupField(_)
			| Self::InvalidTopLevelStatement { .. }
			| Self::InvalidFormatString(_)
			| Self::DuplicateVariableDeclaration { .. }
			| Self::UnexpectedEOF { .. }
			| Self::UnexpectedToken { .. }
			| Self::UnexpectedGenericEOF => Severity::AlwaysError,

			Self::Info(_) => Severity::AlwaysInfo,
		}
	}
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
	/// Creates an empty diagnostic set.
	pub fn empty() -> Self {
		Self(IndexSet::new())
	}

	/// Returns the diagnostics that are warnings, as well as their spans.
	pub fn warnings(&self) -> Vec<(&DiagnosticInfo, Span)> {
		self.0
			.iter()
			.filter_map(|diagnostic| {
				(diagnostic.info.severity() == Severity::AlwaysWarn || diagnostic.info.severity() == Severity::ProdWarning).then_some((&diagnostic.info, diagnostic.span))
			})
			.collect()
	}

	/// Returns the diagnostics that are errors, as well as their spans.
	pub fn errors(&self) -> Vec<(&DiagnosticInfo, Span)> {
		self.0
			.iter()
			.filter_map(|diagnostic| {
				(diagnostic.info.severity() == Severity::AlwaysError || diagnostic.info.severity() == Severity::ProdError).then_some((&diagnostic.info, diagnostic.span))
			})
			.collect()
	}

	pub fn dev_only(&self) -> Vec<&Diagnostic> {
		self.0
			.iter()
			.filter(|diagnostic| {
				diagnostic.info.severity() == Severity::AlwaysError || diagnostic.info.severity() == Severity::AlwaysInfo || diagnostic.info.severity() == Severity::AlwaysWarn
			})
			.collect()
	}

	pub fn all(&self) -> Vec<&Diagnostic> {
		self.0.iter().collect()
	}

	pub fn dev_errors(&self) -> Vec<(&DiagnosticInfo, Span)> {
		self.0
			.iter()
			.filter_map(|diagnostic| (diagnostic.info.severity() == Severity::AlwaysError).then_some((&diagnostic.info, diagnostic.span)))
			.collect()
	}

	pub fn dev_warnings(&self) -> Vec<(&DiagnosticInfo, Span)> {
		self.0
			.iter()
			.filter_map(|diagnostic| (diagnostic.info.severity() == Severity::AlwaysWarn).then_some((&diagnostic.info, diagnostic.span)))
			.collect()
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
