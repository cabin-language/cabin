use crate::{
	api::{
		context::Context,
		scope::{ScopeId, ScopeType},
	},
	ast::statements::Statement,
	comptime::CompileTime,
	diagnostics::Diagnostic,
	io::Io,
	lexer::TokenType,
	parser::{Parse as _, TokenQueue, TokenQueueFunctionality as _, TryParse},
	transpiler::{TranspileError, TranspileToC},
	Span,
	Spanned,
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
	pub fn parse_with_scope_type<System: Io>(tokens: &mut TokenQueue, context: &mut Context<System>, scope_type: ScopeType) -> Result<Block, Diagnostic> {
		context.scope_tree.enter_new_scope(scope_type);
		let scope_id = context.scope_tree.unique_id();
		let start = tokens.pop(TokenType::LeftBrace, context)?.span;

		let mut statements = Vec::new();
		while !tokens.next_is(TokenType::RightBrace) {
			statements.push(Statement::parse(tokens, context));
		}

		let end = tokens.pop(TokenType::RightBrace, context)?.span;

		context.scope_tree.exit_scope().unwrap();

		Ok(Block {
			statements,
			inner_scope_id: scope_id,
			span: start.to(end),
		})
	}

	pub(crate) const fn inner_scope_id(&self) -> ScopeId {
		self.inner_scope_id
	}
}

impl TryParse for Block {
	type Output = Block;

	fn try_parse<System: Io>(tokens: &mut TokenQueue, context: &mut Context<System>) -> Result<Self::Output, Diagnostic> {
		Block::parse_with_scope_type(tokens, context, ScopeType::Block)
	}
}

impl CompileTime for Block {
	/// The output for evaluating blocks at compile-time is a generic `Expression`. This is because while some blocks
	/// will not be able to be fully evaluated and will remain as blocks, some others *will* be able to be resolved
	/// fully, and those will return either the expressed from their tail statement, or `Expression::Void` if no tail
	/// statement was present.
	type Output = Block;

	fn evaluate_at_compile_time<System: Io>(self, context: &mut Context<System>) -> Self::Output {
		let mut statements = Vec::new();

		for statement in self.statements {
			let evaluated_statement = statement.evaluate_at_compile_time(context);
			statements.push(evaluated_statement);
		}

		Block {
			statements,
			inner_scope_id: self.inner_scope_id,
			span: self.span,
		}
	}
}

impl TranspileToC for Block {
	fn to_c<System: Io>(&self, context: &mut Context<System>, _output: Option<String>) -> Result<String, TranspileError> {
		let mut builder = vec!["{".to_owned()];
		for statement in &self.statements {
			builder.push(format!("\t{}", statement.to_c(context, None)?));
		}
		builder.push("}".to_owned());
		Ok(builder.join("\n"))
	}
}

impl Spanned for Block {
	fn span<System: Io>(&self, _context: &Context<System>) -> Span {
		self.span.to_owned()
	}
}
