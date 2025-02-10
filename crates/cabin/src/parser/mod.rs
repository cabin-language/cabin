use std::collections::{HashMap, VecDeque};

use crate::{
	api::{
		context::Context,
		scope::{ScopeId, ScopeType},
		traits::TryAs,
	},
	comptime::{memory::VirtualPointer, CompileTime},
	diagnostics::{Diagnostic, DiagnosticInfo},
	lexer::{Span, Token, TokenType},
	parser::{
		expressions::{
			field_access::FieldAccessType,
			literal::LiteralObject,
			object::{Field, ObjectConstructor},
			Spanned as _,
		},
		statements::{declaration::Declaration, tag::TagList, Statement},
	},
	Error,
};

pub mod expressions;
pub mod statements;

#[derive(Clone, Debug, thiserror::Error)]
pub enum ParseError {
	#[error("Unexpected token: Expected {expected} but found {actual}")]
	UnexpectedToken { expected: TokenType, actual: TokenType },

	#[error("Unexpected token: Expected {expected} but found {actual}")]
	UnexpectedTokenExpected { expected: &'static str, actual: TokenType },

	#[error("Unexpected end of file: Expected {expected} but found end of file")]
	UnexpectedEOF { expected: TokenType },

	#[error("Unexpected end of file: Expected more tokens")]
	UnexpectedGenericEOF,

	#[error("The variable \"{name}\" was declared twice")]
	DuplicateVariableDeclaration { name: String },

	#[error("Only declarations are allowed at the top level of a module")]
	InvalidTopLevelStatement { statement: Statement },

	#[error("Invalid formatted string: {0}")]
	InvalidFormatString(String),
}

#[derive(Debug)]
pub struct Program {
	statements: Vec<Statement>,
	inner_scope_id: ScopeId,
}

impl Parse for Program {
	type Output = Self;

	fn parse(tokens: &mut TokenQueue, context: &mut Context) -> Self::Output {
		context.scope_data.enter_new_scope(ScopeType::File);
		let inner_scope_id = context.scope_data.unique_id();
		let mut statements = Vec::new();

		while !tokens.is_all_whitespace() {
			statements.push(Statement::parse(tokens, context));
		}

		context.scope_data.exit_scope().unwrap();

		Program { statements, inner_scope_id }
	}
}

impl CompileTime for Program {
	type Output = Program;

	fn evaluate_at_compile_time(self, context: &mut Context) -> Self::Output {
		let scope_reverter = context.scope_data.set_current_scope(self.inner_scope_id);
		let evaluated = Self {
			statements: self.statements.into_iter().map(|statement| statement.evaluate_at_compile_time(context)).collect(),
			inner_scope_id: self.inner_scope_id,
		};
		scope_reverter.revert(context);
		evaluated
	}
}

#[derive(Debug)]
pub struct Module {
	declarations: Vec<Declaration>,
	inner_scope_id: ScopeId,
}

impl Parse for Module {
	type Output = Self;

	fn parse(tokens: &mut TokenQueue, context: &mut Context) -> Self::Output {
		context.scope_data.enter_new_scope(ScopeType::File);
		let inner_scope_id = context.scope_data.unique_id();
		let mut declarations = Vec::new();

		while !tokens.is_all_whitespace() {
			let statement = Statement::parse(tokens, context);

			match statement {
				Statement::Declaration(declaration) => {
					declarations.push(declaration);
				},
				Statement::Error(_span) => {},
				statement => context.add_diagnostic(Diagnostic {
					span: statement.span(context),
					info: DiagnosticInfo::Error(Error::Parse(ParseError::InvalidTopLevelStatement { statement })),
				}),
			};
		}

		context.scope_data.exit_scope().unwrap();
		Module { declarations, inner_scope_id }
	}
}

impl CompileTime for Module {
	type Output = Module;

	fn evaluate_at_compile_time(self, context: &mut Context) -> Self::Output {
		let scope_reverter = context.scope_data.set_current_scope(self.inner_scope_id);
		let evaluated = Self {
			declarations: self.declarations.into_iter().map(|statement| statement.evaluate_at_compile_time(context)).collect(),
			inner_scope_id: self.inner_scope_id,
		};
		scope_reverter.revert(context);
		evaluated
	}
}

impl Module {
	pub fn to_pointer(&self, context: &mut Context) -> VirtualPointer {
		LiteralObject {
			type_name: "Module".into(),
			fields: self
				.declarations
				.iter()
				.map(|declaration| {
					(
						declaration.name().to_owned(),
						*declaration
							.value(context)
							.clone()
							.evaluate_at_compile_time(context)
							.try_as::<VirtualPointer>()
							.unwrap_or(&VirtualPointer::ERROR),
					)
				})
				.collect(),
			internal_fields: HashMap::new(),
			field_access_type: FieldAccessType::Normal,
			outer_scope_id: context.scope_data.unique_id(),
			inner_scope_id: None,
			name: "module".into(),
			address: None,
			span: Span::unknown(),
			tags: TagList::default(),
		}
		.store_in_memory(context)
	}
}

/// A trait for treating a collection of tokens as a queue of tokens that can be parsed. This is
/// traditionally implemented for `VecDeque<Token>`.
pub trait TokenQueueFunctionality {
	/// Removes and returns the next token's value in the queue if the token matches the given token type. If it
	/// does not (or the token stream is empty), an error is returned.
	///
	/// # Parameters
	/// - `token_type` - The type of token to pop.
	///
	/// # Returns
	/// A `Result` containing either the value of the popped token or an `Error`.
	fn pop(&mut self, token_type: TokenType) -> Result<Token, Diagnostic>;

	/// Removes and returns the next token's type in the queue if the token matches the given token type. If it
	/// does not (or the token stream is empty), an error is returned.
	///
	/// # Parameters
	/// - `token_type` - The type of token to pop.
	///
	/// # Returns
	/// A `Result` containing either the type of the popped token or an `Error`.
	fn pop_type(&mut self, token_type: TokenType) -> Result<TokenType, Diagnostic>;

	/// Returns a reference to the next token in the queue without removing it. If the queue is empty, `None`
	/// is returned.
	///
	/// # Returns
	/// A reference to the next token in the queue or `None` if the queue is empty.
	fn peek(&self) -> Result<&str, Diagnostic>;

	fn peek_type(&self) -> Result<TokenType, Diagnostic>;

	fn peek_type2(&self) -> Result<TokenType, Diagnostic>;

	/// Returns whether the next token in the queue matches the given token type.
	fn next_is(&self, token_type: TokenType) -> bool {
		self.peek_type().map_or(false, |token| token == token_type)
	}

	/// Returns whether the next next token in the queue matches the given token type.
	fn next_next_is(&self, token_type: TokenType) -> bool {
		self.peek_type2().map_or(false, |token| token == token_type)
	}

	/// Returns whether the next token in the queue matches one of the given token types.
	///
	/// # Parameters
	/// - `token_types` - The token types to check against.
	///
	/// # Returns
	/// Whether the next token in the queue matches one of the given token types.
	fn next_is_one_of(&self, token_types: &[TokenType]) -> bool {
		token_types.iter().any(|token_type| self.next_is(token_type.to_owned()))
	}

	fn current_position(&self) -> Option<Span>;

	fn is_all_whitespace(&self) -> bool;
}

impl TokenQueueFunctionality for TokenQueue {
	fn peek(&self) -> Result<&str, Diagnostic> {
		let mut index = 0;
		let mut next = self.get(index).ok_or_else(|| Diagnostic {
			span: Span::unknown(),
			info: DiagnosticInfo::Error(Error::Parse(ParseError::UnexpectedGenericEOF)),
		})?;
		while next.token_type.is_whitespace() {
			index += 1;
			next = self.get(index).ok_or_else(|| Diagnostic {
				span: Span::unknown(),
				info: DiagnosticInfo::Error(Error::Parse(ParseError::UnexpectedGenericEOF)),
			})?;
		}
		Ok(&next.value)
	}

	fn peek_type(&self) -> Result<TokenType, Diagnostic> {
		let mut index = 0;
		let mut next = self.get(index).ok_or_else(|| Diagnostic {
			span: Span::unknown(),
			info: DiagnosticInfo::Error(Error::Parse(ParseError::UnexpectedGenericEOF)),
		})?;
		while next.token_type.is_whitespace() {
			index += 1;
			next = self.get(index).ok_or_else(|| Diagnostic {
				span: Span::unknown(),
				info: DiagnosticInfo::Error(Error::Parse(ParseError::UnexpectedGenericEOF)),
			})?;
		}
		Ok(next.token_type)
	}

	fn peek_type2(&self) -> Result<TokenType, Diagnostic> {
		let mut index = 0;

		// The one time I'd enjoy a do-while loop
		let mut next = self.get(index).ok_or_else(|| Diagnostic {
			span: Span::unknown(),
			info: DiagnosticInfo::Error(Error::Parse(ParseError::UnexpectedGenericEOF)),
		})?;
		index += 1;
		while next.token_type.is_whitespace() {
			next = self.get(index).ok_or_else(|| Diagnostic {
				span: Span::unknown(),
				info: DiagnosticInfo::Error(Error::Parse(ParseError::UnexpectedGenericEOF)),
			})?;
			index += 1;
		}

		let mut next_next = self.get(index).ok_or_else(|| Diagnostic {
			span: Span::unknown(),
			info: DiagnosticInfo::Error(Error::Parse(ParseError::UnexpectedGenericEOF)),
		})?;
		while next_next.token_type.is_whitespace() {
			index += 1;
			next_next = self.get(index).ok_or_else(|| Diagnostic {
				span: Span::unknown(),
				info: DiagnosticInfo::Error(Error::Parse(ParseError::UnexpectedGenericEOF)),
			})?;
		}

		Ok(next_next.token_type)
	}

	fn is_all_whitespace(&self) -> bool {
		self.iter().all(|token| token.token_type.is_whitespace())
	}

	fn pop(&mut self, token_type: TokenType) -> Result<Token, Diagnostic> {
		let mut maybe_whitespace = TokenType::Whitespace;
		while maybe_whitespace.is_whitespace() {
			if let Some(token) = self.pop_front() {
				maybe_whitespace = token.token_type;

				if token.token_type == token_type {
					return Ok(token);
				}

				if !maybe_whitespace.is_whitespace() {
					return Err(Diagnostic {
						span: token.span,
						info: DiagnosticInfo::Error(Error::Parse(ParseError::UnexpectedToken {
							expected: token_type,
							actual: token.token_type,
						})),
					});
				}
			}
		}

		return Err(Diagnostic {
			span: Span::unknown(),
			info: DiagnosticInfo::Error(Error::Parse(ParseError::UnexpectedEOF { expected: token_type })),
		});
	}

	fn pop_type(&mut self, token_type: TokenType) -> Result<TokenType, Diagnostic> {
		let mut maybe_whitespace = TokenType::Whitespace;
		while maybe_whitespace.is_whitespace() {
			if let Some(token) = self.pop_front() {
				maybe_whitespace = token.token_type;

				if token.token_type == token_type {
					return Ok(token.token_type);
				}

				if !maybe_whitespace.is_whitespace() {
					return Err(Diagnostic {
						span: token.span,
						info: DiagnosticInfo::Error(Error::Parse(ParseError::UnexpectedToken {
							expected: token_type,
							actual: token.token_type,
						})),
					});
				}
			}
		}

		return Err(Diagnostic {
			span: Span::unknown(),
			info: DiagnosticInfo::Error(Error::Parse(ParseError::UnexpectedEOF { expected: token_type })),
		});
	}

	fn current_position(&self) -> Option<Span> {
		self.front().map(|front| front.span)
	}
}

impl Module {
	pub fn into_literal(self, context: &mut Context) -> anyhow::Result<LiteralObject> {
		Ok(LiteralObject {
			type_name: "Object".into(),
			fields: self
				.declarations
				.into_iter()
				.map(|declaration| {
					let name = declaration.name().to_owned();
					let value = declaration.value(context);
					(name, value.try_as::<VirtualPointer>().unwrap().to_owned())
				})
				.collect(),
			internal_fields: HashMap::new(),
			field_access_type: FieldAccessType::Normal,
			inner_scope_id: Some(self.inner_scope_id),
			outer_scope_id: self.inner_scope_id,
			name: "anonymous_module".into(),
			address: None,
			span: Span::unknown(),
			tags: TagList::default(),
		})
	}

	pub fn into_object(self, context: &mut Context) -> anyhow::Result<ObjectConstructor> {
		Ok(ObjectConstructor {
			type_name: "Module".into(),
			fields: self
				.declarations
				.into_iter()
				.map(|declaration| {
					let name = declaration.name().to_owned();
					let value = Some(declaration.value(context).clone());
					Field { name, value, field_type: None }
				})
				.collect(),
			internal_fields: HashMap::new(),
			field_access_type: FieldAccessType::Normal,
			inner_scope_id: self.inner_scope_id,
			outer_scope_id: self.inner_scope_id,
			name: "anonymous_module".into(),
			span: Span::unknown(),
			tags: TagList::default(),
		})
	}
}

pub enum ListType {
	AngleBracketed,
	Braced,
	Bracketed,
	Parenthesized,
	Tag,
}

impl ListType {
	const fn opening(&self) -> TokenType {
		match self {
			Self::AngleBracketed => TokenType::LeftAngleBracket,
			Self::Braced => TokenType::LeftBrace,
			Self::Bracketed => TokenType::LeftBracket,
			Self::Parenthesized => TokenType::LeftParenthesis,
			Self::Tag => TokenType::TagOpening,
		}
	}

	const fn closing(&self) -> TokenType {
		match self {
			Self::AngleBracketed => TokenType::RightAngleBracket,
			Self::Braced => TokenType::RightBrace,
			Self::Parenthesized => TokenType::RightParenthesis,
			Self::Bracketed | Self::Tag => TokenType::RightBracket,
		}
	}
}

pub trait TryParse {
	type Output;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic>;
}

pub trait Parse {
	type Output;

	fn parse(tokens: &mut TokenQueue, context: &mut Context) -> Self::Output;
}

pub type TokenQueue = VecDeque<Token>;

pub trait ToCabin {
	fn to_cabin(&self) -> String;
}
