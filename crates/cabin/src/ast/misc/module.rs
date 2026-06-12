use std::collections::HashMap;

use crate::{
	Context,
	Span,
	Spanned as _,
	ast::{
		expressions::{identifier::Identifier, literal::Object},
		statements::{Statement, declaration::Declaration},
	},
	comptime::CompileTime,
	diagnostics::{Diagnostic, DiagnosticInfo},
	parser::{Parse, TokenQueue, TokenQueueFunctionality as _},
	scope::{ScopeId, ScopeType},
};

#[derive(Debug)]
pub struct Module {
	declarations: Vec<Declaration>,
	inner_scope_id: ScopeId,
}

impl Parse for Module {
	type Output = Self;

	fn parse(tokens: &mut TokenQueue, context: &mut Context) -> Self::Output {
		let module_scope = context.scope.enter_new_scope(ScopeType::File);
		let inner_scope_id = context.scope.unique_id();
		let mut declarations = Vec::new();

		while !tokens.is_all_whitespace() {
			let statement = Statement::parse(tokens, context);

			match statement {
				Statement::Declaration(declaration) => {
					declarations.push(declaration);
				},
				Statement::Error(_span) => {},
				other_statement => context.add_diagnostic(Diagnostic {
					file: context.file.clone(),
					span: other_statement.span(context),
					info: DiagnosticInfo::InvalidTopLevelStatement { statement: other_statement },
				}),
			};
		}

		context.scope.exit_scope(module_scope).unwrap();
		Module { declarations, inner_scope_id }
	}
}

impl CompileTime for Module {
	type Output = Module;

	fn evaluate_at_compile_time(self, context: &mut Context) -> Self::Output {
		Self {
			declarations: self.declarations.into_iter().map(|statement| statement.evaluate_at_compile_time(context)).collect(),
			inner_scope_id: self.inner_scope_id,
		}
	}
}

impl Module {
	pub fn into_object(self, context: &mut Context) -> Object {
		let mut fields = HashMap::new();
		for declaration in self.declarations {
			let _ = fields.insert(declaration.name().to_owned(), declaration.value(context).evaluate_to_literal(context));
		}

		Object {
			span: Span::none(),
			type_name: Identifier::create_virtual("Module", context),
			fields,
		}
	}
}
