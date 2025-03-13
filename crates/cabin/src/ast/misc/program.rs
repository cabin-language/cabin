use crate::{
	ast::statements::Statement,
	comptime::CompileTime,
	interpreter::Runtime,
	io::{IoReader, IoWriter},
	parser::{Parse, TokenQueue, TokenQueueFunctionality as _},
	scope::{ScopeId, ScopeType},
	transpiler::{TranspileError, TranspileToC},
	Context,
};

#[derive(Debug)]
pub struct Program {
	statements: Vec<Statement>,
	inner_scope_id: ScopeId,
}

impl Parse for Program {
	type Output = Self;

	fn parse<Input: IoReader, Output: IoWriter, Error: IoWriter>(tokens: &mut TokenQueue, context: &mut Context<Input, Output, Error>) -> Self::Output {
		context.scope_tree.enter_new_scope(ScopeType::File);
		let inner_scope_id = context.scope_tree.unique_id();
		let mut statements = Vec::new();

		while !tokens.is_all_whitespace() {
			statements.push(Statement::parse(tokens, context));
		}

		context.scope_tree.exit_scope().unwrap();

		Program { statements, inner_scope_id }
	}
}

impl CompileTime for Program {
	type Output = Program;

	fn evaluate_at_compile_time<Input: IoReader, Output: IoWriter, Error: IoWriter>(self, context: &mut Context<Input, Output, Error>) -> Self::Output {
		let evaluated = Self {
			statements: self.statements.into_iter().map(|statement| statement.evaluate_at_compile_time(context)).collect(),
			inner_scope_id: self.inner_scope_id,
		};
		evaluated
	}
}

impl Runtime for Program {
	type Output = Program;

	fn evaluate_at_runtime<Input: IoReader, Output: IoWriter, Error: IoWriter>(self, context: &mut Context<Input, Output, Error>) -> Self::Output {
		let evaluated = Self {
			statements: self.statements.into_iter().map(|statement| statement.evaluate_at_runtime(context)).collect(),
			inner_scope_id: self.inner_scope_id,
		};
		evaluated
	}
}

impl TranspileToC for Program {
	fn to_c<Input: IoReader, Output: IoWriter, Error: IoWriter>(&self, context: &mut Context<Input, Output, Error>, _output: Option<String>) -> Result<String, TranspileError> {
		let type_prelude = self
			.statements
			.iter()
			.map(|statement| statement.c_type_prelude(context))
			.collect::<Result<Vec<_>, _>>()?
			.join("\n");

		let prelude = self
			.statements
			.iter()
			.map(|statement| statement.c_prelude(context))
			.collect::<Result<Vec<_>, _>>()?
			.join("\n");

		let body = self
			.statements
			.iter()
			.map(|statement| statement.to_c(context, None))
			.collect::<Result<Vec<_>, _>>()?
			.join("\n");

		Ok(format!("{type_prelude}\n\n{prelude}\n\n{body}",))
	}
}
