use std::{collections::HashMap, fmt::Debug};

use super::CompileTimeError;
use crate::{
	api::context::Context,
	ast::expressions::{
		new_literal::{EvaluatedLiteral, Literal, LiteralMut},
		Expression,
		ExpressionOrPointer,
	},
	comptime::CompileTime,
	diagnostics::Diagnostic,
	transpiler::{TranspileError, TranspileToC},
	typechecker::{Type, Typed},
	Span,
	Spanned,
};

/// A pointer to a `LiteralObject` in `VirtualMemory`.
///
/// `VirtualPointers` are hygienic; You can only get a pointer by storing something in `VirtualMemory` and storing the
/// address it gives back to you; And you can't get the internal numeric address for a `VirtualPointer`. This means that
/// all `VirtualPointer` instances *always point to a valid location in `VirtualMemory` that has a `LiteralObject` in it*.
/// This also requires that it's impossible to remove objects from `VirtualMemory`.
///
/// That being said, note that `VirtualPointers` aren't type-safe; As in, they do not hold data about the *type* of
/// `LiteralObject` that they point to. You can check the type of a `LiteralObject` in a number of ways, such as using
/// the `type_name` or `object_type` fields, or pattern matching on the result of `LiteralConvertible::from_literal()`.
///
/// See the documentation on `LiteralObject` for more information about `VirtualPointers`, `LiteralObjects`, how they interact,
/// and when to use which. Also see the documentation for `VirtualMemory` for more information about virtual memory.
///
/// This internally just wraps a `usize`, so cloning and copying is incredibly cheap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExpressionPointer(usize);

impl ExpressionPointer {
	pub(crate) const ERROR: ExpressionPointer = ExpressionPointer(0);

	/// Retrieves the `LiteralObject` value that this pointer points to.
	///
	/// This is theoretically infallible and will always yield a valid `LiteralObject`; Read the documentation on `VirtualPointers`
	/// about pointer hygiene for more information about edge cases involving invalid pointers. If in the unlikely event that the
	/// given pointer is invalid, the program will `panic!`.
	///
	/// This is equivalent to calling `virtual_memory.get(pointer)`.
	///
	/// # Returns
	///
	/// A reference to the `LiteralObject` that this `VirtualPointer` points to.
	pub fn expression<'a>(&self, context: &'a Context) -> &'a Expression {
		context.virtual_memory.get(self)
	}

	pub(crate) fn expression_mut<'a>(&self, context: &'a mut Context) -> &'a mut Expression {
		context.virtual_memory.memory.get_mut(&self.0).unwrap()
	}

	pub(crate) fn is_literal(&self, context: &mut Context) -> bool {
		match self.expression(context).to_owned() {
			Expression::EvaluatedLiteral(_) => true,
			Expression::Name(name) => name.value(context).is_some_and(|value| value.is_literal(context)),
			_ => false,
		}
	}

	pub(crate) fn is_error(&self) -> bool {
		self == &ExpressionPointer::ERROR
	}

	pub(crate) fn evaluate_to_literal(self, context: &mut Context) -> LiteralPointer {
		self.evaluate_at_compile_time(context).as_literal(context)
	}

	pub(crate) fn try_as_literal(self, context: &mut Context) -> Result<LiteralPointer, ()> {
		match self.expression(context).to_owned() {
			Expression::EvaluatedLiteral(_) | Expression::Literal(_) => Ok(LiteralPointer(self)),
			Expression::Name(name) => name.value(context).unwrap_or(ExpressionPointer::ERROR).try_as_literal(context),
			_expr => {
				dbg!(_expr);
				Err(())
			},
		}
	}

	pub(crate) fn as_literal(self, context: &mut Context) -> LiteralPointer {
		self.try_as_literal(context).unwrap_or_else(|_| {
			context.add_diagnostic(Diagnostic {
				file: context.file.clone(),
				span: self.span(context),
				info: CompileTimeError::ExpressionUsedAsType.into(),
			});
			LiteralPointer::ERROR
		})
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LiteralPointer(ExpressionPointer);

impl LiteralPointer {
	pub const ERROR: LiteralPointer = LiteralPointer(ExpressionPointer::ERROR);

	pub fn get_literal<'ctx>(&self, context: &'ctx Context) -> Literal<'ctx> {
		match self.0.expression(context) {
			Expression::EvaluatedLiteral(literal) => Literal::Evaluated(literal),
			Expression::Literal(literal) => Literal::Unevaluated(literal),
			_ => unreachable!(),
		}
	}

	/// If this `LiteralPointer` points to an `EvaluatedLiteral`, a reference to that
	/// `EvaluatedLiteral` is returned.
	///
	/// Otherwise, when this `LiteralPointer` points to an `UnevaluatedLiteral`, the literal it
	/// points to is evaluated, and the resulting `EvaluatedLiteral` replaces the old `UnevaluatedLiteral`
	/// in (virtual) memory. Thus, this pointer, and all other pointers to that value, now point to the
	/// new `EvaluatedLiteral`. A reference to new `EvaluatedLiteral` is returned.
	///
	/// # Parameters
	///
	/// - `context` - Global state data about the compiler, including it's `VirtualMemory`.
	///
	/// # Returns
	///
	/// A reference to the (now) evaluated literal that this pointer points to.
	pub fn evaluated_literal<'ctx>(&self, context: &'ctx mut Context) -> &'ctx EvaluatedLiteral {
		if matches!(self.get_literal(context), Literal::Evaluated(_)) {
			let Literal::Evaluated(evaluated)  = self.get_literal(context) else { unreachable!() };
			return evaluated;
		}

		let Literal::Unevaluated(unevaluated) = self.get_literal(context) else { unreachable!() };
		let evaluated = unevaluated.clone().evaluate_at_compile_time(context);
		let _ = context.virtual_memory.memory.insert(self.0 .0, Expression::EvaluatedLiteral(evaluated));
		self.evaluated_literal(context)
	}

	pub fn literal_mut<'ctx>(&self, context: &'ctx mut Context) -> LiteralMut<'ctx> {
		match self.0.expression_mut(context) {
			Expression::EvaluatedLiteral(literal) => LiteralMut::Evaluated(literal),
			Expression::Literal(literal) => LiteralMut::Unevaluated(literal),
			_ => unreachable!(),
		}
	}
}

impl From<LiteralPointer> for ExpressionPointer {
	fn from(value: LiteralPointer) -> Self {
		value.0
	}
}

impl CompileTime for ExpressionPointer {
	type Output = ExpressionPointer;

	fn evaluate_at_compile_time(self, context: &mut Context) -> Self::Output {
		let evaluated = context.virtual_memory.get(&self).clone().evaluate_at_compile_time(context);
		match evaluated {
			ExpressionOrPointer::Expression(expression) => {
				let _ = context.virtual_memory.memory.insert(self.0, expression);
				self
			},
			ExpressionOrPointer::Pointer(pointer) => pointer,
		}
	}
}

impl Typed for ExpressionPointer {
	fn get_type(&self, context: &mut Context) -> Type {
		self.expression(context).clone().get_type(context)
	}
}

impl TranspileToC for ExpressionPointer {
	fn to_c(&self, _context: &mut Context, output: Option<String>) -> Result<String, TranspileError> {
		Ok(format!("{}&literal_{}", output.map(|name| format!("{name} = ")).unwrap_or_default(), self.0))
	}

	fn c_prelude(&self, context: &mut Context) -> Result<String, TranspileError> {
		self.expression(context).to_owned().c_prelude(context)
	}

	fn c_type_prelude(&self, context: &mut Context) -> Result<String, TranspileError> {
		self.expression(context).to_owned().c_type_prelude(context)
	}
}

/// Technically this can be used to get the internal numeric value a la:
///
/// `let value: usize = format!("{pointer}").parse().unwrap();`
///
/// ...but hey, not much we can do about it. It's pretty hacky anyway so it should be a pretty glaring sign
/// that it's not really meant to be used that way. If nothing else it's a backdoor for some obscure situation
/// where the numeric value is needed.
impl std::fmt::Display for ExpressionPointer {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}

impl Spanned for ExpressionPointer {
	fn span(&self, context: &Context) -> Span {
		self.expression(context).span(context)
	}
}

/// A virtual memory, which holds `LiteralObjects`. This is a singleton struct that exists on the compiler's context as
/// `context.virtual_memory`.
///
/// Virtual memory is where literals are stored, such as literal strings or numbers, as well as any other objects that are
/// fully known at compile-time, such as groups, functions, etc. Read the documentation on `LiteralObject` for more information.
///
/// Values stored in virtual memory can be accessed via `VirtualPointers`, which are retrieved when storing an object in virtual
/// memory via `virtual_memory.store()`.
pub struct VirtualMemory {
	/// The internal memory storage as a simple `HashMap` between `usize` (pointers/address) and `LiteralObject` values.
	memory: HashMap<usize, Expression>,
}

impl VirtualMemory {
	/// Creates an empty virtual memory with no entries. This should be called once at the beginning of compilation, when the compiler's
	/// `context` is created.
	///
	/// # Returns
	///
	/// The created empty virtual memory.
	pub fn empty() -> VirtualMemory {
		VirtualMemory {
			memory: HashMap::from([(0, Expression::EvaluatedLiteral(EvaluatedLiteral::ErrorLiteral(Span::unknown())))]),
		}
	}

	/// Stores a value in virtual memory. This takes ownership of the value, and the value will live for as long as virtual memory,
	/// except in special cases such as when memory is overwritten via things like `move_overwrite()`.
	///
	/// A pointer to the location in memory where the object is stored is returned, and a reference to the object can be retrieved
	/// from the pointer using either `virtual_memory.get(pointer)` or `pointer.virtual_deref()`.
	///
	/// The `LiteralObject` stored will have it's `address` field appropriately updated.
	///
	/// # Parameters
	///
	/// - `value` - The `LiteralObject` to store in virtual memory.
	///
	/// # Returns
	///
	/// A `VirtualPointer` that points to the object that was stored.
	pub(crate) fn store(&mut self, value: Expression) -> ExpressionPointer {
		let address = self.next_unused_virtual_address();
		let _ = self.memory.insert(address, value);
		ExpressionPointer(address)
	}

	/// Returns an immutable reference to a `LiteralObject` stored in virtual memory. This is equivalent to calling `.virtual_deref()`
	/// on a `VirtualPointer`.
	///
	/// This is theoretically infallible and will always yield a valid `LiteralObject`; Read the documentation on `VirtualPointers`
	/// about pointer hygiene for more information about edge cases involving invalid pointers. If in the unlikely event that the
	/// given pointer is invalid, the program will `panic!`.
	///
	/// # Parameters
	///
	/// - `address` - A `VirtualPointer` to the location to get the `LiteralObject` from in virtual memory.
	pub(crate) fn get(&self, address: &ExpressionPointer) -> &Expression {
		self.memory.get(&address.0).unwrap()
	}

	/// Returns the first unused virtual address. When storing an object in memory, this is used to determine what address to give it.
	/// This also safeguards against removals and reusals; i.e., it is currently impossible to remove values from memory, but if that
	/// were to be implemented, this would still return the very first unused address, even if a previous value had lived there.
	///
	/// # Returns
	///
	/// The first address in virtual memory that doesn't point to an object.
	fn next_unused_virtual_address(&self) -> usize {
		let mut next_unused_virtual_address = 1;
		while self.memory.contains_key(&next_unused_virtual_address) {
			next_unused_virtual_address += 1;
		}
		next_unused_virtual_address
	}
}
