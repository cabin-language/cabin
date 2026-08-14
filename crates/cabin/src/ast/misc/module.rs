use std::collections::HashMap;

use derive_getters::Getters;

use crate::{
	Context,
	Span,
	SpanLike as _,
	Spanned as _,
	ast::{
		expressions::{action::EvaluatedAction, either::EvaluatedEither, evaluated::EvaluatedExpression, extend::EvaluatedExtend, group::EvaluatedGroup, object::Object},
		statements::{
			Statement,
			declaration::{Declaration, EvaluatedDeclaration},
		},
	},
	comptime::{
		Dot,
		Evaluate,
		storage::{ExpressionRef, Storable},
	},
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
	type Output = EvaluatedModule;

	fn evaluate(self, context: &mut Context) -> Self::Output {
		EvaluatedModule {
			declarations: self
				.declarations
				.into_iter()
				.filter_map(|statement| {
					let evaluated = statement.evaluate(context);
					let span = evaluated.value().span().owned();
					let top_level = TopLevelDeclaration::new(evaluated, context);
					if top_level.is_none() {
						context.add_diagnostic_at(&span, DiagnosticInfo::BadTopLevelDeclaration);
					}

					top_level
				})
				.collect(),
			scope: self.scope,
		}
	}
}

#[derive(Debug, Clone, Getters)]
pub struct EvaluatedModule {
	declarations: Vec<TopLevelDeclaration>,
	scope: ScopeRef,
}

impl EvaluatedModule {
	pub(crate) fn into_object(self, context: &Context) -> Object {
		let mut fields = HashMap::new();
		for declaration in self.declarations {
			let name = declaration.inner.name().source_identifier().to_owned();
			let value = declaration.inner.value().to_owned();
			let _ = fields.insert(name, value);
		}
		Object::new(context.builtin("Module"), fields, Span::none())
	}
}

impl Dot for EvaluatedModule {
	fn dot(&self, _context: &Context, name: &str) -> Option<ExpressionRef> {
		self.declarations
			.iter()
			.find_map(|declaration| (declaration.inner.name().source_identifier() == name).then(|| declaration.inner.value().to_owned()))
	}
}

#[derive(Debug, Clone)]
pub struct TopLevelDeclaration {
	inner: EvaluatedDeclaration,
}

impl TopLevelDeclaration {
	fn new(inner: EvaluatedDeclaration, context: &Context) -> Option<TopLevelDeclaration> {
		match inner.value().value(context) {
			Storable::Object(_) => Some(Self { inner }),
			Storable::Expression(expression) => matches!(
				expression,
				EvaluatedExpression::Group(_) | EvaluatedExpression::Action(_) | EvaluatedExpression::Extend(_) | EvaluatedExpression::Either(_)
			)
			.then_some(Self { inner }),
			Storable::Uninitialized => None,
		}
	}

	pub fn value<'ctx>(&self, context: &'ctx Context) -> TopLevelDeclarationValue<'ctx> {
		match self.inner.value().value(context) {
			Storable::Object(object) => TopLevelDeclarationValue::Object(object),
			Storable::Expression(expression) => match expression {
				EvaluatedExpression::Group(group) => TopLevelDeclarationValue::Group(group),
				EvaluatedExpression::Action(action) => TopLevelDeclarationValue::Action(action),
				EvaluatedExpression::Extend(extend) => TopLevelDeclarationValue::Extend(extend),
				EvaluatedExpression::Either(either) => TopLevelDeclarationValue::Either(either),
				_ => unreachable!(),
			},
			Storable::Uninitialized => unreachable!(),
		}
	}
}

#[derive(Debug, Clone)]
pub enum TopLevelDeclarationValue<'ctx> {
	Group(&'ctx EvaluatedGroup),
	Action(&'ctx EvaluatedAction),
	Extend(&'ctx EvaluatedExtend),
	Either(&'ctx EvaluatedEither),
	Object(&'ctx Object),
}
