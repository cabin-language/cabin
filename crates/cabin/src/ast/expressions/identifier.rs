use std::{fmt::Debug, hash::Hash};

use derive_getters::Getters;

use crate::{
	SpanLike,
	Spanned,
	api::context::Context,
	ast::expressions::{evaluated::EvaluatedExpression, object::Object},
	comptime::{
		Evaluate,
		storage::{Evaluated, Pointer},
	},
	diagnostics::{Diagnostic, DiagnosticInfo},
	lexer::{Token, TokenType},
	log,
	parser::{TokenQueue, TokenQueueFunctionality as _, TryParse},
	scope::ScopeRef,
	transpiler::ToCabin,
};

#[derive(Clone, Getters)]
pub struct Identifier {
	/// The internal string value of this name. This is the value as it appears in the Cabin source code; In other words,
	/// it's unmangled.
	token: Token,

	/// The unique ID of the scope that this name is used in. This is used to get the value
	/// that the name points to, because it needs to get the value from the scope it's used.
	#[getter(copy)]
	scope: ScopeRef,
}

impl TryParse for Identifier {
	type Output = Identifier;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> anyhow::Result<Self::Output, Diagnostic> {
		let token = tokens.pop(TokenType::Identifier, context)?;

		let identifier = Identifier {
			token,
			scope: context.scope.get_ref(),
		};

		Ok(identifier)
	}
}

impl Evaluate for Identifier {
	type Output = Pointer<Evaluated>;

	fn evaluate(self, context: &mut Context) -> Self::Output {
		log!(context, "Evaluating identifier \"{}\"", self.source_identifier());
		self.value(context).unwrap_or_else(|| EvaluatedExpression::Identifier(self).store(context).into())
	}
}

impl ToCabin for Identifier {
	fn to_cabin(&self) -> String {
		self.token.value.clone()
	}
}

impl AsRef<Identifier> for Identifier {
	fn as_ref(&self) -> &Identifier {
		self
	}
}

impl Spanned for Identifier {
	fn span(&self) -> impl SpanLike {
		self.token().span()
	}
}

impl PartialEq for Identifier {
	fn eq(&self, other: &Self) -> bool {
		self.token.value == other.token.value
	}
}

impl Eq for Identifier {}

impl Hash for Identifier {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.token.value.hash(state);
	}
}

impl From<&Identifier> for Identifier {
	fn from(val: &Identifier) -> Self {
		val.clone()
	}
}

impl Debug for Identifier {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.source_identifier())
	}
}

impl Identifier {
	pub fn value(&self, context: &mut Context) -> Option<Pointer<Evaluated>> {
		context
			.get_variable(self)
			.ok_or_else(|| {
				context.add_diagnostic_at(self, DiagnosticInfo::UnknownVariable(self.source_identifier().to_owned()));
			})
			.ok()
			.map(|ptr| ptr.with_span(self.span().into_owned()))
	}

	pub fn stringify(&self, context: &mut Context) -> Object {
		Object::text(self.source_identifier(), context, self.span().into_owned())
	}

	pub const fn create_virtual(name: String, context: &Context) -> Identifier {
		Identifier {
			token: Token::create_virtual(TokenType::Identifier, name),
			scope: context.scope.get_ref(),
		}
	}

	pub const fn create_virtual_in_scope(name: String, scope: ScopeRef) -> Identifier {
		Identifier {
			token: Token::create_virtual(TokenType::Identifier, name),
			scope,
		}
	}

	pub const fn synthetic(token: Token, context: &Context) -> Identifier {
		Identifier {
			token,
			scope: context.scope.get_ref(),
		}
	}

	pub fn source_identifier(&self) -> &str {
		&self.token.value
	}

	pub fn mangled_name(&self) -> String {
		format!("u_{}", self.token.value)
	}
}
