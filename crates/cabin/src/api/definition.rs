use derive_getters::Getters;

use crate::{
	Context,
	ast::expressions::identifier::Identifier,
	comptime::{Evaluate, storage::Pointer, types::Type},
};

/// A variable declaration before being evaluated at compile-time.
///
/// Some variable declarations have an explicit type, such as action parameters. These
/// will have `Some()` for `variable_type`.
///
/// Some variable declarations have an initial value, such as `let` declarations.
/// These will have `Some()` for `value`.
///
/// Some variables won't immediately have either, such as a `for` loop binding. In this
/// case I will kill myself.
#[derive(Debug, Clone, Getters)]
pub struct VariableDeclaration {
	name: Identifier,
	variable_type: Option<Type>,
	pub value: Pointer,
}

impl Evaluate for VariableDeclaration {
	type Output = VariableDeclaration;

	fn evaluate(self, context: &mut Context) -> Self::Output {
		VariableDeclaration {
			name: self.name,
			variable_type: self.variable_type.map(|variable_type| Type::Evaluated(variable_type.evaluate(context))),
			value: self.value.evaluate(context),
		}
	}
}

impl VariableDeclaration {
	pub const fn with_value_and_type(name: Identifier, expression: Pointer, variable_type: Type) -> VariableDeclaration {
		VariableDeclaration {
			name,
			value: expression,
			variable_type: Some(variable_type),
		}
	}

	pub const fn with_value(name: Identifier, expression: Pointer) -> VariableDeclaration {
		VariableDeclaration {
			name,
			value: expression,
			variable_type: None,
		}
	}

	pub fn empty(name: Identifier, context: &mut Context) -> VariableDeclaration {
		VariableDeclaration {
			name,
			value: context.storage.calloc(),
			variable_type: None,
		}
	}

	pub fn with_type(name: Identifier, expression: Type, context: &mut Context) -> VariableDeclaration {
		VariableDeclaration {
			name,
			variable_type: Some(expression),
			value: context.storage.calloc(),
		}
	}
}
