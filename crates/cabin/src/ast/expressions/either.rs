use std::collections::HashMap;

use convert_case::{Case, Casing as _};

use super::{field_access::GetProperty, parameter::EvaluatedParameter};
use crate::{
	Span,
	Spanned,
	api::context::Context,
	ast::{
		expressions::{
			Expression,
			identifier::Identifier,
			literal::{EvaluatedLiteral, Object},
			parameter::Parameter,
		},
		misc::tag::TagList,
	},
	comptime::{CompileTime, memory::ExpressionPointer},
	diagnostics::{Diagnostic, DiagnosticInfo},
	if_then_else_default,
	if_then_some,
	lexer::{Token, TokenType},
	parse_list,
	parser::{ListType, Parse as _, TokenQueue, TokenQueueFunctionality as _, TryParse},
	scope::ScopeType,
	typechecker::Type,
};

/// An `either`. In Cabin, `eithers` represent choices between empty values. They are analogous to
/// something like a Java enum. For example, in Cabin, `true` and `false` aren't keywords; They're
/// `either` variants:
///
/// ```cabin
/// let Boolean = either {
///     true,
///     false
/// };
///
/// let true = Boolean::true;
/// let false = Boolean::false;
/// ```
///
/// This is loosely equivalent to the following;
///
/// ```cabin
/// let true = new Object {};
/// let false = new Object {};
///
/// let Boolean = true | false;
/// ```
#[derive(Debug, Clone)]
pub struct Either {
	variants: Vec<EitherVariant>,
	span: Span,
	tags: TagList,
	compile_time_parameters: Vec<Parameter>,
}

#[derive(Debug, Clone)]
pub struct EitherVariant {
	name: Identifier,
	subtype: Option<ExpressionPointer>,
	value: ExpressionPointer,
}

impl TryParse for Either {
	type Output = Either;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		let start = tokens.pop(TokenType::KeywordEither, context)?.span;
		let mut variants = Vec::new();

		let either_scope = context.scope.enter_new_scope(ScopeType::Either);

		let compile_time_parameters = if_then_else_default!(tokens.next_is(TokenType::LeftAngleBracket), {
			let mut compile_time_parameters = Vec::new();
			_ = parse_list!(tokens, context, ListType::AngleBracketed, {
				let name = Identifier::try_parse(tokens, context)?;
				let mut span = name.span(context);
				let parameter_type = if_then_some!(tokens.next_is(TokenType::Colon), {
					_ = tokens.pop(TokenType::Colon, context)?;
					let parameter_type = Expression::parse(tokens, context);
					span = span.to(parameter_type.span(context));
					parameter_type
				})
				.unwrap_or_else(|| Expression::Identifier(Identifier::create_virtual("Any", context)).store_in_memory(context));
				let error = Expression::error(Span::none(), context);
				if let Err(error) = context.scope.declare_new_variable(name.clone(), error) {
					context.add_diagnostic(Diagnostic {
						file: context.file.clone(),
						span: name.span(context),
						info: error,
					});
				}
				compile_time_parameters.push(Parameter { name, parameter_type, span });
			});
			compile_time_parameters
		});

		let end = parse_list!(tokens, context, ListType::Braced, {
			let name = Identifier::try_parse(tokens, context)?;
			let subtype = if_then_some!(tokens.next_is(TokenType::Colon), {
				_ = tokens.pop(TokenType::Colon, context)?;
				Expression::parse(tokens, context)
			});

			if name.source_identifier() != name.source_identifier().to_case(Case::Snake) {
				context.add_diagnostic(Diagnostic {
					span: name.span(context),
					file: context.file.clone(),
					info: DiagnosticInfo::NonSnakeCaseName {
						original_name: name.source_identifier().to_owned(),
					},
				});
			}

			let variant_span = subtype.as_ref().map_or_else(|| name.span(context), |t| t.span(context));

			variants.push(EitherVariant {
				value: Expression::EvaluatedLiteral(EvaluatedLiteral::Object(Object::synthetic(
					Identifier::synthetic(Token::synthetic(TokenType::Identifier, "Any", name.span(context).to(variant_span)), context),
					HashMap::new(),
					variant_span,
				)))
				.store_in_memory(context),
				name,
				subtype,
			});
		})
		.span;

		context.scope.exit_scope(either_scope).unwrap();

		Ok(Either {
			variants,
			span: start.to(end),
			tags: TagList::default(),
			compile_time_parameters,
		})
	}
}

impl CompileTime for Either {
	type Output = EvaluatedEither;

	fn evaluate_at_compile_time(mut self, context: &mut Context) -> Self::Output {
		// Tags
		self.tags = self.tags.evaluate_at_compile_time(context);

		// Warning for empty either
		if self.variants.is_empty() {
			context.add_diagnostic(Diagnostic {
				span: self.span(context),
				file: context.file.clone(),
				info: DiagnosticInfo::EmptyEither,
			});
		}

		let mut variants = Vec::new();
		for variant in self.variants {
			variants.push(EvaluatedEitherVariant {
				name: variant.name,
				value: variant.value,
				subtype: variant.subtype.map(|subtype| Type::Literal(subtype.evaluate_to_literal(context))),
			});
		}

		EvaluatedEither {
			variants,
			span: self.span,
			tags: self.tags,
			compile_time_parameters: self
				.compile_time_parameters
				.into_iter()
				.map(|parameter| parameter.evaluate_at_compile_time(context))
				.collect(),
		}
	}
}

impl Spanned for Either {
	fn span(&self, _context: &Context) -> Span {
		self.span.to_owned()
	}
}

impl GetProperty for EvaluatedEither {
	fn double_colon(&self, name: &Identifier, _context: &mut Context) -> ExpressionPointer {
		self.variants
			.iter()
			.find_map(|variant| (name == &variant.name).then_some(variant.value))
			.unwrap_or(ExpressionPointer::ERROR)
			.to_owned()
	}

	fn dot(&self, name: &Identifier, _context: &mut Context) -> ExpressionPointer {
		self.variants
			.iter()
			.find_map(|variant| (name == &variant.name).then_some(variant.value))
			.unwrap_or(ExpressionPointer::ERROR)
			.to_owned()
	}
}

#[derive(Debug, Clone)]
pub struct EvaluatedEither {
	variants: Vec<EvaluatedEitherVariant>,
	compile_time_parameters: Vec<EvaluatedParameter>,
	span: Span,
	tags: TagList,
}

#[derive(Debug, Clone)]
pub struct EvaluatedEitherVariant {
	name: Identifier,
	subtype: Option<Type>,
	value: ExpressionPointer,
}
