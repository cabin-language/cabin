use std::{fmt::Debug, hash::Hash};

use crate::{
	api::context::Context,
	ast::expressions::Expression,
	comptime::{memory::ExpressionPointer, CompileTime, CompileTimeError},
	diagnostics::{Diagnostic, DiagnosticInfo},
	lexer::TokenType,
	parser::{TokenQueue, TokenQueueFunctionality as _, TryParse},
	scope::ScopeId,
	transpiler::{TranspileError, TranspileToC},
	Span,
	Spanned,
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

	scope_id: ScopeId,
}

impl Name {
	pub(crate) fn value(&self, context: &mut Context) -> Option<ExpressionPointer> {
		context
			.scope_tree
			.get_variable_from_id(self, self.scope_id)
			.ok_or_else(|| {
				context.add_diagnostic(Diagnostic {
					info: DiagnosticInfo::Error(crate::Error::CompileTime(CompileTimeError::UnknownVariable(self.unmangled_name().to_owned()))),
					span: self.span,
				});
			})
			.ok()
	}
}

impl TryParse for Name {
	type Output = Self;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> anyhow::Result<Self::Output, Diagnostic> {
		let token = tokens.pop(TokenType::Identifier)?;

		Ok(Name {
			name: token.value,
			span: token.span,
			should_mangle: true,
			scope_id: context.scope_tree.unique_id(),
		})
	}
}

impl CompileTime for Name {
	type Output = ExpressionPointer;

	fn evaluate_at_compile_time(self, context: &mut Context) -> Self::Output {
		context.scope_tree.get_variable_from_id(self.clone(), self.scope_id).unwrap_or_else(|| {
			context.add_diagnostic(Diagnostic {
				span: self.span(context),
				info: DiagnosticInfo::Error(crate::Error::CompileTime(CompileTimeError::UnknownVariable(self.unmangled_name().to_owned()))),
			});
			return Expression::error(self.span(context), context);
		})
	}
}

impl TranspileToC for Name {
	fn to_c(&self, _context: &mut Context, _output: Option<String>) -> Result<String, TranspileError> {
		Ok(self.mangled_name())
	}
}

impl<T: AsRef<str>> From<T> for Name {
	fn from(value: T) -> Self {
		Name {
			name: value.as_ref().to_owned(),
			span: Span::unknown(),
			scope_id: ScopeId::global(),
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
	fn span(&self, _context: &Context) -> Span {
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
		write!(f, "{}", self.unmangled_name())
	}
}

impl Name {
	pub(crate) fn hardcoded<T: AsRef<str>>(name: T) -> Name {
		Name {
			name: name.as_ref().to_owned(),
			span: Span::unknown(),
			scope_id: ScopeId::global(),
			should_mangle: true,
		}
	}

	pub(crate) fn unmangled_name(&self) -> &str {
		&self.name
	}

	pub(crate) fn mangled_name(&self) -> String {
		if self.should_mangle {
			format!("u_{}", self.name)
		} else {
			self.unmangled_name().to_owned()
		}
	}
}
