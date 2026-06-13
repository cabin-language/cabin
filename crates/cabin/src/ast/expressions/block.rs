use derive_getters::Getters;

use crate::{
	Span,
	Spanned,
	api::{
		context::Context,
		scope::{ScopeRef, ScopeType},
	},
	ast::{
		expressions::EvaluatedExpression,
		statements::{EvaluatedStatement, Statement},
	},
	comptime::{Evaluate, memory::Pointer},
	diagnostics::{Diagnostic, DiagnosticInfo},
	lexer::{Token, TokenType},
	log,
	parser::{Parse as _, TokenQueue, TokenQueueFunctionality as _, TryParse as _},
	util::ExitScope,
};

#[derive(Debug, Clone, Getters)]
pub struct Block {
	/// The statements inside this block.
	statements: Vec<Statement>,

	/// The scope ID of the inside of this block.
	inner_scope: ScopeRef,

	tokens: BlockTokens,
}

#[derive(Debug, Clone, Getters)]
pub struct BlockTokens {
	left_brace: Token,
	right_brace: Token,
}

impl Block {
	pub fn try_parse(tokens: &mut TokenQueue, context: &mut Context, scope_type: ScopeType) -> Result<Block, Diagnostic> {
		let left_brace = tokens.pop(TokenType::LeftBrace, context)?;

		let block_scope = context.scope.enter_new_scope(scope_type);
		let inner_scope_id = context.scope.unique_id();

		let mut statements = Vec::new();
		while !tokens.next_is(TokenType::RightBrace) {
			statements.push(Statement::try_parse(tokens, context).or_exit_scope(context)?);
		}

		context.scope.exit_scope(block_scope).unwrap();

		let right_brace = tokens.pop(TokenType::RightBrace, context)?;

		Ok(Block {
			statements,
			inner_scope: inner_scope_id,
			tokens: BlockTokens { left_brace, right_brace },
		})
	}

	pub const fn inner_scope_id(&self) -> ScopeRef {
		self.inner_scope
	}
}

impl Evaluate for Block {
	type Output = Pointer;

	fn evaluate(self, context: &mut Context) -> Self::Output {
		log!(context, "Evaluating block");
		let mut statements = Vec::new();
		let scope_reverter = context.scope.set_current_scope(self.inner_scope);

		let count = self.statements.len();
		let last_span = self.statements.last().map(|last| last.span());

		for (index, statement) in self.statements.into_iter().enumerate() {
			let evaluated_statement = statement.evaluate(context);
			if let EvaluatedStatement::Tail(tail) = evaluated_statement {
				let tail_end = tail.span().end();
				if index != count - 1 {
					context.add_diagnostic(Diagnostic {
						span: Span::range(tail_end, last_span.unwrap().end()),
						info: DiagnosticInfo::UnreachableCode,
						file: context.file.clone(),
					});
				}

				let value = tail.value();
				if value.value(context).as_object().is_some() {
					scope_reverter.revert(context);
					return *value;
				}

				statements.push(EvaluatedStatement::Tail(tail));
			} else {
				statements.push(evaluated_statement);
			};
		}

		scope_reverter.revert(context);

		EvaluatedExpression::Block(EvaluatedBlock {
			statements,
			inner_scope: self.inner_scope,
			tokens: self.tokens,
		})
		.store(context)
	}
}

impl Spanned for Block {
	fn span(&self) -> Span {
		self.tokens.left_brace.span.to(self.tokens.right_brace.span)
	}
}

#[derive(Debug, Clone, Getters)]
pub struct EvaluatedBlock {
	/// The statements inside this block.
	statements: Vec<EvaluatedStatement>,

	/// The scope ID of the inside of this block.
	inner_scope: ScopeRef,

	tokens: BlockTokens,
}

impl Spanned for EvaluatedBlock {
	fn span(&self) -> Span {
		self.tokens.left_brace.span.to(self.tokens.right_brace.span)
	}
}
