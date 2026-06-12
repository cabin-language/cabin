use std::collections::HashMap;

use super::literal::{EvaluatedLiteral, Object};
use crate::{
	Span,
	api::context::Context,
	ast::{
		expressions::{Expression, Spanned, identifier::Identifier},
		misc::tag::TagList,
	},
	comptime::{
		CompileTime,
		memory::{ExpressionPointer, LiteralPointer},
	},
	diagnostics::{Diagnostic, DiagnosticInfo},
	if_then_some,
	lexer::TokenType,
	parse_list,
	parser::{ListType, Parse as _, TokenQueue, TokenQueueFunctionality as _, TryParse},
	transpiler::{TranspileError, TranspileToC},
	typechecker::{Type, Typed},
};

#[derive(Debug, Clone)]
pub struct NewExpression {
	pub type_name: Identifier,
	pub fields: HashMap<Identifier, ExpressionPointer>,
	pub span: Span,
	pub tags: TagList,
}

impl TryParse for NewExpression {
	type Output = NewExpression;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		let start = tokens.pop(TokenType::KeywordNew, context)?.span;

		// Name
		let name = Identifier::try_parse(tokens, context)?;

		// Fields
		let mut fields = HashMap::new();
		let end = parse_list!(tokens, context, ListType::Braced, {
			let mut documentation = if_then_some!(tokens.next_is(TokenType::Comment), tokens.pop(TokenType::Comment, context).unwrap().value);

			// Parse tags
			let tags = if_then_some!(tokens.next_is(TokenType::TagOpening), TagList::try_parse(tokens, context)?);

			if documentation.is_none() && tokens.next_is(TokenType::Comment) {
				documentation = Some(tokens.pop(TokenType::Comment, context).unwrap().value);
			}

			// Name
			let mut field_name = Identifier::try_parse(tokens, context)?;
			field_name.documentation = documentation.clone();

			// Value
			let _ = tokens.pop(TokenType::Equal, context)?;
			let value = Expression::parse(tokens, context);
			if let Some(tags) = tags {
				value.expression_mut(context).set_tags(tags);
			}

			if let Some(documentation) = documentation {
				value.expression_mut(context).set_documentation(&documentation);
			}

			// Add field
			_ = fields.insert(field_name, value);
		})
		.span;

		// Return
		Ok(NewExpression {
			type_name: name,
			fields,
			span: start.to(end),
			tags: TagList::default(),
		})
	}
}

impl CompileTime for NewExpression {
	type Output = Expression;

	fn evaluate_at_compile_time(mut self, context: &mut Context) -> Self::Output {
		self.tags = self.tags.evaluate_at_compile_time(context);

		// Explicit fields
		for (name, value) in self.fields.clone() {
			let field_value = value.evaluate_at_compile_time(context);
			_ = self.fields.insert(name, field_value);
		}

		// Validate fields
		if self.type_name.source_identifier() != "Any" {
			let object_type = self.get_type(context);
			let Type::Literal(type_literal) = object_type;
			if let EvaluatedLiteral::Group(group) = type_literal.evaluated_literal(context).to_owned() {
				for (field_name, field) in &group.fields {
					// Wrong field type
					if let Some(field_value) = self.fields.get(field_name) {
						let expected_type = field_value.get_type(context);
						if !field_value.get_type(context).is_assignable_to(&field.field_type, context) {
							context.add_diagnostic(Diagnostic {
								span: field_value.span(context),
								info: DiagnosticInfo::TypeMismatch(expected_type, field.field_type.clone()),
								file: context.file.clone(),
							});
						}
					}
					// Missing field
					else {
						context.add_diagnostic(Diagnostic {
							span: self.span.to(self.type_name.span(context)),
							info: DiagnosticInfo::MissingField(field_name.source_identifier().to_owned()),
							file: context.file.clone(),
						});
					}
				}

				// Extra fields
				for field_name in self.fields.keys() {
					if !group.fields.contains_key(field_name) {
						context.add_diagnostic(Diagnostic {
							span: field_name.span(context),
							info: DiagnosticInfo::ExtraField(field_name.source_identifier().to_owned()),
							file: context.file.clone(),
						});
					}
				}
			}
		}

		if let Ok(literal) = self.try_into_literal(context) {
			Expression::EvaluatedLiteral(EvaluatedLiteral::Object(literal))
		} else {
			Expression::New(self)
		}
	}
}

impl Typed for NewExpression {
	fn get_type(&self, context: &mut Context) -> Type {
		Type::Literal(self.type_name.value(context).map_or(LiteralPointer::ERROR, |value| value.as_literal(context)))
	}
}

impl TranspileToC for NewExpression {
	fn to_c(&self, _context: &mut Context, _output: Option<String>) -> Result<String, TranspileError> {
		Ok("NULL".to_owned()) // TODO: obj constr c
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

impl Spanned for NewExpression {
	fn span(&self, _context: &Context) -> Span {
		self.span
	}
}

impl NewExpression {
	pub fn try_into_literal(&self, context: &mut Context) -> Result<Object, ()> {
		let mut fields = HashMap::new();
		for (field_name, field_value) in &self.fields {
			if let Ok(literal) = field_value.try_as_literal(context) {
				let _ = fields.insert(field_name.to_owned(), literal);
			} else {
				return Err(());
			}
		}

		Ok(Object {
			span: self.span,
			type_name: self.type_name.clone(),
			fields,
		})
	}
}
