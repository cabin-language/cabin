use std::sync::atomic::{AtomicUsize, Ordering};

use crate::{
	api::{context::context, scope::ScopeId},
	comptime::{memory::VirtualPointer, CompileTime},
	if_then_else_default,
	if_then_some,
	lexer::TokenType,
	parse_list,
	parser::{
		expressions::{literal::LiteralConvertible, parameter::Parameter, Expression},
		ListType,
		Parse,
		TokenQueue,
		TokenQueueFunctionality,
	},
};

#[derive(Debug, Clone)]
pub struct DefaultExtend {
	compile_time_parameters: Vec<VirtualPointer>,
	pub type_to_extend: Expression,
	pub type_to_be: Option<(Expression, Expression)>,
	pub id: usize,
}

static DEFAULT_EXTEND_ID: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone)]
pub struct DefaultExtendPointer {
	scope_id: ScopeId,
	id: usize,
}

impl Parse for DefaultExtend {
	type Output = DefaultExtendPointer;

	fn parse(tokens: &mut TokenQueue) -> Result<Self::Output, crate::Error> {
		let _ = tokens.pop(TokenType::KeywordDefault)?;
		let _ = tokens.pop(TokenType::KeywordExtend)?;

		let compile_time_parameters = if_then_else_default!(tokens.next_is(TokenType::LeftAngleBracket), {
			let mut compile_time_parameters = Vec::new();
			let _ = parse_list!(tokens, ListType::AngleBracketed, {
				let parameter = Parameter::parse(tokens)?;
				context().scope_data.declare_new_variable(
					Parameter::from_literal(parameter.virtual_deref()).unwrap().name().to_owned(),
					Expression::Pointer(parameter),
				)?;
				compile_time_parameters.push(parameter);
			});
			compile_time_parameters
		});

		let type_to_extend = Expression::parse(tokens)?;

		let type_to_be = if_then_some!(tokens.next_is(TokenType::KeywordToBe), {
			let _ = tokens.pop(TokenType::KeywordToBe)?;
			let type_to_be = Expression::parse(tokens)?;
			let _ = tokens.pop(TokenType::KeywordWith)?;
			let declaration = Expression::parse(tokens)?;
			(type_to_be, declaration)
		});

		let _ = tokens.pop(TokenType::Semicolon)?;

		let id = DEFAULT_EXTEND_ID.fetch_add(1, Ordering::Relaxed);

		let extension = DefaultExtend {
			compile_time_parameters,
			type_to_be,
			type_to_extend,
			id,
		};

		context().scope_data.add_default_extension(extension);

		Ok(DefaultExtendPointer {
			scope_id: context().scope_data.unique_id(),
			id,
		})
	}
}

impl CompileTime for DefaultExtendPointer {
	type Output = DefaultExtendPointer;

	fn evaluate_at_compile_time(self) -> anyhow::Result<Self::Output> {
		context()
			.scope_data
			.map_default_extension_from_id(self.scope_id, self.id, DefaultExtend::evaluate_at_compile_time)?;

		Ok(self)
	}
}

impl CompileTime for DefaultExtend {
	type Output = DefaultExtend;

	fn evaluate_at_compile_time(self) -> anyhow::Result<Self::Output> {
		let type_to_extend = self.type_to_extend.evaluate_at_compile_time()?;
		let type_to_be = self
			.type_to_be
			.map(|(to_be, declaration)| anyhow::Ok((to_be.evaluate_at_compile_time()?, declaration.evaluate_at_compile_time()?)))
			.transpose()?;
		let compile_time_parameters = self
			.compile_time_parameters
			.into_iter()
			.map(|parameter| parameter.evaluate_at_compile_time())
			.collect::<anyhow::Result<Vec<_>>>()?;

		Ok(DefaultExtend {
			type_to_be,
			type_to_extend,
			compile_time_parameters,
			id: self.id,
		})
	}
}
