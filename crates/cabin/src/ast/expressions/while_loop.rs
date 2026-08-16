use derive_getters::Getters;

use crate::{
	Context,
	SpanLike,
	Spanned,
	ast::expressions::{Expression, block::Block},
	comptime::{Evaluate, storage::Pointer},
	diagnostics::Diagnostic,
	lexer::{Token, TokenType},
	parser::{TokenQueue, TokenQueueFunctionality as _, TryParse},
	scope::ScopeType::{self},
};

#[derive(Debug, Clone, Getters)]
pub struct WhileLoop {
	condition: Pointer,
	body: Block,
	tokens: WhileLoopTokens,
}

#[derive(Debug, Clone, Getters)]
pub struct WhileLoopTokens {
	keyword_while: Token,
}

impl TryParse for WhileLoop {
	type Output = WhileLoop;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		let keyword_while = tokens.pop(TokenType::KeywordWhile, context)?;
		let expression = Expression::try_parse(tokens, context)?;
		let body = Block::try_parse(tokens, context, ScopeType::While)?;

		Ok(WhileLoop {
			condition: expression,
			body,
			tokens: WhileLoopTokens { keyword_while },
		})
	}
}

impl Evaluate for WhileLoop {
	type Output = Pointer;

	fn evaluate(self, context: &mut Context) -> Self::Output {
		loop {
			let condition = self.condition.clone().evaluate(context);

			// loop can be run at compile-time
			if condition.is_fully_known(context) {
				// loop is done
				if condition == context.get_false() {
					return context.none();
				}
			}
			// loop cant be run at compile-time
			else {
				return context.none();
			}
		}
	}
}

impl Spanned for WhileLoop {
	fn span(&self) -> impl SpanLike {
		self.tokens().keyword_while().span().to(self.body.span())
	}
}
