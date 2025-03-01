use std::collections::HashMap;

use super::new_literal::{Literal, Object};
use crate::{
	api::context::Context,
	ast::{
		expressions::{name::Name, parameter::Parameter, Expression, Spanned},
		misc::tag::TagList,
	},
	comptime::{memory::ExpressionPointer, CompileTime},
	diagnostics::{Diagnostic, DiagnosticInfo},
	if_then_else_default,
	if_then_some,
	lexer::TokenType,
	parse_list,
	parser::{ListType, Parse as _, TokenQueue, TokenQueueFunctionality as _, TryParse},
	transpiler::{TranspileError, TranspileToC},
	Span,
};

#[derive(Debug, Clone)]
pub struct ObjectConstructor {
	pub type_name: Name,
	pub fields: HashMap<Name, ExpressionPointer>,
	pub span: Span,
	pub tags: TagList,
}

impl TryParse for ObjectConstructor {
	type Output = ObjectConstructor;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		let start = tokens.pop(TokenType::KeywordNew)?.span;

		// Name
		let name = Name::try_parse(tokens, context)?;

		let _compile_time_parameters = if_then_else_default!(tokens.next_is(TokenType::LeftAngleBracket), {
			let mut compile_time_parameters = Vec::new();
			let _ = parse_list!(tokens, ListType::AngleBracketed, {
				let parameter = Parameter::try_parse(tokens, context)?;
				let name = parameter.name();
				let error = Expression::error(Span::unknown(), context);
				if let Err(error) = context.scope_tree.declare_new_variable(name.clone(), error) {
					context.add_diagnostic(Diagnostic {
						span: name.span(context),
						info: DiagnosticInfo::Error(error),
					});
				}
				compile_time_parameters.push(parameter);
			});
			compile_time_parameters
		});

		// Fields
		let mut fields = HashMap::new();
		let end = parse_list!(tokens, ListType::Braced, {
			// Parse tags
			let _tags = if_then_some!(tokens.next_is(TokenType::TagOpening), TagList::try_parse(tokens, context)?);

			// Name
			let field_name = Name::try_parse(tokens, context)?;

			// Value
			let _ = tokens.pop(TokenType::Equal)?;
			let value = Expression::parse(tokens, context);

			// Add field
			_ = fields.insert(field_name, value);
		})
		.span;

		// Return
		Ok(ObjectConstructor {
			type_name: name,
			fields,
			span: start.to(end),
			tags: TagList::default(),
		})
	}
}

impl CompileTime for ObjectConstructor {
	type Output = ExpressionPointer;

	fn evaluate_at_compile_time(mut self, context: &mut Context) -> Self::Output {
		self.tags = self.tags.evaluate_at_compile_time(context);

		// Explicit fields
		for (name, value) in self.fields.clone() {
			let field_value = value.evaluate_at_compile_time(context);
			_ = self.fields.insert(name, field_value);
		}

		if let Ok(literal) = self.try_into_literal(context) {
			Expression::Literal(Literal::Object(literal))
		} else {
			Expression::ObjectConstructor(self)
		}
		.store_in_memory(context)
	}
}

impl TranspileToC for ObjectConstructor {
	fn to_c(&self, _context: &mut Context, _output: Option<String>) -> Result<String, TranspileError> {
		Ok(format!("NULL"))
	}

	fn c_prelude(&self, context: &mut Context) -> Result<String, TranspileError> {
		let mut builder = vec![format!("({}) {{", self.type_name.to_c(context, None)?)];
		for (name, value) in &self.fields {
			builder.push(format!("\t.{} = {}", name.to_c(context, None)?, value.to_c(context, None)?));
		}
		builder.push("}".to_owned());

		Ok(builder.join("\n"))
	}
}

impl Spanned for ObjectConstructor {
	fn span(&self, _context: &Context) -> Span {
		self.span
	}
}

impl ObjectConstructor {
	pub(crate) fn try_into_literal(&self, context: &mut Context) -> Result<Object, ()> {
		let mut fields = HashMap::new();
		for (field_name, field_value) in &self.fields {
			if let Ok(literal) = field_value.try_as_literal(context) {
				let _ = fields.insert(field_name.to_owned(), literal);
			} else {
				return Err(());
			}
		}

		Ok(Object {
			type_name: self.type_name.clone(),
			fields,
		})
	}
}
