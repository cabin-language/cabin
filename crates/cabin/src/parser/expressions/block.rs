use crate::{
	api::{
		context::context,
		scope::{ScopeId, ScopeType},
	},
	comptime::CompileTime,
	debug_start,
	lexer::{Span, TokenType},
	parser::{expressions::Expression, statements::Statement, Parse, TokenQueue, TokenQueueFunctionality as _},
	transpiler::TranspileToC,
};

use std::fmt::Write as _;

use super::Spanned;

#[derive(Debug, Clone)]
pub struct Block {
	pub statements: Vec<Statement>,
	pub inner_scope_id: ScopeId,
	span: Span,
}

impl Block {
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
	pub fn parse_with_scope_type(tokens: &mut TokenQueue, scope_type: ScopeType) -> anyhow::Result<Block> {
		let debug_section = debug_start!("{} a {} expression", "Compile-Time Evaluating".bold().green(), "block".cyan());
		context().scope_data.enter_new_scope(scope_type);

		let scope_id = context().scope_data.unique_id();

		let start = tokens.pop(TokenType::LeftBrace)?.span;

		let mut statements = Vec::new();
		while !tokens.next_is(TokenType::RightBrace) {
			statements.push(Statement::parse(tokens)?);
		}

		let end = tokens.pop(TokenType::RightBrace)?.span;

		context().scope_data.exit_scope()?;

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

	fn parse(tokens: &mut TokenQueue) -> anyhow::Result<Self::Output> {
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
