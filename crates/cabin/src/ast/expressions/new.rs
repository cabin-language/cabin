use std::fmt::Write as _;

use derive_getters::Getters;

use crate::{
	Span,
	api::context::Context,
	ast::{
		expressions::{Expression, Spanned, evaluated::EvaluatedExpression, identifier::Identifier, object::Object},
		misc::tag::{EvaluatedTagList, TagList},
	},
	comptime::{Evaluate, memory::Pointer},
	diagnostics::Diagnostic,
	if_then_some,
	lexer::{Token, TokenType},
	parser::{TokenQueue, TokenQueueFunctionality as _, TryParse},
	transpiler::ToCabin,
	util::Indent as _,
};

#[derive(Debug, Clone, Getters)]
pub struct NewExpression {
	type_name: Option<Identifier>,
	fields: Vec<NewField>,
	tokens: NewTokens,
}

#[derive(Debug, Clone, Getters)]
pub struct NewTokens {
	keyword_new: Token,
	left_brace: Token,
	right_brace: Token,
}

#[derive(Debug, Clone, Getters)]
pub struct NewField {
	tags: Option<TagList>,
	name: Identifier,
	value: Expression,
	tokens: NewFieldTokens,
}

#[derive(Debug, Clone, Getters)]
pub struct NewFieldTokens {
	equals: Token,
	comma: Option<Token>,
}

impl TryParse for NewExpression {
	type Output = NewExpression;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		let keyword_new = tokens.pop(TokenType::KeywordNew, context)?;

		// name
		let type_name = Identifier::try_parse(tokens, context).ok();

		let left_brace = tokens.pop(TokenType::LeftBrace, context)?;

		// fields
		let mut fields = Vec::new();
		while !tokens.next_is(TokenType::RightBrace) {
			// field tags
			let tags = if_then_some!(tokens.next_is(TokenType::TagOpening), TagList::try_parse(tokens, context)?);

			// field name
			let field_name = Identifier::try_parse(tokens, context)?;

			// field value
			let equals = tokens.pop(TokenType::Equal, context)?;
			let mut value = Expression::try_parse(tokens, context)?;
			if let Some(tags) = &tags {
				value.set_tags(tags.clone());
			}

			let comma = tokens.pop(TokenType::Comma, context).ok();
			let has_comma = comma.is_some();

			// Add field
			fields.push(NewField {
				tags,
				name: field_name,
				value,
				tokens: NewFieldTokens { equals, comma },
			});

			if !has_comma {
				break;
			}
		}

		let right_brace = tokens.pop(TokenType::RightBrace, context)?;

		// return
		Ok(NewExpression {
			type_name,
			fields,
			tokens: NewTokens {
				keyword_new,
				left_brace,
				right_brace,
			},
		})
	}
}

impl Evaluate for NewExpression {
	type Output = Pointer;

	fn evaluate(self, context: &mut Context) -> Self::Output {
		let mut fields = Vec::new();

		for field in self.fields.clone() {
			fields.push(EvaluatedNewField {
				tokens: field.tokens,
				name: field.name,
				tags: field.tags.evaluate(context),
				value: field.value.evaluate(context),
			});
		}

		let evaluated = EvaluatedNewExpression {
			group: self.type_name.evaluate(context),
			fields,
			tokens: self.tokens,
		};

		match evaluated.into_object(context) {
			Ok(obj) => obj.store(context),
			Err(new) => EvaluatedExpression::New(new).store(context),
		}
	}
}

impl Spanned for NewExpression {
	fn span(&self) -> Span {
		self.tokens.keyword_new.span.to(self.tokens.right_brace.span)
	}
}

impl ToCabin for NewExpression {
	fn to_cabin(&self) -> String {
		let mut code = format!("new {} {{", self.type_name.to_cabin());

		if !self.fields.is_empty() {
			code += "\n";
		}

		for field in &self.fields {
			code += &format!("{},\n", field.to_cabin()).indented();
		}

		// pop trailing comma
		if !self.fields.is_empty() {
			writeln!(code, "{}", code.get(0..code.len() - 2).unwrap().to_owned()).unwrap();
		}

		code += "}";
		code
	}
}

impl Spanned for EvaluatedNewExpression {
	fn span(&self) -> Span {
		self.tokens.keyword_new.span.to(self.tokens.right_brace.span)
	}
}

impl ToCabin for NewField {
	fn to_cabin(&self) -> String {
		let mut code = String::new();
		if self.tags.is_some() {
			writeln!(code, "{}", self.tags.to_cabin());
		}
		write!(code, "{} = {}", self.name.to_cabin(), self.value.to_cabin()).unwrap();
		code
	}
}

#[derive(Debug, Clone, Getters)]
pub struct EvaluatedNewExpression {
	group: Option<Pointer>,
	fields: Vec<EvaluatedNewField>,
	tokens: NewTokens,
}

impl EvaluatedNewExpression {
	pub fn get_field_value(&self, name: &Identifier) -> Option<Pointer> {
		self.fields.iter().find(|field| &field.name == name).map(|field| field.value)
	}

	fn into_object(self, context: &Context) -> Result<Object, Self> {
		if self.fields.iter().all(|field| field.value.is_fully_known(context)) {
			let span = self.span();
			Ok(Object::new(
				self.group.unwrap(),
				self.fields.into_iter().map(|field| (field.name.source_identifier().to_owned(), field.value)).collect(),
				span,
			))
		} else {
			Err(self)
		}
	}
}

#[derive(Debug, Clone, Getters)]
pub struct EvaluatedNewField {
	tags: Option<EvaluatedTagList>,
	name: Identifier,
	value: Pointer,
	tokens: NewFieldTokens,
}
