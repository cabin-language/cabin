use crate::{
	Span,
	Spanned,
	api::{
		context::Context,
		scope::{ScopeId, ScopeType},
	},
	ast::{
		expressions::{Expression, block},
		statements::Statement,
	},
	comptime::{CompileTime as _, memory::ExpressionPointer},
	diagnostics::{Diagnostic, DiagnosticInfo},
	lexer::TokenType,
	parser::{Parse as _, TokenQueue, TokenQueueFunctionality as _, TryParse},
	transpiler::{TranspileError, TranspileToC},
};

#[derive(Debug, Clone)]
pub struct Block {
	/// The statements inside this block.
	statements: Vec<Statement>,

	/// The scope ID of the inside of this block.
	inner_scope_id: ScopeId,

	/// The span of this block. See `Spanned::span()` for more information.
	span: Span,
}

impl Block {
	/// Parses a block expression and sets the scope type of the inner scope.
	///
	/// # Parameters
	///
	/// - `tokens` - The token stream to parse
	/// - `scope_type`- The scope type of the inside of the block
	///
	/// # Returns
	///
	/// The parsed block expression
	///
	/// # Errors
	///
	/// If an unexpected token was encountered.
	pub fn parse_with_scope_type(tokens: &mut TokenQueue, context: &mut Context, scope_type: ScopeType) -> Result<Block, Diagnostic> {
		let block_scope = context.scope.enter_new_scope(scope_type);

		let scope_id = context.scope.unique_id();
		let start = tokens.pop(TokenType::LeftBrace, context)?.span;

		let mut statements = Vec::new();
		while !tokens.next_is(TokenType::RightBrace) {
			statements.push(Statement::parse(tokens, context));
		}

		let end = tokens.pop(TokenType::RightBrace, context)?.span;

		context.scope.exit_scope(block_scope).unwrap();

		Ok(Block {
			statements,
			inner_scope_id: scope_id,
			span: start.to(end),
		})
	}

	pub const fn inner_scope_id(&self) -> ScopeId {
		self.inner_scope_id
	}

	pub fn evaluate_eager(self, context: &mut Context) -> ExpressionPointer {
		let mut statements = Vec::new();
		let scope_reverter = context.scope.set_current_scope(self.inner_scope_id);

		let count = self.statements.len();
		let last_span = self.statements.last().map(|last| last.span(context));

		for (index, statement) in self.statements.into_iter().enumerate() {
			let evaluated_statement = statement.evaluate_at_compile_time(context);
			if let Statement::Tail(tail) = evaluated_statement {
				let tail_end = tail.span.end();
				if index != count - 1 {
					context.add_diagnostic(Diagnostic {
						span: Span::range(tail_end, last_span.unwrap().end()),
						info: DiagnosticInfo::UnreachableCode,
						file: context.file.clone(),
					});
				}

				let value = tail.value;
				if let Ok(literal) = value.try_as_literal(context) {
					scope_reverter.revert(context);
					return literal.into();
				}

				statements.push(Statement::Tail(tail));
			} else {
				statements.push(evaluated_statement);
			};
		}

		scope_reverter.revert(context);

		Expression::Block(Block {
			statements,
			inner_scope_id: self.inner_scope_id,
			span: self.span,
		})
		.store_in_memory(context)
	}

	pub fn evaluate_lazy(self, context: &mut Context) -> Block {
		let snapshot = context.snapshot();
		let scope_reverter = context.scope.set_current_scope(self.inner_scope_id);

		let mut statements = Vec::new();

		let count = self.statements.len();
		let last_span = self.statements.last().map(|last| last.span(context));

		for (index, statement) in self.statements.into_iter().enumerate() {
			let evaluated_statement = statement.evaluate_at_compile_time(context);
			if let Statement::Tail(tail) = &evaluated_statement {
				let tail_end = tail.span.end();
				if index != count - 1 {
					context.add_diagnostic(Diagnostic {
						span: Span::range(tail_end, last_span.unwrap().end()),
						info: DiagnosticInfo::UnreachableCode,
						file: context.file.clone(),
					});
				}
			}
			statements.push(evaluated_statement);
		}

		scope_reverter.revert(context);
		context.roll_back(snapshot);

		Block {
			statements,
			inner_scope_id: self.inner_scope_id,
			span: self.span,
		}
	}
}

impl TryParse for Block {
	type Output = Block;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		Block::parse_with_scope_type(tokens, context, ScopeType::Block)
	}
}

impl TranspileToC for Block {
	fn to_c(&self, context: &mut Context, _output: Option<String>) -> Result<String, TranspileError> {
		let mut builder = vec!["{".to_owned()];
		for statement in &self.statements {
			builder.push(format!("\t{}", statement.to_c(context, None)?));
		}
		builder.push("}".to_owned());
		Ok(builder.join("\n"))
	}
}

impl Spanned for Block {
	fn span(&self, _context: &Context) -> Span {
		self.span.to_owned()
	}
}
