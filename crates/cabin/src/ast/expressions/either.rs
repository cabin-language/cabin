use convert_case::{Case, Casing as _};

use super::field_access::Dot;
use crate::{
	api::context::Context,
	ast::{
		expressions::{
			name::Name,
			new_literal::{Literal, Object},
			Expression,
		},
		misc::tag::TagList,
	},
	comptime::{memory::ExpressionPointer, CompileTime},
	diagnostics::{Diagnostic, DiagnosticInfo, Warning},
	lexer::TokenType,
	parse_list,
	parser::{ListType, TokenQueue, TokenQueueFunctionality as _, TryParse},
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
	variants: Vec<(Name, ExpressionPointer)>,
	span: Span,
	tags: TagList,
}

impl TryParse for Either {
	type Output = Either;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		let start = tokens.pop(TokenType::KeywordEither)?.span;
		let mut variants = Vec::new();
		let end = parse_list!(tokens, ListType::Braced, {
			let name = Name::try_parse(tokens, context)?;
			if name.unmangled_name() != name.unmangled_name().to_case(Case::Snake) {
				context.add_diagnostic(Diagnostic {
					span: name.span(context),
					info: DiagnosticInfo::Warning(Warning::NonSnakeCaseName {
						original_name: name.unmangled_name().to_owned(),
					}),
				});
			}
			variants.push((name, Expression::Literal(Literal::Object(Object::empty())).store_in_memory(context)));
		})
		.span;

		Ok(Either {
			variants,
			span: start.to(end),
			tags: TagList::default(),
		})
	}
}

impl CompileTime for Either {
	type Output = Either;

	fn evaluate_at_compile_time(mut self, context: &mut Context) -> Self::Output {
		// Tags
		self.tags = self.tags.evaluate_at_compile_time(context);

		// Warning for empty either
		if self.variants.is_empty() {
			context.add_diagnostic(Diagnostic {
				span: self.span(context),
				info: DiagnosticInfo::Warning(Warning::EmptyEither),
			});
		}

		self
	}
}

impl Spanned for Either {
	fn span(&self, _context: &Context) -> Span {
		self.span.to_owned()
	}
}

impl Dot for Either {
	fn dot(&self, name: &Name, context: &mut Context) -> ExpressionPointer {
		self.variants
			.iter()
			.find_map(|(variant_name, value)| (name == variant_name).then_some(value))
			.unwrap_or(&ExpressionPointer::ERROR)
			.to_owned()
	}
}
