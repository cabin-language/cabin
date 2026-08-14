use std::{collections::HashMap, fmt::Write as _};

use convert_case::{Case, Casing as _};
use derive_getters::Getters;

use crate::{
	SpanLike,
	Spanned,
	api::context::Context,
	ast::{
		expressions::{Expression, UnevaluatedExpression, identifier::Identifier, object::Object},
		misc::compile_time_parameter::{CompileTimeParameters, EvaluatedCompileTimeParameters},
	},
	comptime::{
		Dot,
		Evaluate,
		storage::{Evaluated, Pointer},
	},
	diagnostics::{Diagnostic, DiagnosticInfo},
	if_then_some,
	lexer::{Token, TokenType},
	log,
	parser::{TokenQueue, TokenQueueFunctionality as _, TryParse},
	scope::ScopeType,
	transpiler::ToCabin,
	util::Indent as _,
};

#[derive(Debug, Clone, Getters)]
pub struct Either {
	variants: Vec<EitherVariant>,
	compile_time_parameters: Option<CompileTimeParameters>,
	tokens: EitherTokens,
}

#[derive(Debug, Clone, Getters)]
pub struct EitherTokens {
	keyword_either: Token,
	left_brace: Token,
	right_brace: Token,
}

#[derive(Debug, Clone, Getters)]
pub struct EitherVariant {
	name: Identifier,
	subtype: Option<Pointer<Expression>>,
	tokens: EitherVariantTokens,
}

#[derive(Debug, Clone, Getters)]
pub struct EitherVariantTokens {
	colon: Option<Token>,
	comma: Option<Token>,
}

impl TryParse for Either {
	type Output = Either;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		let keyword_either = tokens.pop(TokenType::KeywordEither, context)?;
		let mut variants = Vec::new();

		let either_scope = context.scope.enter_new_scope(ScopeType::Either);

		let compile_time_parameters = CompileTimeParameters::try_parse(tokens, context).ok();
		if let Some(compile_time_parameters) = &compile_time_parameters {
			for parameter in compile_time_parameters.parameters() {
				if let Err(error) = context
					.scope
					.declare_new_variable_from_id(parameter.name(), parameter.as_declaration(context), either_scope)
				{
					context.add_diagnostic(Diagnostic::at(parameter.name(), error));
				};
			}
		}

		let left_brace = tokens.pop(TokenType::LeftBrace, context)?;
		while !tokens.next_is(TokenType::RightBrace) {
			let name = Identifier::try_parse(tokens, context)?;

			let (colon, subtype) = if_then_some!(tokens.next_is(TokenType::Colon), {
				(tokens.pop(TokenType::Colon, context)?, UnevaluatedExpression::try_parse(tokens, context)?)
			})
			.unzip();

			if name.source_identifier() != name.source_identifier().to_case(Case::Snake) {
				context.add_diagnostic(Diagnostic::at(&name, DiagnosticInfo::NonSnakeCaseName {
					original_name: name.source_identifier().to_owned(),
				}));
			}

			let comma = tokens.pop(TokenType::Comma, context).ok();
			let has_comma = comma.is_some();

			variants.push(EitherVariant {
				name,
				subtype,
				tokens: EitherVariantTokens { colon, comma },
			});

			if !has_comma {
				break;
			}
		}

		let right_brace = tokens.pop(TokenType::RightBrace, context)?;

		context.scope.exit_scope(either_scope).unwrap();

		Ok(Either {
			variants,
			compile_time_parameters,
			tokens: EitherTokens {
				keyword_either,
				left_brace,
				right_brace,
			},
		})
	}
}

impl Evaluate for Either {
	type Output = EvaluatedEither;

	fn evaluate(self, context: &mut Context) -> Self::Output {
		log!(context, "Evaluating either");

		// Warning for empty either
		if self.variants.is_empty() {
			context.add_diagnostic_at(&self, DiagnosticInfo::EmptyEither);
		}

		let mut variants = Vec::new();
		for variant in self.variants {
			variants.push(EvaluatedEitherVariant {
				object: Object::new(context.builtin("EitherVariant"), HashMap::from([]), variant.span().into_owned()).store(context),
				name: variant.name,
				subtype: variant.subtype.evaluate(context),
				tokens: variant.tokens,
			});
		}

		EvaluatedEither {
			variants,
			tokens: self.tokens,
			compile_time_parameters: self.compile_time_parameters.evaluate(context),
		}
	}
}

impl Spanned for Either {
	fn span(&self) -> impl SpanLike {
		self.tokens().keyword_either().span().to(self.tokens().right_brace().span())
	}
}

impl ToCabin for Either {
	fn to_cabin(&self) -> String {
		let mut code = "either".to_owned();
		code += &self.compile_time_parameters.to_cabin();
		code += " {{";
		if !self.variants.is_empty() {
			code += "\n";
		}

		for variant in &self.variants {
			code += &format!("{},\n", variant.to_cabin()).indented();
		}

		// pop trailing comma
		if !self.variants.is_empty() {
			writeln!(code, "{}", code.get(0..code.len() - 2).unwrap().to_owned()).unwrap();
		}

		code += "}";
		code
	}
}

impl ToCabin for EitherVariant {
	fn to_cabin(&self) -> String {
		let mut code = self.name.to_cabin();
		if let Some(subtype) = &self.subtype {
			write!(code, ": {}", subtype.to_cabin()).unwrap();
		}
		code
	}
}

impl Spanned for EvaluatedEither {
	fn span(&self) -> impl SpanLike {
		self.tokens().keyword_either().span().to(self.tokens().right_brace().span())
	}
}

#[derive(Debug, Clone, Getters)]
pub struct EvaluatedEither {
	variants: Vec<EvaluatedEitherVariant>,
	compile_time_parameters: Option<EvaluatedCompileTimeParameters>,
	tokens: EitherTokens,
}

#[derive(Debug, Clone, Getters)]
pub struct EvaluatedEitherVariant {
	name: Identifier,
	subtype: Option<Pointer<Evaluated>>,
	tokens: EitherVariantTokens,
	object: Pointer<Object>,
}

impl Spanned for EitherVariant {
	fn span(&self) -> impl SpanLike {
		let start = self.name.span();

		if let Some(subtype) = &self.subtype {
			if let Some(comma) = &self.tokens.comma {
				return start.to(comma.span());
			}
			return start.to(subtype.span());
		}

		if let Some(comma) = &self.tokens.comma {
			return start.to(comma.span());
		}

		start.to(self.name.span())
	}
}

impl Dot for EvaluatedEither {
	fn dot(&self, _context: &Context, name: &str) -> Option<Pointer<Evaluated>> {
		self.variants
			.iter()
			.find_map(|variant| (variant.name.source_identifier() == name).then(|| variant.object.clone().into()))
	}
}
