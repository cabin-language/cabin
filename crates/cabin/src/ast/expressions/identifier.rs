use std::{borrow::Cow, fmt::Debug, hash::Hash};

use crate::{
	Span,
	Spanned,
	api::context::Context,
	ast::expressions::{evaluated::EvaluatedExpression, object::Object},
	comptime::{Evaluate, memory::Pointer},
	diagnostics::{Diagnostic, DiagnosticInfo},
	lexer::{Token, TokenType},
	log,
	parser::{TokenQueue, TokenQueueFunctionality as _, TryParse},
	scope::ScopeRef,
	transpiler::ToCabin,
};

#[derive(Clone)]
pub struct Identifier {
	/// The internal string value of this name. This is the value as it appears in the Cabin source code; In other words,
	/// it's unmangled.
	token: Token,

	/// The unique ID of the scope that this name is used in. This is used to get the value
	/// that the name points to, because it needs to get the value from the scope it's used.
	scope_id: ScopeRef,
}

impl TryParse for Identifier {
	type Output = Identifier;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> anyhow::Result<Self::Output, Diagnostic> {
		let token = tokens.pop(TokenType::Identifier, context)?;

		let identifier = Identifier {
			token,
			scope_id: context.scope.unique_id(),
		};

		Ok(identifier)
	}
}

impl Evaluate for Identifier {
	type Output = Pointer;

	fn evaluate(self, context: &mut Context) -> Self::Output {
		log!(context, "Evaluating identifier");
		self.value(context).unwrap_or_else(|| EvaluatedExpression::Identifier(self).store(context))
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
	fn span(&self) -> Span {
		self.token.span
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
	pub fn value(&self, context: &mut Context) -> Option<Pointer> {
		context
			.scope
			.get_variable_from_id(self.source_identifier(), self.scope_id)
			.ok_or_else(|| {
				context.add_diagnostic(Diagnostic {
					file: context.file.clone(),
					info: DiagnosticInfo::UnknownVariable(self.source_identifier().to_owned()),
					span: self.span(),
				});
			})
			.ok()
	}

	pub fn stringify(&self, context: &Context) -> Object {
		Object::text(self.source_identifier(), context, self.span())
	}

	pub fn create_virtual<'str, S: Into<Cow<'str, str>>>(name: S, context: &Context) -> Identifier {
		Identifier {
			token: Token::create_virtual(TokenType::Identifier, name),
			scope_id: context.scope.unique_id(),
		}
	}

	pub const fn synthetic(token: Token, context: &Context) -> Identifier {
		Identifier {
			token,
			scope_id: context.scope.unique_id(),
		}
	}

	pub fn source_identifier(&self) -> &str {
		&self.token.value
	}

	pub fn mangled_name(&self) -> String {
		format!("u_{}", self.token.value)
	}
}
