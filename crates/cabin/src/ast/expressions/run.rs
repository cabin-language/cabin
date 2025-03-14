use crate::{
	api::context::Context,
	ast::expressions::Spanned,
	comptime::{memory::ExpressionPointer, CompileTime},
	interpreter::Runtime,
	io::{IoReader, IoWriter},
	transpiler::{TranspileError, TranspileToC},
	Span,
};

/// A `Run` expression in the language. Run-expressions forcibly run an expression at runtime instead of compile-time. Since
/// Cabin runs all code at compile-time by default, this is the only way to forcibly run an expression at runtime.
///
/// Note that an expressions sub-expressions are still run at compile-time. For example, consider the expression:
///
/// ```
/// run ((1 + 2) + (3 + 4))
/// ```
///
/// This evaluates at compile-time to:
///
/// ```
/// run (3 + 7)
/// ```
///
/// To fully run the entire expression at runtime, one would have to nest run expressions:
///
/// ```
/// run (run (1 + 2) + run (3 + 4))
/// ```
///
/// The syntax for this expression is:
///
/// `run <expression>`
#[derive(Debug, Clone)]
pub struct RunExpression {
	expression: ExpressionPointer,
	span: Span,
}

impl CompileTime for RunExpression {
	type Output = RunExpression;

	fn evaluate_at_compile_time<Input: IoReader, Output: IoWriter, Error: IoWriter>(self, _context: &mut Context<Input, Output, Error>) -> Self::Output {
		// TODO: evaluate subexpressions
		self
	}
}

impl Runtime for RunExpression {
	type Output = ExpressionPointer;

	fn evaluate_at_runtime<Input: IoReader, Output: IoWriter, Error: IoWriter>(self, context: &mut Context<Input, Output, Error>) -> Self::Output {
		self.expression.evaluate_at_runtime(context)
	}
}

impl TranspileToC for RunExpression {
	fn to_c<Input: IoReader, Output: IoWriter, Error: IoWriter>(&self, context: &mut Context<Input, Output, Error>, output: Option<String>) -> Result<String, TranspileError> {
		self.expression.to_c(context, output)
	}
}

impl Spanned for RunExpression {
	fn span<Input: IoReader, Output: IoWriter, Error: IoWriter>(&self, _context: &Context<Input, Output, Error>) -> Span {
		self.span.to_owned()
	}
}

/// Indicates that this type of expression can be prefixed by the `run` keyword. In this case,
/// the expression needs to implement how the `run` keyword should act on it via
/// `evaluate_subexpressions_at_compile_time()`.
pub trait RuntimeableExpression {
	fn evaluate_subexpressions_at_compile_time<Input: IoReader, Output: IoWriter, Error: IoWriter>(self, context: &mut Context<Input, Output, Error>) -> Self;
}
