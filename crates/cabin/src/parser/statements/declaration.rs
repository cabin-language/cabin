use super::Statement;
use crate::{
	api::{context::context, scope::ScopeId},
	comptime::CompileTime,
	debug_start,
	err,
	if_then_some,
	lexer::TokenType,
	mapped_err,
	parser::{
		expressions::{name::Name, Expression},
		statements::tag::TagList,
		Parse,
		TokenQueue,
		TokenQueueFunctionality,
	},
	transpiler::TranspileToC,
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
}

impl Declaration {
	pub const fn name(&self) -> &Name {
		&self.name
	}

	pub fn value(&self) -> anyhow::Result<&Expression> {
		context().scope_data.get_variable_from_id(self.name.clone(), self.scope_id).ok_or_else(|| {
			err! {
				base = format!("Attempted to get the value for the declaration of \"{}\", but it has no value stored.", self.name.unmangled_name().bold().cyan()),
			}
		})
	}

	pub const fn declaration_type(&self) -> &DeclarationType {
		&self.declaration_type
	}
}

impl Parse for Declaration {
	type Output = Statement;

	fn parse(tokens: &mut TokenQueue) -> Result<Self::Output, crate::Error> {
		let debug_section = debug_start!("{} a {}", "Parsing".bold().green(), "declaration".cyan());

		// Tags
		let tags = if_then_some!(tokens.next_is(TokenType::TagOpening), TagList::parse(tokens)?);

		if tags.is_some() && !tokens.next_is(TokenType::KeywordLet) {
			let mut expression = Expression::parse(tokens)?;
			expression.set_tags(tags.unwrap());
			let _ = tokens.pop(TokenType::Semicolon)?;
			return Ok(Statement::Expression(expression));
		}

		// Name
		let _ = tokens.pop(TokenType::KeywordLet)?;
		let name = Name::parse(tokens)?;

		// Value
		let _ = tokens.pop(TokenType::Equal)?;

		let mut value = Expression::parse(tokens)?;

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
		debug_section.finish();
		Ok(Statement::Declaration(Declaration {
			name,
			scope_id: context().scope_data.unique_id(),
			declaration_type: DeclarationType::Normal,
		}))
	}
}

impl CompileTime for Declaration {
	type Output = Declaration;

	fn evaluate_at_compile_time(self) -> anyhow::Result<Self::Output> {
		let debug_section = debug_start!(
			"{} the declaration for the variable {}",
			"Compile-Time Evaluating".bold().green(),
			self.name.unmangled_name().red()
		);
		let evaluated = self
			.value()
			.map_err(mapped_err! {
				while = format!("getting the value of the declaration of \"{}\"", self.name.unmangled_name().bold().cyan()),
			})?
			.clone()
			.evaluate_at_compile_time()?; // TODO: use a mapping function instead of cloning
		context().scope_data.reassign_variable_from_id(&self.name, evaluated, self.scope_id)?;

		// Return the declaration
		debug_section.finish();
		Ok(self)
	}
}

impl TranspileToC for Declaration {
	fn to_c(&self) -> anyhow::Result<String> {
		Ok(format!(
			"void* {} = {};",
			self.name.to_c()?,
			self.value()?.to_c().map_err(mapped_err! {
				while = format!("transpiling the value of the initial declaration for the variable \"{}\" to C", self.name.unmangled_name()),
			})?
		))
	}
}
