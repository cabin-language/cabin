use crate::{api::context::Context, diagnostics::DiagnosticInfo, io::Io, typechecker::Type};

pub mod memory;

/// A trait for AST nodes to implement that allows them to be evaluated at compile-time. After
/// parsing, the program's abstract syntax tree is evaluated at compile-time as much as possible.
pub trait CompileTime {
	type Output;

	/// Evaluates this AST node at compile-time, as much as possible. For example, for if-expressions, this
	/// will evaluate the condition, and if the condition is fully evaluable at compile-time and resolves to
	/// `true`, it will run the `if` body.
	///
	/// # Errors
	///
	/// An error can occur during compile-time evaluation for any number of reasons, such as the user writing a
	/// variable name that doesn't exist. The specific error returned by this is implementation-specific.
	fn evaluate_at_compile_time<System: Io>(self, context: &mut Context<System>) -> Self::Output;
}

#[derive(thiserror::Error, Debug, Clone, Hash, PartialEq, Eq)]
pub enum CompileTimeError {
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
}

impl From<CompileTimeError> for DiagnosticInfo {
	fn from(value: CompileTimeError) -> Self {
		DiagnosticInfo::Error(crate::Error::CompileTime(value))
	}
}
