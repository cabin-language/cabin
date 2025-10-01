use crate::{
	api::context::Context,
	ast::expressions::{Expression, Spanned},
	comptime::{memory::ExpressionPointer, CompileTime},
	io::Io,
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

	fn evaluate_at_compile_time<System: Io>(self, _context: &mut Context<System>) -> Self::Output {
		todo!()
	}
}

impl Spanned for UnaryOperation {
	fn span<System: Io>(&self, _context: &Context<System>) -> Span {
		self.span
	}
}
