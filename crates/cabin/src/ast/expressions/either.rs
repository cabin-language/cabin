use convert_case::{Case, Casing as _};

use super::{field_access::Dot, parameter::EvaluatedParameter};
use crate::{
	api::context::Context,
	ast::{
		expressions::{
			literal::{EvaluatedLiteral, Object},
			name::Name,
			parameter::Parameter,
			Expression,
		},
		misc::tag::TagList,
	},
	comptime::{memory::ExpressionPointer, CompileTime},
	diagnostics::{Diagnostic, DiagnosticInfo, Warning},
	if_then_else_default,
	if_then_some,
	io::{IoReader, IoWriter},
	lexer::TokenType,
	parse_list,
	parser::{ListType, Parse, TokenQueue, TokenQueueFunctionality as _, TryParse},
	scope::ScopeType,
	typechecker::Type,
	Span,
	Spanned,
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
/// let true = Boolean.true;
/// let false = Boolean.false;
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
	name: Name,
	subtype: Option<ExpressionPointer>,
	value: ExpressionPointer,
}

impl TryParse for Either {
	type Output = Either;

	fn try_parse<Input: IoReader, Output: IoWriter, Error: IoWriter>(tokens: &mut TokenQueue, context: &mut Context<Input, Output, Error>) -> Result<Self::Output, Diagnostic> {
		let start = tokens.pop(TokenType::KeywordEither, context)?.span;
		let mut variants = Vec::new();

		context.scope_tree.enter_new_scope(ScopeType::Either);

		let compile_time_parameters = if_then_else_default!(tokens.next_is(TokenType::LeftAngleBracket), {
			let mut compile_time_parameters = Vec::new();
			_ = parse_list!(tokens, context, ListType::AngleBracketed, {
				let name = Name::try_parse(tokens, context)?;
				let mut span = name.span(context);
				let parameter_type = if_then_some!(tokens.next_is(TokenType::Colon), {
					_ = tokens.pop(TokenType::Colon, context)?;
					let parameter_type = Expression::parse(tokens, context);
					span = span.to(parameter_type.span(context));
					parameter_type
				})
				.unwrap_or(Expression::Name(Name::new("Anything", context, name.span(context))).store_in_memory(context));
				let error = Expression::error(Span::unknown(), context);
				if let Err(error) = context.scope_tree.declare_new_variable(name.clone(), error) {
					context.add_diagnostic(Diagnostic {
						file: context.file.clone(),
						span: name.span(context),
						info: DiagnosticInfo::Error(error),
					});
				}
				compile_time_parameters.push(Parameter { name, parameter_type, span });
			});
			compile_time_parameters
		});

		let end = parse_list!(tokens, context, ListType::Braced, {
			let name = Name::try_parse(tokens, context)?;
			let subtype = if_then_some!(tokens.next_is(TokenType::Colon), {
				_ = tokens.pop(TokenType::Colon, context)?;
				Expression::parse(tokens, context)
			});

			if name.unmangled_name() != name.unmangled_name().to_case(Case::Snake) {
				context.add_diagnostic(Diagnostic {
					span: name.span(context),
					file: context.file.clone(),
					info: DiagnosticInfo::Warning(Warning::NonSnakeCaseName {
						original_name: name.unmangled_name().to_owned(),
					}),
				});
			}

			variants.push(EitherVariant {
				name,
				subtype,
				value: Expression::EvaluatedLiteral(EvaluatedLiteral::Object(Object::empty())).store_in_memory(context),
			});
		})
		.span;

		context.scope_tree.exit_scope().unwrap();

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

	fn evaluate_at_compile_time<Input: IoReader, Output: IoWriter, Error: IoWriter>(mut self, context: &mut Context<Input, Output, Error>) -> Self::Output {
		// Tags
		self.tags = self.tags.evaluate_at_compile_time(context);

		// Warning for empty either
		if self.variants.is_empty() {
			context.add_diagnostic(Diagnostic {
				span: self.span(context),
				file: context.file.clone(),
				info: DiagnosticInfo::Warning(Warning::EmptyEither),
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
	fn span<Input: IoReader, Output: IoWriter, Error: IoWriter>(&self, _context: &Context<Input, Output, Error>) -> Span {
		self.span.to_owned()
	}
}

impl Dot for EvaluatedEither {
	fn dot<Input: IoReader, Output: IoWriter, Error: IoWriter>(&self, name: &Name, _context: &mut Context<Input, Output, Error>) -> ExpressionPointer {
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
	name: Name,
	subtype: Option<Type>,
	value: ExpressionPointer,
}
