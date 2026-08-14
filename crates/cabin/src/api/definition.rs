use derive_getters::Getters;
use try_as::{
	macros::{From, TryAsMut, TryAsRef},
	traits as try_as_traits,
	traits::{TryAsMut, TryAsRef},
};

use crate::{
	Context,
	ast::expressions::identifier::Identifier,
	comptime::{
		Evaluate,
		storage::{EvaluatedRef, ExpressionRef},
	},
};

#[derive(Debug, Clone, From, TryAsRef, TryAsMut)]
pub enum VariableDeclaration {
	Unevaluated(UnevaluatedVariableDeclaration),
	Evaluated(EvaluatedVariableDeclaration),
}

impl Evaluate for VariableDeclaration {
	type Output = EvaluatedVariableDeclaration;

	fn evaluate(self, context: &mut Context) -> Self::Output {
		match self {
			VariableDeclaration::Unevaluated(unevaluated) => unevaluated.evaluate(context),
			VariableDeclaration::Evaluated(evaluated) => evaluated,
		}
	}
}

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
pub struct UnevaluatedVariableDeclaration {
	name: Identifier,
	variable_type: Option<ExpressionRef>,
	value: Option<ExpressionRef>,
}

#[derive(Debug, Clone, Getters)]
pub struct EvaluatedVariableDeclaration {
	name: Identifier,
	variable_type: Option<EvaluatedRef>,
	pub(crate) value: Option<EvaluatedRef>,
}

impl Evaluate for UnevaluatedVariableDeclaration {
	type Output = EvaluatedVariableDeclaration;

	fn evaluate(self, context: &mut Context) -> Self::Output {
		EvaluatedVariableDeclaration {
			name: self.name,
			variable_type: self.variable_type.evaluate(context),
			value: self.value.evaluate(context),
		}
	}
}

impl UnevaluatedVariableDeclaration {
	pub fn with_value_and_type(name: Identifier, expression: ExpressionRef, variable_type: ExpressionRef) -> UnevaluatedVariableDeclaration {
		UnevaluatedVariableDeclaration {
			name,
			value: Some(expression),
			variable_type: Some(variable_type),
		}
	}

	pub fn with_value(name: Identifier, expression: ExpressionRef) -> UnevaluatedVariableDeclaration {
		UnevaluatedVariableDeclaration {
			name,
			value: Some(expression),
			variable_type: None,
		}
	}

	pub fn with_type(name: Identifier, expression: ExpressionRef) -> UnevaluatedVariableDeclaration {
		UnevaluatedVariableDeclaration {
			name,
			variable_type: Some(expression),
			value: None,
		}
	}
}
