use std::{collections::HashMap, fmt::Debug, sync::LazyLock};

use crate::{
	api::{
		context::Context,
		scope::{ScopeId, ScopeType},
	},
	ast::{
		expressions::{
			block::Block,
			field_access::FieldAccessType,
			literal::{LiteralConvertible, LiteralObject},
			name::Name,
			object::InternalFieldValue,
			parameter::Parameter,
			Expression,
		},
		misc::tag::TagList,
	},
	comptime::{memory::VirtualPointer, CompileTime},
	diagnostics::{Diagnostic, DiagnosticInfo},
	if_then_else_default,
	if_then_some,
	lexer::TokenType,
	parse_list,
	parser::{ListType, Parse as _, TokenQueue, TokenQueueFunctionality as _, TryParse},
	Span,
	Spanned,
};

#[derive(Debug, Clone)]
pub struct FunctionDeclaration {
	tags: TagList,
	compile_time_parameters: Vec<Parameter>,
	parameters: Vec<Parameter>,
	return_type: Option<Expression>,
	body: Option<Expression>,
	outer_scope_id: ScopeId,
	inner_scope_id: Option<ScopeId>,
	this_object: Option<Expression>,
	name: Name,
	span: Span,
}

impl TryParse for FunctionDeclaration {
	type Output = VirtualPointer;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		// "function" keyword
		let start = tokens.pop(TokenType::KeywordAction)?.span;
		let mut end = start;

		// Compile-time parameters
		let compile_time_parameters = if_then_else_default!(tokens.next_is(TokenType::LeftAngleBracket), {
			let mut compile_time_parameters = Vec::new();
			end = parse_list!(tokens, ListType::AngleBracketed, {
				let parameter = Parameter::from_literal(Parameter::try_parse(tokens, context)?.virtual_deref(context)).unwrap();
				compile_time_parameters.push(parameter);
			})
			.span;
			compile_time_parameters
		});

		// Parameters
		let parameters = if_then_else_default!(tokens.next_is(TokenType::LeftParenthesis), {
			let mut parameters = Vec::new();
			end = parse_list!(tokens, ListType::Parenthesized, {
				let parameter = Parameter::from_literal(Parameter::try_parse(tokens, context)?.virtual_deref(context)).unwrap();
				parameters.push(parameter);
			})
			.span;
			parameters
		});

		// Return Type
		let return_type = if_then_some!(tokens.next_is(TokenType::Colon), {
			let _ = tokens.pop(TokenType::Colon)?;
			let expression = Expression::parse(tokens, context);
			end = expression.span(context);
			expression
		});

		// Body
		let (body, inner_scope_id) = if_then_some!(tokens.next_is(TokenType::LeftBrace), {
			let block = Block::parse_with_scope_type(tokens, context, ScopeType::Function)?;
			let inner_scope_id = block.inner_scope_id();
			for parameter in &compile_time_parameters {
				if let Err(error) = context
					.scope_tree
					.declare_new_variable_from_id(parameter.name().clone(), Expression::ErrorExpression(Span::unknown()), block.inner_scope_id())
				{
					context.add_diagnostic(Diagnostic {
						span: parameter.name().span(context),
						info: DiagnosticInfo::Error(error),
					});
				};
			}
			for parameter in &parameters {
				if let Err(error) = context
					.scope_tree
					.declare_new_variable_from_id(parameter.name().clone(), Expression::ErrorExpression(Span::unknown()), block.inner_scope_id())
				{
					context.add_diagnostic(Diagnostic {
						span: parameter.name().span(context),
						info: DiagnosticInfo::Error(error),
					});
				}
			}
			end = block.span(context);
			(Expression::Block(block), inner_scope_id)
		})
		.unzip();

		// Return
		Ok(Self {
			tags: TagList::default(),
			parameters,
			compile_time_parameters,
			return_type,
			body,
			outer_scope_id: context.scope_tree.unique_id(),
			inner_scope_id,
			this_object: None,
			name: Name::non_mangled("anonymous_function"),
			span: start.to(end),
		}
		.to_literal()
		.store_in_memory(context))
	}
}

impl CompileTime for FunctionDeclaration {
	type Output = FunctionDeclaration;

	fn evaluate_at_compile_time(self, context: &mut Context) -> Self::Output {
		let scope_reverter = context.scope_tree.set_current_scope(self.outer_scope_id);

		// Compile-time parameters
		let compile_time_parameters = {
			let mut compile_time_parameters = Vec::new();
			for parameter in self.compile_time_parameters {
				compile_time_parameters.push(parameter.evaluate_at_compile_time(context));
			}
			compile_time_parameters
		};

		// Parameters
		let parameters = {
			let mut parameters = Vec::new();
			for parameter in self.parameters {
				parameters.push(parameter.evaluate_at_compile_time(context));
			}
			parameters
		};

		// Return type
		let return_type = self.return_type.map(|return_type| return_type.evaluate_as_type(context));

		let tags = self.tags.evaluate_at_compile_time(context);

		context.toggle_side_effects(false);
		let body = self.body.map(|body| body.evaluate_at_compile_time(context));
		context.untoggle_side_effects();

		// Return
		let function = FunctionDeclaration {
			compile_time_parameters,
			parameters,
			body,
			return_type,
			tags,
			this_object: self.this_object,
			name: self.name,
			span: self.span,
			outer_scope_id: self.outer_scope_id,
			inner_scope_id: self.inner_scope_id,
		};

		// Return as a pointer
		scope_reverter.revert(context);
		function
	}
}

static ERROR: LazyLock<FunctionDeclaration> = LazyLock::new(|| FunctionDeclaration {
	tags: TagList::default(),
	compile_time_parameters: Vec::new(),
	parameters: Vec::new(),
	return_type: None,
	body: None,
	outer_scope_id: ScopeId::global(),
	inner_scope_id: None,
	this_object: None,
	name: "Error".into(),
	span: Span::unknown(),
});

impl LiteralConvertible for FunctionDeclaration {
	fn to_literal(self) -> LiteralObject {
		LiteralObject {
			address: None,
			fields: HashMap::from([]),
			internal_fields: HashMap::from([
				("compile_time_parameters".to_owned(), InternalFieldValue::ParameterList(self.compile_time_parameters)),
				("parameters".to_owned(), InternalFieldValue::ParameterList(self.parameters)),
				("body".to_owned(), InternalFieldValue::OptionalExpression(self.body)),
				("return_type".to_owned(), InternalFieldValue::OptionalExpression(self.return_type)),
				("this_object".to_owned(), InternalFieldValue::OptionalExpression(self.this_object)),
			]),
			name: self.name,
			field_access_type: FieldAccessType::Normal,
			outer_scope_id: self.outer_scope_id,
			inner_scope_id: self.inner_scope_id,
			span: self.span,
			type_name: "Function".into(),
			tags: self.tags,
		}
	}

	fn from_literal(literal: &LiteralObject) -> anyhow::Result<Self> {
		if literal.type_name() != &"Function".into() {
			anyhow::bail!("")
		}

		Ok(FunctionDeclaration {
			compile_time_parameters: literal.get_internal_field::<Vec<Parameter>>("compile_time_parameters")?.to_owned(),
			parameters: literal.get_internal_field::<Vec<Parameter>>("parameters")?.to_owned(),
			body: literal.get_internal_field::<Option<Expression>>("body")?.to_owned(),
			return_type: literal.get_internal_field::<Option<Expression>>("return_type")?.to_owned(),
			this_object: literal.get_internal_field::<Option<Expression>>("this_object")?.to_owned(),
			tags: literal.tags.clone(),
			outer_scope_id: literal.outer_scope_id(),
			inner_scope_id: literal.inner_scope_id,
			name: literal.name.clone(),
			span: literal.span,
		})
	}
}

impl Spanned for FunctionDeclaration {
	fn span(&self, _context: &Context) -> Span {
		self.span
	}
}

impl FunctionDeclaration {
	pub(crate) fn error() -> &'static FunctionDeclaration {
		&ERROR
	}

	pub(crate) const fn body(&self) -> Option<&Expression> {
		self.body.as_ref()
	}

	pub(crate) const fn return_type(&self) -> Option<&Expression> {
		self.return_type.as_ref()
	}

	pub(crate) fn parameters(&self) -> &[Parameter] {
		&self.parameters
	}

	pub(crate) const fn tags(&self) -> &TagList {
		&self.tags
	}

	pub(crate) const fn name(&self) -> &Name {
		&self.name
	}

	pub(crate) const fn this_object(&self) -> Option<&Expression> {
		self.this_object.as_ref()
	}

	pub(crate) fn set_this_object(&mut self, this_object: Expression) {
		self.this_object = Some(this_object);
	}

	pub(crate) fn compile_time_parameters(&self) -> &[Parameter] {
		&self.compile_time_parameters
	}

	pub(crate) fn set_name(&mut self, name: Name) {
		self.name = name;
	}
}
