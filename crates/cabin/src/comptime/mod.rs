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
	fn evaluate_at_compile_time(self) -> Self::Output;
}

#[derive(thiserror::Error, Debug, Clone)]
pub enum CompileTimeError {
	#[error("This expression can't be fully evaluated at compile-time; Default values for group fields must be known at compile-time.")]
	GroupValueNotKnownAtCompileTime,

	#[error("run has no effect on this type of expression")]
	RunNonFunctionCall,

	#[error("This type of value can't be iterated over")]
	IterateOverNonList,

	#[error("This type of value can't be called")]
	CallNonFunction,
}
