use std::{collections::HashMap, fmt::Debug};

use crate::{
	api::{context::Context, scope::ScopeId},
	ast::{
		expressions::{
			field_access::FieldAccessType,
			literal::{LiteralConvertible, LiteralObject},
			name::Name,
			object::InternalFieldValue,
			Expression,
			Spanned,
		},
		misc::tag::TagList,
	},
	comptime::{memory::VirtualPointer, CompileTime, CompileTimeError},
	diagnostics::{Diagnostic, DiagnosticInfo},
	lexer::TokenType,
	parser::{Parse as _, TokenQueue, TokenQueueFunctionality as _, TryParse},
	Span,
};

#[derive(Clone)]
pub struct Parameter {
	name: Name,
	parameter_type: Box<Expression>,
	span: Span,
	scope_id: ScopeId,
}

impl TryParse for Parameter {
	type Output = VirtualPointer;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		let name = Name::try_parse(tokens, context)?;
		let _ = tokens.pop(TokenType::Colon)?;
		let parameter_type = Expression::parse(tokens, context);
		Ok(Parameter {
			span: name.span(context).to(parameter_type.span(context)),
			name,
			parameter_type: Box::new(parameter_type),
			scope_id: context.scope_tree.unique_id(),
		}
		.to_literal()
		.store_in_memory(context))
	}
}

impl CompileTime for Parameter {
	type Output = Parameter;

	fn evaluate_at_compile_time(self, context: &mut Context) -> Self::Output {
		let type_span = self.parameter_type.span(context);
		let evaluated = self.parameter_type.evaluate_as_type(context);

		if !matches!(evaluated, Expression::Pointer(_) | Expression::ErrorExpression(_)) {
			context.add_diagnostic(Diagnostic {
				span: type_span,
				info: DiagnosticInfo::Error(crate::Error::CompileTime(CompileTimeError::ExpressionUsedAsType)),
			});
		}

		let parameter = Parameter {
			name: self.name.clone(),
			parameter_type: Box::new(evaluated),
			span: self.span,
			scope_id: self.scope_id,
		};

		parameter
	}
}

impl Spanned for Parameter {
	fn span(&self, _context: &Context) -> Span {
		self.span.to_owned()
	}
}

impl Parameter {
	pub(crate) const fn name(&self) -> &Name {
		&self.name
	}

	pub(crate) const fn parameter_type(&self) -> &Expression {
		&self.parameter_type
	}
}

impl LiteralConvertible for Parameter {
	fn to_literal(self) -> LiteralObject {
		LiteralObject {
			address: None,
			fields: HashMap::from([]),
			internal_fields: HashMap::from([("type".to_owned(), InternalFieldValue::Expression(*self.parameter_type))]),
			name: self.name,
			field_access_type: FieldAccessType::Normal,
			outer_scope_id: self.scope_id,
			inner_scope_id: Some(self.scope_id),
			span: self.span,
			type_name: "Parameter".into(),
			tags: TagList::default(),
		}
	}

	fn from_literal(literal: &LiteralObject) -> anyhow::Result<Self> {
		Ok(Parameter {
			name: literal.name().to_owned(),
			parameter_type: Box::new(literal.get_internal_field::<Expression>("type")?.to_owned()),
			scope_id: literal.outer_scope_id(),
			span: literal.span,
		})
	}
}

impl Debug for Parameter {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{:?}: {:?}", self.name, self.parameter_type)
	}
}
