use std::{fmt::Debug, hash::Hash};

use colored::Colorize as _;

use super::Spanned;
use crate::{
	api::context::context,
	comptime::{CompileTime, CompileTimeError},
	diagnostics::{Diagnostic, DiagnosticInfo},
	lexer::{Span, TokenType},
	parser::{expressions::Expression, ToCabin, TokenQueue, TokenQueueFunctionality as _, TryParse},
	transpiler::TranspileToC,
};

#[derive(Clone, Eq)]
pub struct Name {
	/// The internal string value of this name. This is the value as it appears in the Cabin source code; In other words,
	/// it's unmangled.
	name: String,

	/// The span of this name. See `Spanned::span()` for more information.
	span: Span,

	/// Whether or not this name should be "mangled" when transpiling it to C.
	///
	/// When transpiling to C, all names are changed to a new "mangled" name to avoid conflicts with internal names and
	/// values inserted by the compiler.
	///
	/// For regular identifiers in the language, this is always `true`; But some special exceptions are made when the
	/// compiler needs to insert names into the program.
	should_mangle: bool,
}

impl TryParse for Name {
	type Output = Self;

	fn try_parse(tokens: &mut TokenQueue) -> anyhow::Result<Self::Output, Diagnostic> {
		let token = tokens.pop(TokenType::Identifier)?;

		Ok(Name {
			name: token.value,
			span: token.span,
			should_mangle: true,
		})
	}
}

impl CompileTime for Name {
	type Output = Expression;

	fn evaluate_at_compile_time(self) -> Self::Output {
		let error = Expression::ErrorExpression(Span::unknown());
		let value = context().scope_data.get_variable(self.clone()).unwrap_or_else(|| {
			context().add_diagnostic(Diagnostic {
				span: self.span(),
				info: DiagnosticInfo::Error(crate::Error::CompileTime(CompileTimeError::UnknownVariable(self.unmangled_name().to_owned()))),
			});
			&error
		});

		if matches!(value, Expression::ErrorExpression(_)) {
			return value.clone();
		}

		value.try_clone_pointer().unwrap_or(Expression::Name(self))
	}
}

impl TranspileToC for Name {
	fn to_c(&self) -> anyhow::Result<String> {
		Ok(self.mangled_name())
	}
}

impl ToCabin for Name {
	fn to_cabin(&self) -> String {
		self.unmangled_name().to_owned()
	}
}

impl<T: AsRef<str>> From<T> for Name {
	fn from(value: T) -> Self {
		Name {
			name: value.as_ref().to_owned(),
			span: Span::unknown(),
			should_mangle: true,
		}
	}
}

impl AsRef<Name> for Name {
	fn as_ref(&self) -> &Name {
		self
	}
}

impl Spanned for Name {
	fn span(&self) -> Span {
		self.span
	}
}

impl PartialEq for Name {
	fn eq(&self, other: &Self) -> bool {
		self.name == other.name
	}
}

impl Hash for Name {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.name.hash(state);
	}
}

impl From<&Name> for Name {
	fn from(val: &Name) -> Self {
		val.clone()
	}
}

impl Debug for Name {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.unmangled_name().red())
	}
}

impl Name {
	pub fn non_mangled<T: AsRef<str>>(name: T) -> Name {
		Name {
			name: name.as_ref().to_owned(),
			span: Span::unknown(),
			should_mangle: false,
		}
	}

	pub fn unmangled_name(&self) -> &str {
		&self.name
	}

	pub fn mangled_name(&self) -> String {
		if self.should_mangle {
			format!("u_{}", self.name)
		} else {
			self.unmangled_name().to_owned()
		}
	}
}
