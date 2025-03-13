use crate::{
	api::context::Context,
	ast::expressions::{Expression, Spanned},
	comptime::{memory::ExpressionPointer, CompileTime},
	io::{IoReader, IoWriter},
	Span,
};

/// A unary operator. These are types of operators that take a single expression and operate on it.
#[derive(Debug, Clone)]
pub enum UnaryOperator {
	QuestionMark,
	ExclamationPoint,
}

/// Unlike binary expressions, which are converted to function calls at parse-time, these cannot be
/// converted to function calls because operators like `?` and `!` can affect control flow.
#[derive(Debug, Clone)]
pub struct UnaryOperation {
	pub operator: UnaryOperator,
	pub expression: ExpressionPointer,
	pub span: Span,
}

impl CompileTime for UnaryOperation {
	type Output = Expression;

	fn evaluate_at_compile_time<Input: IoReader, Output: IoWriter, Error: IoWriter>(self, _context: &mut Context<Input, Output, Error>) -> Self::Output {
		todo!()
	}
}

impl Spanned for UnaryOperation {
	fn span<Input: IoReader, Output: IoWriter, Error: IoWriter>(&self, _context: &Context<Input, Output, Error>) -> Span {
		self.span
	}
}
