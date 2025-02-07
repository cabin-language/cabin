use std::fmt::Write as _;

use crate::{
	api::{
		context::context,
		scope::{ScopeId, ScopeType},
	},
	comptime::CompileTime,
	debug_start,
	lexer::{Span, TokenType},
	parser::{
		expressions::{Expression, Spanned},
		statements::Statement,
		Parse,
		TokenQueue,
		TokenQueueFunctionality as _,
	},
	transpiler::TranspileToC,
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
	/// Creates a new `Block`.
	///
	/// # Parameters
	///
	/// - `statements` - The statements inside the block
	/// - `inner_scope_id` - The ID of the scope inside this block
	/// - `span` - The span of the block
	///
	/// # Returns
	///
	/// The created block
	pub const fn new(statements: Vec<Statement>, inner_scope_id: ScopeId, span: Span) -> Block {
		Block { statements, inner_scope_id, span }
	}

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
	pub fn parse_with_scope_type(tokens: &mut TokenQueue, scope_type: ScopeType) -> Result<Block, crate::Error> {
		let debug_section = debug_start!("{} a {} expression", "Compile-Time Evaluating".bold().green(), "block".cyan());
		context().scope_data.enter_new_scope(scope_type);

		let scope_id = context().scope_data.unique_id();

		let start = tokens.pop(TokenType::LeftBrace)?.span;

		let mut statements = Vec::new();
		while !tokens.next_is(TokenType::RightBrace) {
			statements.push(Statement::parse(tokens)?);
		}

		let end = tokens.pop(TokenType::RightBrace)?.span;

		context().scope_data.exit_scope().unwrap();

		debug_section.finish();
		Ok(Block {
			statements,
			inner_scope_id: scope_id,
			span: start.to(end),
		})
	}
}

impl Parse for Block {
	type Output = Block;

	fn parse(tokens: &mut TokenQueue) -> Result<Self::Output, crate::Error> {
		Block::parse_with_scope_type(tokens, ScopeType::Block)
	}
}

impl CompileTime for Block {
	/// The output for evaluating blocks at compile-time is a generic `Expression`. This is because while some blocks
	/// will not be able to be fully evaluated and will remain as blocks, some others *will* be able to be resolved
	/// fully, and those will return either the expressed from their tail statement, or `Expression::Void` if no tail
	/// statement was present.
	type Output = Expression;

	fn evaluate_at_compile_time(self) -> anyhow::Result<Self::Output> {
		let mut statements = Vec::new();
		let _scope_reverter = context().scope_data.set_current_scope(self.inner_scope_id);
		for statement in self.statements {
			let evaluated_statement = statement.evaluate_at_compile_time()?;

			// Tail statement
			if let Statement::Tail(tail_statement) = evaluated_statement {
				if tail_statement.value.try_as_literal().is_ok() {
					return Ok(tail_statement.value);
				}
				statements.push(Statement::Tail(tail_statement));
			}
			// Not tail statement
			else {
				statements.push(evaluated_statement);
			}
		}

		Ok(Expression::Block(Block {
			statements,
			inner_scope_id: self.inner_scope_id,
			span: self.span,
		}))
	}
}

impl TranspileToC for Block {
	fn to_c(&self) -> anyhow::Result<String> {
		let mut builder = String::new();
		builder += "({";
		for statement in &self.statements {
			for line in statement.to_c()?.lines() {
				write!(builder, "\n{line}").unwrap();
			}
		}
		builder += "\n})";

		Ok(builder)
	}
}

impl Spanned for Block {
	fn span(&self) -> Span {
		self.span.to_owned()
	}
}

impl Block {
	pub fn inner_scope_id(&self) -> ScopeId {
		self.inner_scope_id
	}
}
