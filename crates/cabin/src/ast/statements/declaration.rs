use convert_case::{Case, Casing as _};

use crate::{
	api::{context::Context, scope::ScopeId},
	ast::{
		expressions::{name::Name, Expression, Spanned},
		misc::tag::TagList,
		statements::Statement,
	},
	comptime::CompileTime,
	diagnostics::{Diagnostic, DiagnosticInfo, Warning},
	if_then_some,
	lexer::{Span, TokenType},
	parser::{Parse as _, TokenQueue, TokenQueueFunctionality as _, TryParse},
	transpiler::{TranspileError, TranspileToC},
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

	pub fn value<'a>(&self, context: &'a Context) -> &'a Expression {
		context.scope_tree.get_variable_from_id(self.name.clone(), self.scope_id).unwrap()
	}

	pub const fn declaration_type(&self) -> &DeclarationType {
		&self.declaration_type
	}
}

impl TryParse for Declaration {
	type Output = Statement;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		// Tags
		let tags = if_then_some!(tokens.next_is(TokenType::TagOpening), TagList::try_parse(tokens, context)?);

		if tags.is_some() && !tokens.next_is(TokenType::KeywordLet) {
			let mut expression = Expression::parse(tokens, context);
			expression.set_tags(tags.unwrap(), context);
			let _ = tokens.pop(TokenType::Semicolon)?;
			return Ok(Statement::Expression(expression));
		}

		// Name
		let start = tokens.pop(TokenType::KeywordLet)?.span;
		let name = Name::try_parse(tokens, context)?;

		// Value
		let _ = tokens.pop(TokenType::Equal)?;

		let mut value = Expression::parse(tokens, context);
		let end = value.span(context);

		if let Expression::Pointer(pointer) = &value {
			let literal = pointer.virtual_deref(context);
			if literal.type_name() == &"Group".into()
				|| literal.type_name() == &"Either".into()
				|| literal.type_name() == &"OneOf".into()
				|| literal.type_name() == &"RepresentAs".into()
			{
				if !name.unmangled_name().is_case(Case::Pascal) {
					context.add_diagnostic(Diagnostic {
						span: name.span(context),
						info: DiagnosticInfo::Warning(Warning::NonPascalCaseGroup {
							original_name: name.unmangled_name().to_owned(),
							type_name: literal.type_name().unmangled_name().to_owned(),
						}),
					});
				}
			} else if !name.unmangled_name().is_case(Case::Snake) {
				context.add_diagnostic(Diagnostic {
					span: name.span(context),
					info: DiagnosticInfo::Warning(Warning::NonSnakeCaseName {
						original_name: name.unmangled_name().to_owned(),
					}),
				});
			}
		} else if !name.unmangled_name().is_case(Case::Snake) {
			context.add_diagnostic(Diagnostic {
				span: name.span(context),
				info: DiagnosticInfo::Warning(Warning::NonSnakeCaseName {
					original_name: name.unmangled_name().to_owned(),
				}),
			});
		}

		// Tags
		if let Some(tags) = tags {
			value.set_tags(tags, context);
		}

		// Set name
		value.try_set_name(name.clone(), context);
		value.try_set_scope_label(name.clone(), context);

		// Add the name declaration to the scope
		if let Err(error) = context.scope_tree.declare_new_variable(name.clone(), value) {
			context.add_diagnostic(Diagnostic {
				span: name.span(context),
				info: DiagnosticInfo::Error(error),
			});
		}

		let _ = tokens.pop(TokenType::Semicolon)?;

		// Return the declaration
		Ok(Statement::Declaration(Declaration {
			name,
			scope_id: context.scope_tree.unique_id(),
			declaration_type: DeclarationType::Normal,
			span: start.to(end),
		}))
	}
}

impl CompileTime for Declaration {
	type Output = Declaration;

	fn evaluate_at_compile_time(self, context: &mut Context) -> Self::Output {
		let evaluated = self.value(context).clone().evaluate_at_compile_time(context); // TODO: use a mapping function instead of cloning
		context.scope_tree.reassign_variable_from_id(&self.name, evaluated, self.scope_id);
		self
	}
}

impl TranspileToC for Declaration {
	fn to_c(&self, context: &mut Context, _output: Option<String>) -> Result<String, TranspileError> {
		let name = self.name().to_c(context, None)?;
		Ok(format!(
			"void* {name};\n{};\nlabel_end_{name}:;\n\n",
			self.value(context).to_owned().to_c(context, Some(name.clone()))?
		))
	}

	fn c_prelude(&self, context: &mut Context) -> Result<String, TranspileError> {
		self.value(context).to_owned().c_prelude(context)
	}

	fn c_type_prelude(&self, context: &mut Context) -> Result<String, TranspileError> {
		self.value(context).to_owned().c_type_prelude(context)
	}
}

impl Spanned for Declaration {
	fn span(&self, _context: &Context) -> Span {
		self.span
	}
}
