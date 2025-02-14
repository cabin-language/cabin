use std::collections::HashMap;

use crate::{
	ast::{
		expressions::{field_access::FieldAccessType, literal::LiteralObject},
		misc::tag::TagList,
		statements::{declaration::Declaration, Statement},
	},
	comptime::{memory::VirtualPointer, CompileTime},
	diagnostics::{Diagnostic, DiagnosticInfo},
	parser::{Parse, ParseError, TokenQueue, TokenQueueFunctionality as _},
	scope::{ScopeId, ScopeType},
	traits::TryAs as _,
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
		let scope_reverter = context.scope_tree.set_current_scope(self.inner_scope_id);
		let evaluated = Self {
			declarations: self.declarations.into_iter().map(|statement| statement.evaluate_at_compile_time(context)).collect(),
			inner_scope_id: self.inner_scope_id,
		};
		scope_reverter.revert(context);
		evaluated
	}
}

impl Module {
	pub(crate) fn to_pointer(&self, context: &mut Context) -> VirtualPointer {
		LiteralObject {
			type_name: "Object".into(),
			fields: self
				.declarations
				.iter()
				.map(|declaration| {
					(
						declaration.name().to_owned(),
						*declaration
							.value(context)
							.clone()
							.evaluate_at_compile_time(context)
							.try_as::<VirtualPointer>()
							.unwrap_or(&VirtualPointer::ERROR),
					)
				})
				.collect(),
			internal_fields: HashMap::new(),
			field_access_type: FieldAccessType::Normal,
			outer_scope_id: ScopeId::stdlib(),
			inner_scope_id: None,
			name: "module".into(),
			address: None,
			span: Span::unknown(),
			tags: TagList::default(),
		}
		.store_in_memory(context)
	}
}
