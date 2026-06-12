use convert_case::{Case, Casing as _};

use crate::{
	Span,
	Spanned,
	api::{context::Context, scope::ScopeId},
	ast::{
		expressions::{
			Expression,
			identifier::Identifier,
			literal::{EvaluatedLiteral, UnevaluatedLiteral},
		},
		misc::tag::TagList,
		statements::Statement,
	},
	comptime::{CompileTime, memory::ExpressionPointer},
	diagnostics::{Diagnostic, DiagnosticInfo},
	if_then_some,
	interpreter::Runtime,
	lexer::TokenType,
	parser::{Parse as _, TokenQueue, TokenQueueFunctionality as _, TryParse},
	transpiler::{TranspileError, TranspileToC},
};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Declaration {
	name: Identifier,
	scope_id: ScopeId,
	span: Span,
}

impl Declaration {
	pub const fn name(&self) -> &Identifier {
		&self.name
	}

	pub fn value(&self, context: &Context) -> ExpressionPointer {
		context.scope.get_variable_from_id(self.name.source_identifier(), self.scope_id).unwrap()
	}
}

impl TryParse for Declaration {
	type Output = Statement;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		// Tags
		let tags = if_then_some!(tokens.next_is(TokenType::TagOpening), TagList::try_parse(tokens, context)?);

		if tags.is_some() && !tokens.next_is(TokenType::KeywordLet) {
			let expression = Expression::parse(tokens, context);
			let _ = tokens.pop(TokenType::Semicolon, context)?;
			return Ok(Statement::Expression(expression));
		}

		// let
		let start = tokens.pop(TokenType::KeywordLet, context)?.span;

		let visible = if_then_some!(tokens.next_is(TokenType::KeywordVisible), tokens.pop(TokenType::KeywordVisible, context).unwrap());
		let editable = if_then_some!(tokens.next_is(TokenType::KeywordEditable), tokens.pop(TokenType::KeywordVisible, context).unwrap());

		// name
		let name = Identifier::try_parse(tokens, context)?;

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
				if !name.source_identifier().is_case(Case::Pascal) {
					context.add_diagnostic(Diagnostic {
						file: context.file.clone(),
						span: name.span(context),
						info: DiagnosticInfo::NonPascalCaseGroup {
							original_name: name.source_identifier().to_owned(),
							type_name: expression_value.kind_name().to_owned(),
						},
					});
				}
			},
			_ => {
				if !name.source_identifier().is_case(Case::Snake) {
					context.add_diagnostic(Diagnostic {
						file: context.file.clone(),
						span: name.span(context),
						info: DiagnosticInfo::NonSnakeCaseName {
							original_name: name.source_identifier().to_owned(),
						},
					});
				}
			},
		}

		// Add the name declaration to the scope
		if let Err(error) = context.scope.declare_new_variable(name.clone(), value) {
			context.add_diagnostic(Diagnostic {
				file: context.file.clone(),
				span: name.span(context),
				info: error,
			});
		}

		let _ = tokens.pop(TokenType::Semicolon, context)?;

		// Return the declaration
		Ok(Statement::Declaration(Declaration {
			name,
			scope_id: context.scope.unique_id(),
			span: start.to(end),
		}))
	}
}

impl CompileTime for Declaration {
	type Output = Declaration;

	fn evaluate_at_compile_time(self, context: &mut Context) -> Self::Output {
		let evaluated = self.value(context).evaluate_at_compile_time(context); // TODO: use a mapping function instead of cloning
		context.scope.reassign_variable_from_id(&self.name, evaluated, self.scope_id);
		self
	}
}

impl Runtime for Declaration {
	type Output = Declaration;

	fn evaluate_at_runtime(self, context: &mut Context) -> Self::Output {
		let evaluated = self.value(context).evaluate_at_runtime(context);
		context.scope.reassign_variable_from_id(&self.name, evaluated, self.scope_id);
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
