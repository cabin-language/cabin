use convert_case::{Case, Casing};

use super::Statement;
use crate::{
	api::{context::context, scope::ScopeId},
	comptime::CompileTime,
	if_then_some,
	lexer::{Span, TokenType},
	mapped_err,
	parser::{
		expressions::{name::Name, Expression, Spanned},
		statements::tag::TagList,
		Parse as _,
		TokenQueue,
		TokenQueueFunctionality,
		TryParse,
	},
	transpiler::TranspileToC,
	Diagnostic,
	DiagnosticInfo,
	Warning,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeclarationType {
	Normal,
	RepresentAs,
}

#[derive(Debug, Clone)]
pub struct Declaration {
	name: Name,
	scope_id: ScopeId,
	declaration_type: DeclarationType,
	span: Span,
}

impl Declaration {
	pub const fn name(&self) -> &Name {
		&self.name
	}

	pub fn value(&self) -> &Expression {
		context().scope_data.get_variable_from_id(self.name.clone(), self.scope_id).unwrap()
	}

	pub const fn declaration_type(&self) -> &DeclarationType {
		&self.declaration_type
	}
}

impl TryParse for Declaration {
	type Output = Statement;

	fn try_parse(tokens: &mut TokenQueue) -> Result<Self::Output, Diagnostic> {
		// Tags
		let tags = if_then_some!(tokens.next_is(TokenType::TagOpening), TagList::try_parse(tokens)?);

		if tags.is_some() && !tokens.next_is(TokenType::KeywordLet) {
			let mut expression = Expression::parse(tokens);
			expression.set_tags(tags.unwrap());
			let _ = tokens.pop(TokenType::Semicolon)?;
			return Ok(Statement::Expression(expression));
		}

		// Name
		let start = tokens.pop(TokenType::KeywordLet)?.span;
		let name = Name::try_parse(tokens)?;

		// Value
		let _ = tokens.pop(TokenType::Equal)?;

		let mut value = Expression::parse(tokens);
		let end = value.span();

		if let Expression::Pointer(pointer) = &value {
			let literal = pointer.virtual_deref();
			if literal.type_name() == &"Group".into()
				|| literal.type_name() == &"Either".into()
				|| literal.type_name() == &"OneOf".into()
				|| literal.type_name() == &"RepresentAs".into()
			{
				if name.unmangled_name() != name.unmangled_name().to_case(Case::Pascal) {
					context().add_diagnostic(Diagnostic {
						span: name.span(),
						error: DiagnosticInfo::Warning(Warning::NonPascalCaseGroup {
							original_name: name.unmangled_name().to_owned(),
							type_name: literal.type_name().unmangled_name().to_owned(),
						}),
					});
				}
			} else if name.unmangled_name() != name.unmangled_name().to_case(Case::Snake) {
				context().add_diagnostic(Diagnostic {
					span: name.span(),
					error: DiagnosticInfo::Warning(Warning::NonSnakeCaseName {
						original_name: name.unmangled_name().to_owned(),
					}),
				});
			}
		} else if name.unmangled_name() != name.unmangled_name().to_case(Case::Snake) {
			context().add_diagnostic(Diagnostic {
				span: name.span(),
				error: DiagnosticInfo::Warning(Warning::NonSnakeCaseName {
					original_name: name.unmangled_name().to_owned(),
				}),
			});
		}

		// Tags
		if let Some(tags) = tags {
			value.set_tags(tags);
		}

		// Set name
		value.try_set_name(name.clone());
		value.try_set_scope_label(name.clone());

		// Add the name declaration to the scope
		context().scope_data.declare_new_variable(name.clone(), value)?;

		let _ = tokens.pop(TokenType::Semicolon)?;

		// Return the declaration
		Ok(Statement::Declaration(Declaration {
			name,
			scope_id: context().scope_data.unique_id(),
			declaration_type: DeclarationType::Normal,
			span: start.to(end),
		}))
	}
}

impl CompileTime for Declaration {
	type Output = Declaration;

	fn evaluate_at_compile_time(self) -> Self::Output {
		let evaluated = self.value().clone().evaluate_at_compile_time(); // TODO: use a mapping function instead of cloning
		context().scope_data.reassign_variable_from_id(&self.name, evaluated, self.scope_id);
		self
	}
}

impl TranspileToC for Declaration {
	fn to_c(&self) -> anyhow::Result<String> {
		Ok(format!(
			"void* {} = {};",
			self.name.to_c()?,
			self.value().to_c().map_err(mapped_err! {
				while = format!("transpiling the value of the initial declaration for the variable \"{}\" to C", self.name.unmangled_name()),
			})?
		))
	}
}

impl Spanned for Declaration {
	fn span(&self) -> Span {
		self.span
	}
}
