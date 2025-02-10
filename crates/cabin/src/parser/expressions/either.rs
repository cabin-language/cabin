use std::{collections::HashMap, fmt::Write as _};

use convert_case::{Case, Casing as _};

use crate::{
	api::{context::context, scope::ScopeId},
	comptime::{memory::VirtualPointer, CompileTime},
	diagnostics::{Diagnostic, DiagnosticInfo, Warning},
	lexer::{Span, TokenType},
	parse_list,
	parser::{
		expressions::{
			field_access::FieldAccessType,
			literal::{CompilerWarning, LiteralConvertible, LiteralObject},
			name::Name,
			object::InternalFieldValue,
			Spanned,
		},
		statements::tag::TagList,
		ListType,
		TokenQueue,
		TokenQueueFunctionality,
		TryParse,
	},
	transpiler::TranspileToC,
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

	fn try_parse(tokens: &mut TokenQueue) -> Result<Self::Output, Diagnostic> {
		let start = tokens.pop(TokenType::KeywordEither)?.span;
		let mut variants = Vec::new();
		let end = parse_list!(tokens, ListType::Braced, {
			let name = Name::try_parse(tokens)?;
			let span = name.span();
			if name.unmangled_name() != name.unmangled_name().to_case(Case::Snake) {
				context().add_diagnostic(Diagnostic {
					span: name.span(),
					info: DiagnosticInfo::Warning(Warning::NonSnakeCaseName {
						original_name: name.unmangled_name().to_owned(),
					}),
				});
			}
			variants.push((name, LiteralObject::empty(span).store_in_memory()));
		})
		.span;

		Ok(Either {
			variants,
			scope_id: context().scope_data.unique_id(),
			name: "anonymous_either".into(),
			span: start.to(end),
			tags: TagList::default(),
		}
		.to_literal()
		.store_in_memory())
	}
}

impl CompileTime for Either {
	type Output = Either;

	fn evaluate_at_compile_time(mut self) -> Self::Output {
		// Tags
		self.tags = self.tags.evaluate_at_compile_time();

		// Warning for empty either
		if self.variants.is_empty() && !self.tags.suppresses_warning(CompilerWarning::EmptyEither) {
			context().add_diagnostic(Diagnostic {
				span: self.span(),
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

impl TranspileToC for Either {
	fn to_c(&self) -> anyhow::Result<String> {
		let mut builder = "{\n".to_owned();
		for (variant_name, _variant_value) in &self.variants {
			write!(builder, "\n\t{},", variant_name.to_c()?).unwrap();
		}

		builder += "\n}";

		Ok(builder)
	}
}

impl Spanned for Either {
	fn span(&self) -> Span {
		self.span.to_owned()
	}
}

impl Either {
	/// Returns the names of the variants in this `either`.
	///
	/// # Returns
	///
	/// The names of the variants in this `either`.
	pub fn variant_names(&self) -> Vec<&Name> {
		self.variants.iter().map(|variant| &variant.0).collect()
	}

	pub fn variants(&self) -> &[(Name, VirtualPointer)] {
		&self.variants
	}

	pub fn set_name(&mut self, name: Name) {
		self.name = name;
		for variant in &mut self.variants {
			variant.1.virtual_deref_mut().type_name = self.name.clone();
		}
	}
}
