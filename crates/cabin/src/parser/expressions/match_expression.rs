use std::fmt::Debug;

use crate::{
	api::context::context,
	cli::theme::Styled,
	comptime::CompileTime,
	debug_start,
	if_then_some,
	lexer::{Span, TokenType},
	parse_list,
	parser::{
		expressions::{block::Block, name::Name, Expression, Spanned},
		ListType,
		Parse,
		TokenQueue,
		TokenQueueFunctionality,
	},
};

#[derive(Clone)]
pub struct Match {
	pub expression: Box<Expression>,
	pub branches: Vec<MatchBranch>,
	pub span: Span,
}

#[derive(Clone)]
pub struct MatchBranch {
	pub name: Option<Name>,
	pub type_to_match: Expression,
	pub body: Block,
}

impl Parse for Match {
	type Output = Match;

	fn parse(tokens: &mut TokenQueue) -> Result<Self::Output, crate::Error> {
		let debug_section = debug_start!("{} a {}", "Parsing".bold().green(), "match expression".cyan());
		let start = tokens.pop(TokenType::KeywordMatch)?.span;
		let expression = Expression::parse(tokens)?;
		let mut branches = Vec::new();
		let end = parse_list!(tokens, ListType::Braced, {
			let first = Name::parse(tokens)?;
			let second = if_then_some!(tokens.next_is(TokenType::Colon), {
				let _ = tokens.pop(TokenType::Colon)?;
				Expression::parse(tokens)?
			});

			let body = Block::parse(tokens)?;

			let branch = match second {
				Some(type_to_match) => {
					context()
						.scope_data
						.declare_new_variable_from_id(first.clone(), Expression::Void(()), body.inner_scope_id())?;
					MatchBranch {
						type_to_match,
						name: Some(first),
						body,
					}
				},
				None => MatchBranch {
					name: None,
					type_to_match: Expression::Name(first),
					body,
				},
			};

			branches.push(branch);
		})
		.span;

		debug_section.finish();
		Ok(Match {
			expression: Box::new(expression),
			branches,
			span: start.to(end),
		})
	}
}

impl CompileTime for Match {
	type Output = Expression;

	fn evaluate_at_compile_time(self) -> anyhow::Result<Self::Output> {
		let debug_section = debug_start!("{} a {}", "Compile-Time Evaluating".bold().green(), "match expression".cyan());
		let expression = self.expression.evaluate_at_compile_time()?;

		// Branches
		let mut branches = Vec::new();
		for branch in self.branches {
			let type_to_match = branch.type_to_match.evaluate_as_type()?;
			branches.push(MatchBranch {
				name: branch.name,
				type_to_match,
				body: branch.body,
			});
		}

		for branch in &branches {
			if expression.is_assignable_to_type(branch.type_to_match.try_as_literal()?.address.unwrap())? {
				debug_section.finish();
				if let Some(name) = &branch.name {
					context().scope_data.reassign_variable_from_id(name, expression, branch.body.inner_scope_id())?;
				}
				return branch.body.clone().evaluate_at_compile_time();
			};
		}

		debug_section.finish();
		Ok(Expression::Match(Match {
			expression: Box::new(expression),
			branches,
			span: self.span,
		}))
	}
}

impl Spanned for Match {
	fn span(&self) -> Span {
		self.span
	}
}

impl Debug for MatchBranch {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"{}{:?} {:?}",
			if let Some(name) = &self.name { format!("{:?}: ", name) } else { String::new() },
			self.type_to_match,
			self.body
		)
	}
}

impl Debug for Match {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"{} {:?} {{\n\t{}\n}}",
			"match".style(context().theme.keyword()),
			self.expression,
			self.branches.iter().map(|branch| format!("{branch:?}")).collect::<Vec<_>>().join("\n")
		)
	}
}
