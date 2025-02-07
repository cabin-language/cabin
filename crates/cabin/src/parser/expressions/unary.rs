use crate::{
	api::{context::context, scope::ScopeType},
	comptime::CompileTime,
	lexer::Span,
	parser::{
		expressions::{
			block::Block,
			match_expression::{Match, MatchBranch},
			Expression,
			Spanned,
			Typed,
		},
		statements::{tail::TailStatement, Statement},
	},
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
	pub expression: Box<Expression>,
	pub span: Span,
}

impl CompileTime for UnaryOperation {
	type Output = Expression;

	fn evaluate_at_compile_time(self) -> anyhow::Result<Self::Output> {
		let expression = self.expression.evaluate_as_type()?;
		match self.operator {
			UnaryOperator::QuestionMark => {
				if expression.get_type()?.virtual_deref().type_name() == &"Attempted".into() {
					Ok(Expression::Block(Block::new(
						vec![Statement::Expression(Expression::Match(Match {
							expression: Box::new(expression),
							branches: vec![MatchBranch {
								name: None,
								type_to_match: Expression::Name("Nothing".into()),
								body: Block::new(
									vec![Statement::Tail(TailStatement {
										label: "action".into(),
										value: Expression::Name("nothing".into()),
										span: Span::unknown(),
									})],
									context().scope_data.new_scope_id(ScopeType::Block),
									Span::unknown(),
								),
							}],
							span: Span::unknown(),
						}))],
						context().scope_data.new_scope_id(ScopeType::Block),
						Span::unknown(),
					)))
				} else {
					todo!()
				}
			},
			UnaryOperator::ExclamationPoint => {
				todo!()
			},
		}
	}
}

impl Spanned for UnaryOperation {
	fn span(&self) -> Span {
		self.span
	}
}
