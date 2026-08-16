use crate::{api::context::Context, comptime::storage::Pointer};

pub mod storage;
pub mod types;

/// A trait for AST nodes to implement that allows them to be evaluated at compile-time. After
/// parsing, the program's abstract syntax tree is evaluated at compile-time as much as possible.
pub trait Evaluate {
	type Output;

	/// Evaluates this AST node at compile-time, as much as possible. For example, for if-expressions, this
	/// will evaluate the condition, and if the condition is fully evaluable at compile-time and resolves to
	/// `true`, it will run the `if` body.
	///
	/// # Errors
	///
	/// An error can occur during compile-time evaluation for any number of reasons, such as the user writing a
	/// variable name that doesn't exist. The specific error returned by this is implementation-specific.
	fn evaluate(self, context: &mut Context) -> Self::Output;
}

impl<T: Evaluate> Evaluate for Option<T> {
	type Output = Option<T::Output>;

	fn evaluate(self, context: &mut Context) -> Self::Output {
		self.map(|t| t.evaluate(context))
	}
}

impl<T: Evaluate> Evaluate for Vec<T> {
	type Output = Vec<T::Output>;

	fn evaluate(self, context: &mut Context) -> Self::Output {
		self.into_iter().map(|t| t.evaluate(context)).collect()
	}
}

pub trait Check
where
	Self: ToOwned,
	Self::Owned: Evaluate,
{
	/// Runs full static analysis on this expression without mutating it or the current program state.
	///
	/// This is done by saving a snapshot of the current program state, fully consuming and evaluating
	/// a clone of the block at compile-time, and then restoring the saved program state from the
	/// snapshot.
	fn check(&self, context: &mut Context)
	where
		Self::Owned: Evaluate,
	{
		let snapshot = context.snapshot();
		let _ = self.to_owned().evaluate(context);
		context.roll_back(snapshot);
	}
}

impl<T> Check for T
where
	T: ToOwned,
	T::Owned: Evaluate,
{
}

pub trait Dot {
	fn dot(&self, context: &mut Context, name: &str) -> Option<Pointer>;
}
