use std::collections::HashMap;

use crate::{
	Context,
	Span,
	ast::{
		expressions::object::Object,
		statements::{Statement, declaration::Declaration},
	},
	comptime::{Dot, Evaluate, storage::Pointer},
	diagnostics::{Diagnostic, DiagnosticInfo},
	parser::{TokenQueue, TryParse},
	scope::{ScopeRef, ScopeType},
};

#[derive(Debug)]
pub struct Module {
	declarations: Vec<Declaration>,
	scope: ScopeRef,
}

impl Default for Module {
	fn default() -> Self {
		Module {
			declarations: Vec::new(),
			scope: ScopeRef::global(),
		}
	}
}

impl TryParse for Module {
	type Output = Self;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		let module_scope = context.scope.enter_new_scope(ScopeType::File);
		let inner_scope_id = context.scope.get_ref();
		let mut declarations = Vec::new();

		while !tokens.is_empty() {
			let statement = Statement::try_parse(tokens, context)?;

			match statement {
				Statement::Declaration(declaration) => {
					declarations.push(declaration);
				},
				Statement::Error(_span) => {},
				other_statement => context.add_diagnostic_at(&other_statement, DiagnosticInfo::InvalidTopLevelStatement),
			};
		}

		context.scope.exit_scope(module_scope).unwrap();
		Ok(Module {
			declarations,
			scope: inner_scope_id,
		})
	}
}

impl Evaluate for Module {
	type Output = Module;

	fn evaluate(self, context: &mut Context) -> Self::Output {
		Module {
			declarations: self.declarations.into_iter().map(|statement| statement.evaluate(context)).collect(),
			scope: self.scope,
		}
	}
}

impl Dot for Module {
	fn dot(&self, _context: &mut Context, name: &str) -> Option<Pointer> {
		self.declarations
			.iter()
			.find_map(|declaration| (declaration.name().source_identifier() == name).then(|| declaration.value().to_owned()))
	}
}

impl Module {
	pub fn into_object(self) -> Object {
		let mut fields = HashMap::new();
		for declaration in self.declarations {
			let _ = fields.insert(declaration.name().source_identifier().to_owned(), declaration.value().to_owned());
		}

		Object::new(None, fields, Span::none())
	}
}
