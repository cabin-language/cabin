use std::collections::HashMap;

use convert_case::{Case, Casing as _};

use crate::{
	api::{context::Context, scope::ScopeId},
	ast::{
		expressions::{
			field_access::FieldAccessType,
			literal::{LiteralConvertible, LiteralObject},
			name::Name,
			object::InternalFieldValue,
		},
		misc::tag::TagList,
	},
	comptime::{memory::VirtualPointer, CompileTime},
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
	variants: Vec<(Name, VirtualPointer)>,
	scope_id: ScopeId,
	name: Name,
	span: Span,
	tags: TagList,
}

impl TryParse for Either {
	type Output = VirtualPointer;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		let start = tokens.pop(TokenType::KeywordEither)?.span;
		let mut variants = Vec::new();
		let end = parse_list!(tokens, ListType::Braced, {
			let name = Name::try_parse(tokens, context)?;
			let span = name.span(context);
			if name.unmangled_name() != name.unmangled_name().to_case(Case::Snake) {
				context.add_diagnostic(Diagnostic {
					span: name.span(context),
					info: DiagnosticInfo::Warning(Warning::NonSnakeCaseName {
						original_name: name.unmangled_name().to_owned(),
					}),
				});
			}
			variants.push((name, LiteralObject::empty(span, context).store_in_memory(context)));
		})
		.span;

		Ok(Either {
			variants,
			scope_id: context.scope_tree.unique_id(),
			name: "anonymous_either".into(),
			span: start.to(end),
			tags: TagList::default(),
		}
		.to_literal()
		.store_in_memory(context))
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

impl LiteralConvertible for Either {
	fn to_literal(self) -> LiteralObject {
		LiteralObject {
			address: None,
			fields: HashMap::from([]),
			internal_fields: HashMap::from([("variants".to_owned(), InternalFieldValue::LiteralMap(self.variants))]),
			name: self.name,
			field_access_type: FieldAccessType::Either,
			outer_scope_id: self.scope_id,
			inner_scope_id: Some(self.scope_id),
			span: self.span,
			type_name: "Either".into(),
			tags: self.tags,
		}
	}

	fn from_literal(literal: &LiteralObject) -> anyhow::Result<Self> {
		Ok(Either {
			variants: literal.get_internal_field::<Vec<(Name, VirtualPointer)>>("variants")?.to_owned(),
			scope_id: literal.outer_scope_id(),
			name: literal.name.clone(),
			span: literal.span,
			tags: literal.tags.clone(),
		})
	}
}

impl Spanned for Either {
	fn span(&self, _context: &Context) -> Span {
		self.span.to_owned()
	}
}
