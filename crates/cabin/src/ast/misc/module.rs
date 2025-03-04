use std::collections::HashMap;

use crate::{
	ast::{
		expressions::new_literal::Object,
		statements::{declaration::Declaration, Statement},
	},
	comptime::{memory::LiteralPointer, CompileTime},
	diagnostics::{Diagnostic, DiagnosticInfo},
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

	fn parse(tokens: &mut TokenQueue, context: &mut Context) -> Self::Output {
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
				statement => context.add_diagnostic(Diagnostic {
					file: context.file.clone(),
					span: statement.span(context),
					info: DiagnosticInfo::Error(crate::Error::Parse(ParseError::InvalidTopLevelStatement { statement })),
				}),
			};
		}

		context.scope_tree.exit_scope().unwrap();
		Module { declarations, inner_scope_id }
	}
}

impl CompileTime for Module {
	type Output = Module;

	fn evaluate_at_compile_time(self, context: &mut Context) -> Self::Output {
		let evaluated = Self {
			declarations: self.declarations.into_iter().map(|statement| statement.evaluate_at_compile_time(context)).collect(),
			inner_scope_id: self.inner_scope_id,
		};
		evaluated
	}
}

impl Module {
	pub(crate) fn into_object(self, context: &mut Context) -> Object {
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
