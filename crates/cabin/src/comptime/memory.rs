use std::{collections::HashMap, fmt::Debug};

use crate::{
	api::{context::Context, scope::ScopeId, traits::TryAs as _},
	comptime::CompileTime,
	lexer::Span,
	parser::{
		expressions::{field_access::FieldAccessType, literal::LiteralObject, Expression, Spanned, Typed},
		statements::tag::TagList,
	},
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VirtualPointer(usize);

impl VirtualPointer {
	pub const ERROR: VirtualPointer = VirtualPointer(0);

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
	pub fn virtual_deref<'a>(&self, context: &'a Context) -> &'a LiteralObject {
		context.virtual_memory.get(self)
	}

	pub fn virtual_deref_mut<'a>(&self, context: &'a mut Context) -> &'a mut LiteralObject {
		context.virtual_memory.memory.get_mut(&self.0).unwrap()
	}

	pub fn is_list(&self, context: &Context) -> bool {
		self.virtual_deref(context).try_as::<Vec<Expression>>().is_ok()
	}
}

impl CompileTime for VirtualPointer {
	type Output = VirtualPointer;

	fn evaluate_at_compile_time(self, context: &mut Context) -> Self::Output {
		let evaluated = self.virtual_deref(context).clone().evaluate_at_compile_time(context);
		let _ = context.virtual_memory.memory.insert(self.0, evaluated);
		self
	}
}

/// Technically this can be used to get the internal numeric value a la:
///
/// `let value: usize = format!("{pointer}").parse().unwrap();`
///
/// ...but hey, not much we can do about it. It's pretty hacky anyway so it should be a pretty glaring sign
/// that it's not really meant to be used that way. If nothing else it's a backdoor for some obscure situation
/// where the numeric value is needed.
impl std::fmt::Display for VirtualPointer {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}

impl Typed for VirtualPointer {
	fn get_type(&self, context: &mut Context) -> anyhow::Result<VirtualPointer> {
		self.virtual_deref(context).clone().get_type(context)
	}
}

impl Spanned for VirtualPointer {
	fn span(&self, context: &Context) -> Span {
		self.virtual_deref(context).span(context)
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
	memory: HashMap<usize, LiteralObject>,
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
			memory: HashMap::from([(0, LiteralObject {
				type_name: "Error".into(),
				fields: HashMap::new(),
				internal_fields: HashMap::new(),
				field_access_type: FieldAccessType::Normal,
				outer_scope_id: ScopeId::global(),
				inner_scope_id: None,
				name: "error".into(),
				address: Some(VirtualPointer::ERROR),
				span: Span::unknown(),
				tags: TagList::default(),
			})]),
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
	pub fn store(&mut self, mut value: LiteralObject) -> VirtualPointer {
		let address = self.next_unused_virtual_address();
		value.address = Some(VirtualPointer(address));
		let _ = self.memory.insert(address, value);
		VirtualPointer(address)
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
	pub fn get(&self, address: &VirtualPointer) -> &LiteralObject {
		self.memory.get(&address.0).unwrap()
	}

	pub fn replace(&mut self, address: VirtualPointer, value: LiteralObject) {
		let _ = self.memory.insert(address.0, value);
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

	/// Moves the value at the first given location to the address at the second given location, overwriting and deleting any previous
	/// value that was stored there.
	///
	/// This is used by `Declaration::evaluate_at_compile_time()` to handle a special case involving `Groups` to fix the compiler
	/// from crashing in certain situations; Read the comments in that function for more information.
	///
	/// The `LiteralObject` that was moved will have it's `address` field appropriately updated.
	///
	/// # Parameters
	///
	/// - `location_of_value_to_move` - A pointer to the value that should be moved in memory.
	/// - `destination_to_overwrite` - A pointer to the destination slot in memory, in which the moved value will now reside and
	/// the previous value at this destination will be deleted.
	pub fn move_overwrite(&mut self, location_of_value_to_move: VirtualPointer, destination_to_overwrite: VirtualPointer) {
		let mut value = self.memory.remove(&location_of_value_to_move.0).unwrap();
		value.address = Some(destination_to_overwrite);
		let _ = self.memory.insert(destination_to_overwrite.0, value);
	}

	/// Returns a `Vec` of the entries of all objects stored in virtual memory. The returned `Vec` contains tuples of owned
	/// pointers and owned `LiteralObjects`, which are clones of the `LiteralObjects` that exist in memory. This doesn't return
	/// references to the original `LiteralObjects` because each one would borrow the compilers `context`, and then it would be
	/// impossible to make further mutable borrows of it.
	///
	/// This is used by the transpiler to transpile all literals in virtual memory into C code. This really shouldn't be needed
	/// or used at any step of compilation other than transpilation.
	///
	/// # Returns
	///
	/// An owned `Vec` containing tuples of owned `VirtualPointers` and `LiteralObjects` representing every object stored in this
	/// virtual memory.
	pub fn entries(&self) -> Vec<(VirtualPointer, &LiteralObject)> {
		self.memory
			.iter()
			.collect::<Vec<_>>()
			.into_iter()
			.map(|(address, object)| (VirtualPointer(*address), object))
			.collect()
	}
}
