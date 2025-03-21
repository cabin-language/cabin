use std::collections::HashMap;

use super::literal::{EvaluatedLiteral, Object};
use crate::{
	api::context::Context,
	ast::{
		expressions::{name::Name, parameter::Parameter, Expression, Spanned},
		misc::tag::TagList,
	},
	comptime::{
		memory::{ExpressionPointer, LiteralPointer},
		CompileTime,
		CompileTimeError,
	},
	diagnostics::{Diagnostic, DiagnosticInfo},
	if_then_else_default,
	if_then_some,
	io::{IoReader, IoWriter},
	lexer::TokenType,
	parse_list,
	parser::{ListType, Parse as _, TokenQueue, TokenQueueFunctionality as _, TryParse},
	transpiler::{TranspileError, TranspileToC},
	typechecker::{Type, Typed},
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

	fn try_parse<Input: IoReader, Output: IoWriter, Error: IoWriter>(tokens: &mut TokenQueue, context: &mut Context<Input, Output, Error>) -> Result<Self::Output, Diagnostic> {
		let start = tokens.pop(TokenType::KeywordNew, context)?.span;

		// Name
		let name = Name::try_parse(tokens, context)?;

		let _compile_time_parameters = if_then_else_default!(tokens.next_is(TokenType::LeftAngleBracket), {
			let mut compile_time_parameters = Vec::new();
			let _ = parse_list!(tokens, context, ListType::AngleBracketed, {
				let parameter = Parameter::try_parse(tokens, context)?;
				let parameter_name = parameter.name();
				let error = Expression::error(Span::unknown(), context);
				if let Err(error) = context.scope_tree.declare_new_variable(parameter_name.clone(), error) {
					context.add_diagnostic(Diagnostic {
						file: context.file.clone(),
						span: parameter_name.span(context),
						info: DiagnosticInfo::Error(error),
					});
				}
				compile_time_parameters.push(parameter);
			});
			compile_time_parameters
		});

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
			let mut field_name = Name::try_parse(tokens, context)?;
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
		Ok(ObjectConstructor {
			type_name: name,
			fields,
			span: start.to(end),
			tags: TagList::default(),
		})
	}
}

impl CompileTime for ObjectConstructor {
	type Output = Expression;

	fn evaluate_at_compile_time<Input: IoReader, Output: IoWriter, Error: IoWriter>(mut self, context: &mut Context<Input, Output, Error>) -> Self::Output {
		self.tags = self.tags.evaluate_at_compile_time(context);

		// Explicit fields
		for (name, value) in self.fields.clone() {
			let field_value = value.evaluate_at_compile_time(context);
			_ = self.fields.insert(name, field_value);
		}

		// Validate fields
		if self.type_name != "Object".into() {
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
								info: CompileTimeError::TypeMismatch(expected_type, field.field_type.clone()).into(),
								file: context.file.clone(),
							});
						}
					}
					// Missing field
					else {
						context.add_diagnostic(Diagnostic {
							span: self.span.to(self.type_name.span(context)),
							info: CompileTimeError::MissingField(field_name.unmangled_name().to_owned()).into(),
							file: context.file.clone(),
						});
					}
				}

				// Extra fields
				for field_name in self.fields.keys() {
					if !group.fields.contains_key(field_name) {
						context.add_diagnostic(Diagnostic {
							span: field_name.span(context),
							info: CompileTimeError::ExtraField(field_name.unmangled_name().to_owned()).into(),
							file: context.file.clone(),
						});
					}
				}
			}
		}

		if let Ok(literal) = self.try_into_literal(context) {
			Expression::EvaluatedLiteral(EvaluatedLiteral::Object(literal))
		} else {
			Expression::ObjectConstructor(self)
		}
	}
}

impl Typed for ObjectConstructor {
	fn get_type<Input: IoReader, Output: IoWriter, Error: IoWriter>(&self, context: &mut Context<Input, Output, Error>) -> Type {
		Type::Literal(self.type_name.value(context).map_or(LiteralPointer::ERROR, |value| value.as_literal(context)))
	}
}

impl TranspileToC for ObjectConstructor {
	fn to_c<Input: IoReader, Output: IoWriter, Error: IoWriter>(&self, _context: &mut Context<Input, Output, Error>, _output: Option<String>) -> Result<String, TranspileError> {
		Ok("NULL".to_owned()) // TODO: obj constr c
	}

	fn c_prelude<Input: IoReader, Output: IoWriter, Error: IoWriter>(&self, context: &mut Context<Input, Output, Error>) -> Result<String, TranspileError> {
		let mut builder = vec![format!("({}) {{", self.type_name.to_c(context, None)?)];
		for (name, value) in &self.fields {
			builder.push(format!("\t.{} = {}", name.to_c(context, None)?, value.to_c(context, None)?));
		}
		builder.push("}".to_owned());

		Ok(builder.join("\n"))
	}
}

impl Spanned for ObjectConstructor {
	fn span<Input: IoReader, Output: IoWriter, Error: IoWriter>(&self, _context: &Context<Input, Output, Error>) -> Span {
		self.span
	}
}

impl ObjectConstructor {
	pub(crate) fn try_into_literal<Input: IoReader, Output: IoWriter, Error: IoWriter>(&self, context: &mut Context<Input, Output, Error>) -> Result<Object, ()> {
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
