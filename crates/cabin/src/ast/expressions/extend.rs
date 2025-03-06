use std::collections::HashMap;

use super::{new_literal::EvaluatedLiteral, parameter::EvaluatedParameter};
use crate::{
	api::{context::Context, scope::ScopeType},
	ast::{
		expressions::{name::Name, parameter::Parameter, Expression},
		misc::tag::TagList,
	},
	comptime::{
		memory::{ExpressionPointer, LiteralPointer},
		CompileTime,
		CompileTimeError,
	},
	diagnostics::{Diagnostic, DiagnosticInfo, Warning},
	if_then_else_default,
	if_then_some,
	lexer::TokenType,
	parse_list,
	parser::{ListType, Parse as _, TokenQueue, TokenQueueFunctionality as _, TryParse},
	typechecker::{Type, Typed as _},
	Span,
	Spanned,
};

///
/// Normal extension:
///
/// ```cabin
/// let Square = extend Number {
///     square = action(this: Number) {
///         it is this * this;
///     };
/// };
/// ```
///
/// Extension to another type:
///
/// ```cabin
/// let AddPoints = extend Point tobe AddableTo<Point, Point> {
///     plus = action(this: Point, other: Point): Point {
///         it is new Point {
///             x = this.x + other.x,
///             y = this.y + other.y
///         };
///     };
/// };
/// ```
#[derive(Debug, Clone)]
pub struct Extend {
	type_to_extend: ExpressionPointer,
	type_to_be: Option<ExpressionPointer>,
	fields: HashMap<Name, ExpressionPointer>,
	span: Span,
	compile_time_parameters: Vec<Parameter>,
}

impl TryParse for Extend {
	type Output = Extend;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		let start = tokens.pop(TokenType::KeywordExtend, context)?.span;

		context.scope_tree.enter_new_scope(ScopeType::Extend);

		let compile_time_parameters = if_then_else_default!(tokens.next_is(TokenType::LeftAngleBracket), {
			let mut parameters = Vec::new();
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
				};
				parameters.push(parameter);
			});
			parameters
		});

		let type_to_extend = Expression::parse(tokens, context);

		let type_to_be = if_then_some!(tokens.next_is(TokenType::KeywordToBe), {
			let _ = tokens.pop(TokenType::KeywordToBe, context)?;
			Expression::parse(tokens, context)
		});

		let mut fields = HashMap::new();
		let end = parse_list!(tokens, context, ListType::Braced, {
			// Parse tags
			let _tags = if_then_some!(tokens.next_is(TokenType::TagOpening), TagList::try_parse(tokens, context)?);

			// Name
			let name = Name::try_parse(tokens, context)?;

			// Value
			let _ = tokens.pop(TokenType::Equal, context)?;
			let value = Expression::parse(tokens, context);

			// Add field
			_ = fields.insert(name, value);
		})
		.span;

		if fields.is_empty() && type_to_be.is_none() {
			context.add_diagnostic(Diagnostic {
				span: start.to(end),
				file: context.file.clone(),
				info: Warning::EmptyExtension.into(),
			});
		}

		context.scope_tree.exit_scope().unwrap();

		Ok(Extend {
			type_to_extend,
			type_to_be,
			fields,
			span: start.to(end),
			compile_time_parameters,
		})
	}
}

impl CompileTime for Extend {
	type Output = EvaluatedExtend;

	fn evaluate_at_compile_time(self, context: &mut Context) -> Self::Output {
		let type_to_extend = Type::Literal(self.type_to_extend.evaluate_to_literal(context));
		let type_to_be = self.type_to_be.map(|to_be| Type::Literal(to_be.evaluate_to_literal(context)));

		let mut fields = HashMap::new();
		for (name, value) in self.fields {
			let value = value.evaluate_to_literal(context);
			_ = fields.insert(name, value);
		}

		// Evaluate compile-time parameters
		let compile_time_parameters = self
			.compile_time_parameters
			.into_iter()
			.map(|parameter| parameter.evaluate_at_compile_time(context))
			.collect::<Vec<_>>();

		// Validate fields
		if let Some(Type::Literal(type_literal)) = &type_to_be {
			if let EvaluatedLiteral::Group(group) = type_literal.evaluated_literal(context).to_owned() {
				// Missing fields
				for (field_name, field_value) in &fields {
					if !fields.contains_key(field_name) {
						context.add_diagnostic(Diagnostic {
							span: self.span.to(self.type_to_be.as_ref().unwrap().span(context)),
							info: CompileTimeError::MissingField(field_name.unmangled_name().to_owned()).into(),
							file: context.file.clone(),
						});
					}
				}

				// Extra fields
				for (field_name, field_value) in &fields {
					if !group.fields.contains_key(field_name) {
						let literal = field_value.evaluated_literal(context).to_owned();
						context.add_diagnostic(Diagnostic {
							span: field_name.span(context).to(literal.span(context)),
							info: CompileTimeError::ExtraField(field_name.unmangled_name().to_owned()).into(),
							file: context.file.clone(),
						});
					}
				}
			}
			// Not group
			else {
				context.add_diagnostic(Diagnostic {
					span: self.type_to_be.as_ref().unwrap().span(context),
					info: CompileTimeError::ExtendToBeNonGroup.into(),
					file: context.file.clone(),
				});
			}
		}

		EvaluatedExtend {
			type_to_extend,
			type_to_be,
			span: self.span,
			fields,
			compile_time_parameters,
		}
	}
}

impl Spanned for Extend {
	fn span(&self, _context: &Context) -> Span {
		self.span
	}
}

#[derive(Debug, Clone)]
pub struct EvaluatedExtend {
	type_to_extend: Type,
	type_to_be: Option<Type>,
	fields: HashMap<Name, LiteralPointer>,
	span: Span,
	compile_time_parameters: Vec<EvaluatedParameter>,
}
