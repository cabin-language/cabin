use crate::{
	Context,
	ast::statements::Statement,
	comptime::Evaluate,
	diagnostics::Diagnostic,
	parser::{TokenQueue, TryParse},
	scope::{ScopeRef, ScopeType},
};

#[derive(Debug)]
pub struct Program {
	statements: Vec<Statement>,
	inner_scope: ScopeRef,
}

impl TryParse for Program {
	type Output = Self;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		let program_scope = context.scope.enter_new_scope(ScopeType::File);
		let inner_scope_id = context.scope.get_ref();
		let mut statements = Vec::new();

		while !tokens.is_empty() {
			statements.push(Statement::try_parse(tokens, context)?);
		}

		context.scope.exit_scope(program_scope).unwrap();

		Ok(Program {
			statements,
			inner_scope: inner_scope_id,
		})
	}
}

impl Evaluate for Program {
	type Output = Program;

	fn evaluate(self, context: &mut Context) -> Self::Output {
		Program {
			statements: self.statements.into_iter().map(|statement| statement.evaluate(context)).collect(),
			inner_scope: self.inner_scope,
		}
	}
}
