use std::collections::{HashMap, VecDeque};

use convert_case::{Case, Casing as _};

use crate::{
	api::{context::Context, scope::ScopeType},
	ast::{
		expressions::{name::Name, parameter::Parameter, Expression},
		misc::tag::TagList,
	},
	comptime::{
		memory::{ExpressionPointer, LiteralPointer},
		CompileTime,
	},
	diagnostics::{Diagnostic, DiagnosticInfo, Warning},
	if_then_else_default,
	if_then_some,
	io::{IoReader, IoWriter},
	lexer::{Token, TokenType},
	parse_list,
	parser::{ListType, Parse as _, ParseError, TokenQueueFunctionality as _, TryParse},
	typechecker::{Type, Typed as _},
	Span,
	Spanned,
};

#[derive(Debug, Clone)]
pub struct GroupField {
	name: Name,
	default_value: Option<ExpressionPointer>,
	field_type: Option<ExpressionPointer>,
}

#[derive(Debug, Clone)]
pub struct GroupFieldLiteral {
	name: Name,
	default_value: Option<LiteralPointer>,
	pub(crate) field_type: Type,
}

#[derive(Debug, Clone)]
pub struct GroupDeclaration {
	fields: HashMap<Name, GroupField>,
	span: Span,
	pub(crate) name: Option<Name>,
}

impl TryParse for GroupDeclaration {
	type Output = GroupDeclaration;

	fn try_parse<Input: IoReader, Output: IoWriter, Error: IoWriter>(
		tokens: &mut VecDeque<Token>,
		context: &mut Context<Input, Output, Error>,
	) -> Result<Self::Output, Diagnostic> {
		let start = tokens.pop(TokenType::KeywordGroup, context)?.span;
		context.scope_tree.enter_new_scope(ScopeType::Group);

		let _compile_time_parameters = if_then_else_default!(tokens.next_is(TokenType::LeftAngleBracket), {
			let mut compile_time_parameters = Vec::new();
			let _ = parse_list!(tokens, context, ListType::AngleBracketed, {
				let parameter = Parameter::try_parse(tokens, context)?;
				let name = parameter.name().to_owned();
				let error = Expression::error(Span::unknown(), context);
				if let Err(error) = context.scope_tree.declare_new_variable(name.clone(), error) {
					context.add_diagnostic(Diagnostic {
						file: context.file.clone(),
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
		let end = parse_list!(tokens, context, ListType::Braced, {
			let mut documentation = if_then_some!(tokens.next_is(TokenType::Comment), tokens.pop(TokenType::Comment, context).unwrap().value);

			//  Group field tags
			let tags = if_then_some!(tokens.next_is(TokenType::TagOpening), TagList::try_parse(tokens, context)?);

			if documentation.is_none() && tokens.next_is(TokenType::Comment) {
				documentation = Some(tokens.pop(TokenType::Comment, context).unwrap().value);
			}

			// Group field name
			let name = Name::try_parse(tokens, context)?;
			if !name.unmangled_name().is_case(Case::Snake) {
				context.add_diagnostic(Diagnostic {
					file: context.file.clone(),
					span: name.span(context),
					info: DiagnosticInfo::Warning(Warning::NonSnakeCaseName {
						original_name: name.unmangled_name().to_owned(),
					}),
				});
			}

			if fields.keys().any(|field_name| field_name == &name) {
				context.add_diagnostic(Diagnostic {
					file: context.file.clone(),
					span: name.span(context),
					info: DiagnosticInfo::Error(crate::Error::Parse(ParseError::DuplicateField(name.unmangled_name().to_owned()))),
				});
			}

			// Group field type
			let field_type = if_then_some!(tokens.next_is(TokenType::Colon), {
				let _ = tokens.pop(TokenType::Colon, context)?;
				Expression::parse(tokens, context)
			});

			// Group field value
			let value = if_then_some!(tokens.next_is(TokenType::Equal), {
				let _ = tokens.pop(TokenType::Equal, context)?;
				let value = Expression::parse(tokens, context);
				if let Some(tags) = tags {
					value.expression_mut(context).set_tags(tags);
				}

				value
			});

			// Add field
			_ = fields.insert(name.clone(), GroupField {
				name,
				default_value: value,
				field_type,
			});
		})
		.span;
		context.scope_tree.exit_scope().unwrap();

		Ok(GroupDeclaration {
			fields,
			span: start.to(end),
			name: None,
		})
	}
}

impl CompileTime for GroupDeclaration {
	type Output = EvaluatedGroupDeclaration;

	fn evaluate_at_compile_time<Input: IoReader, Output: IoWriter, Error: IoWriter>(self, context: &mut Context<Input, Output, Error>) -> Self::Output {
		let mut fields = HashMap::new();

		for (name, field) in self.fields {
			// Field value
			let value = field.default_value.map(|value| value.evaluate_to_literal(context));

			// Field type
			let field_type = if let Some(field_type) = field.field_type {
				Type::Literal(field_type.evaluate_to_literal(context))
			} else if let Some(default_value) = field.default_value {
				default_value.get_type(context)
			} else {
				Type::Literal(LiteralPointer::ERROR)
			};

			// Add the field
			_ = fields.insert(name.clone(), GroupFieldLiteral {
				name,
				default_value: value,
				field_type,
			});
		}

		// Store in memory and return a pointer
		EvaluatedGroupDeclaration {
			fields,
			span: self.span,
			name: self.name,
		}
	}
}

impl Spanned for GroupDeclaration {
	fn span<Input: IoReader, Output: IoWriter, Error: IoWriter>(&self, _context: &Context<Input, Output, Error>) -> Span {
		self.span
	}
}

#[derive(Debug, Clone)]
pub struct EvaluatedGroupDeclaration {
	pub(crate) fields: HashMap<Name, GroupFieldLiteral>,
	span: Span,
	pub(crate) name: Option<Name>,
}
