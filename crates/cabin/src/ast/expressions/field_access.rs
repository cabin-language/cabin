use derive_getters::Getters;

use crate::{
	Context,
	SpanLike,
	Spanned,
	ast::expressions::{Expression, binary::PrimaryExpression, identifier::Identifier},
	comptime::{
		Dot as _,
		Evaluate,
		storage::{Pointer, Storable},
	},
	diagnostics::{Diagnostic, DiagnosticInfo},
	lexer::{Token, TokenType},
	log,
	parser::{TokenQueue, TokenQueueFunctionality as _, TryParse},
};

#[derive(Debug, Clone, Getters)]
pub struct FieldAccess {
	pub(crate) left: Pointer,
	pub(crate) right: Identifier,
	pub(crate) tokens: FieldAccessTokens,
}

#[derive(Debug, Clone, Getters)]
pub struct FieldAccessTokens {
	pub(crate) operator: Token,
}

impl TryParse for FieldAccess {
	type Output = Pointer;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		let mut expression = PrimaryExpression::try_parse(tokens, context)?;

		while tokens.next_is(TokenType::Dot) {
			let operator = tokens.pop(TokenType::Dot, context)?;
			let right = Identifier::try_parse(tokens, context)?;
			expression = Expression::FieldAccess(FieldAccess {
				left: expression,
				right,
				tokens: FieldAccessTokens { operator },
			})
			.store(context);
		}

		Ok(expression)
	}
}

impl Evaluate for FieldAccess {
	type Output = Pointer;

	fn evaluate(self, context: &mut Context) -> Self::Output {
		log!(context, "Evaluating field access");

		let span = self.span().into_owned();
		let left = self.left.evaluate(context);
		let result = left.dot(context, self.right.source_identifier());

		match result {
			Some(field) => {
				if let Storable::Expression(Expression::Action(mut action)) = field.value(context).clone()
					&& action.takes_this()
				{
					action.this = Some(left);
					return Expression::Action(action).store(context);
				}

				field.with_span(span)
			},
			None => {
				context.add_diagnostic(Diagnostic::at(&self.right, DiagnosticInfo::NoSuchField(self.right.source_identifier().to_owned())));

				Expression::FieldAccess(FieldAccess {
					left,
					right: self.right,
					tokens: self.tokens,
				})
				.store(context)
			},
		}
	}
}

impl Spanned for FieldAccess {
	fn span(&self) -> impl SpanLike {
		self.left().span().to(self.right.span())
	}
}
