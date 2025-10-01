use std::collections::HashMap;

use crate::{
	ast::{
		expressions::literal::Object,
		statements::{declaration::Declaration, Statement},
	},
	comptime::CompileTime,
	diagnostics::{Diagnostic, DiagnosticInfo},
	io::Io,
	parser::{Parse, ParseError, TokenQueue, TokenQueueFunctionality as _},
	scope::{ScopeId, ScopeType},
	Context,
	Span,
	Spanned as _,
};

#[derive(Debug)]
pub struct Module {
	declarations: Vec<Declaration>,
	inner_scope_id: ScopeId,
}

impl Parse for Module {
	type Output = Self;

	fn parse<System: Io>(tokens: &mut TokenQueue, context: &mut Context<System>) -> Self::Output {
		context.scope_tree.enter_new_scope(ScopeType::File);
		let inner_scope_id = context.scope_tree.unique_id();
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
					info: DiagnosticInfo::Error(crate::Error::Parse(ParseError::InvalidTopLevelStatement { statement: other_statement })),
				}),
			};
		}

		context.scope_tree.exit_scope().unwrap();
		Module { declarations, inner_scope_id }
	}
}

impl CompileTime for Module {
	type Output = Module;

	fn evaluate_at_compile_time<System: Io>(self, context: &mut Context<System>) -> Self::Output {
		Self {
			declarations: self.declarations.into_iter().map(|statement| statement.evaluate_at_compile_time(context)).collect(),
			inner_scope_id: self.inner_scope_id,
		}
	}
}

impl Module {
	pub(crate) fn into_object<System: Io>(self, context: &mut Context<System>) -> Object {
		let mut fields = HashMap::new();
		for declaration in self.declarations {
			let _ = fields.insert(declaration.name().to_owned(), declaration.value(context).evaluate_to_literal(context));
		}

		Object {
			span: Span::unknown(),
			type_name: "Module".into(),
			fields,
		}
	}
}
