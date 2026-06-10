use std::{borrow::Cow, fmt::Debug, hash::Hash};

use crate::{
	Span,
	Spanned,
	api::context::Context,
	comptime::{CompileTime, CompileTimeError, memory::ExpressionPointer},
	diagnostics::{Diagnostic, DiagnosticInfo},
	lexer::{Token, TokenType},
	parser::{TokenQueue, TokenQueueFunctionality as _, TryParse},
	scope::ScopeId,
	transpiler::{TranspileError, TranspileToC},
	typechecker::{Type, Typed},
};

#[derive(Clone, Eq)]
pub struct Identifier {
	/// The internal string value of this name. This is the value as it appears in the Cabin source code; In other words,
	/// it's unmangled.
	token: Token,

	/// Whether or not this name should be "mangled" when transpiling it to C.
	///
	/// When transpiling to C, all names are changed to a new "mangled" name to avoid conflicts with internal names and
	/// values inserted by the compiler.
	///
	/// For regular identifiers in the language, this is always `true`; But some special exceptions are made when the
	/// compiler needs to insert names into the program.
	should_mangle: bool,

	/// The unique ID of the scope that this name is used in. This is used to get the value
	/// that the name points to, because it needs to get the value from the scope it's used.
	scope_id: ScopeId,

	pub documentation: Option<String>,
}

impl TryParse for Identifier {
	type Output = Self;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> anyhow::Result<Self::Output, Diagnostic> {
		let token = tokens.pop(TokenType::Identifier, context)?;

		let identifier = Identifier {
			token,
			should_mangle: true,
			scope_id: context.scope.unique_id(),
			documentation: None,
		};

		if let Some(name_query) = context.name_query {
			if identifier.token.span.contains(name_query) {
				context.name_query_result = Some(identifier.clone());
			}
		}

		Ok(identifier)
	}
}

impl CompileTime for Identifier {
	type Output = Identifier;

	fn evaluate_at_compile_time(self, _context: &mut Context) -> Self::Output {
		self
	}
}

impl TranspileToC for Identifier {
	fn to_c(&self, _context: &mut Context, _output: Option<String>) -> Result<String, TranspileError> {
		Ok(self.mangled_name())
	}
}

impl Typed for Identifier {
	fn get_type(&self, context: &mut Context) -> Type {
		self.value(context).unwrap_or(ExpressionPointer::ERROR).get_type(context)
	}
}

impl AsRef<Identifier> for Identifier {
	fn as_ref(&self) -> &Identifier {
		self
	}
}

impl Spanned for Identifier {
	fn span(&self, _context: &Context) -> Span {
		self.token.span
	}
}

impl PartialEq for Identifier {
	fn eq(&self, other: &Self) -> bool {
		self.token.value == other.token.value
	}
}

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
	pub fn value(&self, context: &mut Context) -> Option<ExpressionPointer> {
		context
			.scope
			.get_variable_from_id(self.source_identifier(), self.scope_id)
			.ok_or_else(|| {
				context.add_diagnostic(Diagnostic {
					file: context.file.clone(),
					info: DiagnosticInfo::Error(crate::Error::CompileTime(CompileTimeError::UnknownVariable(self.source_identifier().to_owned()))),
					span: self.span(context),
				});
			})
			.ok()
	}

	pub fn create_virtual<'a, S: Into<Cow<'a, str>>>(name: S, context: &Context) -> Identifier {
		Identifier {
			token: Token::create_virtual(TokenType::Identifier, name),
			should_mangle: true,
			scope_id: context.scope.unique_id(),
			documentation: None,
		}
	}

	pub fn synthetic(token: Token, context: &Context) -> Identifier {
		Identifier {
			token,
			should_mangle: true,
			scope_id: context.scope.unique_id(),
			documentation: None,
		}
	}

	pub fn source_identifier(&self) -> &str {
		&self.token.value
	}

	pub fn mangled_name(&self) -> String {
		if self.should_mangle {
			format!("u_{}", self.token.value)
		} else {
			self.source_identifier().to_owned()
		}
	}
}
