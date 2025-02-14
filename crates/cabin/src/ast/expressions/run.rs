use crate::{
	api::context::Context,
	ast::expressions::{Expression, Spanned},
	comptime::{memory::VirtualPointer, CompileTime},
	diagnostics::Diagnostic,
	lexer::TokenType,
	parser::{Parse as _, TokenQueue, TokenQueueFunctionality as _, TryParse},
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
	expression: Box<Expression>,
	span: Span,
}

impl TryParse for RunExpression {
	type Output = RunExpression;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		let mut span = tokens.pop(TokenType::KeywordRuntime)?.span;
		let expression = Box::new(Expression::parse(tokens, context));
		span = span.to(expression.span(context));
		Ok(RunExpression { span, expression })
	}
}

impl CompileTime for RunExpression {
	type Output = RunExpression;

	fn evaluate_at_compile_time(self, context: &mut Context) -> Self::Output {
		RunExpression {
			expression: Box::new(self.expression.evaluate_subexpressions_at_compile_time(context)),
			span: self.span,
		}
	}
}

impl TranspileToC for RunExpression {
	fn to_c(&self, context: &mut Context, output: Option<String>) -> Result<String, TranspileError> {
		self.expression.to_c(context, output)
	}
}

impl Spanned for RunExpression {
	fn span(&self, _context: &Context) -> Span {
		self.span.to_owned()
	}
}

/// Indicates that this type of expression can be prefixed by the `run` keyword. In this case,
/// the expression needs to implement how the `run` keyword should act on it via
/// `evaluate_subexpressions_at_compile_time()`.
pub trait RuntimeableExpression: Sized {
	fn evaluate_subexpressions_at_compile_time(self, context: &mut Context) -> Self;
}
