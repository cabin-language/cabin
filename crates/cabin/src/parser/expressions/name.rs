use std::{fmt::Debug, hash::Hash};

use colored::Colorize as _;

use super::Spanned;
use crate::{
	api::context::context,
	comptime::CompileTime,
	debug_log,
	debug_start,
	lexer::{Span, TokenType},
	mapped_err,
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

	fn try_parse(tokens: &mut TokenQueue) -> anyhow::Result<Self::Output, crate::Diagnostic> {
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

	fn evaluate_at_compile_time(self) -> anyhow::Result<Self::Output> {
		let debug_section = debug_start!("{} the name {}", "Compile-Time Evaluating".bold().green(), self.unmangled_name().red());
		let value = context()
			.scope_data
			.get_variable(self.clone())
			.ok_or_else(|| anyhow::anyhow!("No variable found with the name {}", self.unmangled_name().bold().cyan()))
			.map_err(mapped_err! {
				while = format!("attempting to get the original value of the name \"{}\" to evaluate it at compile-time", self.unmangled_name().bold().cyan()),
				position = self.span(),
				details = unindent::unindent(&format!(
					"
					Here you reference a variable called \"{name}\", but no variable called \"{name}\" exists at this
					part of the program. If this is a typo and you don't expect a variable with this name to exist, you
					may be trying to refer to one of these variables, which are the ones with the closest names that are
					present here:

					{closest}
					", 
					name = self.unmangled_name().bold().red(),
					closest = context()
						.scope_data
						.get_closest_variables(&self, 3)
						.iter()
						.map(|(name, _)| format!("    - {}", name.unmangled_name().bold().green()))
						.collect::<Vec<_>>()
						.join("\n")
						.trim_start()
				)),
			})?;

		debug_log!("Name {} evaluated to a {}", self.unmangled_name().red(), value.kind_name().cyan());

		debug_section.finish();
		Ok(value.try_clone_pointer().unwrap_or(Expression::Name(self)))
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
