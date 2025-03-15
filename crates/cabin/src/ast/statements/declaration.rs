use convert_case::{Case, Casing as _};

use crate::{
	api::{context::Context, scope::ScopeId},
	ast::{
		expressions::{
			literal::{EvaluatedLiteral, UnevaluatedLiteral},
			name::Name,
			Expression,
		},
		misc::tag::TagList,
		statements::Statement,
	},
	comptime::{memory::ExpressionPointer, CompileTime},
	diagnostics::{Diagnostic, DiagnosticInfo, Warning},
	if_then_some,
	interpreter::Runtime,
	io::{IoReader, IoWriter},
	lexer::TokenType,
	parser::{Parse as _, TokenQueue, TokenQueueFunctionality as _, TryParse},
	transpiler::{TranspileError, TranspileToC},
	Span,
	Spanned,
};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Declaration {
	name: Name,
	scope_id: ScopeId,
	span: Span,
}

impl Declaration {
	pub(crate) const fn name(&self) -> &Name {
		&self.name
	}

	pub(crate) fn value<Input: IoReader, Output: IoWriter, Error: IoWriter>(&self, context: &Context<Input, Output, Error>) -> ExpressionPointer {
		context.scope_tree.get_variable_from_id(self.name.clone(), self.scope_id).unwrap()
	}
}

impl TryParse for Declaration {
	type Output = Statement;

	fn try_parse<Input: IoReader, Output: IoWriter, Error: IoWriter>(tokens: &mut TokenQueue, context: &mut Context<Input, Output, Error>) -> Result<Self::Output, Diagnostic> {
		// Tags
		let tags = if_then_some!(tokens.next_is(TokenType::TagOpening), TagList::try_parse(tokens, context)?);

		if tags.is_some() && !tokens.next_is(TokenType::KeywordLet) {
			let expression = Expression::parse(tokens, context);
			let _ = tokens.pop(TokenType::Semicolon, context)?;
			return Ok(Statement::Expression(expression));
		}

		// Name
		let start = tokens.pop(TokenType::KeywordLet, context)?.span;
		let name = Name::try_parse(tokens, context)?;

		// Value
		let _ = tokens.pop(TokenType::Equal, context)?;

		let value = Expression::parse(tokens, context);
		let end = value.span(context);

		let expression = value.expression_mut(context);
		expression.set_name(name.clone());
		let expression_value = value.expression(context);

		match expression_value {
			Expression::Literal(UnevaluatedLiteral::Group(_) | UnevaluatedLiteral::Either(_) | UnevaluatedLiteral::Extend(_))
			| Expression::EvaluatedLiteral(EvaluatedLiteral::Group(_) | EvaluatedLiteral::Extend(_) | EvaluatedLiteral::Either(_)) => {
				if !name.unmangled_name().is_case(Case::Pascal) {
					context.add_diagnostic(Diagnostic {
						file: context.file.clone(),
						span: name.span(context),
						info: DiagnosticInfo::Warning(Warning::NonPascalCaseGroup {
							original_name: name.unmangled_name().to_owned(),
							type_name: expression_value.kind_name().to_owned(),
						}),
					});
				}
			},
			_ => {
				if !name.unmangled_name().is_case(Case::Snake) {
					context.add_diagnostic(Diagnostic {
						file: context.file.clone(),
						span: name.span(context),
						info: DiagnosticInfo::Warning(Warning::NonSnakeCaseName {
							original_name: name.unmangled_name().to_owned(),
						}),
					});
				}
			},
		}

		// Add the name declaration to the scope
		if let Err(error) = context.scope_tree.declare_new_variable(name.clone(), value) {
			context.add_diagnostic(Diagnostic {
				file: context.file.clone(),
				span: name.span(context),
				info: DiagnosticInfo::Error(error),
			});
		}

		let _ = tokens.pop(TokenType::Semicolon, context)?;

		// Return the declaration
		Ok(Statement::Declaration(Declaration {
			name,
			scope_id: context.scope_tree.unique_id(),
			span: start.to(end),
		}))
	}
}

impl CompileTime for Declaration {
	type Output = Declaration;

	fn evaluate_at_compile_time<Input: IoReader, Output: IoWriter, Error: IoWriter>(self, context: &mut Context<Input, Output, Error>) -> Self::Output {
		let evaluated = self.value(context).evaluate_at_compile_time(context); // TODO: use a mapping function instead of cloning
		context.scope_tree.reassign_variable_from_id(&self.name, evaluated, self.scope_id);
		self
	}
}

impl Runtime for Declaration {
	type Output = Declaration;

	fn evaluate_at_runtime<Input: IoReader, Output: IoWriter, Error: IoWriter>(self, context: &mut Context<Input, Output, Error>) -> Self::Output {
		let evaluated = self.value(context).evaluate_at_runtime(context);
		context.scope_tree.reassign_variable_from_id(&self.name, evaluated, self.scope_id);
		self
	}
}

impl TranspileToC for Declaration {
	fn to_c<Input: IoReader, Output: IoWriter, Error: IoWriter>(&self, context: &mut Context<Input, Output, Error>, _output: Option<String>) -> Result<String, TranspileError> {
		let name = self.name().to_c(context, None)?;
		Ok(format!(
			"void* {name};\n{};\nlabel_end_{name}:;\n\n",
			self.value(context).to_owned().to_c(context, Some(name.clone()))?
		))
	}

	fn c_prelude<Input: IoReader, Output: IoWriter, Error: IoWriter>(&self, context: &mut Context<Input, Output, Error>) -> Result<String, TranspileError> {
		self.value(context).to_owned().c_prelude(context)
	}

	fn c_type_prelude<Input: IoReader, Output: IoWriter, Error: IoWriter>(&self, context: &mut Context<Input, Output, Error>) -> Result<String, TranspileError> {
		self.value(context).to_owned().c_type_prelude(context)
	}
}

impl Spanned for Declaration {
	fn span<Input: IoReader, Output: IoWriter, Error: IoWriter>(&self, _context: &Context<Input, Output, Error>) -> Span {
		self.span
	}
}
